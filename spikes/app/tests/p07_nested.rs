//! Proof 7: a second connection on a task that is streaming rows is refused
//! with `NestedConnection`, instead of waiting forever on a full pool.
//! All in one test: the registry is process-wide.

use futures_util::StreamExt;
use morm::nested::{check, open_streams};
use morm::{Conn, Db, Error};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;

const ROWS: &str = "SELECT generate_series(1, 5)";

fn nested(r: morm::Result<Vec<tokio_postgres::Row>>) -> bool {
    matches!(r, Err(Error::NestedConnection))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stream_loops_refuse_a_second_connection() {
    let one = Db::connect(&app::dsn(), 1).unwrap();
    let db = Db::connect(&app::dsn(), 4).unwrap();
    let limit = Duration::from_secs(10);

    // No stream open: queries run, and the check is the fast path.
    assert_eq!(open_streams(), 0);
    one.query("SELECT 1", &[]).await.unwrap();

    // A query inside the loop, same task, pool of one: refused, not stuck.
    timeout(limit, async {
        let mut s = one.stream(ROWS);
        let mut refused = 0;
        while let Some(r) = s.next().await {
            r.unwrap();
            refused += nested(one.query("SELECT 1", &[]).await) as u32;
        }
        assert_eq!(refused, 5);
    })
    .await
    .expect("a nested query waited on the pool");
    assert_eq!(open_streams(), 0, "a finished stream stays registered");
    one.query("SELECT 1", &[]).await.unwrap();

    // Registration is on first poll: a stream not yet polled holds nothing.
    let s = db.stream(ROWS);
    db.query("SELECT 1", &[]).await.unwrap();
    drop(s);

    // A nested stream is refused too.
    let mut outer = db.stream(ROWS);
    outer.next().await.unwrap().unwrap();
    let mut inner = db.stream(ROWS);
    assert!(matches!(
        inner.next().await,
        Some(Err(Error::NestedConnection))
    ));
    drop(outer);

    // Another task may query while this one streams.
    let mut s = db.stream(ROWS);
    s.next().await.unwrap().unwrap();
    let other = tokio::spawn({
        let db = db.clone();
        async move { db.query("SELECT 1", &[]).await.map(|_| ()) }
    });
    other.await.unwrap().unwrap();
    assert!(nested(db.query("SELECT 1", &[]).await));
    drop(s);
    assert_eq!(open_streams(), 0, "a dropped stream stays registered");

    // `join!` polls both futures on one task: the query is refused.
    let (streamed, queried) = timeout(limit, async {
        let mut s = one.stream(ROWS);
        tokio::join!(
            async {
                let mut n = 0;
                while let Some(r) = s.next().await {
                    r.unwrap();
                    n += 1;
                    tokio::task::yield_now().await;
                }
                n
            },
            async {
                tokio::task::yield_now().await;
                one.query("SELECT 1", &[]).await
            }
        )
    })
    .await
    .expect("join! deadlocked");
    assert_eq!(streamed, 5);
    assert!(nested(queried));

    // A stream made here but polled on another task belongs to that task.
    let (to_main, mut from_task) = tokio::sync::mpsc::channel(1);
    let (to_task, mut from_main) = tokio::sync::mpsc::channel(1);
    let mut s = db.stream(ROWS);
    let task = tokio::spawn({
        let db = db.clone();
        async move {
            s.next().await.unwrap().unwrap();
            let refused_there = nested(db.query("SELECT 1", &[]).await);
            to_main.send(s).await.ok().unwrap();
            // After the stream moves away, this task may query again.
            from_main.recv().await.unwrap();
            let allowed_there = db.query("SELECT 1", &[]).await.is_ok();
            (refused_there, allowed_there)
        }
    });
    let mut s = from_task.recv().await.unwrap();
    // Not yet polled here: still registered to the other task.
    db.query("SELECT 1", &[]).await.unwrap();
    // Polled here: it moves to this task.
    s.next().await.unwrap().unwrap();
    to_task.send(()).await.unwrap();
    assert!(nested(db.query("SELECT 1", &[]).await));
    let (refused_there, allowed_there) = task.await.unwrap();
    assert!(refused_there && allowed_there);
    while let Some(r) = s.next().await {
        r.unwrap();
    }
    db.query("SELECT 1", &[]).await.unwrap();
    assert_eq!(open_streams(), 0);

    // The cost with no stream open: one relaxed load per checkout.
    let n = 50_000_000u32;
    let start = Instant::now();
    for _ in 0..n {
        std::hint::black_box(check()).unwrap();
    }
    let per = start.elapsed().as_nanos() as f64 / n as f64;
    println!("check() with no stream open: {per:.2} ns");
    assert!(per < 5.0, "the fast path took {per:.2} ns");

    // The cost while streams are open on other tasks: every checkout on every
    // task takes the spike's one process-wide mutex and scans its list. Each
    // count of open streams is measured from one thread, then from eight
    // threads checking at once.
    for open in [1usize, 10, 100, 1000] {
        let (stop, _) = tokio::sync::broadcast::channel::<()>(1);
        let ready = Arc::new(tokio::sync::Barrier::new(open + 1));
        let mut held = Vec::new();
        for _ in 0..open {
            let (mut rx, ready) = (stop.subscribe(), ready.clone());
            held.push(tokio::spawn(async move {
                let _reg = morm::nested::register_current();
                ready.wait().await;
                let _ = rx.recv().await;
            }));
        }
        ready.wait().await;
        assert_eq!(open_streams(), open);
        let n = (2_000_000 / open).max(20_000) as u32;
        let alone = per_check(1, n);
        let eight = per_check(8, n);
        println!(
            "check() with {open} streams open elsewhere: {alone:.1} ns from one thread, {eight:.1} ns each from eight at once"
        );
        drop(stop);
        for h in held {
            h.await.unwrap();
        }
        assert_eq!(open_streams(), 0);
    }
}

/// Mean time of one `check()` on each of `threads` threads running at once,
/// none of them streaming, so each check scans the whole list.
fn per_check(threads: usize, n: u32) -> f64 {
    let go = Arc::new(std::sync::Barrier::new(threads));
    let hs: Vec<_> = (0..threads)
        .map(|_| {
            let go = go.clone();
            std::thread::spawn(move || {
                go.wait();
                let start = Instant::now();
                for _ in 0..n {
                    std::hint::black_box(check()).unwrap();
                }
                start.elapsed().as_nanos() as f64 / n as f64
            })
        })
        .collect();
    hs.into_iter().map(|h| h.join().unwrap()).sum::<f64>() / threads as f64
}
