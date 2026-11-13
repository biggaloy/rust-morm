//! Proof 17: stale state on a hot standby. The swap runs on the primary,
//! `SPIKE_DSN`, and the app's sessions are on its streaming standby,
//! `SPIKE_STANDBY_DSN`, as a `read_only` pool's would be.

use enum_proofs::stale::{self, Cure, Mode, Target, Via};
use enum_proofs::{Report, env, must_connect, server_version, show, wait_replay};
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() {
    let dsn = env("SPIKE_DSN");
    let standby = env("SPIKE_STANDBY_DSN");
    let m = must_connect(&dsn).await;
    let version = server_version(&m).await;
    let watch = must_connect(&standby).await;
    let in_recovery: bool = watch
        .query_one("SELECT pg_is_in_recovery()", &[])
        .await
        .unwrap()
        .get(0);
    assert!(in_recovery, "SPIKE_STANDBY_DSN isn't a standby");
    let mut settings = Vec::new();
    for s in [
        "max_standby_streaming_delay",
        "hot_standby_feedback",
        "server_version",
    ] {
        let v: String = watch
            .query_one(&format!("SHOW {s}"), &[])
            .await
            .unwrap()
            .get(0);
        settings.push(format!("{s} = {v}"));
    }
    println!("# Proof 17 on Postgres {version}, primary and streaming standby");
    println!();
    println!("standby: {}", settings.join("; "));
    let t = Target {
        name: format!("pg {version} standby"),
        prefix: "p17".to_owned(),
        app_dsn: standby.clone(),
        m,
        via: Via::Standby { watch },
        lc_messages: None,
    };
    let mut rep = Report::default();
    stale::statements(&t, Mode::Drop, &mut rep).await;
    stale::statements(&t, Mode::Keep, &mut rep).await;
    stale::rowtype_param(&t, &mut rep).await;
    stale::functions(&t, Mode::Drop, Cure::Replace, &mut rep).await;
    stale::functions(&t, Mode::Keep, Cure::Replace, &mut rep).await;
    conflicts(&t, &dsn, &standby, &mut rep).await;
    rep.finish("proof 17");
}

fn swap(sch: &str, table: &str) -> String {
    format!(
        r#"BEGIN;
ALTER TYPE "{sch}"."st" RENAME TO "st_old";
CREATE TYPE "{sch}"."st" AS ENUM ('b', 'a');
ALTER TABLE "{sch}"."{table}" ALTER COLUMN s TYPE "{sch}"."st" USING s::text::"{sch}"."st";
DROP TYPE "{sch}"."st_old";
COMMIT;"#
    )
}

/// Whether replaying a swap cancels what the standby is doing.
async fn conflicts(t: &Target, primary: &str, standby: &str, rep: &mut Report) {
    rep.header(
        &format!(
            "{}: replaying a swap while the standby reads the table",
            t.name
        ),
        &[
            "standby session",
            "what it was doing",
            "outcome",
            "seconds from the swap's commit",
        ],
    );
    let p = must_connect(primary).await;
    let watch = must_connect(standby).await;
    for (case, table) in [
        ("running", "t_running"),
        ("idle in a transaction", "t_idle"),
        ("holding only a prepared statement", "t_prepared"),
    ] {
        let sch = format!("p17_conflict_{}", table.trim_start_matches("t_"));
        p.batch_execute(&format!(
            r#"DROP SCHEMA IF EXISTS "{sch}" CASCADE;
CREATE SCHEMA "{sch}";
CREATE TYPE "{sch}"."st" AS ENUM ('a', 'b');
CREATE TABLE "{sch}"."{table}" (id int4, s "{sch}"."st");
INSERT INTO "{sch}"."{table}" VALUES (1, 'a');"#
        ))
        .await
        .unwrap();
        wait_replay(&p, &watch).await;
        let s = must_connect(standby).await;
        let read = format!(r#"SELECT count(*) FROM "{sch}"."{table}""#);
        let mut running = None;
        let mut prepared = None;
        match case {
            "running" => {
                let sql = format!(r#"SELECT count(*) FROM "{sch}"."{table}", pg_sleep(40)"#);
                running = Some(tokio::spawn(async move {
                    let r = s.c.query(&sql, &[]).await;
                    (
                        Instant::now(),
                        r.map(|_| "finished".to_owned())
                            .unwrap_or_else(|e| show(&e)),
                    )
                }));
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            "idle in a transaction" => {
                s.batch_execute("BEGIN ISOLATION LEVEL REPEATABLE READ")
                    .await
                    .unwrap();
                s.query(&read, &[]).await.unwrap();
                prepared = Some(s);
            }
            _ => {
                let st = s.prepare(&read).await.unwrap();
                s.query(&st, &[]).await.unwrap();
                prepared = Some(s);
            }
        }
        p.batch_execute(&swap(&sch, table)).await.unwrap();
        let committed = Instant::now();
        wait_replay(&p, &watch).await;
        let replayed = committed.elapsed().as_secs_f64();
        let outcome = match (running, prepared) {
            (Some(task), _) => {
                let (at, o) = task.await.unwrap();
                format!(
                    "{o}; replay waited {replayed:.1} s; the query ended {:.1} s after the commit",
                    at.duration_since(committed).as_secs_f64()
                )
            }
            (None, Some(s)) => {
                let next = s
                    .query(&read, &[])
                    .await
                    .map(|_| "next statement ran".to_owned())
                    .unwrap_or_else(|e| show(&e));
                let next = match s.closed_by() {
                    Some(why) => format!("{next}, after the server sent {why}"),
                    None => next,
                };
                format!("next statement: {next}; replay waited {replayed:.1} s")
            }
            _ => unreachable!(),
        };
        rep.row(&["S", case, &outcome, &format!("{replayed:.1}")]);
    }
}
