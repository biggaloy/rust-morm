//! Proof 4: `async |tx|` closures. The plain form takes any closure; the
//! retry form reruns one on a serialization failure. Every option goes on one
//! prefix builder before the closure. Closures that mutate what they capture
//! don't compile with retry: see `ui/`.

use models_b::Customer;
use morm::Isolation::Serializable;
use morm::{Conn, Db, Param, Tx};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio::sync::Barrier;

async fn setup(table: &str, size: usize) -> Db {
    let admin = app::admin().await;
    admin
        .batch_execute(&format!(
            "DROP TABLE IF EXISTS {table}; CREATE TABLE {table} (id int8 PRIMARY KEY, v int NOT NULL)"
        ))
        .await
        .unwrap();
    Db::connect(&app::dsn(), size).unwrap()
}

async fn count(db: &Db, sql: &'static str) -> i64 {
    db.query(sql, &[]).await.unwrap()[0].get(0)
}

/// The plain form runs once and may move or mutate its captures.
#[tokio::test]
async fn the_plain_form_takes_any_closure() {
    let db = setup("spike_tx_plain", 4).await;
    let owned = String::from("moved in");
    let mut calls = 0;
    let h = tokio::spawn({
        let db = db.clone();
        async move {
            let n = db
                .transaction(async move |tx: &Tx| {
                    let s = owned; // consumes the capture: FnOnce only
                    tx.query("INSERT INTO spike_tx_plain VALUES (1, 1)", &[])
                        .await?;
                    Ok(s.len())
                })
                .await
                .unwrap();
            db.transaction(async |tx| {
                calls += 1; // mutates a capture
                tx.query("INSERT INTO spike_tx_plain VALUES (2, 2)", &[])
                    .await
            })
            .await
            .unwrap();
            (n, calls)
        }
    });
    assert_eq!(h.await.unwrap(), (8, 1));
    assert_eq!(count(&db, "SELECT count(*) FROM spike_tx_plain").await, 2);
}

/// Reads a setting inside the transaction.
async fn show(tx: &Tx, sql: &'static str) -> morm::Result<String> {
    Ok(tx.query(sql, &[]).await?[0].get::<_, String>(0))
}

/// `.isolation` and per-transaction settings go before the closure, with or
/// without `.retry`, in any order. Each shape runs under `tokio::spawn` with a
/// closure that reads a captured row by reference, so its future is `Send`.
/// The settings reach the transaction and end with it.
#[tokio::test]
async fn every_option_goes_before_the_closure() {
    let db = setup("spike_tx_opts", 1).await;
    let h = tokio::spawn({
        let db = db.clone();
        async move {
            let row = Customer {
                id: 7,
                name: "seven".into(),
            };
            let mut seen = Vec::new();

            // No retry: the closure may still mutate a capture.
            db.isolation(Serializable)
                .statement_timeout(Duration::from_millis(1234))
                .lock_timeout(Duration::from_millis(250))
                .transaction(async |tx| {
                    seen.push(show(tx, "SHOW transaction_isolation").await?);
                    seen.push(show(tx, "SHOW statement_timeout").await?);
                    seen.push(show(tx, "SHOW lock_timeout").await?);
                    let p: [&dyn Param; 1] = [&row.id];
                    tx.query("INSERT INTO spike_tx_opts VALUES ($1, 1)", &p)
                        .await?;
                    Ok(())
                })
                .await
                .unwrap();

            // Retry first, then the other options.
            let got = db
                .retry(3)
                .isolation(Serializable)
                .statement_timeout(Duration::from_millis(4321))
                .transaction(async |tx| {
                    let p: [&dyn Param; 1] = [&row.id];
                    tx.query("UPDATE spike_tx_opts SET v = v + 1 WHERE id = $1", &p)
                        .await?;
                    Ok((
                        show(tx, "SHOW transaction_isolation").await?,
                        show(tx, "SHOW statement_timeout").await?,
                        row.name.len(),
                    ))
                })
                .await
                .unwrap();

            // A setting first, then retry.
            let lock = db
                .lock_timeout(Duration::from_millis(99))
                .retry(1)
                .transaction(async |tx| {
                    let p: [&dyn Param; 1] = [&row.id];
                    tx.query("SELECT 1 FROM spike_tx_opts WHERE id = $1", &p)
                        .await?;
                    show(tx, "SHOW lock_timeout").await
                })
                .await
                .unwrap();
            (seen, got, lock)
        }
    });
    let (seen, got, lock) = h.await.unwrap();
    assert_eq!(seen, ["serializable", "1234ms", "250ms"]);
    assert_eq!(
        got,
        (
            "serializable".to_owned(),
            "4321ms".to_owned(),
            "seven".len()
        )
    );
    assert_eq!(lock, "99ms");
    // The pool has one connection, and the settings didn't outlive the transaction.
    let rows = db
        .query(
            "SELECT current_setting('statement_timeout'), current_setting('lock_timeout'), current_setting('transaction_isolation')",
            &[],
        )
        .await
        .unwrap();
    assert_eq!(
        (
            rows[0].get::<_, &str>(0),
            rows[0].get::<_, &str>(1),
            rows[0].get::<_, &str>(2)
        ),
        ("0", "0", "read committed")
    );
    let v: i32 = db
        .query("SELECT v FROM spike_tx_opts WHERE id = 7", &[])
        .await
        .unwrap()[0]
        .get(0);
    assert_eq!(v, 2);
}

/// Two serializable transactions that each read the table and then write
/// conflict, so Postgres fails one with 40001 and `.retry` runs it again.
/// The closures capture by shared reference and run under `tokio::spawn`.
#[tokio::test]
async fn retry_reruns_the_closure_on_a_serialization_failure() {
    let db = setup("spike_tx_retry", 4).await;
    let attempts = Arc::new(AtomicU32::new(0));
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for id in [1i64, 2] {
        let (db, attempts, barrier) = (db.clone(), attempts.clone(), barrier.clone());
        handles.push(tokio::spawn(async move {
            let first = AtomicU32::new(1);
            db.isolation(Serializable)
                .retry(5)
                .transaction(async |tx| {
                    attempts.fetch_add(1, Ordering::Relaxed);
                    tx.query("SELECT sum(v) FROM spike_tx_retry", &[]).await?;
                    // Only the first attempts wait for each other, so both read first.
                    if first.swap(0, Ordering::Relaxed) == 1 {
                        barrier.wait().await;
                    }
                    let p: [&dyn Param; 1] = [&id];
                    tx.query("INSERT INTO spike_tx_retry VALUES ($1, 1)", &p)
                        .await?;
                    Ok(())
                })
                .await
        }));
    }
    for h in handles {
        h.await.unwrap().unwrap();
    }
    assert_eq!(count(&db, "SELECT count(*) FROM spike_tx_retry").await, 2);
    assert!(
        attempts.load(Ordering::Relaxed) >= 3,
        "no attempt was retried"
    );
}

/// Without `.retry`, the same conflict surfaces as 40001.
#[tokio::test]
async fn without_retry_the_failure_surfaces() {
    let db = setup("spike_tx_noretry", 4).await;
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for id in [1i64, 2] {
        let (db, barrier) = (db.clone(), barrier.clone());
        handles.push(tokio::spawn(async move {
            db.isolation(Serializable)
                .transaction(async |tx| {
                    tx.query("SELECT sum(v) FROM spike_tx_noretry", &[]).await?;
                    barrier.wait().await;
                    let p: [&dyn Param; 1] = [&id];
                    tx.query("INSERT INTO spike_tx_noretry VALUES ($1, 1)", &p)
                        .await?;
                    Ok(())
                })
                .await
        }));
    }
    let mut codes = Vec::new();
    for h in handles {
        if let Err(e) = h.await.unwrap() {
            codes.push(e.code().map(str::to_owned));
        }
    }
    assert_eq!(codes, [Some("40001".to_owned())]);
}

/// Counts its clones: what a `move` closure that owns rows pays per copy.
struct Probe<'a>(&'a AtomicU32);
impl Clone for Probe<'_> {
    fn clone(&self) -> Self {
        self.0.fetch_add(1, Ordering::Relaxed);
        Probe(self.0)
    }
}

/// Runs `.retry(n)` with a `move` closure owning a `Probe`, failing the first
/// `fail` attempts with a real 40001. Returns (attempts, clones).
async fn clones(db: &Db, n: u32, fail: u32) -> (u32, u32) {
    let made = AtomicU32::new(0);
    // Interior mutability carries state across attempts: `Clone` can't catch it.
    let attempts = AtomicU32::new(0);
    let probe = Probe(&made);
    let attempts_ref = &attempts;
    db.retry(n)
        .transaction(async move |tx| {
            let _owned = &probe;
            if attempts_ref.fetch_add(1, Ordering::Relaxed) < fail {
                tx.query(
                    "DO $$ BEGIN RAISE EXCEPTION 'spike' USING ERRCODE = '40001'; END $$",
                    &[],
                )
                .await?;
            }
            Ok(())
        })
        .await
        .unwrap();
    (
        attempts.load(Ordering::Relaxed),
        made.load(Ordering::Relaxed),
    )
}

/// An attempt that could still be retried runs a copy; the last attempt
/// allowed runs the original. So `.retry(0)` copies nothing, and a
/// transaction that succeeds at once copies the closure once.
#[tokio::test]
async fn retry_copies_the_closure_only_while_a_retry_is_left() {
    let db = Db::connect(&app::dsn(), 1).unwrap();
    assert_eq!(clones(&db, 0, 0).await, (1, 0));
    assert_eq!(clones(&db, 3, 0).await, (1, 1));
    assert_eq!(clones(&db, 2, 2).await, (3, 2));
    assert_eq!(clones(&db, 3, 2).await, (3, 3));
}

/// A `move` closure that changes its own copy compiles with `.retry`, and is
/// safe: each attempt starts from a fresh copy, so a change made by a failed
/// attempt never reaches the next one.
#[tokio::test]
async fn a_move_closure_may_change_its_own_copy() {
    let db = Db::connect(&app::dsn(), 1).unwrap();
    let mut row = Customer {
        id: 1,
        name: "a".into(),
    };
    row.name.push('b');
    let name = row.name;
    let seen = std::sync::Mutex::new(Vec::new());
    let (seen_ref, attempts) = (&seen, AtomicU32::new(0));
    let attempts_ref = &attempts;
    db.retry(2)
        .transaction(async move |tx| {
            let mut name = name;
            name.push('!'); // changes this attempt's copy only
            seen_ref.lock().unwrap().push(name);
            if attempts_ref.fetch_add(1, Ordering::Relaxed) < 2 {
                tx.query(
                    "DO $$ BEGIN RAISE EXCEPTION 'spike' USING ERRCODE = '40001'; END $$",
                    &[],
                )
                .await?;
            }
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(*seen.lock().unwrap(), ["ab!", "ab!", "ab!"]);
}
