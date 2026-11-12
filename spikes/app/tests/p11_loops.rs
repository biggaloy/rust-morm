//! Proof 11: the loop detector sees one statement repeating in a short window
//! on one task, at low cost. The exclusions aren't built: the step file
//! doesn't list them (see `proofs.md`).

use morm::loops::Detector;
use morm::{Conn, Db};
use std::time::{Duration, Instant};

const A: &str = "SELECT 1 /* a */";
const B: &str = "SELECT 1 /* b */";

#[tokio::test]
async fn a_statement_repeating_on_one_task_is_seen_once() {
    let d = Detector::new(Duration::from_millis(500), 10);
    // Nine is under the threshold, interleaved with another statement.
    for _ in 0..9 {
        assert!(!d.observe(A));
        assert!(!d.observe(B));
    }
    // The tenth run of A is the loop; it's reported once, not on every run.
    assert!(d.observe(A));
    for _ in 0..20 {
        assert!(!d.observe(A));
    }
    assert_eq!(*d.warnings.lock().unwrap(), [A]);
}

#[tokio::test]
async fn slow_repeats_and_other_tasks_are_not_loops() {
    let d = std::sync::Arc::new(Detector::new(Duration::from_millis(30), 3));
    for _ in 0..4 {
        assert!(!d.observe(A));
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    // The same statement once on each of many tasks is not a loop.
    let mut tasks = Vec::new();
    for _ in 0..20 {
        let d = d.clone();
        tasks.push(tokio::spawn(async move { d.observe(B) }));
    }
    for t in tasks {
        assert!(!t.await.unwrap());
    }
    assert!(d.warnings.lock().unwrap().is_empty());
}

/// Every query through `Db` passes the detector: an N+1 loop is reported.
#[tokio::test]
async fn queries_feed_the_detector() {
    let db = Db::connect(&app::dsn(), 1).unwrap();
    for _ in 0..12 {
        db.query("SELECT 2 /* n+1 */", &[]).await.unwrap();
    }
    assert_eq!(*db.loops().warnings.lock().unwrap(), ["SELECT 2 /* n+1 */"]);
}

#[tokio::test(flavor = "current_thread")]
async fn the_cost_per_statement() {
    let d = Detector::new(Duration::from_millis(100), 10);
    let stmts: Vec<&'static str> = (0..6).map(|i| &*format!("SELECT {i}").leak()).collect();
    let n = 2_000_000;
    let start = Instant::now();
    for i in 0..n {
        std::hint::black_box(d.observe(stmts[i % stmts.len()]));
    }
    let per = start.elapsed().as_nanos() as f64 / n as f64;
    println!("observe(): {per:.1} ns per statement");
    assert!(per < 1000.0, "observe() took {per:.1} ns");
}
