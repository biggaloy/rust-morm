//! The stale-statement design's evidence (SA and SB, second review EF9): what a
//! session that cached statements and compiled functions before an enum swap
//! gets after it, with and without the relabel (each label of the kept type
//! renamed `_morm_retired_<label>`) and the reset (an operator created and
//! dropped, then the schema comment raised), and what a no-op `ALTER
//! FUNCTION ... COST` and `ALTER PROCEDURE ... SECURITY` run by the owner,
//! who isn't superuser, do for it.
//!
//! The swap always runs straight on the primary, `SPIKE_DSN`, as the type's
//! owner. `SPIKE_P19_VIA` says where the old session is: `direct`, `bouncer`
//! (through `SPIKE_BOUNCER_DSN`, one server connection, prepared statements
//! kept) or `standby` (on `SPIKE_STANDBY_DSN`).

use enum_proofs::{Report, Session, connect, env, must_connect, server_version, show, wait_replay};
use tokio_postgres::types::{ToSql, Type};
use tokio_postgres::{Client, Statement};

const OWNER: &str = "p19_owner";

fn dsn_for(base: &str, user: &str) -> String {
    let kept: Vec<&str> = base
        .split_whitespace()
        .filter(|kv| !kv.starts_with("user="))
        .collect();
    format!("{} user={user}", kept.join(" "))
}

#[derive(Clone, Copy, PartialEq)]
enum Variant {
    None,
    Relabel,
    Reset,
    Both,
}

impl Variant {
    fn name(self) -> &'static str {
        match self {
            Variant::None => "no relabel, no reset",
            Variant::Relabel => "relabel only",
            Variant::Reset => "reset only",
            Variant::Both => "relabel and reset",
        }
    }
    fn tag(self) -> &'static str {
        match self {
            Variant::None => "none",
            Variant::Relabel => "relabel",
            Variant::Reset => "reset",
            Variant::Both => "both",
        }
    }
    fn relabel(self) -> bool {
        matches!(self, Variant::Relabel | Variant::Both)
    }
    fn reset(self) -> bool {
        matches!(self, Variant::Reset | Variant::Both)
    }
}

/// One cached statement or call, and what it must give after the swap under
/// each variant: none, relabel only, reset only, both.
struct Case {
    name: &'static str,
    sql: String,
    types: Vec<Type>,
    args: Vec<&'static str>,
    writes: bool,
    expect: [&'static str; 4],
}

fn cases(s: &str) -> Vec<Case> {
    let st = format!(r#""{s}"."st""#);
    vec![
        Case {
            name: "b_lt('a','c'), (($1::st) < ($2::st))::text",
            sql: format!("SELECT (($1::{st}) < ($2::{st}))::text"),
            types: vec![Type::TEXT, Type::TEXT],
            args: vec!["a", "c"],
            writes: false,
            expect: ["true", "22P02 retired", "false", "false"],
        },
        Case {
            name: "b_const, ('a'::st < 'c'::st)::text",
            sql: format!("SELECT ('a'::{st} < 'c'::{st})::text"),
            types: vec![],
            args: vec![],
            writes: false,
            expect: ["true", "true", "false", "false"],
        },
        Case {
            name: "b_audit('c'), INSERT ... WHERE 'a'::st < ($1::st)",
            sql: format!(
                r#"INSERT INTO "{s}"."audit" (note) SELECT 'a<' || ($1::text) WHERE 'a'::{st} < ($1::{st}) RETURNING note"#
            ),
            types: vec![Type::TEXT],
            args: vec!["c"],
            writes: true,
            expect: ["a<c", "22P02 retired", "no row", "no row"],
        },
        Case {
            name: "a free (($1::st))::text with 'a'",
            sql: format!("SELECT (($1::{st}))::text"),
            types: vec![Type::TEXT],
            args: vec!["a"],
            writes: false,
            expect: ["a", "22P02 retired", "a", "a"],
        },
        Case {
            name: "f_const(), RETURN ('a'::st < 'c'::st)",
            sql: format!(r#"SELECT "{s}"."f_const"()"#),
            types: vec![],
            args: vec![],
            writes: false,
            expect: ["true", "true", "false", "false"],
        },
        Case {
            name: "f_decl('a'), DECLARE v st; v := p; v < 'c'",
            sql: format!(r#"SELECT "{s}"."f_decl"(($1::text))"#),
            types: vec![Type::TEXT],
            args: vec!["a"],
            writes: false,
            expect: ["true", "22P02 retired", "42883", "22P02 retired"],
        },
        Case {
            name: "p_decl('a'), the same in a procedure",
            sql: format!(r#"CALL "{s}"."p_decl"(($1::text), NULL)"#),
            types: vec![Type::TEXT],
            args: vec!["a"],
            writes: false,
            expect: ["true", "22P02 retired", "42883", "22P02 retired"],
        },
        Case {
            name: "f_two('a','c'), two variables, v < w",
            sql: format!(r#"SELECT "{s}"."f_two"(($1::text), ($2::text))"#),
            types: vec![Type::TEXT, Type::TEXT],
            args: vec!["a", "c"],
            writes: false,
            expect: ["true", "22P02 retired", "true", "22P02 retired"],
        },
        Case {
            name: "f_row(1), a t%ROWTYPE variable, (r.n + 1)::text",
            sql: format!(r#"SELECT "{s}"."f_row"(($1::text)::int4)"#),
            types: vec![Type::TEXT],
            args: vec!["1"],
            writes: false,
            expect: ["42804", "42804", "2", "2"],
        },
        Case {
            name: "s_res, SELECT n FROM t, whose type changes",
            sql: format!(r#"SELECT n FROM "{s}"."t" WHERE id = ($1::int4)"#),
            types: vec![Type::INT4],
            args: vec![],
            writes: false,
            expect: ["0A000", "0A000", "0A000", "0A000"],
        },
    ]
}

/// What a run gave, as a short token: the first column's text, `no row`, or
/// the SQLSTATE, with `retired` when a `22P02` names a `_morm_retired_` type.
fn token(r: &Result<Vec<tokio_postgres::Row>, tokio_postgres::Error>) -> (String, String) {
    match r {
        Ok(rows) if rows.is_empty() => ("no row".to_owned(), "no row".to_owned()),
        Ok(rows) => {
            let v = match rows[0].try_get::<_, Option<String>>(0) {
                Ok(v) => v.unwrap_or_else(|| "NULL".to_owned()),
                Err(_) => "a non-text value".to_owned(),
            };
            (v.clone(), v)
        }
        Err(e) => {
            let code = e
                .as_db_error()
                .map(|d| d.code().code().to_owned())
                .unwrap_or_else(|| "client".to_owned());
            let retired = e
                .as_db_error()
                .is_some_and(|d| d.message().contains("_morm_retired_"));
            let t = if code == "22P02" && retired {
                "22P02 retired".to_owned()
            } else {
                code
            };
            (t, show(e))
        }
    }
}

async fn run(
    c: &Client,
    st: &Statement,
    case: &Case,
) -> Result<Vec<tokio_postgres::Row>, tokio_postgres::Error> {
    if case.types.first() == Some(&Type::INT4) {
        return c.query(st, &[&1_i32]).await;
    }
    let args: Vec<&(dyn ToSql + Sync)> =
        case.args.iter().map(|a| a as &(dyn ToSql + Sync)).collect();
    c.query(st, &args).await
}

fn setup_sql(s: &str) -> String {
    format!(
        r#"DROP SCHEMA IF EXISTS "{s}" CASCADE;
DROP SCHEMA IF EXISTS "{s}_morm" CASCADE;
CREATE SCHEMA "{s}" AUTHORIZATION {OWNER};
CREATE SCHEMA "{s}_morm" AUTHORIZATION {OWNER};
SET ROLE {OWNER};
CREATE TYPE "{s}"."st" AS ENUM ('a', 'b', 'c');
CREATE TABLE "{s}"."t" (id int4, s "{s}"."st", n int4);
INSERT INTO "{s}"."t" VALUES (1, 'a', 1), (2, 'c', 2);
CREATE TABLE "{s}"."audit" (note text);
CREATE TABLE "{s}"."other" (x int4);
CREATE FUNCTION "{s}"."f_const"() RETURNS text LANGUAGE plpgsql AS $$
BEGIN RETURN ('a'::"{s}"."st" < 'c'::"{s}"."st")::text; END $$;
CREATE FUNCTION "{s}"."f_decl"(p text) RETURNS text LANGUAGE plpgsql AS $$
DECLARE v "{s}"."st";
BEGIN v := p; RETURN (v < 'c'::"{s}"."st")::text; END $$;
CREATE PROCEDURE "{s}"."p_decl"(p text, INOUT r text) LANGUAGE plpgsql AS $$
DECLARE v "{s}"."st";
BEGIN v := p; r := (v < 'c'::"{s}"."st")::text; END $$;
CREATE FUNCTION "{s}"."f_two"(p text, q text) RETURNS text LANGUAGE plpgsql AS $$
DECLARE v "{s}"."st"; w "{s}"."st";
BEGIN v := p; w := q; RETURN (v < w)::text; END $$;
CREATE FUNCTION "{s}"."f_row"(p int4) RETURNS text LANGUAGE plpgsql AS $$
DECLARE r "{s}"."t"%ROWTYPE;
BEGIN SELECT * INTO r FROM "{s}"."t" WHERE id = p; RETURN (r.n + 1)::text; END $$;
RESET ROLE;
GRANT USAGE ON SCHEMA "{s}" TO PUBLIC;"#
    )
}

/// SA's swap, shaped like the online cutover with the old type kept.
fn swap_sql(s: &str, v: Variant) -> String {
    let mut q = format!(
        r#"BEGIN;
ALTER TYPE "{s}"."st" RENAME TO "_morm_retired_m2_st";
CREATE TYPE "{s}"."st" AS ENUM ('c', 'b', 'a', 'd');
ALTER TABLE "{s}"."t" RENAME COLUMN s TO "_morm_retired_m2_s";
ALTER TABLE "{s}"."t" ADD COLUMN s "{s}"."st";
UPDATE "{s}"."t" SET s = "_morm_retired_m2_s"::text::"{s}"."st";
ALTER TABLE "{s}"."t" ALTER COLUMN n TYPE int8;
"#
    );
    if v.relabel() {
        for l in ["a", "b", "c"] {
            q.push_str(&format!(
                r#"ALTER TYPE "{s}"."_morm_retired_m2_st" RENAME VALUE '{l}' TO '_morm_retired_{l}';
"#
            ));
        }
    }
    if v.reset() {
        q.push_str(&format!(
            r#"CREATE OPERATOR "{s}_morm".### (LEFTARG = int4, RIGHTARG = int4, FUNCTION = int4eq);
DROP OPERATOR "{s}_morm".### (int4, int4);
COMMENT ON SCHEMA "{s}_morm" IS 'morm statement generation 2';
"#
        ));
    }
    q.push_str("COMMIT;");
    q
}

#[tokio::main]
async fn main() {
    let dsn = env("SPIKE_DSN");
    let via = env("SPIKE_P19_VIA");
    let app_dsn = match via.as_str() {
        "direct" => dsn.clone(),
        "bouncer" => env("SPIKE_BOUNCER_DSN"),
        "standby" => env("SPIKE_STANDBY_DSN"),
        other => panic!("SPIKE_P19_VIA = {other}: use direct, bouncer or standby"),
    };
    let m = must_connect(&dsn).await;
    let version = server_version(&m).await;
    m.batch_execute(&format!(
        "DO $$ BEGIN CREATE ROLE {OWNER} LOGIN NOSUPERUSER; EXCEPTION WHEN duplicate_object THEN NULL; END $$"
    ))
    .await
    .unwrap();
    let owner = must_connect(&dsn_for(&dsn, OWNER)).await;
    let is_su: bool = owner
        .query_one(
            "SELECT rolsuper FROM pg_roles WHERE rolname = current_user",
            &[],
        )
        .await
        .unwrap()
        .get(0);
    println!("# The stale-statement design on Postgres {version}, old session {via}");
    let mut rep = Report::default();
    rep.check("the swap's role isn't superuser", !is_su, OWNER);
    let watch = if via == "standby" {
        Some(must_connect(&app_dsn).await)
    } else {
        None
    };
    for v in [
        Variant::None,
        Variant::Relabel,
        Variant::Reset,
        Variant::Both,
    ] {
        variant(&mut rep, &m, &owner, watch.as_ref(), &app_dsn, &via, v).await;
    }
    if via == "direct" {
        open_transaction(&mut rep, &m, &owner, &app_dsn).await;
    }
    rep.finish(&format!("p19 ({via})"));
}

async fn replay(m: &Client, watch: Option<&Session>) {
    if let Some(w) = watch {
        wait_replay(m, w).await;
    }
}

async fn variant(
    rep: &mut Report,
    m: &Client,
    owner: &Client,
    watch: Option<&Session>,
    app_dsn: &str,
    via: &str,
    v: Variant,
) {
    let s = format!("p19_{via}_{}", v.tag());
    m.batch_execute(&setup_sql(&s)).await.unwrap();
    replay(m, watch).await;
    let a = must_connect(app_dsn).await;
    let all = cases(&s);
    let mut kept = Vec::new();
    rep.header(
        &format!("{}: before the swap, then after it", v.name()),
        &[
            "cached statement or call",
            "before",
            "after",
            "expected after",
            "error",
        ],
    );
    for c in all.iter().filter(|c| !(c.writes && via == "standby")) {
        let st = a
            .prepare_typed(&c.sql, &c.types)
            .await
            .unwrap_or_else(|e| panic!("{}: {}", c.name, show(&e)));
        let before = token(&run(&a, &st, c).await).0;
        kept.push((c, st, before));
    }
    owner.batch_execute(&swap_sql(&s, v)).await.unwrap();
    replay(m, watch).await;
    let col = v as usize;
    for (c, st, before) in &kept {
        let (after, detail) = token(&run(&a, st, c).await);
        let want = c.expect[col];
        rep.row(&[c.name, before, &after, want, &detail]);
        rep.check(
            &format!("{} / {}", v.name(), c.name),
            after == want,
            &detail,
        );
        if want == "42804" {
            rep.check(
                &format!("{} / {}: routine", v.name(), c.name),
                detail.contains("[plpgsql_param_eval_recfield]"),
                &detail,
            );
        }
    }
    if v.relabel() {
        let rows = m
            .query(
                &format!(r#"SELECT "_morm_retired_m2_s"::text FROM "{s}"."t" ORDER BY id"#),
                &[],
            )
            .await
            .unwrap();
        let got: Vec<String> = rows.iter().map(|r| r.get(0)).collect();
        rep.check(
            &format!(
                "{}: the retired column keeps its rows, read as placeholders",
                v.name()
            ),
            got == ["_morm_retired_a", "_morm_retired_c"],
            &format!("{got:?}"),
        );
    }
    if v == Variant::Relabel || v == Variant::Both {
        alter(rep, m, owner, watch, &a, &s, v).await;
    }
}

/// SB: the owner's no-op ALTER makes the old session compile the function
/// and the procedure again.
async fn alter(
    rep: &mut Report,
    m: &Client,
    owner: &Client,
    watch: Option<&Session>,
    a: &Client,
    s: &str,
    v: Variant,
) {
    let cost = owner
        .batch_execute(&format!(
            r#"ALTER PROCEDURE "{s}"."p_decl"(text, text) COST 100"#
        ))
        .await;
    let cost = cost.map(|_| "ok".to_owned()).unwrap_or_else(|e| show(&e));
    rep.row(&["ALTER PROCEDURE ... COST 100", "", &cost, "an error", ""]);
    rep.check(
        &format!("{}: a procedure takes no COST", v.name()),
        cost.contains("invalid attribute in procedure definition"),
        &cost,
    );
    let row = |n: &'static str| {
        format!(
            r#"SELECT xmin::text || ctid::text FROM pg_proc WHERE oid = '"{s}"."{n}"'::regproc"#
        )
    };
    let before_f: String = m.query_one(&row("f_decl"), &[]).await.unwrap().get(0);
    let before_p: String = m.query_one(&row("p_decl"), &[]).await.unwrap().get(0);
    owner
        .batch_execute(&format!(
            r#"ALTER FUNCTION "{s}"."f_decl"(text) COST 100;
ALTER FUNCTION "{s}"."f_two"(text, text) COST 100;
ALTER PROCEDURE "{s}"."p_decl"(text, text) SECURITY INVOKER;"#
        ))
        .await
        .unwrap();
    let after_f: String = m.query_one(&row("f_decl"), &[]).await.unwrap().get(0);
    let after_p: String = m.query_one(&row("p_decl"), &[]).await.unwrap().get(0);
    rep.check(
        &format!(
            "{}: ALTER FUNCTION ... COST <same> writes a new pg_proc row",
            v.name()
        ),
        before_f != after_f,
        &format!("{before_f} -> {after_f}"),
    );
    rep.check(
        &format!(
            "{}: ALTER PROCEDURE ... SECURITY <same> writes a new pg_proc row",
            v.name()
        ),
        before_p != after_p,
        &format!("{before_p} -> {after_p}"),
    );
    replay(m, watch).await;
    rep.header(
        &format!(
            "{}: after the owner's no-op ALTER, in the same old session",
            v.name()
        ),
        &["call", "result", "expected"],
    );
    for c in cases(s).iter().filter(|c| {
        c.name.starts_with("f_decl") || c.name.starts_with("p_decl") || c.name.starts_with("f_two")
    }) {
        let st = a.prepare_typed(&c.sql, &c.types).await;
        let got = match st {
            Ok(st) => token(&run(a, &st, c).await),
            Err(e) => (show(&e), show(&e)),
        };
        rep.row(&[c.name, &got.1, "false"]);
        rep.check(
            &format!("{}: after ALTER, {}", v.name(), c.name),
            got.0 == "false",
            &got.1,
        );
    }
}

/// A `READ COMMITTED` transaction open across the swap keeps the old answer
/// until it next takes a lock (README notice R-1251). The swap here has the
/// reset and no relabel, so the old type still knows the labels.
async fn open_transaction(rep: &mut Report, m: &Client, owner: &Client, dsn: &str) {
    let s = "p19_direct_open";
    m.batch_execute(&setup_sql(s)).await.unwrap();
    let a = connect(dsn).await.unwrap();
    let st = a
        .prepare_typed(
            &format!(r#"SELECT (($1::"{s}"."st") < ($2::"{s}"."st"))::text"#),
            &[Type::TEXT, Type::TEXT],
        )
        .await
        .unwrap();
    a.batch_execute("BEGIN ISOLATION LEVEL READ COMMITTED")
        .await
        .unwrap();
    let before = ask(&a, &st).await;
    owner
        .batch_execute(&swap_sql(s, Variant::Reset))
        .await
        .unwrap();
    let same_tx = ask(&a, &st).await;
    a.query(&format!(r#"SELECT count(*) FROM "{s}"."other""#), &[])
        .await
        .unwrap();
    let after_lock = ask(&a, &st).await;
    a.batch_execute("COMMIT").await.unwrap();
    rep.header(
        "A READ COMMITTED transaction open across the swap, 'b' < 'c' (true before, false after)",
        &["when", "result"],
    );
    rep.row(&["before the swap", &before]);
    rep.row(&["after it commits, same transaction", &same_tx]);
    rep.row(&["after the transaction reads another table", &after_lock]);
    rep.check("before: true", before == "true", &before);
    rep.check(
        "open across the swap: the old answer until a lock",
        same_tx == "true",
        &same_tx,
    );
    rep.check(
        "after reading another table: the new answer",
        after_lock == "false",
        &after_lock,
    );
}

async fn ask(c: &Client, st: &Statement) -> String {
    c.query_one(st, &[&"b", &"c"])
        .await
        .map(|r| r.get::<_, String>(0))
        .unwrap_or_else(|e| show(&e))
}
