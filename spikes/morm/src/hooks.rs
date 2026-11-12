//! Proof 5: hook signatures, the lazy `db`, and `#[hooks]` recording overrides.

use crate::param::Param;
use crate::pool::Conn;
use crate::{Db, Error, Model, Result};
use std::future::Future;
use std::marker::PhantomData;

/// One hook, as `Overrides::has` asks about it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u16)]
pub enum Hook {
    Validate = 1,
    PreInsert = 1 << 1,
    PostInsert = 1 << 2,
    ValidateChanges = 1 << 3,
    PreUpdate = 1 << 4,
    PostUpdate = 1 << 5,
    PreDelete = 1 << 6,
    PostDelete = 1 << 7,
    AfterCommit = 1 << 8,
}

/// Which hooks `M`'s impl overrides. Only `#[morm::hooks]` (and `#[model]`,
/// for a model without hooks) writes one: there's no public constructor and
/// no public constant, and it's typed by its model, so another model's
/// `OVERRIDES` can't be copied in.
pub struct Overrides<M: ?Sized>(u16, PhantomData<fn() -> M>);

impl<M: ?Sized> Overrides<M> {
    pub const fn has(&self, h: Hook) -> bool {
        self.0 & h as u16 != 0
    }
}

pub struct Changes<M: Model> {
    sets: Vec<(&'static str, Box<dyn Param + Send>)>,
    _m: PhantomData<fn() -> M>,
}

impl<M: Model> Changes<M> {
    pub fn new() -> Self {
        Self {
            sets: Vec::new(),
            _m: PhantomData,
        }
    }
    /// Sets `col`, replacing an earlier value for it.
    pub fn set(mut self, col: &'static str, v: impl Param + Send + 'static) -> Self {
        self.put(col, v);
        self
    }
    pub fn put(&mut self, col: &'static str, v: impl Param + Send + 'static) {
        self.sets.retain(|(c, _)| *c != col);
        self.sets.push((col, Box::new(v)));
    }
    pub fn columns(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.sets.iter().map(|(c, _)| *c)
    }
    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }
}

impl<M: Model> Default for Changes<M> {
    fn default() -> Self {
        Self::new()
    }
}

/// What an update or delete reaches. The spike's primary key is `id int8`.
pub enum Target<M: Model> {
    Id(i64),
    Ids(Vec<i64>),
    Filter {
        sql: &'static str,
        params: Vec<Box<dyn Param + Send>>,
    },
    All,
    #[doc(hidden)]
    __Model(PhantomData<fn() -> M>, std::convert::Infallible),
}

impl<M: Model> Target<M> {
    fn where_sql(&self, first: usize) -> (String, Vec<&dyn Param>) {
        match self {
            Target::Id(id) => (format!(" WHERE \"id\" = ${first}"), vec![id as &dyn Param]),
            Target::Ids(ids) => (
                format!(" WHERE \"id\" = ANY(${first})"),
                vec![ids as &dyn Param],
            ),
            Target::Filter { sql, params } => (
                format!(" WHERE {sql}"),
                params.iter().map(|p| &**p as &dyn Param).collect(),
            ),
            Target::All => (String::new(), Vec::new()),
            Target::__Model(_, n) => match *n {},
        }
    }
}

pub enum Committed<M: Model> {
    Inserted(Vec<M>),
    Copied {
        count: u64,
    },
    Updated {
        changes: Changes<M>,
        target: Target<M>,
    },
    Upserted {
        inserted: u64,
        updated: u64,
    },
    Deleted {
        target: Target<M>,
    },
}

/// Every hook has an empty default, so an unused one compiles away. The
/// signatures are the contract's.
pub trait Hooks: Model {
    /// Required, and written by `#[morm::hooks]`, never by hand.
    const OVERRIDES: Overrides<Self>;

    fn validate(&self) -> Result<()> {
        Ok(())
    }
    fn pre_insert(rows: &mut [Self], db: &impl Conn) -> impl Future<Output = Result<()>> + Send {
        let _ = (rows, db);
        async { Ok(()) }
    }
    fn post_insert(rows: &[Self], db: &impl Conn) -> impl Future<Output = Result<()>> + Send {
        let _ = (rows, db);
        async { Ok(()) }
    }
    fn validate_changes(changes: &Changes<Self>) -> Result<()> {
        let _ = changes;
        Ok(())
    }
    fn pre_update(
        changes: &mut Changes<Self>,
        target: &Target<Self>,
        db: &impl Conn,
    ) -> impl Future<Output = Result<()>> + Send {
        let _ = (changes, target, db);
        async { Ok(()) }
    }
    fn post_update(
        changes: &Changes<Self>,
        target: &Target<Self>,
        affected: u64,
        db: &impl Conn,
    ) -> impl Future<Output = Result<()>> + Send {
        let _ = (changes, target, affected, db);
        async { Ok(()) }
    }
    fn pre_delete(
        target: &Target<Self>,
        db: &impl Conn,
    ) -> impl Future<Output = Result<()>> + Send {
        let _ = (target, db);
        async { Ok(()) }
    }
    fn post_delete(
        target: &Target<Self>,
        affected: u64,
        db: &impl Conn,
    ) -> impl Future<Output = Result<()>> + Send {
        let _ = (target, affected, db);
        async { Ok(()) }
    }
    fn after_commit(event: Committed<Self>) -> impl Future<Output = Result<()>> + Send {
        let _ = event;
        async { Ok(()) }
    }
}

/// A generic insert that runs hooks the way morm will: only those overridden,
/// all through one lazy connection that the insert itself reuses.
pub async fn insert<M: Hooks>(db: &Db, rows: &mut [M]) -> Result<u64> {
    if M::OVERRIDES.has(Hook::Validate) {
        for r in rows.iter() {
            r.validate()?;
        }
    }
    let lazy = db.lazy();
    if M::OVERRIDES.has(Hook::PreInsert) {
        M::pre_insert(rows, &lazy).await?;
    }
    let mut n = 0;
    for r in rows.iter() {
        lazy.query(M::INSERT, &r.params()).await?;
        n += 1;
    }
    if M::OVERRIDES.has(Hook::PostInsert) {
        M::post_insert(rows, &lazy).await?;
    }
    Ok(n)
}

/// Runs a statement built at runtime: the spike leaks it to get the
/// `&'static str` that `Conn::query` takes. Spike-only.
async fn run_built(c: &impl Conn, sql: String, params: &[&dyn Param]) -> Result<u64> {
    let sql: &'static str = Box::leak(sql.into_boxed_str());
    let rows = c.query(sql, params).await?;
    Ok(rows.first().map(|r| r.get::<_, i64>(0) as u64).unwrap_or(0))
}

/// A generic update with the contract's update hooks, then `after_commit`.
pub async fn update<M: Hooks>(db: &Db, mut changes: Changes<M>, target: Target<M>) -> Result<u64> {
    if M::OVERRIDES.has(Hook::ValidateChanges) {
        M::validate_changes(&changes)?;
    }
    let lazy = db.lazy();
    if M::OVERRIDES.has(Hook::PreUpdate) {
        M::pre_update(&mut changes, &target, &lazy).await?;
    }
    if changes.is_empty() {
        return Err(Error::Validation("nothing to update"));
    }
    let affected = {
        let set: Vec<String> = changes
            .columns()
            .enumerate()
            .map(|(i, c)| format!("\"{c}\" = ${}", i + 1))
            .collect();
        let (wh, wp) = target.where_sql(changes.sets.len() + 1);
        let mut params: Vec<&dyn Param> = changes
            .sets
            .iter()
            .map(|(_, v)| &**v as &dyn Param)
            .collect();
        params.extend(wp);
        let sql = format!(
            "WITH u AS (UPDATE \"{}\" SET {}{} RETURNING 1) SELECT count(*) FROM u",
            M::TABLE,
            set.join(", "),
            wh
        );
        run_built(&lazy, sql, &params).await?
    };
    if M::OVERRIDES.has(Hook::PostUpdate) {
        M::post_update(&changes, &target, affected, &lazy).await?;
    }
    drop(lazy);
    if M::OVERRIDES.has(Hook::AfterCommit) {
        M::after_commit(Committed::Updated { changes, target }).await?;
    }
    Ok(affected)
}

/// A generic delete with the contract's delete hooks, then `after_commit`.
pub async fn delete<M: Hooks>(db: &Db, target: Target<M>) -> Result<u64> {
    let lazy = db.lazy();
    if M::OVERRIDES.has(Hook::PreDelete) {
        M::pre_delete(&target, &lazy).await?;
    }
    let affected = {
        let (wh, wp) = target.where_sql(1);
        let sql = format!(
            "WITH d AS (DELETE FROM \"{}\"{} RETURNING 1) SELECT count(*) FROM d",
            M::TABLE,
            wh
        );
        run_built(&lazy, sql, &wp).await?
    };
    if M::OVERRIDES.has(Hook::PostDelete) {
        M::post_delete(&target, affected, &lazy).await?;
    }
    drop(lazy);
    if M::OVERRIDES.has(Hook::AfterCommit) {
        M::after_commit(Committed::Deleted { target }).await?;
    }
    Ok(affected)
}

#[doc(hidden)]
pub mod __private {
    use super::{Hook, Overrides};
    use std::marker::PhantomData;

    /// What `#[morm::hooks]` and `#[model]` call. Not for hand-written impls.
    pub const fn overrides<M: ?Sized>(hooks: &[Hook]) -> Overrides<M> {
        let mut bits = 0u16;
        let mut i = 0;
        while i < hooks.len() {
            bits |= hooks[i] as u16;
            i += 1;
        }
        Overrides(bits, PhantomData)
    }
}
