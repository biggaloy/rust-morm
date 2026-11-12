//! Spike runtime for step 02. Each module is the smallest form of one design
//! the proofs need: just enough to compile, run and measure it. Throwaway code.

pub mod groups;
pub mod hooks;
pub mod loops;
pub mod nested;
pub mod param;
pub mod pool;
pub mod tx;

pub use groups::{Bits, GlobalGroup, GlobalGroups, Grouped, Mask, MaskBits, Req, ScopedGroup};
pub use hooks::{Changes, Committed, Hook, Hooks, Overrides, Target, delete, insert, update};
pub use morm_macros::{base, groups, hooks, model};
pub use param::{Param, SqlType};
pub use pool::{Conn, Db, Lazy, Manager};
pub use tokio_postgres::Row;
pub use tx::{Isolation, Once, Retry, RetryBody, Tx, TxOptions};

use std::marker::PhantomData;

#[derive(Debug)]
pub enum Error {
    Postgres(tokio_postgres::Error),
    NestedConnection,
    Pool(String),
    Validation(&'static str),
}

impl From<tokio_postgres::Error> for Error {
    fn from(e: tokio_postgres::Error) -> Self {
        Error::Postgres(e)
    }
}

impl Error {
    /// The SQLSTATE, for a database error.
    pub fn code(&self) -> Option<&str> {
        match self {
            Error::Postgres(e) => e.code().map(|c| c.code()),
            _ => None,
        }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Implemented by `#[model]` for a model and for each of its views.
pub trait Model: Sized + Send + Sync + 'static {
    type Global: MaskBits;
    type Scoped: MaskBits;
    const TABLE: &'static str;
    const COLUMNS: &'static [&'static str];
    const SELECT: &'static str;
    /// The group each column needs, in `COLUMNS` order.
    const GROUPS: &'static [Req];
    /// `INSERT` of every column, `$1` onwards in `COLUMNS` order.
    const INSERT: &'static str;
    fn from_row(row: &Row) -> Result<Self>;
    /// Each field as a parameter, in `COLUMNS` order.
    fn params(&self) -> Vec<&dyn Param>;
}

/// `Model::F.<field>`. `C` is the field's own marker type, so each column is a
/// distinct type and can carry its own `UniqueKey` impl.
pub struct Column<M, T, C> {
    name: &'static str,
    _p: PhantomData<fn() -> (M, T, C)>,
}

impl<M, T, C> Column<M, T, C> {
    #[doc(hidden)]
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            _p: PhantomData,
        }
    }
    pub const fn name(&self) -> &'static str {
        self.name
    }
}

impl<M, T, C> Clone for Column<M, T, C> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<M, T, C> Copy for Column<M, T, C> {}

/// `Model::UK.<name>`: a unique key that isn't one plain column.
pub struct Key<M, K>(PhantomData<fn() -> (M, K)>);

impl<M, K> Key<M, K> {
    #[doc(hidden)]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

/// A conflict target for upsert.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a unique key of `{M}`",
    label = "no unique key covers this",
    note = "upsert needs a conflict target with a unique key: mark the field `#[col(unique)]`, or declare `#[unique(..)]` on the model and pass `{M}::UK.<name>`"
)]
pub trait UniqueKey<M> {
    const CONFLICT: &'static str;
}

/// The upsert statement for `M`, conflicting on `key`.
pub fn upsert_sql<M: Model, K: UniqueKey<M>>(_key: K, update: &[&str]) -> String {
    let cols: Vec<String> = M::COLUMNS.iter().map(|c| format!("\"{c}\"")).collect();
    let vals: Vec<String> = (1..=cols.len()).map(|i| format!("${i}")).collect();
    let set: Vec<String> = update
        .iter()
        .map(|c| format!("\"{c}\" = EXCLUDED.\"{c}\""))
        .collect();
    format!(
        "INSERT INTO \"{}\" ({}) VALUES ({}) ON CONFLICT {} DO UPDATE SET {}",
        M::TABLE,
        cols.join(", "),
        vals.join(", "),
        K::CONFLICT,
        set.join(", ")
    )
}

#[doc(hidden)]
pub mod __private {
    pub use crate::hooks::__private::overrides;
    pub use inventory;
    pub use serde;

    /// What `morm-mgr` reads: one entry per model, only under `--cfg morm_mgr`.
    pub struct ModelMeta(pub &'static str);
    inventory::collect!(ModelMeta);

    pub fn registered() -> impl Iterator<Item = &'static str> {
        inventory::iter::<ModelMeta>.into_iter().map(|m| m.0)
    }
}

/// `#[model]`'s registration. `cfg(morm_mgr)` is read here, when `morm` is
/// built, so user crates never see the cfg.
#[cfg(morm_mgr)]
#[doc(hidden)]
#[macro_export]
macro_rules! __morm_register {
    ($meta:literal) => {
        $crate::__private::inventory::submit! { $crate::__private::ModelMeta($meta) }
    };
}

#[cfg(not(morm_mgr))]
#[doc(hidden)]
#[macro_export]
macro_rules! __morm_register {
    ($meta:literal) => {};
}
