//! Proofs 14 and 15, written once and run straight against Postgres (14, 15),
//! through PgBouncer in transaction mode (16) and on a hot standby (17).
//!
//! The app side connects to the target, and the migration side always connects
//! straight to the primary, as `morm-admin` does.

use crate::{Label, Report, Session, connect, label, must_connect, show, wait_replay};
use tokio_postgres::types::{ToSql, Type};
use tokio_postgres::{Client, Error, Row, Statement};

pub enum Via {
    Direct,
    Bouncer { admin_dsn: String },
    Standby { watch: Session },
}

pub struct Target {
    /// Shown in every row, such as `pg 18 direct` or `pgbouncer 1.22.0`.
    pub name: String,
    /// Schemas this target makes start with it, so targets on one server don't meet.
    pub prefix: String,
    pub app_dsn: String,
    /// The migration's connection, straight to the primary.
    pub m: Session,
    pub via: Via,
    /// A locale to read the stale errors in, such as `de_DE.UTF-8`.
    pub lc_messages: Option<String>,
}

impl Target {
    pub async fn sync(&self) {
        if let Via::Standby { watch } = &self.via {
            wait_replay(&self.m, watch).await;
        }
    }
    pub async fn migrate(&self, sql: &str) {
        self.m
            .batch_execute(sql)
            .await
            .unwrap_or_else(|e| panic!("migration failed: {}\n{sql}", show(&e)));
        self.sync().await;
    }
    pub async fn app(&self) -> Session {
        must_connect(&self.app_dsn).await
    }
    fn bouncer(&self) -> bool {
        matches!(self.via, Via::Bouncer { .. })
    }
    fn standby(&self) -> bool {
        matches!(self.via, Via::Standby { .. })
    }
    /// PgBouncer's `RECONNECT`: every server connection closes once released.
    async fn reconnect(&self) -> String {
        let Via::Bouncer { admin_dsn } = &self.via else {
            unreachable!()
        };
        let admin = must_connect(admin_dsn).await;
        let r = match admin.simple_query("RECONNECT").await {
            Ok(_) => "ok".to_owned(),
            Err(e) => show(&e),
        };
        // Server connections close as they're released; give PgBouncer a moment.
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        r
    }
    /// The backend pid a new statement runs on, to show which server session a
    /// client reached.
    async fn pid(c: &Client) -> String {
        match c.query_one("SELECT pg_backend_pid()", &[]).await {
            Ok(r) => r.get::<_, i32>(0).to_string(),
            Err(e) => show(&e),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    /// The old type is dropped after the swap.
    Drop,
    /// The old type is kept, renamed to `st_old`.
    Keep,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Drop => "drop",
            Mode::Keep => "keep",
        }
    }
}

/// The labels' rows: `a` is row 1, and `d`, which only the new type has, row 4.
fn id_of(l: &str) -> i32 {
    match l {
        "a" => 1,
        "b" => 2,
        "c" => 3,
        "d" => 4,
        _ => unreachable!(),
    }
}

fn setup_sql(sch: &str) -> String {
    format!(
        r#"DROP SCHEMA IF EXISTS "{sch}" CASCADE;
CREATE SCHEMA "{sch}";
CREATE TYPE "{sch}"."st" AS ENUM ('a', 'b', 'c');
CREATE TABLE "{sch}"."t" (id int4 PRIMARY KEY, s "{sch}"."st" NOT NULL, ss "{sch}"."st"[] NOT NULL, note text);
INSERT INTO "{sch}"."t" VALUES (1, 'a', '{{a}}'), (2, 'b', '{{b}}'), (3, 'c', '{{c}}'),
  (11, 'a', '{{a}}'), (12, 'b', '{{b}}'), (13, 'c', '{{c}}');"#
    )
}

/// A swap as `--fast` runs it, in one transaction: the new type has the old
/// labels in another order plus `d`, and row 4 holds `d`.
fn swap_sql(sch: &str, mode: Mode) -> String {
    let drop = match mode {
        Mode::Drop => format!(r#"DROP TYPE "{sch}"."st_old";"#),
        Mode::Keep => String::new(),
    };
    format!(
        r#"BEGIN;
ALTER TYPE "{sch}"."st" RENAME TO "st_old";
CREATE TYPE "{sch}"."st" AS ENUM ('c', 'b', 'a', 'd');
ALTER TABLE "{sch}"."t"
  ALTER COLUMN s TYPE "{sch}"."st" USING s::text::"{sch}"."st",
  ALTER COLUMN ss TYPE "{sch}"."st"[] USING ss::text[]::"{sch}"."st"[];
INSERT INTO "{sch}"."t" VALUES (4, 'd', '{{d}}'), (14, 'd', '{{d}}');
{drop}
COMMIT;"#
    )
}

#[derive(Clone, Copy, PartialEq)]
enum Stmt {
    /// Returns the column: `SELECT s FROM t WHERE id = ($1::int4)`.
    RetCol,
    /// An enum parameter on the swapped table.
    Param,
    /// An enum array parameter on the swapped table.
    Array,
    /// An insert of the label, returning it as text.
    Insert,
    /// An update filtered by the label, returning it as text.
    Update,
    /// An array column compared with an array built from the parameter.
    ArrCol,
    /// Returns the type, touching no table of it: `SELECT ($1::st)`.
    FreeRet,
    /// `SELECT ($1::st)::text`.
    FreeParam,
    /// `SELECT ($1::st[])::text[]`.
    FreeArray,
}

const STATEMENTS: [Stmt; 9] = [
    Stmt::RetCol,
    Stmt::Param,
    Stmt::Array,
    Stmt::Insert,
    Stmt::Update,
    Stmt::ArrCol,
    Stmt::FreeRet,
    Stmt::FreeParam,
    Stmt::FreeArray,
];

impl Stmt {
    fn sql(self, sch: &str) -> String {
        let st = format!(r#""{sch}"."st""#);
        let t = format!(r#""{sch}"."t""#);
        match self {
            Stmt::RetCol => format!("SELECT s FROM {t} WHERE id = ($1::int4)"),
            Stmt::Param => {
                format!("SELECT id FROM {t} WHERE s = ($1::{st}) AND id <= 4 ORDER BY id")
            }
            Stmt::Array => {
                format!("SELECT id FROM {t} WHERE s = ANY(($1::{st}[])) AND id <= 4 ORDER BY id")
            }
            Stmt::Insert => format!(
                "INSERT INTO {t} (id, s, ss) SELECT greatest(max(id), 99) + 1, ($1::{st}), ARRAY[($1::{st})] FROM {t} RETURNING s::text"
            ),
            Stmt::Update => format!(
                "UPDATE {t} SET note = ($1::{st})::text WHERE s = ($1::{st}) AND id <= 4 RETURNING s::text"
            ),
            Stmt::ArrCol => {
                format!("SELECT id FROM {t} WHERE ss @> ARRAY[($1::{st})] AND id <= 4 ORDER BY id")
            }
            Stmt::FreeRet => format!("SELECT ($1::{st})"),
            Stmt::FreeParam => format!("SELECT ($1::{st})::text"),
            Stmt::FreeArray => format!("SELECT ($1::{st}[])::text[]"),
        }
    }
    fn name(self) -> &'static str {
        match self {
            Stmt::RetCol => "returns the column",
            Stmt::Param => "enum parameter",
            Stmt::Array => "enum array parameter",
            Stmt::Insert => "insert",
            Stmt::Update => "update",
            Stmt::ArrCol => "array column @> ARRAY[$1]",
            Stmt::FreeRet => "no table: SELECT ($1::st)",
            Stmt::FreeParam => "no table: SELECT ($1::st)::text",
            Stmt::FreeArray => "no table: SELECT ($1::st[])::text[]",
        }
    }
    fn carrier(self) -> Type {
        match self {
            Stmt::RetCol => Type::INT4,
            Stmt::Param
            | Stmt::Insert
            | Stmt::Update
            | Stmt::ArrCol
            | Stmt::FreeRet
            | Stmt::FreeParam => Type::TEXT,
            Stmt::Array | Stmt::FreeArray => Type::TEXT_ARRAY,
        }
    }
    fn expected(self, l: &str) -> String {
        match self {
            Stmt::RetCol | Stmt::FreeRet | Stmt::FreeParam | Stmt::Insert | Stmt::Update => {
                l.to_owned()
            }
            Stmt::Param | Stmt::ArrCol => format!("[{}]", id_of(l)),
            Stmt::Array => {
                let mut ids = vec![1, id_of(l)];
                ids.dedup();
                format!("{ids:?}")
            }
            Stmt::FreeArray => format!("{{{l},a}}"),
        }
    }
    fn render(self, rows: &[Row]) -> Result<String, Error> {
        Ok(match self {
            Stmt::RetCol | Stmt::FreeRet => match rows.first() {
                Some(r) => r.try_get::<_, Label>(0)?.0,
                None => "no row".to_owned(),
            },
            Stmt::Insert | Stmt::Update => match rows.first() {
                Some(r) => r.try_get::<_, String>(0)?,
                None => "no row".to_owned(),
            },
            Stmt::Param | Stmt::Array | Stmt::ArrCol => {
                let ids: Result<Vec<i32>, Error> = rows.iter().map(|r| r.try_get(0)).collect();
                format!("{:?}", ids?)
            }
            Stmt::FreeParam => rows[0].try_get::<_, String>(0)?,
            Stmt::FreeArray => format!("{{{}}}", rows[0].try_get::<_, Vec<String>>(0)?.join(",")),
        })
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Ctx {
    Out,
    In,
}

impl Ctx {
    fn name(self) -> &'static str {
        match self {
            Ctx::Out => "outside a transaction",
            Ctx::In => "in a transaction",
        }
    }
}

enum How<'a> {
    Prepared(&'a Statement),
    Typed,
}

async fn run_once(c: &Client, how: &How<'_>, s: Stmt, sch: &str, l: &str) -> Result<String, Error> {
    let id = id_of(l);
    let one = label(l);
    let list = vec![label(l), label("a")];
    let p: &(dyn ToSql + Sync) = match s {
        Stmt::RetCol => &id,
        Stmt::Param
        | Stmt::Insert
        | Stmt::Update
        | Stmt::ArrCol
        | Stmt::FreeRet
        | Stmt::FreeParam => &one,
        Stmt::Array | Stmt::FreeArray => &list,
    };
    let rows = match how {
        How::Prepared(st) => c.query(*st, &[p]).await?,
        How::Typed => c.query_typed(&s.sql(sch), &[(p, s.carrier())]).await?,
    };
    s.render(&rows)
}

async fn run(c: &Client, ctx: Ctx, how: &How<'_>, s: Stmt, sch: &str, l: &str) -> String {
    if ctx == Ctx::In {
        c.batch_execute("BEGIN").await.unwrap();
    }
    let r = run_once(c, how, s, sch, l).await;
    if ctx == Ctx::In {
        c.batch_execute("ROLLBACK").await.unwrap();
    }
    outcome(r, &s.expected(l))
}

fn outcome(r: Result<String, Error>, want: &str) -> String {
    match r {
        Ok(v) if v == want => format!("right: {v}"),
        Ok(v) => format!("WRONG: {v}, want {want}"),
        Err(e) => show(&e),
    }
}

async fn prepare_and_run(c: &Client, ctx: Ctx, sql: &str, s: Stmt, sch: &str, l: &str) -> String {
    match c.prepare_typed(sql, &[s.carrier()]).await {
        Ok(p) => run(c, ctx, &How::Prepared(&p), s, sch, l).await,
        Err(e) => format!("prepare: {}", show(&e)),
    }
}

/// Proof 14's statements: cached before a swap, run after it.
pub async fn statements(t: &Target, mode: Mode, rep: &mut Report) {
    let sch = format!("{}_stmt_{}", t.prefix, mode.name());
    rep.header(
        &format!(
            "{}: cached statements, old type {}",
            t.name,
            match mode {
                Mode::Drop => "dropped",
                Mode::Keep => "kept as st_old",
            }
        ),
        &["context", "statement", "step", "outcome"],
    );
    t.migrate(&setup_sql(&sch)).await;

    // A standby can't write, so it runs no insert or update.
    let list: Vec<Stmt> = STATEMENTS
        .into_iter()
        .filter(|s| !(t.standby() && matches!(s, Stmt::Insert | Stmt::Update)))
        .collect();
    let mut ctxs = Vec::new();
    for ctx in [Ctx::Out, Ctx::In] {
        let s = t.app().await;
        let mut stmts = Vec::new();
        for st in list.iter().copied() {
            let p = s
                .prepare_typed(&st.sql(&sch), &[st.carrier()])
                .await
                .unwrap();
            let o = run(&s, ctx, &How::Prepared(&p), st, &sch, "b").await;
            rep.row(&[ctx.name(), st.name(), "before the swap, label b", &o]);
            stmts.push(p);
        }
        ctxs.push((ctx, s, stmts));
    }

    t.migrate(&swap_sql(&sch, mode)).await;

    for (ctx, s, stmts) in &ctxs {
        let ctx = *ctx;
        for (st, p) in list.iter().zip(stmts) {
            let st = *st;
            for (step, l) in [
                ("run 1, label d", "d"),
                ("run 2, label d", "d"),
                ("run 3, label b", "b"),
            ] {
                let o = run(s, ctx, &How::Prepared(p), st, &sch, l).await;
                rep.row(&[ctx.name(), st.name(), step, &o]);
            }
            let o = prepare_and_run(s, ctx, &st.sql(&sch), st, &sch, "d").await;
            let step = if t.bouncer() {
                "prepared again, same client, same text"
            } else {
                "prepared again"
            };
            rep.row(&[ctx.name(), st.name(), step, &o]);
            if t.bouncer() {
                let n = t.app().await;
                let o = prepare_and_run(&n, ctx, &st.sql(&sch), st, &sch, "d").await;
                rep.row(&[ctx.name(), st.name(), "new client, same text", &o]);
                let text = format!("{} /* g2 */", st.sql(&sch));
                let o = prepare_and_run(s, ctx, &text, st, &sch, "d").await;
                rep.row(&[ctx.name(), st.name(), "same client, new text", &o]);
            }
            let o = run(s, ctx, &How::Typed, st, &sch, "d").await;
            rep.row(&[ctx.name(), st.name(), "query_typed, unnamed", &o]);
            let o = run(s, ctx, &How::Prepared(p), st, &sch, "d").await;
            rep.row(&[ctx.name(), st.name(), "old statement again", &o]);
        }
    }

    if let Some(lc) = &t.lc_messages {
        let (_, s, stmts) = &ctxs[0];
        let set = s
            .batch_execute(&format!("SET lc_messages = '{lc}'"))
            .await
            .map(|_| "ok".to_owned())
            .unwrap_or_else(|e| show(&e));
        rep.row(&[
            "outside a transaction",
            "-",
            &format!("SET lc_messages = '{lc}'"),
            &set,
        ]);
        for (st, p) in list.iter().zip(stmts) {
            let o = run(s, Ctx::Out, &How::Prepared(p), *st, &sch, "d").await;
            rep.row(&[
                "outside a transaction",
                st.name(),
                &format!("old statement, lc_messages {lc}"),
                &o,
            ]);
        }
        s.batch_execute("RESET lc_messages").await.unwrap();
    }

    if t.bouncer() {
        let r = t.reconnect().await;
        rep.row(&["-", "-", "PgBouncer RECONNECT", &r]);
        for ctx in [Ctx::Out, Ctx::In] {
            let n = t.app().await;
            for st in list.iter().copied() {
                let o = prepare_and_run(&n, ctx, &st.sql(&sch), st, &sch, "d").await;
                rep.row(&[ctx.name(), st.name(), "after RECONNECT, new client", &o]);
            }
        }
    } else {
        for ctx in [Ctx::Out, Ctx::In] {
            let n = t.app().await;
            for st in list.iter().copied() {
                let o = prepare_and_run(&n, ctx, &st.sql(&sch), st, &sch, "d").await;
                rep.row(&[ctx.name(), st.name(), "new connection", &o]);
            }
        }
    }
}

/// Proof 14's plpgsql part: a function with a `%ROWTYPE` variable, compiled
/// before its table's column changes from `integer` to `bigint`.
pub async fn rowtype_param(t: &Target, rep: &mut Report) {
    let sch = format!("{}_rowtype", t.prefix);
    rep.header(
        &format!(
            "{}: a %ROWTYPE function compiled before a column type change",
            t.name
        ),
        &["session", "step", "outcome"],
    );
    t.migrate(&format!(
        r#"DROP SCHEMA IF EXISTS "{sch}" CASCADE;
CREATE SCHEMA "{sch}";
CREATE TABLE "{sch}"."r" (id int4, v int4);
INSERT INTO "{sch}"."r" VALUES (1, 5);
CREATE FUNCTION "{sch}"."f_row"(p int4) RETURNS text LANGUAGE plpgsql AS $$
DECLARE x "{sch}"."r"%ROWTYPE;
BEGIN
  SELECT * INTO x FROM "{sch}"."r" WHERE id = p;
  RETURN (x.v + 1)::text;
END $$;"#
    ))
    .await;
    let call = format!(r#"SELECT "{sch}"."f_row"(($1::int4))"#);
    let one = 1_i32;
    let go = async |c: &Client| -> String {
        let r = c.query_typed(&call, &[(&one, Type::INT4)]).await;
        outcome(r.and_then(|rows| rows[0].try_get::<_, String>(0)), "6")
    };
    let s = t.app().await;
    rep.row(&["S", "compiled", &go(&s).await]);
    let pid_before = Target::pid(&s).await;
    // Through PgBouncer every client reaches the same server session.
    let s2 = if t.bouncer() {
        None
    } else {
        Some(t.app().await)
    };
    if let Some(s2) = &s2 {
        rep.row(&["S2", "compiled", &go(s2).await]);
    }
    t.migrate(&format!(
        r#"ALTER TABLE "{sch}"."r" ALTER COLUMN v TYPE int8"#
    ))
    .await;
    rep.row(&["S", "call 1 after the change", &go(&s).await]);
    rep.row(&["S", "call 2", &go(&s).await]);
    if let Some(lc) = &t.lc_messages {
        let set = s
            .batch_execute(&format!("SET lc_messages = '{lc}'"))
            .await
            .map(|_| "ok".to_owned())
            .unwrap_or_else(|e| show(&e));
        rep.row(&["S", &format!("SET lc_messages = '{lc}'"), &set]);
        rep.row(&["S", &format!("call, lc_messages {lc}"), &go(&s).await]);
        s.batch_execute("RESET lc_messages").await.unwrap();
    }
    s.batch_execute("DISCARD PLANS").await.unwrap();
    rep.row(&["S", "after DISCARD PLANS", &go(&s).await]);
    if let Some(s2) = &s2 {
        rep.row(&["S2", "call", &go(s2).await]);
        s2.batch_execute("DISCARD ALL").await.unwrap();
        rep.row(&["S2", "after DISCARD ALL", &go(s2).await]);
    }
    drop(s);
    let n = t.app().await;
    let pid_after = Target::pid(&n).await;
    let same = if pid_before == pid_after {
        "same server session"
    } else {
        "another server session"
    };
    rep.row(&["new connection", &format!("call ({same})"), &go(&n).await]);
    if t.bouncer() {
        drop(n);
        let r = t.reconnect().await;
        rep.row(&["-", "PgBouncer RECONNECT", &r]);
        let n = t.app().await;
        rep.row(&["new client after RECONNECT", "call", &go(&n).await]);
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Cure {
    /// A no-op `CREATE OR REPLACE` of each function, from the migration's session.
    Replace,
    /// PgBouncer's `RECONNECT`.
    Reconnect,
}

#[derive(Clone, Copy, PartialEq)]
enum Func {
    Declare,
    Into,
    RowType,
    Trigger,
}

impl Func {
    fn name(self) -> &'static str {
        match self {
            Func::Declare => "DECLARE v st",
            Func::Into => "SELECT ... INTO v st",
            Func::RowType => "%ROWTYPE",
            Func::Trigger => "trigger naming no type",
        }
    }
}

fn functions_sql(sch: &str) -> String {
    format!(
        r#"CREATE OR REPLACE FUNCTION "{sch}"."f_declare"(p text) RETURNS text LANGUAGE plpgsql AS $$
DECLARE v "{sch}"."st";
BEGIN
  v := p;
  RETURN v::text;
END $$;
CREATE OR REPLACE FUNCTION "{sch}"."f_into"(p int4) RETURNS text LANGUAGE plpgsql AS $$
DECLARE v "{sch}"."st";
BEGIN
  SELECT s INTO v FROM "{sch}"."t" WHERE id = p;
  RETURN v::text;
END $$;
CREATE OR REPLACE FUNCTION "{sch}"."f_rowtype"(p int4) RETURNS text LANGUAGE plpgsql AS $$
DECLARE r "{sch}"."t"%ROWTYPE;
BEGIN
  SELECT * INTO r FROM "{sch}"."t" WHERE id = p;
  RETURN r.s::text;
END $$;
CREATE OR REPLACE FUNCTION "{sch}"."trg"() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF NEW.s IS DISTINCT FROM OLD.s THEN
    NEW.note := 'changed';
  ELSE
    NEW.note := 'same';
  END IF;
  RETURN NEW;
END $$;"#
    )
}

async fn call(c: &Client, sch: &str, f: Func, l: &str) -> String {
    let id = id_of(l);
    let text = l.to_owned();
    let r = match f {
        Func::Declare => {
            c.query_typed(
                &format!(r#"SELECT "{sch}"."f_declare"(($1::text))"#),
                &[(&text, Type::TEXT)],
            )
            .await
        }
        Func::Into => {
            c.query_typed(
                &format!(r#"SELECT "{sch}"."f_into"(($1::int4))"#),
                &[(&id, Type::INT4)],
            )
            .await
        }
        Func::RowType => {
            c.query_typed(
                &format!(r#"SELECT "{sch}"."f_rowtype"(($1::int4))"#),
                &[(&id, Type::INT4)],
            )
            .await
        }
        Func::Trigger => {
            // A fresh statement, so only the trigger function was compiled
            // before the swap. It runs on rows 11 to 14, which nothing else
            // reads, and always changes the label (to `a`, or from `a` back
            // to the call's label), so the trigger writes `changed`.
            let to = label(l);
            let row = id + 10;
            c.query_typed(
                &format!(
                    r#"UPDATE "{sch}"."t" SET s = CASE WHEN s::text = 'a' THEN ($1::"{sch}"."st") ELSE 'a'::"{sch}"."st" END WHERE id = ($2::int4) RETURNING note"#
                ),
                &[(&to, Type::TEXT), (&row, Type::INT4)],
            )
            .await
        }
    };
    let want = if f == Func::Trigger { "changed" } else { l };
    outcome(
        r.and_then(|rows| match rows.first() {
            Some(r) => r.try_get::<_, String>(0),
            None => Ok("no row".to_owned()),
        }),
        want,
    )
}

/// Proof 15: plpgsql compiled in a session before a swap, called after it.
pub async fn functions(t: &Target, mode: Mode, cure: Cure, rep: &mut Report) {
    let sch = format!(
        "{}_fn_{}_{}",
        t.prefix,
        mode.name(),
        match cure {
            Cure::Replace => "replace",
            Cure::Reconnect => "reconnect",
        }
    );
    rep.header(
        &format!(
            "{}: plpgsql compiled before a swap, old type {}",
            t.name,
            match mode {
                Mode::Drop => "dropped",
                Mode::Keep => "kept as st_old",
            }
        ),
        &["session", "function", "step", "outcome"],
    );
    t.migrate(&format!(
        "{}\n{}\nCREATE TRIGGER t_trg BEFORE UPDATE ON \"{sch}\".\"t\" FOR EACH ROW EXECUTE FUNCTION \"{sch}\".\"trg\"();",
        setup_sql(&sch),
        functions_sql(&sch)
    ))
    .await;
    // A standby can't write, so the trigger can't fire there.
    let funcs: &[Func] = if t.standby() {
        &[Func::Declare, Func::Into, Func::RowType]
    } else {
        &[Func::Declare, Func::Into, Func::RowType, Func::Trigger]
    };

    // Straight to Postgres and on a standby, each remedy gets its own session,
    // compiled before the swap: S1 calls again and runs DISCARD PLANS, S2 runs
    // DISCARD ALL, and S3 waits for the no-op CREATE OR REPLACE. Through
    // PgBouncer every client reaches the one server session, so the steps
    // run one after another there, on new clients.
    let s1 = t.app().await;
    let (s2, s3) = if t.bouncer() {
        (None, None)
    } else {
        (Some(t.app().await), Some(t.app().await))
    };
    for f in funcs {
        rep.row(&[
            "S1",
            f.name(),
            "compiled, label b",
            &call(&s1, &sch, *f, "b").await,
        ]);
        for (who, s) in [("S2", &s2), ("S3", &s3)] {
            if let Some(s) = s {
                rep.row(&[
                    who,
                    f.name(),
                    "compiled, label b",
                    &call(s, &sch, *f, "b").await,
                ]);
            }
        }
    }

    t.migrate(&swap_sql(&sch, mode)).await;

    for f in funcs {
        for (step, l) in [
            ("call 1, label d", "d"),
            ("call 2, label d", "d"),
            ("call 3, label b", "b"),
        ] {
            rep.row(&["S1", f.name(), step, &call(&s1, &sch, *f, l).await]);
        }
    }
    if t.bouncer() {
        drop(s1);
        let c = t.app().await;
        for f in funcs {
            rep.row(&[
                "new client",
                f.name(),
                "call, label d",
                &call(&c, &sch, *f, "d").await,
            ]);
        }
        c.batch_execute("DISCARD PLANS").await.unwrap();
        for f in funcs {
            rep.row(&[
                "new client",
                f.name(),
                "after DISCARD PLANS",
                &call(&c, &sch, *f, "d").await,
            ]);
        }
        c.batch_execute("DISCARD ALL").await.unwrap();
        for f in funcs {
            rep.row(&[
                "new client",
                f.name(),
                "after DISCARD PLANS, then DISCARD ALL",
                &call(&c, &sch, *f, "d").await,
            ]);
        }
    } else {
        s1.batch_execute("DISCARD PLANS").await.unwrap();
        for f in funcs {
            rep.row(&[
                "S1",
                f.name(),
                "after DISCARD PLANS",
                &call(&s1, &sch, *f, "d").await,
            ]);
        }
        let s2 = s2.as_ref().unwrap();
        s2.batch_execute("DISCARD ALL").await.unwrap();
        for f in funcs {
            rep.row(&[
                "S2",
                f.name(),
                "after DISCARD ALL",
                &call(s2, &sch, *f, "d").await,
            ]);
        }
        let fresh = t.app().await;
        for f in funcs {
            rep.row(&[
                "fresh session",
                f.name(),
                "call, label d",
                &call(&fresh, &sch, *f, "d").await,
            ]);
        }
    }

    match cure {
        Cure::Replace => {
            if let Some(s3) = &s3 {
                for f in funcs {
                    rep.row(&[
                        "S3",
                        f.name(),
                        "before the no-op CREATE OR REPLACE",
                        &call(s3, &sch, *f, "d").await,
                    ]);
                }
            }
            t.migrate(&functions_sql(&sch)).await;
            let fresh;
            let s = match &s3 {
                Some(s3) => s3,
                None => {
                    fresh = t.app().await;
                    &fresh
                }
            };
            let who = if s3.is_some() { "S3" } else { "new client" };
            for f in funcs {
                rep.row(&[
                    who,
                    f.name(),
                    "after a no-op CREATE OR REPLACE elsewhere",
                    &call(s, &sch, *f, "d").await,
                ]);
            }
        }
        Cure::Reconnect => {
            let r = t.reconnect().await;
            rep.row(&["-", "-", "PgBouncer RECONNECT", &r]);
            let c = t.app().await;
            for f in funcs {
                rep.row(&[
                    "new client after RECONNECT",
                    f.name(),
                    "call, label d",
                    &call(&c, &sch, *f, "d").await,
                ]);
            }
        }
    }
}

/// Opens a target's sessions, or says why it can't.
pub async fn try_connect(dsn: &str) -> Result<Session, String> {
    connect(dsn).await.map_err(|e| show(&e))
}
