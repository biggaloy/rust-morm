//! Proof 18: `plpgsql_check` on a running `postgres:NN` server, installed
//! from PGDG with no restart, used as step 59 will use it.
//!
//! `SPIKE_DSN` is a superuser connection to the database `postgres`, and
//! `SPIKE_PG_STARTED` the server's `pg_postmaster_start_time()` read before
//! the package was installed. A superuser makes the `morm_plpgsql_check`
//! template; everything else runs as `p18_user`, which has `CREATEDB` and
//! isn't a superuser, in a clone of that template, as morm's scratch user
//! will. Each check calls `plpgsql_check_function_tb` with the function's
//! `regprocedure`, `fatal_errors => false` and `all_warnings => true`, as
//! R-1076 says morm will.

use enum_proofs::{Report, Session, env, must_connect, server_version, show};
use tokio_postgres::Client;

const USER: &str = "p18_user";

fn dsn_for(base: &str, user: &str, db: &str, extra: &str) -> String {
    let kept: Vec<&str> = base
        .split_whitespace()
        .filter(|kv| !kv.starts_with("user=") && !kv.starts_with("dbname="))
        .collect();
    format!("{} user={user} dbname={db} {extra}", kept.join(" "))
}

async fn exec(c: &Client, sql: &str) -> String {
    match c.batch_execute(sql).await {
        Ok(()) => "ok".to_owned(),
        Err(e) => show(&e),
    }
}

/// One finding, as `level sqlstate message (detail) @ line`.
#[derive(Debug, Clone)]
struct Finding {
    level: String,
    sqlstate: String,
    message: String,
    detail: Option<String>,
    line: Option<i32>,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.level, self.sqlstate, self.message)?;
        if let Some(d) = &self.detail {
            write!(f, " (detail: {d})")?;
        }
        if let Some(l) = self.line {
            write!(f, " @ line {l}")?;
        }
        Ok(())
    }
}

fn render(r: &Result<Vec<Finding>, String>) -> String {
    match r {
        Err(e) => format!("the check failed: {e}"),
        Ok(v) if v.is_empty() => "no findings".to_owned(),
        Ok(v) => v
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join("; "),
    }
}

fn errors(r: &Result<Vec<Finding>, String>) -> usize {
    r.as_ref()
        .map(|v| v.iter().filter(|f| f.level == "error").count())
        .unwrap_or(0)
}

fn has(r: &Result<Vec<Finding>, String>, level: &str, sqlstate: &str, text: &str) -> bool {
    r.as_ref()
        .map(|v| {
            v.iter()
                .any(|f| f.level == level && f.sqlstate == sqlstate && f.message.contains(text))
        })
        .unwrap_or(false)
}

/// Checks a function the way morm will. `args` adds named arguments, such as
/// a trigger's table.
async fn check(c: &Client, sig: &str, args: &str) -> Result<Vec<Finding>, String> {
    let sql = format!(
        "SELECT level, sqlstate, message, detail, lineno FROM plpgsql_check_function_tb('{sig}'::regprocedure{args}, fatal_errors => false, all_warnings => true)"
    );
    let rows = c.query(&sql, &[]).await.map_err(|e| show(&e))?;
    Ok(rows
        .iter()
        .map(|r| Finding {
            level: r.get::<_, Option<String>>(0).unwrap_or_default(),
            sqlstate: r.get::<_, Option<String>>(1).unwrap_or_default(),
            message: r.get::<_, Option<String>>(2).unwrap_or_default(),
            detail: r.get(3),
            line: r.get(4),
        })
        .collect())
}

async fn mode(c: &Client) -> String {
    match c
        .query_one("SELECT current_setting('plpgsql_check.mode', true)", &[])
        .await
    {
        Ok(r) => r
            .get::<_, Option<String>>(0)
            .unwrap_or_else(|| "NULL".to_owned()),
        Err(e) => show(&e),
    }
}

/// The tables and functions every check database starts with. The columns
/// `gone` and `gone2` are dropped once the functions exist.
const SCHEMA: &str = r#"
CREATE TABLE t (id int4, a int4, gone int4, gone2 int4);
INSERT INTO t VALUES (1, 1, 1, 1);
CREATE TABLE other (id int4, gone int4);
CREATE TABLE n (big int8, small int4, wide numeric(20,4), long text);
INSERT INTO n VALUES (1, 1, 1, 'x');
CREATE SCHEMA hidden;
CREATE TABLE hidden.only_here (a int4);
INSERT INTO hidden.only_here VALUES (7);
CREATE TABLE tr (id int4, a int4, gone int4);
CREATE VIEW tr_v AS SELECT id, a FROM tr;
"#;

/// The false shapes R-1079 lists, each with no pragma and then with each
/// candidate pragma: (name, signature, body without, [(pragma, body with)],
/// how to call it with no extension, what that call returns).
fn false_shapes() -> Vec<(
    &'static str,
    &'static str,
    String,
    Vec<(&'static str, String)>,
    &'static str,
    &'static str,
)> {
    let f = |name: &str, args: &str, ret: &str, body: &str| {
        format!(
            "CREATE OR REPLACE FUNCTION {name}({args}) RETURNS {ret} LANGUAGE plpgsql AS $$\n{body}\n$$;"
        )
    };
    vec![
        (
            "a temp table the function creates",
            "f_temp()",
            f(
                "f_temp",
                "",
                "int4",
                "BEGIN\n  CREATE TEMP TABLE tt (a int4) ON COMMIT DROP;\n  INSERT INTO tt VALUES (1);\n  RETURN (SELECT a FROM tt);\nEND",
            ),
            vec![(
                "PERFORM 'PRAGMA:TABLE: tt (a int4)';",
                f(
                    "f_temp",
                    "",
                    "int4",
                    "BEGIN\n  CREATE TEMP TABLE tt (a int4) ON COMMIT DROP;\n  PERFORM 'PRAGMA:TABLE: tt (a int4)';\n  INSERT INTO tt VALUES (1);\n  RETURN (SELECT a FROM tt);\nEND",
                ),
            )],
            "SELECT f_temp()",
            "1",
        ),
        (
            "a table the function creates",
            "f_made()",
            f(
                "f_made",
                "",
                "int4",
                "BEGIN\n  CREATE TABLE IF NOT EXISTS made_here (a int4);\n  INSERT INTO made_here VALUES (1);\n  RETURN (SELECT max(a) FROM made_here);\nEND",
            ),
            vec![(
                "PERFORM 'PRAGMA:TABLE: made_here (a int4)';",
                f(
                    "f_made",
                    "",
                    "int4",
                    "BEGIN\n  CREATE TABLE IF NOT EXISTS made_here (a int4);\n  PERFORM 'PRAGMA:TABLE: made_here (a int4)';\n  INSERT INTO made_here VALUES (1);\n  RETURN (SELECT max(a) FROM made_here);\nEND",
                ),
            )],
            "SELECT f_made()",
            "1",
        ),
        (
            "a temp table the caller creates",
            "f_caller()",
            f(
                "f_caller",
                "",
                "int4",
                "BEGIN\n  RETURN (SELECT count(*)::int4 FROM caller_tmp);\nEND",
            ),
            vec![(
                "PERFORM 'PRAGMA:TABLE: caller_tmp (a int4)';",
                f(
                    "f_caller",
                    "",
                    "int4",
                    "BEGIN\n  PERFORM 'PRAGMA:TABLE: caller_tmp (a int4)';\n  RETURN (SELECT count(*)::int4 FROM caller_tmp);\nEND",
                ),
            )],
            "CREATE TEMP TABLE caller_tmp (a int4); INSERT INTO caller_tmp VALUES (1), (2); SELECT f_caller()",
            "2",
        ),
        (
            "a record filled by EXECUTE",
            "f_exec_rec(text)",
            f(
                "f_exec_rec",
                "q text",
                "int4",
                "DECLARE r record;\nBEGIN\n  EXECUTE q INTO r;\n  RETURN r.a;\nEND",
            ),
            vec![(
                "PERFORM 'PRAGMA:TYPE: r (a int4)';",
                f(
                    "f_exec_rec",
                    "q text",
                    "int4",
                    "DECLARE r record;\nBEGIN\n  EXECUTE q INTO r;\n  PERFORM 'PRAGMA:TYPE: r (a int4)';\n  RETURN r.a;\nEND",
                ),
            )],
            "SELECT f_exec_rec('SELECT 3 AS a')",
            "3",
        ),
        (
            "a record with other fields by branch",
            "f_branch_rec(int4)",
            f(
                "f_branch_rec",
                "p int4",
                "int4",
                "DECLARE r record;\nBEGIN\n  IF p > 0 THEN\n    SELECT 1 AS a INTO r;\n  ELSE\n    SELECT 2 AS b INTO r;\n  END IF;\n  IF p > 0 THEN\n    RETURN r.a;\n  END IF;\n  RETURN r.b;\nEND",
            ),
            vec![
                (
                    "PERFORM 'PRAGMA:TYPE: r (a int4, b int4)';",
                    f(
                        "f_branch_rec",
                        "p int4",
                        "int4",
                        "DECLARE r record;\nBEGIN\n  IF p > 0 THEN\n    SELECT 1 AS a INTO r;\n  ELSE\n    SELECT 2 AS b INTO r;\n  END IF;\n  PERFORM 'PRAGMA:TYPE: r (a int4, b int4)';\n  IF p > 0 THEN\n    RETURN r.a;\n  END IF;\n  RETURN r.b;\nEND",
                    ),
                ),
                (
                    "PERFORM 'PRAGMA:DISABLE:CHECK'; in a block around the reads",
                    f(
                        "f_branch_rec",
                        "p int4",
                        "int4",
                        "DECLARE r record;\nBEGIN\n  IF p > 0 THEN\n    SELECT 1 AS a INTO r;\n  ELSE\n    SELECT 2 AS b INTO r;\n  END IF;\n  BEGIN\n    PERFORM 'PRAGMA:DISABLE:CHECK';\n    IF p > 0 THEN\n      RETURN r.a;\n    END IF;\n    RETURN r.b;\n  END;\nEND",
                    ),
                ),
            ],
            "SELECT f_branch_rec(1) + f_branch_rec(-1)",
            "3",
        ),
        (
            "set_config of search_path",
            "f_path()",
            f(
                "f_path",
                "",
                "int4",
                "BEGIN\n  PERFORM set_config('search_path', 'hidden', true);\n  RETURN (SELECT max(a) FROM only_here);\nEND",
            ),
            vec![
                (
                    "PERFORM 'PRAGMA:TABLE: only_here (a int4)';",
                    f(
                        "f_path",
                        "",
                        "int4",
                        "BEGIN\n  PERFORM set_config('search_path', 'hidden', true);\n  PERFORM 'PRAGMA:TABLE: only_here (a int4)';\n  RETURN (SELECT max(a) FROM only_here);\nEND",
                    ),
                ),
                (
                    "PERFORM 'PRAGMA:DISABLE:CHECK'; in a block around the read",
                    f(
                        "f_path",
                        "",
                        "int4",
                        "BEGIN\n  PERFORM set_config('search_path', 'hidden', true);\n  BEGIN\n    PERFORM 'PRAGMA:DISABLE:CHECK';\n    RETURN (SELECT max(a) FROM only_here);\n  END;\nEND",
                    ),
                ),
            ],
            "SELECT f_path()",
            "7",
        ),
        (
            "a constant 1/0",
            "f_div(int4)",
            f(
                "f_div",
                "p int4",
                "int4",
                "BEGIN\n  IF p > 100 THEN\n    PERFORM 1/0;\n  END IF;\n  RETURN p;\nEND",
            ),
            vec![(
                "PERFORM 'PRAGMA:DISABLE:CHECK'; in a block around it",
                f(
                    "f_div",
                    "p int4",
                    "int4",
                    "BEGIN\n  IF p > 100 THEN\n    BEGIN\n      PERFORM 'PRAGMA:DISABLE:CHECK';\n      PERFORM 1/0;\n    END;\n  END IF;\n  RETURN p;\nEND",
                ),
            )],
            "SELECT f_div(4)",
            "4",
        ),
    ]
}

#[tokio::main]
async fn main() {
    let base = env("SPIKE_DSN");
    let su = must_connect(&base).await;
    let version = server_version(&su).await;
    println!("# Proof 18 on Postgres {version}");
    let mut rep = Report::default();

    rep.header("Install and the template", &["who", "step", "outcome"]);
    let started: String = su
        .query_one("SELECT pg_postmaster_start_time()::text", &[])
        .await
        .unwrap()
        .get(0);
    let before = std::env::var("SPIKE_PG_STARTED").unwrap_or_default();
    rep.check(
        "the server wasn't restarted to install the package",
        !before.is_empty() && before == started,
        &format!("started {before} before the install, {started} now"),
    );
    let avail = su
        .query(
            "SELECT default_version FROM pg_available_extensions WHERE name = 'plpgsql_check'",
            &[],
        )
        .await
        .unwrap();
    rep.check(
        "plpgsql_check is available",
        avail.len() == 1,
        &avail
            .first()
            .map(|r| r.get::<_, String>(0))
            .unwrap_or_default(),
    );
    // One statement at a time: a multi-statement string runs as one
    // transaction, and DROP DATABASE refuses that.
    for sql in [
        "DROP DATABASE IF EXISTS p18_check WITH (FORCE)",
        "DROP DATABASE IF EXISTS p18_plain WITH (FORCE)",
        "DO $$ BEGIN IF EXISTS (SELECT FROM pg_database WHERE datname = 'morm_plpgsql_check') THEN ALTER DATABASE morm_plpgsql_check IS_TEMPLATE false; END IF; END $$",
        "DROP DATABASE IF EXISTS morm_plpgsql_check WITH (FORCE)",
        "DROP ROLE IF EXISTS p18_user",
        "CREATE ROLE p18_user LOGIN CREATEDB",
        "CREATE DATABASE morm_plpgsql_check",
    ] {
        su.batch_execute(sql)
            .await
            .unwrap_or_else(|e| panic!("{sql}: {}", show(&e)));
    }
    let tpl = must_connect(&dsn_for(&base, "postgres", "morm_plpgsql_check", "")).await;
    rep.row(&[
        "superuser",
        "CREATE EXTENSION plpgsql_check in morm_plpgsql_check",
        &exec(&tpl, "CREATE EXTENSION plpgsql_check").await,
    ]);
    let extversion: String = tpl
        .query_one(
            "SELECT extversion FROM pg_extension WHERE extname = 'plpgsql_check'",
            &[],
        )
        .await
        .unwrap()
        .get(0);
    rep.row(&["superuser", "extension version", &extversion]);
    drop(tpl);
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    rep.row(&[
        "superuser",
        "ALTER DATABASE morm_plpgsql_check IS_TEMPLATE true ALLOW_CONNECTIONS false",
        &exec(
            &su,
            "ALTER DATABASE morm_plpgsql_check IS_TEMPLATE true ALLOW_CONNECTIONS false",
        )
        .await,
    ]);

    let user_pg = must_connect(&dsn_for(&base, USER, "postgres", "")).await;
    let is_super: bool = user_pg
        .query_one(
            "SELECT rolsuper FROM pg_roles WHERE rolname = current_user",
            &[],
        )
        .await
        .unwrap()
        .get(0);
    rep.check("p18_user isn't a superuser", !is_super, "");
    let r = exec(
        &user_pg,
        "CREATE DATABASE p18_check TEMPLATE morm_plpgsql_check",
    )
    .await;
    rep.check("p18_user clones the template", r == "ok", &r);
    let r = exec(&user_pg, "CREATE DATABASE p18_plain TEMPLATE template0").await;
    rep.check("p18_user makes a database from template0", r == "ok", &r);
    let plain = must_connect(&dsn_for(&base, USER, "p18_plain", "")).await;
    let r = exec(&plain, "CREATE EXTENSION plpgsql_check").await;
    rep.check(
        "p18_user's CREATE EXTENSION plpgsql_check fails with 42501",
        r.starts_with("42501"),
        &r,
    );
    let n: i64 = plain
        .query_one(
            "SELECT count(*) FROM pg_extension WHERE extname = 'plpgsql_check'",
            &[],
        )
        .await
        .unwrap()
        .get(0);
    rep.check(
        "the database made from template0 holds no extension",
        n == 0,
        &format!("{n} rows"),
    );

    let c = must_connect(&dsn_for(&base, USER, "p18_check", "")).await;
    let v: Option<String> = c
        .query_one(
            "SELECT extversion FROM pg_extension WHERE extname = 'plpgsql_check'",
            &[],
        )
        .await
        .unwrap()
        .get(0);
    rep.check(
        "the clone holds the extension",
        v.as_deref() == Some(extversion.as_str()),
        &format!("{v:?}"),
    );
    rep.check(
        "nothing sets plpgsql_check.mode, so current_setting is NULL",
        mode(&c).await == "NULL",
        &mode(&c).await,
    );

    setup(&c).await;
    setup(&plain).await;

    branch(&mut rep, &c).await;
    shapes(&mut rep, &c, &plain).await;
    disable_block(&mut rep, &c).await;
    stale_pragma(&mut rep, &c).await;
    narrowing(&mut rep, &c).await;
    pragmas(&mut rep, &c).await;
    execute(&mut rep, &c).await;
    triggers(&mut rep, &c).await;
    disabled_mode(&mut rep, &base, &su).await;
    in_comment(&mut rep, &c).await;

    drop(c);
    drop(plain);
    drop(user_pg);
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    for sql in [
        "DROP DATABASE p18_check WITH (FORCE)",
        "DROP DATABASE p18_plain WITH (FORCE)",
        "ALTER DATABASE morm_plpgsql_check IS_TEMPLATE false",
        "DROP DATABASE morm_plpgsql_check WITH (FORCE)",
        "DROP ROLE p18_user",
    ] {
        su.batch_execute(sql)
            .await
            .unwrap_or_else(|e| panic!("{sql}: {}", show(&e)));
    }
    rep.finish("proof 18");
}

async fn setup(c: &Client) {
    c.batch_execute(SCHEMA).await.unwrap();
}

/// A column dropped from a branch a passing call never reaches.
async fn branch(rep: &mut Report, c: &Client) {
    rep.header(
        "A column dropped from a branch a passing call misses",
        &["what", "outcome"],
    );
    c.batch_execute(
        "CREATE FUNCTION f_branch(p int4) RETURNS int4 LANGUAGE plpgsql AS $$
BEGIN
  IF p > 100 THEN
    RETURN (SELECT gone FROM t LIMIT 1);
  END IF;
  RETURN p;
END $$;
ALTER TABLE t DROP COLUMN gone;",
    )
    .await
    .unwrap();
    let call = match c.query_one("SELECT f_branch(1)", &[]).await {
        Ok(r) => format!("returns {}", r.get::<_, i32>(0)),
        Err(e) => show(&e),
    };
    rep.row(&["f_branch(1) after the drop", &call]);
    let r = check(c, "f_branch(int4)", "").await;
    rep.check(
        "plpgsql_check reports 42703 for the dropped column",
        has(&r, "error", "42703", "gone"),
        &render(&r),
    );
}

async fn shapes(rep: &mut Report, c: &Client, plain: &Client) {
    rep.header(
        "False shapes: each healthy function's finding, and the pragma line that clears it",
        &["shape", "pragma", "findings"],
    );
    for (shape, sig, without, with, run, want) in false_shapes() {
        c.batch_execute(&without).await.unwrap();
        let r = check(c, sig, "").await;
        rep.row(&[shape, "none", &render(&r)]);
        rep.check(
            &format!("{shape}: a false error without a pragma"),
            errors(&r) > 0,
            &render(&r),
        );
        let mut cleared = Vec::new();
        for (pragma, body) in &with {
            c.batch_execute(body).await.unwrap();
            let r = check(c, sig, "").await;
            rep.row(&[shape, pragma, &render(&r)]);
            if errors(&r) == 0 {
                cleared.push(*pragma);
            }
            // The same line, in a database with no extension: to Postgres
            // it's only a string.
            plain.batch_execute(body).await.unwrap();
            let ran = match plain.simple_query(run).await {
                Ok(msgs) => msgs
                    .iter()
                    .filter_map(|m| match m {
                        tokio_postgres::SimpleQueryMessage::Row(r) => r.get(0).map(str::to_owned),
                        _ => None,
                    })
                    .next_back()
                    .unwrap_or_default(),
                Err(e) => show(&e),
            };
            plain.batch_execute("DISCARD TEMP").await.unwrap();
            rep.check(
                &format!("{shape}: `{pragma}` runs with no extension"),
                ran == want,
                &format!("{run} gave {ran}"),
            );
        }
        rep.check(
            &format!("{shape}: a pragma line clears it"),
            !cleared.is_empty(),
            &cleared.join(" / "),
        );
    }
}

async fn disable_block(rep: &mut Report, c: &Client) {
    rep.header(
        "PRAGMA:DISABLE:CHECK hides a real break until its block ends",
        &["what", "outcome"],
    );
    c.batch_execute(
        "CREATE FUNCTION f_disable(p int4) RETURNS int4 LANGUAGE plpgsql AS $$
BEGIN
  BEGIN
    PERFORM 'PRAGMA:DISABLE:CHECK';
    IF p > 100 THEN
      RETURN (SELECT gone2 FROM t LIMIT 1);
    END IF;
  END;
  IF p > 200 THEN
    RETURN (SELECT gone2 + 1 FROM t LIMIT 1);
  END IF;
  RETURN p;
END $$;
ALTER TABLE t DROP COLUMN gone2;",
    )
    .await
    .unwrap();
    let r = check(c, "f_disable(int4)", "").await;
    rep.row(&["findings", &render(&r)]);
    let lines: Vec<i32> = r
        .as_ref()
        .map(|v| {
            v.iter()
                .filter(|f| f.sqlstate == "42703")
                .filter_map(|f| f.line)
                .collect()
        })
        .unwrap_or_default();
    rep.check(
        "only the read after the block is reported (line 10), not the one inside it (line 6)",
        lines == [10],
        &format!("42703 at lines {lines:?}"),
    );
}

async fn stale_pragma(rep: &mut Report, c: &Client) {
    rep.header(
        "A PRAGMA:TABLE line that no longer matches",
        &["what", "outcome"],
    );
    c.batch_execute(
        "CREATE FUNCTION f_stale_pragma(p int4) RETURNS int4 LANGUAGE plpgsql AS $$
BEGIN
  PERFORM 'PRAGMA:TABLE: t (id int4, a int4, gone int4)';
  IF p > 100 THEN
    RETURN (SELECT gone FROM t LIMIT 1);
  END IF;
  RETURN p;
END $$;",
    )
    .await
    .unwrap();
    let r = check(c, "f_stale_pragma(int4)", "").await;
    rep.row(&["findings, with t's column gone dropped", &render(&r)]);
    let call = match c.query_one("SELECT f_stale_pragma(101)", &[]).await {
        Ok(r) => format!("returns {}", r.get::<_, i32>(0)),
        Err(e) => show(&e),
    };
    rep.row(&["f_stale_pragma(101)", &call]);
    rep.check(
        "the stale pragma hides the dropped column, which fails when that line runs",
        errors(&r) == 0 && call.starts_with("42703"),
        &render(&r),
    );
}

async fn narrowing(rep: &mut Report, c: &Client) {
    rep.header(
        "Narrowing and widening",
        &["function", "arguments", "findings"],
    );
    c.batch_execute(
        "CREATE FUNCTION f_narrow() RETURNS int4 LANGUAGE plpgsql AS $$
DECLARE v int4;
BEGIN
  SELECT big INTO v FROM n LIMIT 1;
  RETURN v;
END $$;
CREATE FUNCTION f_widen() RETURNS int8 LANGUAGE plpgsql AS $$
DECLARE v int8;
BEGIN
  SELECT small INTO v FROM n LIMIT 1;
  RETURN v;
END $$;
CREATE FUNCTION f_numeric() RETURNS numeric LANGUAGE plpgsql AS $$
DECLARE v numeric(6,2);
BEGIN
  SELECT wide INTO v FROM n LIMIT 1;
  RETURN v;
END $$;
CREATE FUNCTION f_varchar() RETURNS text LANGUAGE plpgsql AS $$
DECLARE v varchar(3);
BEGIN
  SELECT long INTO v FROM n LIMIT 1;
  RETURN v;
END $$;",
    )
    .await
    .unwrap();
    for (sig, label) in [
        ("f_narrow()", "int4 variable reading int8"),
        ("f_widen()", "int8 variable reading int4"),
        ("f_numeric()", "numeric(6,2) variable reading numeric(20,4)"),
        ("f_varchar()", "varchar(3) variable reading text"),
    ] {
        let r = check(c, sig, "").await;
        rep.row(&[label, "all_warnings => true", &render(&r)]);
        if sig == "f_narrow()" {
            rep.check(
                "a narrowing is level performance, 42804, detail cast \"bigint\" value to \"integer\" type",
                r.as_ref().is_ok_and(|v| {
                    v.iter().any(|f| {
                        f.level == "performance"
                            && f.sqlstate == "42804"
                            && f.detail.as_deref() == Some("cast \"bigint\" value to \"integer\" type")
                    })
                }),
                &render(&r),
            );
            let d = c
                .query(
                    "SELECT level, sqlstate, message, detail, lineno FROM plpgsql_check_function_tb('f_narrow()'::regprocedure)",
                    &[],
                )
                .await
                .map(|rows| rows.len())
                .map_err(|e| show(&e));
            rep.row(&[label, "default arguments", &format!("{d:?} findings")]);
            rep.check(
                "with default arguments a narrowing gives no finding",
                d == Ok(0),
                &format!("{d:?}"),
            );
        }
        if sig == "f_widen()" {
            rep.check(
                "a widening looks the same: performance, 42804, detail cast \"integer\" value to \"bigint\" type",
                r.as_ref().is_ok_and(|v| {
                    v.iter().any(|f| {
                        f.level == "performance"
                            && f.sqlstate == "42804"
                            && f.detail.as_deref() == Some("cast \"integer\" value to \"bigint\" type")
                    })
                }),
                &render(&r),
            );
        }
    }
}

async fn execute(rep: &mut Report, c: &Client) {
    rep.header("EXECUTE reading a dropped column", &["query", "findings"]);
    c.batch_execute(
        "CREATE TABLE ex (id int4, gone int4);
CREATE FUNCTION f_exec_const() RETURNS void LANGUAGE plpgsql AS $$
BEGIN
  EXECUTE 'SELECT gone FROM ex';
END $$;
CREATE FUNCTION f_exec_format() RETURNS void LANGUAGE plpgsql AS $$
BEGIN
  EXECUTE format('SELECT %I FROM ex', 'gone');
END $$;
CREATE FUNCTION f_exec_built(c text) RETURNS void LANGUAGE plpgsql AS $$
BEGIN
  EXECUTE 'SELECT ' || c || ' FROM ex';
END $$;
ALTER TABLE ex DROP COLUMN gone;",
    )
    .await
    .unwrap();
    for (sig, label, caught) in [
        ("f_exec_const()", "a constant string", true),
        ("f_exec_format()", "format of constants", true),
        ("f_exec_built(text)", "built at run time", false),
    ] {
        let r = check(c, sig, "").await;
        rep.row(&[label, &render(&r)]);
        rep.check(
            &format!(
                "{label}: the dropped column {}",
                if caught {
                    "is reported"
                } else {
                    "isn't reported"
                }
            ),
            has(&r, "error", "42703", "gone") == caught,
            &render(&r),
        );
    }
}

async fn triggers(rep: &mut Report, c: &Client) {
    rep.header(
        "Trigger forms, with a column the triggers read dropped",
        &["form", "arguments", "findings"],
    );
    c.batch_execute(
        "CREATE FUNCTION trg_row() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  NEW.a := NEW.gone;
  RETURN NEW;
END $$;
CREATE TRIGGER tr_row BEFORE INSERT ON tr FOR EACH ROW EXECUTE FUNCTION trg_row();
CREATE FUNCTION trg_after_row() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  PERFORM NEW.gone;
  RETURN NULL;
END $$;
CREATE TRIGGER tr_after_row AFTER UPDATE ON tr FOR EACH ROW EXECUTE FUNCTION trg_after_row();
CREATE FUNCTION trg_stmt() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  PERFORM (SELECT max(gone) FROM tr);
  RETURN NULL;
END $$;
CREATE TRIGGER tr_stmt AFTER DELETE ON tr FOR EACH STATEMENT EXECUTE FUNCTION trg_stmt();
CREATE FUNCTION trg_transition() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  PERFORM (SELECT count(*) FROM ot);
  PERFORM (SELECT max(gone) FROM nt);
  RETURN NULL;
END $$;
CREATE TRIGGER tr_transition AFTER UPDATE ON tr REFERENCING OLD TABLE AS ot NEW TABLE AS nt FOR EACH STATEMENT EXECUTE FUNCTION trg_transition();
CREATE FUNCTION trg_transition_ok() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  PERFORM (SELECT count(*) FROM nt);
  RETURN NULL;
END $$;
CREATE TRIGGER tr_transition_ok AFTER INSERT ON tr REFERENCING NEW TABLE AS nt FOR EACH STATEMENT EXECUTE FUNCTION trg_transition_ok();
CREATE FUNCTION trg_instead() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  INSERT INTO tr (id, a, gone) VALUES (NEW.id, NEW.a, 0);
  RETURN NEW;
END $$;
CREATE TRIGGER tr_instead INSTEAD OF INSERT ON tr_v FOR EACH ROW EXECUTE FUNCTION trg_instead();
CREATE FUNCTION trg_none() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  NEW.a := NEW.gone;
  RETURN NEW;
END $$;
CREATE FUNCTION evt() RETURNS event_trigger LANGUAGE plpgsql AS $$
BEGIN
  PERFORM (SELECT max(gone) FROM tr);
END $$;
CREATE FUNCTION f_sql() RETURNS int4 LANGUAGE sql AS $$ SELECT 1 $$;
CREATE PROCEDURE p_proc() LANGUAGE plpgsql AS $$
BEGIN
  PERFORM (SELECT max(gone) FROM tr);
END $$;
CREATE FUNCTION f_poly(x anyelement) RETURNS anyelement LANGUAGE plpgsql AS $$
BEGIN
  PERFORM (SELECT max(gone) FROM tr);
  RETURN x;
END $$;
ALTER TABLE tr DROP COLUMN gone;",
    )
    .await
    .unwrap();
    // The table and transition tables of each trigger, from pg_trigger, as
    // morm reads them.
    for (form, func) in [
        ("BEFORE ROW", "trg_row"),
        ("AFTER ROW", "trg_after_row"),
        ("AFTER STATEMENT", "trg_stmt"),
        ("transition tables", "trg_transition"),
        ("transition tables, healthy", "trg_transition_ok"),
        ("INSTEAD OF on a view", "trg_instead"),
    ] {
        let row = c
            .query_one(
                &format!("SELECT tgrelid::regclass::text, tgoldtable::text, tgnewtable::text FROM pg_trigger WHERE tgfoid = '{func}'::regproc"),
                &[],
            )
            .await
            .unwrap();
        let rel: String = row.get(0);
        let old: Option<String> = row.get(1);
        let new: Option<String> = row.get(2);
        let mut args = format!(", '{rel}'::regclass");
        if let Some(o) = &old {
            args.push_str(&format!(", oldtable => '{o}'"));
        }
        if let Some(n) = &new {
            args.push_str(&format!(", newtable => '{n}'"));
        }
        let r = check(c, &format!("{func}()"), &args).await;
        rep.row(&[form, &args[2..], &render(&r)]);
        if func == "trg_transition_ok" {
            rep.check(&format!("{form}: no finding"), errors(&r) == 0, &render(&r));
        } else {
            rep.check(
                &format!("{form}: 42703 for the dropped column"),
                has(&r, "error", "42703", "gone"),
                &render(&r),
            );
        }
        if old.is_some() || new.is_some() {
            let r = check(c, &format!("{func}()"), &format!(", '{rel}'::regclass")).await;
            rep.row(&[
                form,
                &format!("'{rel}'::regclass, no transition tables"),
                &render(&r),
            ]);
            if func == "trg_transition_ok" {
                rep.check(
                    &format!("{form}: 42P01 without the transition tables"),
                    has(&r, "error", "42P01", ""),
                    &render(&r),
                );
            }
        }
    }
    for (form, sig, want) in [
        (
            "a trigger function with no trigger, no table",
            "trg_none()",
            "22023",
        ),
        ("an event trigger function", "evt()", "42703"),
        ("a procedure", "p_proc()", "42703"),
        ("a polymorphic function", "f_poly(anyelement)", "42703"),
        ("a language sql function", "f_sql()", "22023"),
    ] {
        let r = check(c, sig, "").await;
        rep.row(&[form, "none", &render(&r)]);
        let ok = match &r {
            Err(e) => e.starts_with(want),
            Ok(_) => has(&r, "error", want, ""),
        };
        rep.check(&format!("{form}: {want}"), ok, &render(&r));
    }
}

/// `plpgsql_check.mode = 'disabled'`, set each way, over f_branch's real
/// 42703.
async fn disabled_mode(rep: &mut Report, base: &str, su: &Client) {
    rep.header(
        "plpgsql_check.mode = disabled, over a function with a real 42703",
        &[
            "set by",
            "current_setting before checking",
            "findings",
            "notices",
        ],
    );
    let run = async |s: &Session, sig: &str| -> (String, Result<Vec<Finding>, String>, String) {
        let before = mode(s).await;
        let _ = s.take_notices();
        let r = check(s, sig, "").await;
        (before, r, s.take_notices().join("; "))
    };
    let user = |extra: &str| dsn_for(base, USER, "p18_check", extra);

    let s = must_connect(&user("")).await;
    let (m, r, n) = run(&s, "f_branch(int4)").await;
    rep.row(&["nothing", &m, &render(&r), &n]);
    rep.check(
        "with nothing set, the check runs",
        errors(&r) > 0,
        &render(&r),
    );

    let s = must_connect(&user("")).await;
    exec(&s, "SET plpgsql_check.mode = 'by_function'").await;
    let (m, r, n) = run(&s, "f_branch(int4)").await;
    rep.row(&["SET by_function", &m, &render(&r), &n]);
    rep.check(
        "with by_function, the check runs",
        errors(&r) > 0,
        &render(&r),
    );

    let s = must_connect(&user("")).await;
    let set = exec(&s, "SET plpgsql_check.mode = 'disabled'").await;
    let (m, r, n) = run(&s, "f_branch(int4)").await;
    rep.row(&[
        &format!("SET, before the library loads ({set})"),
        &m,
        &render(&r),
        &n,
    ]);
    rep.check(
        "SET disabled: no findings, only a NOTICE",
        r.as_ref().is_ok_and(|v| v.is_empty()) && n.contains("NOTICE"),
        &n,
    );

    let s = must_connect(&user("")).await;
    let _ = check(&s, "f_branch(int4)", "").await;
    let set = exec(&s, "SET plpgsql_check.mode = 'disabled'").await;
    let (m, r, n) = run(&s, "f_branch(int4)").await;
    rep.row(&[
        &format!("SET, after the library loaded ({set})"),
        &m,
        &render(&r),
        &n,
    ]);
    rep.check(
        "SET disabled after loading: no findings, only a NOTICE",
        r.as_ref().is_ok_and(|v| v.is_empty()) && n.contains("NOTICE"),
        &n,
    );

    let set = exec(
        su,
        "ALTER ROLE p18_user SET plpgsql_check.mode = 'disabled'",
    )
    .await;
    let s = must_connect(&user("")).await;
    let (m, r, n) = run(&s, "f_branch(int4)").await;
    rep.row(&[&format!("the role ({set})"), &m, &render(&r), &n]);
    rep.check(
        "a role setting shows in current_setting before the library loads",
        m == "disabled",
        &m,
    );
    rep.check(
        "a role setting: no findings, only a NOTICE",
        r.as_ref().is_ok_and(|v| v.is_empty()) && n.contains("NOTICE"),
        &n,
    );
    exec(su, "ALTER ROLE p18_user RESET plpgsql_check.mode").await;

    let s = must_connect(&user("options='-c plpgsql_check.mode=disabled'")).await;
    let (m, r, n) = run(&s, "f_branch(int4)").await;
    rep.row(&["the connection's options", &m, &render(&r), &n]);
    rep.check(
        "a connection setting shows in current_setting before the library loads",
        m == "disabled",
        &m,
    );
    rep.check(
        "a connection setting: no findings, only a NOTICE",
        r.as_ref().is_ok_and(|v| v.is_empty()) && n.contains("NOTICE"),
        &n,
    );

    // On a function: a copy of f_branch, so the others keep their findings.
    let s = must_connect(&user("")).await;
    s.batch_execute(
        "CREATE FUNCTION f_branch_mode(p int4) RETURNS int4 LANGUAGE plpgsql AS $$
BEGIN
  IF p > 100 THEN
    RETURN (SELECT gone FROM t LIMIT 1);
  END IF;
  RETURN p;
END $$;",
    )
    .await
    .unwrap();
    let set_fresh = exec(
        &s,
        "ALTER FUNCTION f_branch_mode(int4) SET plpgsql_check.mode = 'disabled'",
    )
    .await;
    rep.row(&[
        "the function, by p18_user before the library loads",
        "-",
        &set_fresh,
        "-",
    ]);
    rep.check(
        "p18_user's SET on a function fails with 42501 before the library loads",
        set_fresh.starts_with("42501"),
        &set_fresh,
    );
    let s = must_connect(&user("")).await;
    let _ = check(&s, "f_branch(int4)", "").await;
    let set_loaded = exec(
        &s,
        "ALTER FUNCTION f_branch_mode(int4) SET plpgsql_check.mode = 'disabled'",
    )
    .await;
    rep.row(&[
        "the function, by p18_user after the library loaded",
        "-",
        &set_loaded,
        "-",
    ]);
    if set_loaded != "ok" {
        let sus = must_connect(&dsn_for(base, "postgres", "p18_check", "")).await;
        let set_su = exec(
            &sus,
            "ALTER FUNCTION f_branch_mode(int4) SET plpgsql_check.mode = 'disabled'",
        )
        .await;
        rep.row(&["the function, by a superuser", "-", &set_su, "-"]);
    }
    let s = must_connect(&user("")).await;
    let (m, r, n) = run(&s, "f_branch_mode(int4)").await;
    let config: Option<Vec<String>> = s
        .query_one(
            "SELECT proconfig FROM pg_proc WHERE oid = 'f_branch_mode(int4)'::regprocedure",
            &[],
        )
        .await
        .unwrap()
        .get(0);
    rep.row(&[
        &format!("the function (proconfig {config:?})"),
        &m,
        &render(&r),
        &n,
    ]);
    rep.check(
        "a function setting doesn't show in current_setting",
        m == "NULL",
        &m,
    );
    rep.check(
        "a function setting: no findings, only a NOTICE",
        r.as_ref().is_ok_and(|v| v.is_empty()) && n.contains("NOTICE"),
        &n,
    );
    let (_, r, _) = run(&s, "f_branch(int4)").await;
    rep.check(
        "a function setting turns the check off for that function alone",
        errors(&r) > 0,
        &render(&r),
    );
}

/// Whether a comment in the body can hide an `error`.
async fn in_comment(rep: &mut Report, c: &Client) {
    rep.header(
        "@plpgsql_check_options in a comment",
        &["function", "arguments", "findings"],
    );
    c.batch_execute(
        "CREATE TABLE tc (id int4, a int4, gone int4);
CREATE FUNCTION trg_comment() RETURNS trigger LANGUAGE plpgsql AS $$
-- @plpgsql_check_options: relid = other
BEGIN
  NEW.id := NEW.gone;
  RETURN NEW;
END $$;
CREATE TRIGGER tc_comment BEFORE INSERT ON tc FOR EACH ROW EXECUTE FUNCTION trg_comment();
CREATE FUNCTION f_comment_warn() RETURNS int4 LANGUAGE plpgsql AS $$
-- @plpgsql_check_options: without_warnings = true
DECLARE v int4;
BEGIN
  SELECT big INTO v FROM n LIMIT 1;
  RETURN v;
END $$;
CREATE FUNCTION f_comment_perf() RETURNS int4 LANGUAGE plpgsql AS $$
-- @plpgsql_check_options: performance_warnings = false
DECLARE v int4;
BEGIN
  SELECT big INTO v FROM n LIMIT 1;
  RETURN v;
END $$;
CREATE FUNCTION f_comment_fatal() RETURNS int4 LANGUAGE plpgsql AS $$
-- @plpgsql_check_options: fatal_errors = true, other_warnings = false, extra_warnings = false
BEGIN
  RETURN (SELECT gone FROM tc LIMIT 1);
END $$;
ALTER TABLE tc DROP COLUMN gone;",
    )
    .await
    .unwrap();
    let r = check(c, "trg_comment()", ", 'tc'::regclass").await;
    rep.row(&[
        "trigger on tc, comment relid = other",
        "'tc'::regclass",
        &render(&r),
    ]);
    let hidden = !has(&r, "error", "42703", "gone");
    rep.check(
        "relid in a comment overrides morm's table and hides the dropped column",
        hidden,
        &render(&r),
    );
    let r2 = check(
        c,
        "trg_comment()",
        ", 'tc'::regclass, use_incomment_options => false",
    )
    .await;
    rep.row(&[
        "trigger on tc, comment relid = other",
        "'tc'::regclass, use_incomment_options => false",
        &render(&r2),
    ]);
    for (sig, label) in [
        (
            "f_comment_warn()",
            "narrowing, comment without_warnings = true",
        ),
        (
            "f_comment_perf()",
            "narrowing, comment performance_warnings = false",
        ),
        (
            "f_comment_fatal()",
            "dropped column, comment fatal_errors and warnings off",
        ),
    ] {
        let r = check(c, sig, "").await;
        rep.row(&[label, "", &render(&r)]);
        let r = check(c, sig, ", use_incomment_options => false").await;
        rep.row(&[label, "use_incomment_options => false", &render(&r)]);
    }
}

/// SH and EF3: which pragma lines hide R-1080's narrowing, and whether a plain
/// word search of `prosrc` sees each one. `E''` and `U&''` literals reach
/// `plpgsql_check` decoded, so their escapes hide the words from the search.
async fn pragmas(rep: &mut Report, c: &Client) {
    rep.header(
        "Pragmas and R-1080's narrowing (an int4 variable reading int8)",
        &[
            "line in the function",
            "narrowing reported",
            "prosrc ~* 'performance_warnings'",
            "prosrc ~* 'disable:check'",
            "findings",
        ],
    );
    let read = "SELECT big INTO v FROM n LIMIT 1;";
    // (name, DECLARE extra, body before the read, wrap the read in a block with this line, hidden?)
    let cases: Vec<(&str, &str, String, Option<&str>, bool)> = vec![
        ("none", "", String::new(), None, false),
        ("PERFORM 'PRAGMA:DISABLE:PERFORMANCE_WARNINGS'", "", "PERFORM 'PRAGMA:DISABLE:PERFORMANCE_WARNINGS';".into(), None, true),
        ("the same in lower case", "", "PERFORM 'pragma:disable:performance_warnings';".into(), None, true),
        ("PERFORM plpgsql_check_pragma('disable:performance_warnings')", "", "PERFORM plpgsql_check_pragma('disable:performance_warnings');".into(), None, true),
        ("aux := plpgsql_check_pragma(...) in DECLARE", "aux int4 := plpgsql_check_pragma('disable:performance_warnings');", String::new(), None, true),
        ("PRAGMA:DISABLE:CHECK in a block around the read", "", String::new(), Some("PERFORM 'PRAGMA:DISABLE:CHECK';"), true),
        ("disable:other_warnings", "", "PERFORM 'PRAGMA:DISABLE:OTHER_WARNINGS';".into(), None, false),
        ("disable:extra_warnings", "", "PERFORM 'PRAGMA:DISABLE:EXTRA_WARNINGS';".into(), None, false),
        ("disable:security_warnings", "", "PERFORM 'PRAGMA:DISABLE:SECURITY_WARNINGS';".into(), None, false),
        ("disable:compatibility_warnings", "", "PERFORM 'PRAGMA:DISABLE:COMPATIBILITY_WARNINGS';".into(), None, false),
        ("disable:performance_warnings in an inner block ending before the read", "", "BEGIN PERFORM 'PRAGMA:DISABLE:PERFORMANCE_WARNINGS'; END;".into(), None, false),
        ("disable then enable:performance_warnings", "", "PERFORM 'PRAGMA:DISABLE:PERFORMANCE_WARNINGS'; PERFORM 'PRAGMA:ENABLE:PERFORMANCE_WARNINGS';".into(), None, false),
        ("'PRAGMA: disable : performance_warnings', with spaces", "", "PERFORM 'PRAGMA: disable : performance_warnings';".into(), None, false),
        ("E'PRAGMA:DISABLE:PERFORMANCE\\x5fWARNINGS'", "", "PERFORM E'PRAGMA:DISABLE:PERFORMANCE\\x5fWARNINGS';".into(), None, true),
        ("U&'PRAGMA:DISABLE:PERFORMANCE\\005FWARNINGS'", "", "PERFORM U&'PRAGMA:DISABLE:PERFORMANCE\\005FWARNINGS';".into(), None, true),
        ("plpgsql_check_pragma(E'disable:performance\\x5fwarnings')", "", "PERFORM plpgsql_check_pragma(E'disable:performance\\x5fwarnings');".into(), None, true),
        ("plpgsql_check_pragma('disable:performance' || '_warnings')", "", "PERFORM plpgsql_check_pragma('disable:performance' || '_warnings');".into(), None, false),
        ("E'PRAGMA:DISABLE:\\x43HECK' in a block around the read", "", String::new(), Some("PERFORM E'PRAGMA:DISABLE:\\x43HECK';"), true),
    ];
    for (i, (name, decl, before, block, hidden)) in cases.into_iter().enumerate() {
        let f = format!("f_pragma_{i}");
        let body = match block {
            Some(line) => format!("BEGIN\n    {line}\n    {read}\n  END;"),
            None => read.to_owned(),
        };
        let sql = format!(
            "CREATE FUNCTION {f}() RETURNS int4 LANGUAGE plpgsql AS $$\nDECLARE v int4; {decl}\nBEGIN\n  {before}\n  {body}\n  RETURN v;\nEND $$;"
        );
        let made = exec(c, &sql).await;
        if made != "ok" {
            rep.check(&format!("{name}: the function is created"), false, &made);
            continue;
        }
        let r = check(c, &format!("{f}()"), "").await;
        let reported = r.as_ref().is_ok_and(|v| {
            v.iter().any(|x| {
                x.level == "performance"
                    && x.sqlstate == "42804"
                    && x.detail.as_deref() == Some("cast \"bigint\" value to \"integer\" type")
            })
        });
        let hits = c
            .query_one(
                &format!("SELECT prosrc ~* 'performance_warnings', prosrc ~* 'disable:check' FROM pg_proc WHERE oid = '{f}()'::regprocedure"),
                &[],
            )
            .await
            .unwrap();
        let (perf, dis): (bool, bool) = (hits.get(0), hits.get(1));
        let ran = c
            .query_one(&format!("SELECT {f}()::text"), &[])
            .await
            .map(|r| r.get::<_, String>(0))
            .unwrap_or_else(|e| show(&e));
        rep.row(&[
            name,
            &reported.to_string(),
            &perf.to_string(),
            &dis.to_string(),
            &render(&r),
        ]);
        rep.check(
            &format!(
                "{name}: the narrowing is {}",
                if hidden { "hidden" } else { "reported" }
            ),
            reported != hidden,
            &render(&r),
        );
        rep.check(&format!("{name}: the function runs"), ran == "1", &ran);
        if name.starts_with("E'") || name.starts_with("U&'") || name.contains("(E'") {
            rep.check(
                &format!("{name}: a plain word search misses it"),
                !perf && !dis,
                &format!("performance_warnings {perf}, disable:check {dis}"),
            );
        }
    }
}
