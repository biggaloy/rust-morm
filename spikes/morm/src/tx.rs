//! Proof 4: `async |tx|` closures, with and without retry.

use crate::param::Param;
use crate::pool::{Conn, Object, run};
use crate::{Db, Result};
use tokio_postgres::Row;

/// An open transaction. Closures receive `&Tx`.
pub struct Tx {
    conn: Object,
}

impl Conn for Tx {
    fn tx(&self) -> Option<&Tx> {
        Some(self)
    }
    async fn query(&self, sql: &'static str, params: &[&dyn Param]) -> Result<Vec<Row>> {
        run(&self.conn, sql, params).await
    }
}

/// `40001` and `40P01`: the errors a retry is for.
fn retryable(e: &crate::Error) -> bool {
    matches!(e.code(), Some("40001" | "40P01"))
}

async fn attempt<F, T>(db: &Db, begin: &str, f: F) -> Result<T>
where
    F: AsyncFnOnce(&Tx) -> Result<T>,
{
    let conn = db.checkout().await?;
    conn.batch_execute(begin).await?;
    let tx = Tx { conn };
    match f(&tx).await {
        Ok(v) => {
            tx.conn.batch_execute("COMMIT").await?;
            Ok(v)
        }
        Err(e) => {
            let _ = tx.conn.batch_execute("ROLLBACK").await;
            Err(e)
        }
    }
}

impl Db {
    /// The plain form: any `async` closure, run once. It may move or mutate
    /// what it captures.
    pub async fn transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: AsyncFnOnce(&Tx) -> Result<T>,
    {
        attempt(self, "BEGIN", f).await
    }

    /// `db.retry(n)`: rerun the closure on a serialization failure or deadlock.
    pub fn retry(&self, n: u32) -> Retrying<'_> {
        Retrying { db: self, n }
    }
}

pub struct Retrying<'d> {
    db: &'d Db,
    n: u32,
}

/// What `.retry(n)` takes: a closure it can call again, so it can't mutate
/// what it captures. Each attempt runs a fresh copy (`Clone`), so a closure
/// that captures by shared reference costs nothing to copy. A closure that
/// mutates a capture holds a `&mut`, which isn't `Clone`, so it's refused.
pub trait RetryBody<T>: AsyncFnOnce(&Tx) -> Result<T> + Clone {}
impl<F, T> RetryBody<T> for F where F: AsyncFnOnce(&Tx) -> Result<T> + Clone {}

impl Retrying<'_> {
    pub async fn transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: RetryBody<T>,
    {
        let mut left = self.n;
        loop {
            match attempt(self.db, "BEGIN ISOLATION LEVEL SERIALIZABLE", f.clone()).await {
                Err(e) if left > 0 && retryable(&e) => {
                    left -= 1;
                    // Give the transaction that won time to commit.
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
                other => return other,
            }
        }
    }

    /// The form the plan first named, `AsyncFn`, kept to show where it fails:
    /// a closure capturing by reference isn't `Send` under `tokio::spawn`.
    pub async fn transaction_async_fn<F, T>(&self, f: F) -> Result<T>
    where
        F: AsyncFn(&Tx) -> Result<T>,
    {
        let mut left = self.n;
        loop {
            match attempt(
                self.db,
                "BEGIN ISOLATION LEVEL SERIALIZABLE",
                async |tx: &Tx| f(tx).await,
            )
            .await
            {
                Err(e) if left > 0 && retryable(&e) => left -= 1,
                other => return other,
            }
        }
    }
}

/// The postfix shape, `db.transaction_postfix(f).retry(n).await`, kept to show
/// why it isn't the design. `.await` on a builder needs `IntoFuture`, whose
/// future must be a named type; on stable that means a boxed `dyn Future +
/// Send`, and proving the closure's future `Send` for that box needs a bound
/// over every lifetime, which forces the closure's captures to be `'static`.
pub mod postfix {
    use super::{Tx, attempt, retryable};
    use crate::{Db, Result};
    use std::future::{Future, IntoFuture};
    use std::marker::PhantomData;
    use std::pin::Pin;

    pub trait Body<'a, T>: Send {
        type Fut: Future<Output = Result<T>> + Send + 'a;
        fn call(self, tx: &'a Tx) -> Self::Fut;
    }
    impl<'a, T, F, Fut> Body<'a, T> for F
    where
        F: FnOnce(&'a Tx) -> Fut + Send,
        Fut: Future<Output = Result<T>> + Send + 'a,
    {
        type Fut = Fut;
        fn call(self, tx: &'a Tx) -> Fut {
            self(tx)
        }
    }

    pub struct Builder<'d, F, T> {
        db: &'d Db,
        f: F,
        n: u32,
        _t: PhantomData<fn() -> T>,
    }

    impl Db {
        pub fn transaction_postfix<F, T>(&self, f: F) -> Builder<'_, F, T>
        where
            F: for<'a> Body<'a, T>,
        {
            Builder {
                db: self,
                f,
                n: 0,
                _t: PhantomData,
            }
        }
    }

    impl<F, T> Builder<'_, F, T> {
        pub fn retry(self, n: u32) -> Self {
            Self { n, ..self }
        }
    }

    impl<'d, F, T> IntoFuture for Builder<'d, F, T>
    where
        F: for<'a> Body<'a, T> + Clone + Sync + 'd,
        T: Send + 'd,
    {
        type Output = Result<T>;
        type IntoFuture = Pin<Box<dyn Future<Output = Result<T>> + Send + 'd>>;
        fn into_future(self) -> Self::IntoFuture {
            Box::pin(async move {
                let mut left = self.n;
                loop {
                    let f = self.f.clone();
                    match attempt(self.db, "BEGIN", async |tx: &Tx| f.call(tx).await).await {
                        Err(e) if left > 0 && retryable(&e) => left -= 1,
                        other => return other,
                    }
                }
            })
        }
    }
}
