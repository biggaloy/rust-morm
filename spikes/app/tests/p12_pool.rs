//! Proof 12: `deadpool`'s core with our own manager: the pool grows on
//! demand, can be resized while running, closes idle connections, and a
//! checkout sends nothing to the server.

use morm::{Conn, Db};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tokio_postgres::Client;

const APP: &str = "spike_p12";

async fn backends(admin: &Client) -> i64 {
    let r = admin
        .query_one(
            "SELECT count(*) FROM pg_stat_activity WHERE application_name = $1",
            &[&APP],
        )
        .await
        .unwrap();
    r.get(0)
}

/// Each backend's `state_change` and query: they move only if a statement ran.
async fn activity(admin: &Client, pids: &[i32]) -> Vec<String> {
    let rows = admin
        .query(
            "SELECT pid::text || ' ' || state_change::text || ' ' || query FROM pg_stat_activity
             WHERE pid = ANY($1) ORDER BY pid",
            &[&pids],
        )
        .await
        .unwrap();
    rows.iter().map(|r| r.get(0)).collect()
}

async fn until(admin: &Client, want: i64) {
    let start = Instant::now();
    while backends(admin).await != want {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "backends never reached {want}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deadpool_with_our_manager() {
    let admin = app::admin().await;
    let db = Db::connect(&format!("{} application_name={APP}", app::dsn()), 4).unwrap();

    // Nothing opens until needed.
    assert_eq!(db.status().size, 0);
    until(&admin, 0).await;

    // Dynamic size: three at once opens three.
    let held = [
        db.checkout().await.unwrap(),
        db.checkout().await.unwrap(),
        db.checkout().await.unwrap(),
    ];
    assert_eq!(db.status().size, 3);
    let mut pids = Vec::new();
    for c in &held {
        pids.push(
            c.query_one("SELECT pg_backend_pid()", &[])
                .await
                .unwrap()
                .get::<_, i32>(0),
        );
    }
    pids.sort();
    drop(held);
    assert_eq!(db.status().available, 3);
    until(&admin, 3).await;

    // Checkout with no extra round trip: a hundred checkouts open nothing new
    // and run nothing on the server.
    let before = activity(&admin, &pids).await;
    let created = db.manager().created.load(Ordering::Relaxed);
    let start = Instant::now();
    for _ in 0..100 {
        let c = db.checkout().await.unwrap();
        drop(c);
    }
    let per = start.elapsed() / 100;
    assert_eq!(db.manager().created.load(Ordering::Relaxed), created);
    assert!(db.manager().recycled.load(Ordering::Relaxed) >= 100);
    assert_eq!(
        activity(&admin, &pids).await,
        before,
        "a checkout ran a statement"
    );
    let t = Instant::now();
    admin.simple_query("").await.unwrap();
    println!("checkout: {per:?}; one empty round trip: {:?}", t.elapsed());

    // Resize down while idle: the extra connections close.
    db.resize(2);
    assert_eq!(db.status().size, 2);
    until(&admin, 2).await;

    // Resize down while busy: they close as they come back.
    db.resize(4);
    let held: Vec<_> = futures_util::future::join_all((0..4).map(|_| db.checkout())).await;
    assert_eq!(db.status().size, 4);
    db.resize(1);
    drop(held);
    assert_eq!(db.status().size, 1);
    until(&admin, 1).await;

    // Resize up: more may open again.
    db.resize(6);
    let held: Vec<_> = futures_util::future::join_all((0..6).map(|_| db.checkout())).await;
    assert!(held.iter().all(Result::is_ok));
    assert_eq!(db.status().size, 6);
    drop(held);

    // Idle shrinking: nothing used for 200 ms is closed.
    assert_eq!(db.shrink_idle(Duration::from_millis(200)), 0);
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert_eq!(db.shrink_idle(Duration::from_millis(200)), 6);
    assert_eq!(db.status().size, 0);
    until(&admin, 0).await;

    // The same on a timer, as the pool's reaper.
    let reaper = db.spawn_reaper(Duration::from_millis(50), Duration::from_millis(100));
    db.query("SELECT 1", &[]).await.unwrap();
    assert_eq!(db.status().size, 1);
    until(&admin, 0).await;
    assert_eq!(db.status().size, 0);
    reaper.abort();

    // And it works again after shrinking to nothing.
    db.query("SELECT 1", &[]).await.unwrap();
}
