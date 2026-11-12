//! Proof 5: hook signatures, the lazy `db`, and `#[hooks]` recording overrides.

use crate::pool::Conn;
use crate::{Db, Model, Result};
use std::future::Future;

/// Which hooks an impl overrides. `#[morm::hooks]` writes it; nothing else
/// can, so a hook can't be skipped by forgetting the attribute.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Overrides(u16);

impl Overrides {
    pub const NONE: Self = Self(0);
    pub const VALIDATE: Self = Self(1);
    pub const PRE_INSERT: Self = Self(1 << 1);
    pub const POST_INSERT: Self = Self(1 << 2);
    pub const VALIDATE_CHANGES: Self = Self(1 << 3);
    pub const PRE_UPDATE: Self = Self(1 << 4);
    pub const POST_UPDATE: Self = Self(1 << 5);
    pub const PRE_DELETE: Self = Self(1 << 6);
    pub const POST_DELETE: Self = Self(1 << 7);
    pub const AFTER_COMMIT: Self = Self(1 << 8);

    #[doc(hidden)]
    pub const fn with(self, o: Self) -> Self {
        Self(self.0 | o.0)
    }
    pub const fn has(self, o: Self) -> bool {
        self.0 & o.0 != 0
    }
}

pub enum Committed<M: Model> {
    Inserted(Vec<M>),
    Deleted { ids: Vec<i64> },
}

/// Every hook has an empty default, so an unused one compiles away.
pub trait Hooks: Model {
    /// Required, and written by `#[morm::hooks]`, never by hand.
    const OVERRIDES: Overrides;

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
    fn pre_delete(ids: &[i64], db: &impl Conn) -> impl Future<Output = Result<()>> + Send {
        let _ = (ids, db);
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
    if M::OVERRIDES.has(Overrides::VALIDATE) {
        for r in rows.iter() {
            r.validate()?;
        }
    }
    let lazy = db.lazy();
    if M::OVERRIDES.has(Overrides::PRE_INSERT) {
        M::pre_insert(rows, &lazy).await?;
    }
    let mut n = 0;
    for r in rows.iter() {
        lazy.query(M::INSERT, &r.params()).await?;
        n += 1;
    }
    if M::OVERRIDES.has(Overrides::POST_INSERT) {
        M::post_insert(rows, &lazy).await?;
    }
    Ok(n)
}
