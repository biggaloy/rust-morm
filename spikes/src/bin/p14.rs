//! Proof 14: stale statements after a type swap, and the plpgsql parameter
//! mismatch after a column type change, straight against Postgres.
//!
//! `SPIKE_DSN` is a superuser connection (the locale test needs one), and
//! `SPIKE_LC` the locale to read the errors in, `de_DE.UTF-8` by default.

use enum_proofs::stale::{self, Mode, Target, Via};
use enum_proofs::{Report, connect, env, must_connect, server_version, show};

#[tokio::main]
async fn main() {
    let dsn = env("SPIKE_DSN");
    let m = must_connect(&dsn).await;
    let version = server_version(&m).await;
    println!("# Proof 14 on Postgres {version}");
    let t = Target {
        name: format!("pg {version} direct"),
        prefix: "p14".to_owned(),
        app_dsn: dsn,
        m,
        via: Via::Direct,
        lc_messages: Some(std::env::var("SPIKE_LC").unwrap_or_else(|_| "de_DE.UTF-8".to_owned())),
    };
    let mut rep = Report::default();
    stale::statements(&t, Mode::Drop, &mut rep).await;
    stale::statements(&t, Mode::Keep, &mut rep).await;
    stale::rowtype_param(&t, &mut rep).await;

    // Whether an app's own role can pick the language of these messages.
    let lc = t.lc_messages.clone().unwrap();
    rep.header("Who can set lc_messages", &["session", "outcome"]);
    t.m.batch_execute("DROP ROLE IF EXISTS p14_plain; CREATE ROLE p14_plain LOGIN")
        .await
        .unwrap();
    let plain = t.app_dsn.replace("user=postgres", "user=p14_plain");
    let s = must_connect(&plain).await;
    let r = s.batch_execute(&format!("SET lc_messages = '{lc}'")).await;
    let o = r.map(|_| "ok".to_owned()).unwrap_or_else(|e| show(&e));
    rep.row(&["a role that isn't superuser, SET", &o]);
    rep.check(
        "a role that isn't superuser can't SET lc_messages",
        o.starts_with("42501"),
        &o,
    );
    drop(s);
    let o = match connect(&format!("{plain} options='-c lc_messages={lc}'")).await {
        Ok(_) => "connected".to_owned(),
        Err(e) => show(&e),
    };
    rep.row(&["a role that isn't superuser, startup option", &o]);
    rep.check(
        "nor send it as a startup option",
        o.starts_with("42501"),
        &o,
    );
    t.m.batch_execute("DROP ROLE p14_plain").await.unwrap();
    rep.finish("proof 14");
}
