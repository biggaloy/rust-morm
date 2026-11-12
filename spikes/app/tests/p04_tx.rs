//! Proof 4: `async |tx|` closures. The plain form takes any closure; the
//! retry form reruns one on a serialization failure. Closures that mutate
//! what they capture don't compile with retry: see `ui/`.

use morm::{Conn, Db, Param, Tx};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::Barrier;

async fn setup(table: &str) -> Db {
    let admin = app::admin().await;
    admin
        .batch_execute(&format!(
            "DROP TABLE IF EXISTS {table}; CREATE TABLE {table} (id int PRIMARY KEY, v int NOT NULL)"
        ))
        .await
        .unwrap();
    Db::connect(&app::dsn(), 4).unwrap()
}

/// The plain form runs once and may move or mutate its captures.
#[tokio::test]
async fn the_plain_form_takes_any_closure() {
    let db = setup("spike_tx_plain").await;
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
    let rows = db
        .query("SELECT count(*) FROM spike_tx_plain", &[])
        .await
        .unwrap();
    assert_eq!(rows[0].get::<_, i64>(0), 2);
}

/// Two serializable transactions that each read the table and then write
/// conflict, so Postgres fails one with 40001 and `.retry` runs it again.
/// The closures capture by shared reference and run under `tokio::spawn`.
#[tokio::test]
async fn retry_reruns_the_closure_on_a_serialization_failure() {
    let db = setup("spike_tx_retry").await;
    let attempts = Arc::new(AtomicU32::new(0));
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for id in [1i32, 2] {
        let (db, attempts, barrier) = (db.clone(), attempts.clone(), barrier.clone());
        handles.push(tokio::spawn(async move {
            let first = AtomicU32::new(1);
            db.retry(5)
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
    let rows = db
        .query("SELECT count(*) FROM spike_tx_retry", &[])
        .await
        .unwrap();
    assert_eq!(rows[0].get::<_, i64>(0), 2);
    assert!(
        attempts.load(Ordering::Relaxed) >= 3,
        "no attempt was retried"
    );
}

/// Without `.retry`, the same conflict surfaces as 40001.
#[tokio::test]
async fn without_retry_the_failure_surfaces() {
    let db = setup("spike_tx_noretry").await;
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for id in [1i32, 2] {
        let (db, barrier) = (db.clone(), barrier.clone());
        handles.push(tokio::spawn(async move {
            db.retry(0)
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
