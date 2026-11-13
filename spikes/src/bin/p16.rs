//! Proof 16: proofs 14 and 15 repeated through PgBouncer in transaction mode.
//!
//! `SPIKE_DSN` is the primary, straight; the migration's side runs there.
//! `SPIKE_BOUNCER_DSN` reaches the same server through PgBouncer, whose admin
//! console is `SPIKE_BOUNCER_ADMIN_DSN`, and `SPIKE_BOUNCER_NAME` names it in
//! the output and in the schemas it makes. PgBouncer runs with
//! `default_pool_size = 1`, so every client reaches the same server session.

use enum_proofs::stale::{self, Cure, Mode, Target, Via};
use enum_proofs::{Label, Report, env, must_connect, server_version, show};
use tokio_postgres::types::Type;

#[tokio::main]
async fn main() {
    let dsn = env("SPIKE_DSN");
    let bouncer = env("SPIKE_BOUNCER_DSN");
    let admin_dsn = env("SPIKE_BOUNCER_ADMIN_DSN");
    let name = env("SPIKE_BOUNCER_NAME");
    let m = must_connect(&dsn).await;
    let version = server_version(&m).await;
    let admin = must_connect(&admin_dsn).await;
    let mut shown = Vec::new();
    for q in ["SHOW VERSION", "SHOW CONFIG"] {
        for msg in admin.simple_query(q).await.unwrap() {
            if let tokio_postgres::SimpleQueryMessage::Row(r) = msg {
                let key = r.get(0).unwrap_or("");
                if q == "SHOW VERSION"
                    || matches!(
                        key,
                        "pool_mode"
                            | "default_pool_size"
                            | "max_prepared_statements"
                            | "server_reset_query"
                            | "server_reset_query_always"
                    )
                {
                    shown.push(format!("{key} = {}", r.get(1).unwrap_or(key)));
                }
            }
        }
    }
    drop(admin);
    println!("# Proof 16 on Postgres {version} through PgBouncer {name}");
    println!();
    println!("{}", shown.join("; "));
    let prefix = format!("p16_{}", name.replace(['.', '-', ' '], "_"));
    let t = Target {
        name: format!("pg {version} through pgbouncer {name}"),
        prefix: prefix.clone(),
        app_dsn: bouncer.clone(),
        m,
        via: Via::Bouncer { admin_dsn },
        lc_messages: None,
    };
    let mut rep = Report::default();

    // The driver looks up an enum column's type with a named statement of its
    // own; through PgBouncer that has to work too.
    t.migrate(&format!(
        r#"DROP SCHEMA IF EXISTS "{prefix}_decode" CASCADE;
CREATE SCHEMA "{prefix}_decode";
CREATE TYPE "{prefix}_decode"."st" AS ENUM ('a', 'b');
CREATE TABLE "{prefix}_decode"."t" (id int4, s "{prefix}_decode"."st");
INSERT INTO "{prefix}_decode"."t" VALUES (1, 'b');"#
    ))
    .await;
    rep.header(
        &format!("{}: decoding an enum column", t.name),
        &["client", "outcome"],
    );
    for who in ["first client", "second client"] {
        let c = must_connect(&bouncer).await;
        let one = 1_i32;
        let r = c
            .query_typed(
                &format!(r#"SELECT s FROM "{prefix}_decode"."t" WHERE id = ($1::int4)"#),
                &[(&one, Type::INT4)],
            )
            .await;
        let o = match r.and_then(|rows| rows[0].try_get::<_, Label>(0)) {
            Ok(l) if l.0 == "b" => "right: b".to_owned(),
            Ok(l) => format!("WRONG: {}", l.0),
            Err(e) => show(&e),
        };
        rep.check(
            &format!("{who}: an enum column decodes through PgBouncer"),
            o == "right: b",
            &o,
        );
    }

    stale::statements(&t, Mode::Drop, &mut rep).await;
    stale::statements(&t, Mode::Keep, &mut rep).await;
    stale::rowtype_param(&t, &mut rep).await;
    for cure in [Cure::Replace, Cure::Reconnect] {
        stale::functions(&t, Mode::Drop, cure, &mut rep).await;
        stale::functions(&t, Mode::Keep, cure, &mut rep).await;
    }
    rep.finish("proof 16");
}
