//! Proof 4: `async |tx|` closures, with and without retry, and every
//! transaction option on one prefix builder before the closure.

use crate::param::Param;
use crate::pool::{Conn, Object, run};
use crate::{Db, Result};
use std::time::Duration;
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Isolation {
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

impl Isolation {
    fn sql(self) -> &'static str {
        match self {
            Isolation::ReadCommitted => " ISOLATION LEVEL READ COMMITTED",
            Isolation::RepeatableRead => " ISOLATION LEVEL REPEATABLE READ",
            Isolation::Serializable => " ISOLATION LEVEL SERIALIZABLE",
        }
    }
}

/// The builder's retry state: `Once` takes any `AsyncFnOnce` closure, `Retry`
/// only one it can run again.
#[derive(Clone, Copy, Debug)]
pub struct Once;
#[derive(Clone, Copy, Debug)]
pub struct Retry(u32);

/// What a transaction is opened with. Every option is set before the
/// closure, on this one prefix builder, so the closure is always the last
/// argument and `transaction` is an `async fn`: its future is `Send` whenever
/// the concrete closure's is, with no boxing and no named future.
/// `db.isolation(..)`, `db.retry(n)`, `db.statement_timeout(..)` and
/// `db.lock_timeout(..)` each start one, and the options chain in any order.
pub struct TxOptions<'d, R> {
    db: &'d Db,
    isolation: Option<Isolation>,
    statement_timeout: Option<Duration>,
    lock_timeout: Option<Duration>,
    retry: R,
}

impl<R> TxOptions<'_, R> {
    pub fn isolation(self, i: Isolation) -> Self {
        Self {
            isolation: Some(i),
            ..self
        }
    }
    /// A per-transaction setting: `SET LOCAL statement_timeout`.
    pub fn statement_timeout(self, d: Duration) -> Self {
        Self {
            statement_timeout: Some(d),
            ..self
        }
    }
    /// A per-transaction setting: `SET LOCAL lock_timeout`.
    pub fn lock_timeout(self, d: Duration) -> Self {
        Self {
            lock_timeout: Some(d),
            ..self
        }
    }

    /// `BEGIN` with the isolation, then each setting as `SET LOCAL`, in one
    /// message. The spike formats it per call; step 20 chooses how it's sent.
    fn begin(&self) -> String {
        let mut s = String::from("BEGIN");
        if let Some(i) = self.isolation {
            s.push_str(i.sql());
        }
        if let Some(d) = self.statement_timeout {
            s.push_str(&format!(
                "; SET LOCAL statement_timeout = '{}ms'",
                d.as_millis()
            ));
        }
        if let Some(d) = self.lock_timeout {
            s.push_str(&format!("; SET LOCAL lock_timeout = '{}ms'", d.as_millis()));
        }
        s
    }
}

impl<'d> TxOptions<'d, Once> {
    fn new(db: &'d Db) -> Self {
        TxOptions {
            db,
            isolation: None,
            statement_timeout: None,
            lock_timeout: None,
            retry: Once,
        }
    }

    pub fn retry(self, n: u32) -> TxOptions<'d, Retry> {
        TxOptions {
            db: self.db,
            isolation: self.isolation,
            statement_timeout: self.statement_timeout,
            lock_timeout: self.lock_timeout,
            retry: Retry(n),
        }
    }

    /// Without retry: any `async` closure, run once. It may move or mutate
    /// what it captures.
    pub async fn transaction<F, T>(self, f: F) -> Result<T>
    where
        F: AsyncFnOnce(&Tx) -> Result<T>,
    {
        attempt(self.db, &self.begin(), f).await
    }
}

impl TxOptions<'_, Retry> {
    /// With retry: the closure must be `Clone`, so it can't mutate what it
    /// captures through a `&mut`. An attempt that could still be retried runs
    /// a fresh copy, and the last attempt allowed runs the original: `.retry(0)`
    /// copies nothing, and a transaction that succeeds at once copies once.
    pub async fn transaction<F, T>(self, f: F) -> Result<T>
    where
        F: RetryBody<T>,
    {
        let begin = self.begin();
        for _ in 0..self.retry.0 {
            match attempt(self.db, &begin, f.clone()).await {
                Err(e) if retryable(&e) => {
                    // Give the transaction that won time to commit, with jitter:
                    // a fixed pause let two transactions that both lost retry in
                    // step and collide again, every time, on Postgres 16 in CI.
                    // `RandomState` is freshly keyed per call, so its hash of
                    // nothing is a cheap random number. The clock's nanoseconds
                    // aren't a safe source: where the clock steps in microseconds
                    // they're a multiple of 1,000, and `% 40` is always 0.
                    use std::hash::BuildHasher;
                    let r = std::collections::hash_map::RandomState::new().hash_one(());
                    let pause = 10 + r % 40;
                    tokio::time::sleep(Duration::from_millis(pause)).await;
                }
                other => return other,
            }
        }
        attempt(self.db, &begin, f).await
    }

    /// The form the plan first named, `AsyncFn`, kept to show where it fails:
    /// a closure capturing by reference isn't `Send` under `tokio::spawn`.
    pub async fn transaction_async_fn<F, T>(self, f: F) -> Result<T>
    where
        F: AsyncFn(&Tx) -> Result<T>,
    {
        let begin = self.begin();
        let mut left = self.retry.0;
        loop {
            match attempt(self.db, &begin, async |tx: &Tx| f(tx).await).await {
                Err(e) if left > 0 && retryable(&e) => left -= 1,
                other => return other,
            }
        }
    }
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
    /// The plain form: `db.transaction(async |tx| ..)`.
    pub async fn transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: AsyncFnOnce(&Tx) -> Result<T>,
    {
        TxOptions::new(self).transaction(f).await
    }
    pub fn isolation(&self, i: Isolation) -> TxOptions<'_, Once> {
        TxOptions::new(self).isolation(i)
    }
    pub fn retry(&self, n: u32) -> TxOptions<'_, Retry> {
        TxOptions::new(self).retry(n)
    }
    pub fn statement_timeout(&self, d: Duration) -> TxOptions<'_, Once> {
        TxOptions::new(self).statement_timeout(d)
    }
    pub fn lock_timeout(&self, d: Duration) -> TxOptions<'_, Once> {
        TxOptions::new(self).lock_timeout(d)
    }
}

/// What `.retry(n)` takes: a closure it can call again, so it can't mutate
/// what it captures. A closure that captures by shared reference costs
/// nothing to copy. A closure that mutates a capture holds a `&mut`, which
/// isn't `Clone`, so it's refused. Interior mutability (`Cell`, atomics,
/// mutexes) goes through a shared reference, so it isn't caught.
pub trait RetryBody<T>: AsyncFnOnce(&Tx) -> Result<T> + Clone {}
impl<F, T> RetryBody<T> for F where F: AsyncFnOnce(&Tx) -> Result<T> + Clone {}

/// The postfix shape, `db.transaction_postfix(f).retry(n).await`, kept to show
/// why it isn't the design. `.await` on a builder needs `IntoFuture`, whose
/// future must be a named type; on stable that means a boxed `dyn Future +
/// Send`, and proving the closure's future `Send` for that box needs a bound
/// over every lifetime, which forces the closure's captures to be `'static`.
/// The same holds for any postfix option, `.isolation(..)` included.
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
