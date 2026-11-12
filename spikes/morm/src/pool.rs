//! Proof 12: `deadpool`'s core with our own manager. Also the `Conn` trait and
//! the lazy connection hooks receive (proof 5).

use crate::param::{Param, typed};
use crate::{Error, Result, Tx, loops, nested};
use deadpool::managed::{self, Metrics, RecycleError, RecycleResult};
use futures_util::TryStreamExt;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::OnceCell;
use tokio_postgres::{Client, NoTls, Row};

/// Opens connections and checks them on checkout without a round trip.
pub struct Manager {
    config: tokio_postgres::Config,
    pub created: AtomicU64,
    pub recycled: AtomicU64,
}

impl managed::Manager for Manager {
    type Type = Client;
    type Error = tokio_postgres::Error;

    async fn create(&self) -> Result<Client, tokio_postgres::Error> {
        let (client, conn) = self.config.connect(NoTls).await?;
        tokio::spawn(async move {
            let _ = conn.await;
        });
        self.created.fetch_add(1, Ordering::Relaxed);
        Ok(client)
    }

    /// No `SELECT 1`: a dead connection shows as closed, and a query on one
    /// that died since fails like any other network error.
    async fn recycle(
        &self,
        client: &mut Client,
        _: &Metrics,
    ) -> RecycleResult<tokio_postgres::Error> {
        self.recycled.fetch_add(1, Ordering::Relaxed);
        if client.is_closed() {
            return Err(RecycleError::message("connection closed"));
        }
        Ok(())
    }
}

pub type Object = managed::Object<Manager>;

struct Inner {
    pool: managed::Pool<Manager>,
    checkouts: AtomicU64,
    pub(crate) loops: loops::Detector,
}

/// The pool handle apps pass around.
#[derive(Clone)]
pub struct Db(Arc<Inner>);

impl Db {
    pub fn connect(dsn: &str, max_size: usize) -> Result<Db> {
        let config: tokio_postgres::Config = dsn.parse()?;
        let manager = Manager {
            config,
            created: AtomicU64::new(0),
            recycled: AtomicU64::new(0),
        };
        let pool = managed::Pool::builder(manager)
            .max_size(max_size)
            .runtime(deadpool::Runtime::Tokio1)
            .wait_timeout(Some(Duration::from_secs(5)))
            .build()
            .map_err(|e| Error::Pool(e.to_string()))?;
        Ok(Db(Arc::new(Inner {
            pool,
            checkouts: AtomicU64::new(0),
            loops: loops::Detector::new(Duration::from_secs(1), 10),
        })))
    }

    /// One exclusive connection. Refused on a task already streaming rows.
    pub async fn checkout(&self) -> Result<Object> {
        nested::check()?;
        self.checkout_unchecked().await
    }

    pub(crate) async fn checkout_unchecked(&self) -> Result<Object> {
        let obj = self
            .0
            .pool
            .get()
            .await
            .map_err(|e| Error::Pool(e.to_string()))?;
        self.0.checkouts.fetch_add(1, Ordering::Relaxed);
        Ok(obj)
    }

    pub fn checkouts(&self) -> u64 {
        self.0.checkouts.load(Ordering::Relaxed)
    }
    pub fn manager(&self) -> &Manager {
        self.0.pool.manager()
    }
    pub fn status(&self) -> deadpool::Status {
        self.0.pool.status()
    }
    /// Change the most connections the pool may open, while it runs.
    pub fn resize(&self, max_size: usize) {
        self.0.pool.resize(max_size)
    }
    /// Close idle connections unused for `idle`. Returns how many closed.
    pub fn shrink_idle(&self, idle: Duration) -> usize {
        self.0
            .pool
            .retain(|_, m| m.last_used() < idle)
            .removed
            .len()
    }
    /// Shrinks on a timer, as the real pool's idle reaper would.
    pub fn spawn_reaper(&self, every: Duration, idle: Duration) -> tokio::task::JoinHandle<()> {
        let db = self.clone();
        tokio::spawn(async move {
            let mut t = tokio::time::interval(every);
            loop {
                t.tick().await;
                db.shrink_idle(idle);
            }
        })
    }
    pub fn loops(&self) -> &loops::Detector {
        &self.0.loops
    }
    /// A connection that is checked out on first use, for hooks.
    pub fn lazy(&self) -> Lazy<'_> {
        Lazy {
            db: self,
            conn: OnceCell::new(),
        }
    }
}

pub(crate) async fn run(c: &Client, sql: &str, params: &[&dyn Param]) -> Result<Vec<Row>> {
    Ok(c.query_typed_raw(sql, typed(params))
        .await?
        .try_collect()
        .await?)
}

/// Connection-like: `db`, a lazy or held connection, or `tx`.
pub trait Conn: Send + Sync {
    fn tx(&self) -> Option<&Tx>;
    fn query(
        &self,
        sql: &'static str,
        params: &[&dyn Param],
    ) -> impl Future<Output = Result<Vec<Row>>> + Send;
}

impl Conn for Db {
    fn tx(&self) -> Option<&Tx> {
        None
    }
    async fn query(&self, sql: &'static str, params: &[&dyn Param]) -> Result<Vec<Row>> {
        self.0.loops.observe(sql);
        let c = self.checkout().await?;
        run(&c, sql, params).await
    }
}

/// Checks out on its first query and keeps that connection.
pub struct Lazy<'a> {
    db: &'a Db,
    conn: OnceCell<Object>,
}

impl Lazy<'_> {
    pub fn checked_out(&self) -> bool {
        self.conn.initialized()
    }
}

impl Conn for Lazy<'_> {
    fn tx(&self) -> Option<&Tx> {
        None
    }
    async fn query(&self, sql: &'static str, params: &[&dyn Param]) -> Result<Vec<Row>> {
        self.db.0.loops.observe(sql);
        let c = self.conn.get_or_try_init(|| self.db.checkout()).await?;
        run(c, sql, params).await
    }
}
