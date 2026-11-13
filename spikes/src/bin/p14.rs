//! Proof 14: stale statements after a type swap, and the plpgsql parameter
//! mismatch after a column type change, straight against Postgres.
//!
//! `SPIKE_DSN` is a superuser connection (the locale test needs one), and
//! `SPIKE_LC` the locale to read the errors in, `de_DE.UTF-8` by default.

use enum_proofs::stale::{self, Mode, Target, Via};
use enum_proofs::{Report, env, must_connect, server_version};

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
    rep.finish("proof 14");
}
