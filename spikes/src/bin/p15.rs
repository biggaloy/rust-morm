//! Proof 15: plpgsql compiled in a session before a type swap, straight
//! against Postgres. `SPIKE_DSN` is a superuser connection.

use enum_proofs::stale::{self, Cure, Mode, Target, Via};
use enum_proofs::{Report, env, must_connect, server_version};

#[tokio::main]
async fn main() {
    let dsn = env("SPIKE_DSN");
    let m = must_connect(&dsn).await;
    let version = server_version(&m).await;
    println!("# Proof 15 on Postgres {version}");
    let t = Target {
        name: format!("pg {version} direct"),
        prefix: "p15".to_owned(),
        app_dsn: dsn,
        m,
        via: Via::Direct,
        lc_messages: None,
    };
    let mut rep = Report::default();
    stale::functions(&t, Mode::Drop, Cure::Replace, &mut rep).await;
    stale::functions(&t, Mode::Keep, Cure::Replace, &mut rep).await;
    rep.finish("proof 15");
}
