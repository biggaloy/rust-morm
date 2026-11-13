//! Proof 13: enum and composite binding with no OID and no variant list.
//!
//! A hand-written `ToSql` declares each value as a built-in carrier, `text`
//! or `text[]`, and the SQL casts it to the type's name. A hand-written
//! `FromSql` decodes the enum's label, or the composite's binary record, and
//! never reads a variant or field list. Each check runs through `query_typed`
//! and through a prepared statement, on a connection opened at the start and
//! on one opened in that phase, before and after `ALTER TYPE ... ADD VALUE`,
//! and after a swap of each type. `SPIKE_DSN` is a superuser connection.

use bytes::{BufMut, BytesMut};
use enum_proofs::{Report, Session, env, must_connect, server_version, show};
use std::error::Error as StdError;
use tokio_postgres::types::{FromSql, IsNull, Kind, ToSql, Type, to_sql_checked};
use tokio_postgres::{Client, Error, Row};

type BoxError = Box<dyn StdError + Sync + Send>;

#[derive(Debug, Clone, Copy, PartialEq)]
enum OrderState {
    New,
    Paid,
    Shipped,
    Refunded,
}

impl OrderState {
    fn label(self) -> &'static str {
        match self {
            OrderState::New => "new",
            OrderState::Paid => "paid",
            OrderState::Shipped => "shipped",
            OrderState::Refunded => "refunded",
        }
    }
}

impl ToSql for OrderState {
    fn to_sql(&self, _: &Type, out: &mut BytesMut) -> Result<IsNull, BoxError> {
        out.put_slice(self.label().as_bytes());
        Ok(IsNull::No)
    }
    /// Only the carrier: the cast in the SQL names the enum.
    fn accepts(ty: &Type) -> bool {
        *ty == Type::TEXT
    }
    to_sql_checked!();
}

impl<'a> FromSql<'a> for OrderState {
    fn from_sql(_: &Type, raw: &'a [u8]) -> Result<Self, BoxError> {
        Ok(match std::str::from_utf8(raw)? {
            "new" => OrderState::New,
            "paid" => OrderState::Paid,
            "shipped" => OrderState::Shipped,
            "refunded" => OrderState::Refunded,
            other => return Err(format!("unknown label {other:?}").into()),
        })
    }
    /// Any enum, whatever its labels, or its text form. The variant list the
    /// driver keeps in `Kind::Enum` is never read.
    fn accepts(ty: &Type) -> bool {
        *ty == Type::TEXT || matches!(ty.kind(), Kind::Enum(_))
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Address {
    street: String,
    zip: i32,
}

impl ToSql for Address {
    /// The composite's text form, `("street",zip)`, in a `text` carrier.
    fn to_sql(&self, _: &Type, out: &mut BytesMut) -> Result<IsNull, BoxError> {
        out.put_u8(b'(');
        out.put_u8(b'"');
        for ch in self.street.chars() {
            if ch == '"' || ch == '\\' {
                out.put_u8(b'\\');
            }
            let mut buf = [0; 4];
            out.put_slice(ch.encode_utf8(&mut buf).as_bytes());
        }
        out.put_u8(b'"');
        out.put_slice(format!(",{})", self.zip).as_bytes());
        Ok(IsNull::No)
    }
    fn accepts(ty: &Type) -> bool {
        *ty == Type::TEXT
    }
    to_sql_checked!();
}

impl<'a> FromSql<'a> for Address {
    /// The binary record: a field count, then each field's OID, length and
    /// bytes. The OIDs on the wire are skipped; no field list is read.
    fn from_sql(_: &Type, raw: &'a [u8]) -> Result<Self, BoxError> {
        let int = |at: usize| -> Result<i32, BoxError> {
            Ok(i32::from_be_bytes(
                raw.get(at..at + 4).ok_or("short record")?.try_into()?,
            ))
        };
        if int(0)? != 2 {
            return Err(format!("expected 2 fields, got {}", int(0)?).into());
        }
        let len = int(8)? as usize;
        let street = std::str::from_utf8(&raw[12..12 + len])?.to_owned();
        let at = 12 + len + 4;
        if int(at)? != 4 {
            return Err("zip isn't 4 bytes".into());
        }
        let zip = int(at + 4)?;
        Ok(Address { street, zip })
    }
    fn accepts(ty: &Type) -> bool {
        matches!(ty.kind(), Kind::Composite(_))
    }
}

const TYPES: &str = r#""p13"."order_state""#;
const ADDR: &str = r#""p13"."address""#;

struct Order {
    id: i32,
    state: OrderState,
    states: Vec<OrderState>,
    addr: Address,
    addrs: Vec<Address>,
}

fn order(id: i32, state: OrderState) -> Order {
    Order {
        id,
        state,
        states: vec![state, OrderState::New],
        addr: Address {
            street: format!("Main \"St\", {id}\\B"),
            zip: 1000 + id,
        },
        addrs: vec![
            Address {
                street: format!("({id})"),
                zip: id,
            },
            Address {
                street: String::new(),
                zip: -id,
            },
        ],
    }
}

fn insert_sql() -> String {
    format!(
        r#"INSERT INTO "p13"."orders" (id, state, states, addr, addrs) VALUES (($1::int4), ($2::{TYPES}), ($3::{TYPES}[]), ($4::{ADDR}), ($5::{ADDR}[]))"#
    )
}
const INSERT_TYPES: [Type; 5] = [
    Type::INT4,
    Type::TEXT,
    Type::TEXT_ARRAY,
    Type::TEXT,
    Type::TEXT_ARRAY,
];

/// The queries a phase runs, each with its carriers.
fn selects() -> Vec<(&'static str, String, Vec<Type>)> {
    let cols = "id, state, states, addr, addrs";
    let t = r#""p13"."orders""#;
    vec![
        (
            "enum parameter",
            format!("SELECT {cols} FROM {t} WHERE state = ($1::{TYPES}) AND id = ($2::int4)"),
            vec![Type::TEXT, Type::INT4],
        ),
        (
            "enum array parameter",
            format!(
                "SELECT {cols} FROM {t} WHERE state = ANY(($1::{TYPES}[])) AND id = ($2::int4)"
            ),
            vec![Type::TEXT_ARRAY, Type::INT4],
        ),
        (
            "composite parameter",
            format!("SELECT {cols} FROM {t} WHERE addr = ($1::{ADDR}) AND id = ($2::int4)"),
            vec![Type::TEXT, Type::INT4],
        ),
        (
            "composite array parameter",
            format!("SELECT {cols} FROM {t} WHERE addr = ANY(($1::{ADDR}[])) AND id = ($2::int4)"),
            vec![Type::TEXT_ARRAY, Type::INT4],
        ),
        (
            "columns read as text",
            format!(
                "SELECT id, state::text, states::text[], addr::text, addrs::text FROM {t} WHERE state = ($1::{TYPES}) AND id = ($2::int4)"
            ),
            vec![Type::TEXT, Type::INT4],
        ),
    ]
}

fn decode(
    row: &Row,
    as_text: bool,
) -> Result<(i32, OrderState, Vec<OrderState>, Address, Vec<Address>), Error> {
    if as_text {
        // The text form: the enum and its array decode from `text` and
        // `text[]`. The composite's text is compared with what Postgres
        // prints for the expected value, and its array only checked to be
        // there, since parsing it adds nothing to the proof.
        let id: i32 = row.try_get(0)?;
        let o = order(id, row.try_get(1)?);
        let addr: String = row.try_get(3)?;
        let addrs: String = row.try_get(4)?;
        let printed = format!(
            "(\"{}\",{})",
            o.addr.street.replace('\\', "\\\\").replace('"', "\"\""),
            o.addr.zip
        );
        let addr = if addr == printed {
            o.addr
        } else {
            Address {
                street: addr,
                zip: 0,
            }
        };
        let addrs = if addrs.starts_with("{\"(") {
            o.addrs
        } else {
            vec![]
        };
        return Ok((id, row.try_get(1)?, row.try_get(2)?, addr, addrs));
    }
    Ok((
        row.try_get(0)?,
        row.try_get(1)?,
        row.try_get(2)?,
        row.try_get(3)?,
        row.try_get(4)?,
    ))
}

fn params<'a>(
    name: &str,
    o: &'a Order,
    list: &'a Vec<OrderState>,
    addrs: &'a Vec<Address>,
) -> Vec<&'a (dyn ToSql + Sync)> {
    match name {
        "enum array parameter" => vec![list, &o.id],
        "composite parameter" => vec![&o.addr, &o.id],
        "composite array parameter" => vec![addrs, &o.id],
        _ => vec![&o.state, &o.id],
    }
}

/// Inserts a row through one path, then reads it back through every select.
async fn exercise(
    rep: &mut Report,
    phase: &str,
    who: &str,
    c: &Client,
    id: i32,
    state: OrderState,
) {
    let o = order(id, state);
    let ins: [&(dyn ToSql + Sync); 5] = [&o.id, &o.state, &o.states, &o.addr, &o.addrs];
    for (path, oid) in [("query_typed", id), ("prepared", id + 1)] {
        let o2 = order(oid, state);
        let ins2: [&(dyn ToSql + Sync); 5] = [&o2.id, &o2.state, &o2.states, &o2.addr, &o2.addrs];
        let args = if path == "query_typed" { ins } else { ins2 };
        let r = match path {
            "query_typed" => {
                let typed: Vec<(&(dyn ToSql + Sync), Type)> = args
                    .iter()
                    .copied()
                    .zip(INSERT_TYPES.iter().cloned())
                    .collect();
                c.query_typed(&insert_sql(), &typed).await.map(|_| ())
            }
            _ => match c.prepare_typed(&insert_sql(), &INSERT_TYPES).await {
                Ok(st) => c.execute(&st, &args).await.map(|_| ()),
                Err(e) => Err(e),
            },
        };
        rep.check(
            &format!("{phase}, {who}: insert {} through {path}", state.label()),
            r.is_ok(),
            &r.err().map(|e| show(&e)).unwrap_or_default(),
        );
    }
    let list = vec![state, OrderState::Paid];
    for row_id in [id, id + 1] {
        let want = order(row_id, state);
        let addrs = vec![
            want.addr.clone(),
            Address {
                street: "elsewhere".into(),
                zip: 1,
            },
        ];
        for (name, sql, types) in selects() {
            for path in ["query_typed", "prepared"] {
                let ps = params(name, &want, &list, &addrs);
                let rows = match path {
                    "query_typed" => {
                        let typed: Vec<(&(dyn ToSql + Sync), Type)> =
                            ps.iter().copied().zip(types.iter().cloned()).collect();
                        c.query_typed(&sql, &typed).await
                    }
                    _ => match c.prepare_typed(&sql, &types).await {
                        Ok(st) => c.query(&st, &ps).await,
                        Err(e) => Err(e),
                    },
                };
                let detail = match rows {
                    Err(e) => Err(show(&e)),
                    Ok(rows) if rows.len() != 1 => Err(format!("{} rows", rows.len())),
                    Ok(rows) => match decode(&rows[0], name == "columns read as text") {
                        Err(e) => Err(show(&e)),
                        Ok(got) => {
                            if got
                                == (
                                    want.id,
                                    want.state,
                                    want.states.clone(),
                                    want.addr.clone(),
                                    want.addrs.clone(),
                                )
                            {
                                Ok(format!("{:?}", got.1))
                            } else {
                                Err(format!("wrong row: {got:?}"))
                            }
                        }
                    },
                };
                rep.check(
                    &format!("{phase}, {who}: {name}, row {row_id}, {path}"),
                    detail.is_ok(),
                    &detail.unwrap_or_else(|e| e),
                );
            }
        }
    }
}

/// Statements the connection opened at the start prepared in an earlier
/// phase, run again now.
async fn reuse(
    rep: &mut Report,
    phase: &str,
    c: &Client,
    kept: &[(String, tokio_postgres::Statement)],
    id: i32,
    state: OrderState,
) {
    let want = order(id, state);
    let list = vec![state, OrderState::Paid];
    let addrs = vec![want.addr.clone()];
    for (name, st) in kept {
        let ps = params(name, &want, &list, &addrs);
        let r = c.query(st, &ps).await;
        let detail = match r {
            Err(e) => show(&e),
            Ok(rows) => match rows
                .first()
                .map(|r| decode(r, name == "columns read as text"))
            {
                None => format!("{} rows", rows.len()),
                Some(Err(e)) => show(&e),
                Some(Ok(got)) => format!("{:?}", got.1),
            },
        };
        rep.row(&[phase, name, &detail]);
    }
}

/// What the driver prepared on this connection on its own: `tokio-postgres`
/// looks up a result column's type it doesn't know, with its labels or
/// fields, and keeps the statements it used.
async fn driver_lookups(c: &Client) -> String {
    let rows = c
        .query(
            "SELECT count(*) FILTER (WHERE statement LIKE '%pg_catalog.pg_type%'), count(*) FILTER (WHERE statement LIKE '%pg_catalog.pg_enum%'), count(*) FILTER (WHERE statement LIKE '%pg_catalog.pg_attribute%') FROM pg_prepared_statements WHERE statement NOT LIKE '%pg_prepared_statements%'",
            &[],
        )
        .await
        .unwrap();
    let (t, e, a): (i64, i64, i64) = (rows[0].get(0), rows[0].get(1), rows[0].get(2));
    format!("pg_type lookup: {t}, pg_enum lookup: {e}, pg_attribute lookup: {a}")
}

async fn param_types(c: &Client) -> String {
    let rows = c
        .query(
            "SELECT DISTINCT parameter_types::text FROM pg_prepared_statements WHERE statement LIKE '%p13%' ORDER BY 1",
            &[],
        )
        .await
        .unwrap();
    rows.iter()
        .map(|r| r.get::<_, String>(0))
        .collect::<Vec<_>>()
        .join(" ")
}

#[tokio::main]
async fn main() {
    let dsn = env("SPIKE_DSN");
    let m = must_connect(&dsn).await;
    let version = server_version(&m).await;
    println!("# Proof 13 on Postgres {version}");
    let mut rep = Report::default();
    m.batch_execute(
        r#"DROP SCHEMA IF EXISTS "p13" CASCADE;
CREATE SCHEMA "p13";
CREATE TYPE "p13"."order_state" AS ENUM ('new', 'paid');
CREATE TYPE "p13"."address" AS (street text, zip int4);
CREATE TABLE "p13"."orders" (id int4 PRIMARY KEY, state "p13"."order_state" NOT NULL, states "p13"."order_state"[] NOT NULL, addr "p13"."address", addrs "p13"."address"[]);"#,
    )
    .await
    .unwrap();

    rep.header("Checks", &["kind", "check", "result", "detail"]);
    // The connection opened at the start, and the statements it prepares
    // first, kept for the later phases.
    let a: Session = must_connect(&dsn).await;
    exercise(
        &mut rep,
        "start",
        "first connection",
        &a,
        10,
        OrderState::Paid,
    )
    .await;
    let mut kept = Vec::new();
    for (name, sql, types) in selects() {
        kept.push((
            name.to_owned(),
            a.prepare_typed(&sql, &types).await.unwrap(),
        ));
    }

    m.batch_execute(r#"ALTER TYPE "p13"."order_state" ADD VALUE 'shipped'"#)
        .await
        .unwrap();
    exercise(
        &mut rep,
        "after ADD VALUE",
        "first connection",
        &a,
        20,
        OrderState::Shipped,
    )
    .await;
    let b = must_connect(&dsn).await;
    exercise(
        &mut rep,
        "after ADD VALUE",
        "new connection",
        &b,
        30,
        OrderState::Shipped,
    )
    .await;
    // The driver's own copy of the enum, read when the first connection first
    // decoded it: the variant list is stale, and nothing above read it.
    let stale = a
        .query(r#"SELECT state FROM "p13"."orders" LIMIT 1"#, &[])
        .await
        .unwrap()[0]
        .columns()[0]
        .type_()
        .clone();
    let variants = match stale.kind() {
        Kind::Enum(v) => v.join(","),
        _ => "not an enum".to_owned(),
    };
    rep.check(
        "after ADD VALUE, first connection: the driver's cached variant list is stale, and decoding never read it",
        !variants.contains("shipped"),
        &format!("cached variants: {variants}"),
    );

    m.batch_execute(
        r#"BEGIN;
ALTER TYPE "p13"."order_state" RENAME TO "order_state_old";
CREATE TYPE "p13"."order_state" AS ENUM ('new', 'shipped', 'paid', 'refunded');
ALTER TABLE "p13"."orders"
  ALTER COLUMN state TYPE "p13"."order_state" USING state::text::"p13"."order_state",
  ALTER COLUMN states TYPE "p13"."order_state"[] USING states::text[]::"p13"."order_state"[];
DROP TYPE "p13"."order_state_old";
ALTER TYPE "p13"."address" RENAME TO "address_old";
CREATE TYPE "p13"."address" AS (street text, zip int4);
ALTER TABLE "p13"."orders"
  ALTER COLUMN addr TYPE "p13"."address" USING addr::text::"p13"."address",
  ALTER COLUMN addrs TYPE "p13"."address"[] USING addrs::text::"p13"."address"[];
DROP TYPE "p13"."address_old";
COMMIT;"#,
    )
    .await
    .unwrap();
    exercise(
        &mut rep,
        "after the swap",
        "first connection",
        &a,
        40,
        OrderState::Refunded,
    )
    .await;
    let c = must_connect(&dsn).await;
    exercise(
        &mut rep,
        "after the swap",
        "new connection",
        &c,
        50,
        OrderState::Refunded,
    )
    .await;

    rep.header(
        "Statements the first connection prepared at the start, run again (proof 14 covers these)",
        &["phase", "statement", "outcome"],
    );
    reuse(
        &mut rep,
        "after the swap, row 20 (shipped)",
        &a,
        &kept,
        20,
        OrderState::Shipped,
    )
    .await;

    rep.header("What reached the server", &["connection", "what", "seen"]);
    rep.row(&[
        "first",
        "parameter types of its prepared statements",
        &param_types(&a).await,
    ]);
    rep.row(&[
        "first",
        "statements the driver prepared to look up types",
        &driver_lookups(&a).await,
    ]);
    let t = must_connect(&dsn).await;
    let st = t
        .prepare_typed(r#"SELECT state::text, addr::text FROM "p13"."orders" WHERE state = ($1::"p13"."order_state")"#, &[Type::TEXT])
        .await
        .unwrap();
    let n = t.query(&st, &[&OrderState::Refunded]).await.unwrap().len();
    rep.row(&[
        "reads only text",
        &format!("lookups after reading {n} rows as text"),
        &driver_lookups(&t).await,
    ]);
    rep.finish("proof 13");
}
