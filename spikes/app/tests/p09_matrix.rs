//! Proof 9, DF1: each R-0186 Rust type against each column type an explicit
//! `sql` can give it, inserted and compared, under three ways of choosing the
//! declared type:
//!
//! - **V**, the value's own type, as the spike first did: `String` is `text`.
//! - **C**, the column's type, where morm knows the column (DF1): a built-in
//!   type's OID, or `UNKNOWN` in text format for a type with no fixed OID.
//! - **R**, a raw parameter (DF1): `String` and `&str` as `UNKNOWN` in text
//!   format, every other type as its own.
//!
//! Each cell is `ok`, `wrong: ..` (a silently different answer), `refused` (the
//! value's encoder doesn't take that declared type, before anything is sent)
//! or `error <SQLSTATE>`. The session runs with `TimeZone = 'Asia/Dhaka'`
//! (+06), so a time that shifts with the session shows. The table is pinned
//! in `p09_matrix.expected`, so a change on any Postgres version fails here.

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use morm::Param;
use morm::param::{Slot, bind, bind_with};
use rust_decimal::Decimal;
use serde_json::json;
use std::net::IpAddr;
use std::time::{Duration, SystemTime};
use tokio_postgres::Client;
use tokio_postgres::types::Type;
use uuid::Uuid;

type P = Box<dyn Param>;

fn p<T: Param + 'static>(v: T) -> P {
    Box::new(v)
}

/// One column type with one value, and what's right.
struct Case {
    rust: &'static str,
    col: &'static str,
    /// The C strategy's slot for this column.
    slot: Slot,
    value: P,
    /// The column's output after inserting `value`, `None` for `NULL`.
    stored: Option<&'static str>,
    /// Compare: a stored literal, then probes with their right row counts.
    fixture: Option<&'static str>,
    probes: Vec<Probe>,
}

struct Probe {
    what: &'static str,
    any: bool,
    slot: Slot,
    value: P,
    want: i64,
}

fn probe(what: &'static str, slot: Slot, value: P, want: i64) -> Probe {
    Probe {
        what,
        any: false,
        slot,
        value,
        want,
    }
}

fn any(what: &'static str, slot: Slot, value: P, want: i64) -> Probe {
    Probe {
        what,
        any: true,
        slot,
        value,
        want,
    }
}

const fn col(t: &'static Type) -> Slot {
    Slot::Builtin(t)
}
const UNK: Slot = Slot::NoOid;

fn utc() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
}
fn naive() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}
fn instant() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_767_225_600)
}
fn ip() -> IpAddr {
    "10.0.0.1".parse().unwrap()
}
fn uid() -> Uuid {
    Uuid::from_u128(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef)
}
fn s(v: &str) -> P {
    p(v.to_owned())
}

fn cases() -> Vec<Case> {
    let c = |rust, col, slot, value, stored, fixture, probes| Case {
        rust,
        col,
        slot,
        value,
        stored,
        fixture,
        probes,
    };
    let dec: Decimal = "12.5".parse().unwrap();
    vec![
        // Integers, floats, bool: the default columns and wider ones.
        c(
            "i16",
            "smallint",
            col(&Type::INT2),
            p(7i16),
            Some("7"),
            Some("7"),
            vec![probe("7", col(&Type::INT2), p(7i16), 1)],
        ),
        c(
            "i16",
            "integer",
            col(&Type::INT4),
            p(7i16),
            Some("7"),
            Some("7"),
            vec![probe("7", col(&Type::INT4), p(7i16), 1)],
        ),
        c(
            "i16",
            "bigint",
            col(&Type::INT8),
            p(7i16),
            Some("7"),
            Some("7"),
            vec![probe("7", col(&Type::INT8), p(7i16), 1)],
        ),
        c(
            "i32",
            "integer",
            col(&Type::INT4),
            p(7i32),
            Some("7"),
            Some("7"),
            vec![probe("7", col(&Type::INT4), p(7i32), 1)],
        ),
        c(
            "i32",
            "bigint",
            col(&Type::INT8),
            p(7i32),
            Some("7"),
            Some("7"),
            vec![probe("7", col(&Type::INT8), p(7i32), 1)],
        ),
        c(
            "i64",
            "bigint",
            col(&Type::INT8),
            p(7i64),
            Some("7"),
            Some("7"),
            vec![probe("7", col(&Type::INT8), p(7i64), 1)],
        ),
        c(
            "f32",
            "real",
            col(&Type::FLOAT4),
            p(1.5f32),
            Some("1.5"),
            Some("1.5"),
            vec![probe("1.5", col(&Type::FLOAT4), p(1.5f32), 1)],
        ),
        c(
            "f32",
            "double precision",
            col(&Type::FLOAT8),
            p(1.5f32),
            Some("1.5"),
            Some("1.5"),
            vec![probe("1.5", col(&Type::FLOAT8), p(1.5f32), 1)],
        ),
        c(
            "f64",
            "double precision",
            col(&Type::FLOAT8),
            p(-2.25f64),
            Some("-2.25"),
            Some("-2.25"),
            vec![probe("-2.25", col(&Type::FLOAT8), p(-2.25f64), 1)],
        ),
        c(
            "bool",
            "boolean",
            col(&Type::BOOL),
            p(true),
            Some("t"),
            Some("true"),
            vec![probe("true", col(&Type::BOOL), p(true), 1)],
        ),
        // Text: the default column and each one an explicit `sql` gives.
        c(
            "String",
            "text",
            col(&Type::TEXT),
            s("Alice"),
            Some("Alice"),
            Some("'Alice'"),
            vec![
                probe("'Alice'", col(&Type::TEXT), s("Alice"), 1),
                probe("'alice'", col(&Type::TEXT), s("alice"), 0),
            ],
        ),
        c(
            "String",
            "citext",
            UNK,
            s("Alice"),
            Some("Alice"),
            Some("'Alice'"),
            vec![probe("'alice'", UNK, s("alice"), 1)],
        ),
        c(
            "&str",
            "citext",
            UNK,
            p("Alice"),
            Some("Alice"),
            Some("'Alice'"),
            vec![probe("'alice'", UNK, p("alice"), 1)],
        ),
        c(
            "String",
            "varchar(5)",
            col(&Type::VARCHAR),
            s("ab"),
            Some("ab"),
            Some("'ab'"),
            vec![
                probe("'ab'", col(&Type::VARCHAR), s("ab"), 1),
                probe("'ab '", col(&Type::VARCHAR), s("ab "), 0),
            ],
        ),
        c(
            "String, too long",
            "varchar(5)",
            col(&Type::VARCHAR),
            s("abcdef"),
            Some("error 22001"),
            None,
            vec![],
        ),
        c(
            "String",
            "char(5)",
            col(&Type::BPCHAR),
            s("ab"),
            Some("ab   "),
            Some("'ab'"),
            vec![
                probe("'ab'", col(&Type::BPCHAR), s("ab"), 1),
                probe("'ab '", col(&Type::BPCHAR), s("ab "), 1),
            ],
        ),
        c(
            "&str",
            "char(5)",
            col(&Type::BPCHAR),
            p("ab"),
            Some("ab   "),
            Some("'ab'"),
            vec![probe("'ab '", col(&Type::BPCHAR), p("ab "), 1)],
        ),
        c(
            "String",
            "json",
            col(&Type::JSON),
            s(r#"{"b":1,"a":2}"#),
            Some(r#"{"b":1,"a":2}"#),
            None,
            vec![],
        ),
        c(
            "String",
            "jsonb",
            col(&Type::JSONB),
            s(r#"{"b":1,"a":2}"#),
            Some(r#"{"a": 2, "b": 1}"#),
            Some(r#"'{"a":2,"b":1}'"#),
            vec![probe(
                "text of the same object",
                col(&Type::JSONB),
                s(r#"{"b":1,"a":2}"#),
                1,
            )],
        ),
        c(
            "Vec<u8>",
            "bytea",
            col(&Type::BYTEA),
            p(vec![0u8, 1, 255]),
            Some(r"\x0001ff"),
            Some(r"'\x0001ff'"),
            vec![probe("bytes", col(&Type::BYTEA), p(vec![0u8, 1, 255]), 1)],
        ),
        // Times: an instant into timestamptz and into timestamp.
        c(
            "DateTime<Utc>",
            "timestamptz",
            col(&Type::TIMESTAMPTZ),
            p(utc()),
            Some("2026-01-01 06:00:00+06"),
            Some("'2026-01-01 00:00:00+00'"),
            vec![probe(
                "the same instant",
                col(&Type::TIMESTAMPTZ),
                p(utc()),
                1,
            )],
        ),
        c(
            "DateTime<Utc>",
            "timestamp",
            col(&Type::TIMESTAMP),
            p(utc()),
            Some("2026-01-01 00:00:00"),
            Some("'2026-01-01 00:00:00'"),
            vec![probe(
                "the same instant, as UTC",
                col(&Type::TIMESTAMP),
                p(utc()),
                1,
            )],
        ),
        c(
            "SystemTime",
            "timestamptz",
            col(&Type::TIMESTAMPTZ),
            p(instant()),
            Some("2026-01-01 06:00:00+06"),
            Some("'2026-01-01 00:00:00+00'"),
            vec![probe(
                "the same instant",
                col(&Type::TIMESTAMPTZ),
                p(instant()),
                1,
            )],
        ),
        c(
            "SystemTime",
            "timestamp",
            col(&Type::TIMESTAMP),
            p(instant()),
            Some("2026-01-01 00:00:00"),
            Some("'2026-01-01 00:00:00'"),
            vec![probe(
                "the same instant, as UTC",
                col(&Type::TIMESTAMP),
                p(instant()),
                1,
            )],
        ),
        c(
            "NaiveDateTime",
            "timestamp",
            col(&Type::TIMESTAMP),
            p(naive()),
            Some("2026-01-01 00:00:00"),
            Some("'2026-01-01 00:00:00'"),
            vec![probe(
                "the same wall time",
                col(&Type::TIMESTAMP),
                p(naive()),
                1,
            )],
        ),
        c(
            "NaiveDateTime",
            "timestamptz",
            col(&Type::TIMESTAMPTZ),
            p(naive()),
            Some("2026-01-01 06:00:00+06"),
            Some("'2026-01-01 00:00:00+00'"),
            vec![probe(
                "the same wall time, as UTC",
                col(&Type::TIMESTAMPTZ),
                p(naive()),
                1,
            )],
        ),
        c(
            "NaiveDate",
            "date",
            col(&Type::DATE),
            p(naive().date()),
            Some("2026-01-01"),
            Some("'2026-01-01'"),
            vec![probe(
                "the same date",
                col(&Type::DATE),
                p(naive().date()),
                1,
            )],
        ),
        c(
            "NaiveTime",
            "time",
            col(&Type::TIME),
            p(NaiveTime::from_hms_opt(12, 34, 56).unwrap()),
            Some("12:34:56"),
            Some("'12:34:56'"),
            vec![probe(
                "the same time",
                col(&Type::TIME),
                p(NaiveTime::from_hms_opt(12, 34, 56).unwrap()),
                1,
            )],
        ),
        c(
            "Uuid",
            "uuid",
            col(&Type::UUID),
            p(uid()),
            Some("01234567-89ab-cdef-0123-456789abcdef"),
            Some("'01234567-89ab-cdef-0123-456789abcdef'"),
            vec![probe("the same uuid", col(&Type::UUID), p(uid()), 1)],
        ),
        c(
            "Value",
            "jsonb",
            col(&Type::JSONB),
            p(json!({"b": 1, "a": 2})),
            Some(r#"{"a": 2, "b": 1}"#),
            Some(r#"'{"a":2,"b":1}'"#),
            vec![probe(
                "the same object",
                col(&Type::JSONB),
                p(json!({"b": 1, "a": 2})),
                1,
            )],
        ),
        c(
            "Value",
            "json",
            col(&Type::JSON),
            p(json!({"b": 1, "a": 2})),
            Some(r#"{"a":2,"b":1}"#),
            None,
            vec![],
        ),
        c(
            "IpAddr",
            "inet",
            col(&Type::INET),
            p(ip()),
            Some("10.0.0.1"),
            Some("'10.0.0.1'"),
            vec![probe("the same address", col(&Type::INET), p(ip()), 1)],
        ),
        c(
            "IpAddr",
            "cidr",
            col(&Type::CIDR),
            p(ip()),
            Some("10.0.0.1/32"),
            Some("'10.0.0.1/32'"),
            vec![probe("the same address", col(&Type::CIDR), p(ip()), 1)],
        ),
        c(
            "Decimal",
            "numeric(10,2)",
            col(&Type::NUMERIC),
            p(dec),
            Some("12.50"),
            Some("12.5"),
            vec![probe("12.5", col(&Type::NUMERIC), p(dec), 1)],
        ),
        // NULL of each kind.
        c(
            "Option<String> None",
            "citext",
            UNK,
            p(None::<String>),
            None,
            None,
            vec![],
        ),
        c(
            "Option<i32> None",
            "integer",
            col(&Type::INT4),
            p(None::<i32>),
            None,
            None,
            vec![],
        ),
        c(
            "Option<DateTime<Utc>> None",
            "timestamp",
            col(&Type::TIMESTAMP),
            p(None::<DateTime<Utc>>),
            None,
            None,
            vec![],
        ),
        c(
            "Option<Value> None",
            "json",
            col(&Type::JSON),
            p(None::<serde_json::Value>),
            None,
            None,
            vec![],
        ),
        // Arrays as columns.
        c(
            "Vec<String>",
            "text[]",
            col(&Type::TEXT_ARRAY),
            p(vec!["Alice".to_owned(), "Bob".into()]),
            Some("{Alice,Bob}"),
            Some("'{Alice,Bob}'"),
            vec![probe(
                "{Alice,Bob}",
                col(&Type::TEXT_ARRAY),
                p(vec!["Alice".to_owned(), "Bob".into()]),
                1,
            )],
        ),
        c(
            "Vec<String>",
            "citext[]",
            UNK,
            p(vec!["Alice".to_owned(), "Bob".into()]),
            Some("{Alice,Bob}"),
            Some("'{Alice,Bob}'"),
            vec![probe(
                "{alice,bob}",
                UNK,
                p(vec!["alice".to_owned(), "bob".into()]),
                1,
            )],
        ),
        c(
            "Vec<String>",
            "varchar(5)[]",
            col(&Type::VARCHAR_ARRAY),
            p(vec!["Alice".to_owned(), "Bob".into()]),
            Some("{Alice,Bob}"),
            None,
            vec![],
        ),
        c(
            "Vec<String>",
            "char(5)[]",
            col(&Type::BPCHAR_ARRAY),
            p(vec!["Alice".to_owned(), "Bob".into()]),
            Some(r#"{Alice,"Bob  "}"#),
            Some("'{Alice,Bob}'"),
            vec![probe(
                "{Alice,\"Bob \"}",
                col(&Type::BPCHAR_ARRAY),
                p(vec!["Alice".to_owned(), "Bob ".into()]),
                1,
            )],
        ),
        c(
            "Vec<String>, empty",
            "citext[]",
            UNK,
            p(Vec::<String>::new()),
            Some("{}"),
            None,
            vec![],
        ),
        c(
            "Vec<Option<String>>",
            "citext[]",
            UNK,
            p(vec![Some("a\"b".to_owned()), None]),
            Some(r#"{"a\"b",NULL}"#),
            None,
            vec![],
        ),
        c(
            "Vec<i64>",
            "bigint[]",
            col(&Type::INT8_ARRAY),
            p(vec![1i64, 2]),
            Some("{1,2}"),
            None,
            vec![],
        ),
        c(
            "Vec<i32>",
            "integer[]",
            col(&Type::INT4_ARRAY),
            p(vec![Some(1i32), None]),
            Some("{1,NULL}"),
            None,
            vec![],
        ),
        c(
            "Vec<i16>",
            "integer[]",
            col(&Type::INT4_ARRAY),
            p(vec![1i16, 2]),
            Some("{1,2}"),
            None,
            vec![],
        ),
        // A list against a scalar column: `= ANY($1)`.
        c(
            "Vec<String> in",
            "text",
            col(&Type::TEXT),
            s("Alice"),
            Some("Alice"),
            Some("'Alice'"),
            vec![any(
                "ANY {Alice,x}",
                col(&Type::TEXT_ARRAY),
                p(vec!["Alice".to_owned(), "x".into()]),
                1,
            )],
        ),
        c(
            "Vec<String> in",
            "citext",
            UNK,
            s("Alice"),
            Some("Alice"),
            Some("'Alice'"),
            vec![any(
                "ANY {alice,x}",
                UNK,
                p(vec!["alice".to_owned(), "x".into()]),
                1,
            )],
        ),
        c(
            "Vec<String> in",
            "char(5)",
            col(&Type::BPCHAR),
            s("ab"),
            Some("ab   "),
            Some("'ab'"),
            vec![any(
                "ANY {\"ab \"}",
                col(&Type::BPCHAR_ARRAY),
                p(vec!["ab ".to_owned()]),
                1,
            )],
        ),
        c(
            "Vec<i64> in",
            "bigint",
            col(&Type::INT8),
            p(7i64),
            Some("7"),
            Some("7"),
            vec![any(
                "ANY {7,8}",
                col(&Type::INT8_ARRAY),
                p(vec![7i64, 8]),
                1,
            )],
        ),
        c(
            "Vec<i32> in",
            "bigint",
            col(&Type::INT8),
            p(7i64),
            Some("7"),
            Some("7"),
            vec![any("ANY {7}", col(&Type::INT8_ARRAY), p(vec![7i32]), 1)],
        ),
        c(
            "Vec<Uuid> in",
            "uuid",
            col(&Type::UUID),
            p(uid()),
            Some("01234567-89ab-cdef-0123-456789abcdef"),
            Some("'01234567-89ab-cdef-0123-456789abcdef'"),
            vec![any(
                "ANY {the uuid}",
                col(&Type::UUID_ARRAY),
                p(vec![uid()]),
                1,
            )],
        ),
        c(
            "Vec<DateTime<Utc>> in",
            "timestamp",
            col(&Type::TIMESTAMP),
            p(utc()),
            Some("2026-01-01 00:00:00"),
            Some("'2026-01-01 00:00:00'"),
            vec![any(
                "ANY {the instant}",
                col(&Type::TIMESTAMP_ARRAY),
                p(vec![utc()]),
                1,
            )],
        ),
    ]
}

/// How a failed statement shows in the table.
fn outcome(e: tokio_postgres::Error) -> String {
    match e.as_db_error() {
        Some(db) => format!("error {}", db.code().code()),
        None if e.to_string().contains("serializing parameter") => "refused".into(),
        None => format!("error {e}"),
    }
}

async fn insert(c: &Client, case: &Case, slot: Slot) -> String {
    let ddl = format!(
        "DROP TABLE IF EXISTS pg_temp.m; CREATE TEMP TABLE m (c {})",
        case.col
    );
    c.batch_execute(&ddl).await.unwrap();
    let (b, ty) = bind(&*case.value, slot);
    if let Err(e) = c
        .query_typed("INSERT INTO m VALUES ($1)", &[(&b, ty)])
        .await
    {
        let got = outcome(e);
        return if Some(got.as_str()) == case.stored {
            "ok".into()
        } else {
            got
        };
    }
    let got: Option<String> = c
        .query_one(
            "SELECT CASE WHEN c IS NULL THEN NULL ELSE concat(c) END FROM m",
            &[],
        )
        .await
        .unwrap()
        .get(0);
    if got.as_deref() == case.stored {
        "ok".into()
    } else {
        format!("wrong: {}", got.as_deref().unwrap_or("NULL"))
    }
}

async fn compare(c: &Client, case: &Case, fixture: &str, pr: &Probe, slot: Slot) -> String {
    let ddl = format!(
        "DROP TABLE IF EXISTS pg_temp.m; CREATE TEMP TABLE m (c {}); INSERT INTO m VALUES ({fixture})",
        case.col
    );
    c.batch_execute(&ddl).await.unwrap();
    let sql = if pr.any {
        "SELECT count(*) FROM m WHERE c = ANY($1)"
    } else {
        "SELECT count(*) FROM m WHERE c = $1"
    };
    let (b, ty) = bind(&*pr.value, slot);
    match c.query_typed(sql, &[(&b, ty)]).await {
        Err(e) => outcome(e),
        Ok(rows) => {
            let n: i64 = rows[0].get(0);
            if n == pr.want {
                "ok".into()
            } else {
                format!("wrong: {n} rows")
            }
        }
    }
}

/// Raw SQL with a `String` parameter: how each wire form of the same value
/// behaves where Postgres has to infer the type from the SQL.
struct Raw {
    sql: &'static str,
    value: &'static str,
    /// The right answer as text, or the error Postgres gives any client.
    want: &'static str,
}

const RAW: &[Raw] = &[
    Raw {
        sql: "SELECT $1",
        value: "x",
        want: "x",
    },
    Raw {
        sql: "SELECT ($1 IS NULL)::text",
        value: "x",
        want: "false",
    },
    Raw {
        sql: "SELECT lower($1)",
        value: "AbC",
        want: "abc",
    },
    Raw {
        sql: "SELECT length($1)::text",
        value: "abc",
        want: "3",
    },
    Raw {
        sql: "SELECT $1 || 'x'",
        value: "a",
        want: "ax",
    },
    Raw {
        sql: "SELECT concat($1, 'x')",
        value: "a",
        want: "ax",
    },
    Raw {
        sql: "SELECT coalesce($1, 'd')",
        value: "x",
        want: "x",
    },
    Raw {
        sql: "SELECT ($1::int + 1)::text",
        value: "12",
        want: "13",
    },
    Raw {
        sql: "SELECT ('abc' LIKE $1)::text",
        value: "a%",
        want: "true",
    },
    Raw {
        sql: "SELECT ('Alice'::citext = $1)::text",
        value: "alice",
        want: "true",
    },
    Raw {
        sql: "SELECT ('ab'::char(5) = $1)::text",
        value: "ab ",
        want: "true",
    },
    Raw {
        sql: "SELECT ('{\"a\":1}'::jsonb = $1)::text",
        value: "{\"a\": 1}",
        want: "true",
    },
    Raw {
        sql: "SELECT ('2026-01-01 00:00:00+00'::timestamptz = $1)::text",
        value: "2026-01-01 06:00:00+06",
        want: "true",
    },
    Raw {
        sql: "SELECT (1234::int4 = $1)::text",
        value: "1234",
        want: "true",
    },
    Raw {
        sql: "SELECT (42::int8 = $1)::text",
        value: "42",
        want: "true",
    },
    Raw {
        sql: "SELECT (1234::int4 = $1)::text",
        value: "12x4",
        want: "error 22P02",
    },
];

/// Raw SQL with a `Vec<String>` parameter: a binary `text[]`, as R sends it,
/// against an array literal sent as `UNKNOWN` in text format.
struct RawList {
    sql: &'static str,
    value: &'static [&'static str],
    want: &'static str,
}

const RAW_LISTS: &[RawList] = &[
    RawList {
        sql: "SELECT count(*)::text FROM (VALUES ('Alice'::citext)) v(c) WHERE c = ANY($1)",
        value: &["alice", "x"],
        want: "1",
    },
    RawList {
        sql: "SELECT count(*)::text FROM (VALUES ('ab'::char(5))) v(c) WHERE c = ANY($1)",
        value: &["ab "],
        want: "1",
    },
    RawList {
        sql: "SELECT count(*)::text FROM (VALUES ('Alice'::text)) v(c) WHERE c = ANY($1)",
        value: &["Alice"],
        want: "1",
    },
    RawList {
        sql: "SELECT array_length($1, 1)::text",
        value: &["a", "b"],
        want: "2",
    },
    RawList {
        sql: "SELECT string_agg(x, ',') FROM unnest($1) x",
        value: &["a", "b"],
        want: "a,b",
    },
    RawList {
        sql: "SELECT $1::text",
        value: &["a", "b"],
        want: "{a,b}",
    },
];

/// The pairs whose answer moves with the session's `TimeZone`, again under
/// `TimeZone = 'UTC'`.
fn utc_cases() -> Vec<Case> {
    let c = |rust, col, slot, value, stored, fixture, probes| Case {
        rust,
        col,
        slot,
        value,
        stored,
        fixture,
        probes,
    };
    vec![
        c(
            "DateTime<Utc>",
            "timestamp",
            col(&Type::TIMESTAMP),
            p(utc()),
            Some("2026-01-01 00:00:00"),
            Some("'2026-01-01 00:00:00'"),
            vec![probe(
                "the same instant, as UTC",
                col(&Type::TIMESTAMP),
                p(utc()),
                1,
            )],
        ),
        c(
            "SystemTime",
            "timestamp",
            col(&Type::TIMESTAMP),
            p(instant()),
            Some("2026-01-01 00:00:00"),
            Some("'2026-01-01 00:00:00'"),
            vec![probe(
                "the same instant, as UTC",
                col(&Type::TIMESTAMP),
                p(instant()),
                1,
            )],
        ),
        c(
            "NaiveDateTime",
            "timestamptz",
            col(&Type::TIMESTAMPTZ),
            p(naive()),
            Some("2026-01-01 00:00:00+00"),
            Some("'2026-01-01 00:00:00+00'"),
            vec![probe(
                "the same wall time, as UTC",
                col(&Type::TIMESTAMPTZ),
                p(naive()),
                1,
            )],
        ),
        c(
            "Vec<DateTime<Utc>> in",
            "timestamp",
            col(&Type::TIMESTAMP),
            p(utc()),
            Some("2026-01-01 00:00:00"),
            Some("'2026-01-01 00:00:00'"),
            vec![any(
                "ANY {the instant}",
                col(&Type::TIMESTAMP_ARRAY),
                p(vec![utc()]),
                1,
            )],
        ),
    ]
}

async fn matrix(c: &Client, cases: Vec<Case>, table: &mut String) {
    for case in cases {
        let row = [
            insert(c, &case, Slot::Value).await,
            insert(c, &case, case.slot).await,
            insert(c, &case, Slot::Raw).await,
        ];
        table.push_str(&format!(
            "| {} | {} | insert | {} |\n",
            case.rust,
            case.col,
            row.join(" | ")
        ));
        if let Some(f) = case.fixture {
            for pr in &case.probes {
                let row = [
                    compare(c, &case, f, pr, Slot::Value).await,
                    compare(c, &case, f, pr, pr.slot).await,
                    compare(c, &case, f, pr, Slot::Raw).await,
                ];
                table.push_str(&format!(
                    "| {} | {} | = {} | {} |\n",
                    case.rust,
                    case.col,
                    pr.what,
                    row.join(" | ")
                ));
            }
        }
    }
}

async fn raw(c: &Client, sql: &str, v: &dyn Param, want: &str, ty: Type, text: bool) -> String {
    let (b, ty) = bind_with(v, ty, text);
    match c.query_typed(sql, &[(&b, ty)]).await {
        Err(e) => {
            let got = outcome(e);
            if got == want { "ok".into() } else { got }
        }
        Ok(rows) => {
            let got: Option<String> = rows[0].get(0);
            match got {
                Some(g) if g == want => "ok".into(),
                Some(g) => format!("wrong: {g}"),
                None => "wrong: NULL".into(),
            }
        }
    }
}

#[tokio::test]
async fn every_type_against_every_column_an_explicit_sql_gives() {
    let c = app::admin().await;
    c.batch_execute("CREATE EXTENSION IF NOT EXISTS citext; SET TimeZone = 'Asia/Dhaka'")
        .await
        .unwrap();
    let version: String = c
        .query_one("SHOW server_version", &[])
        .await
        .unwrap()
        .get(0);

    let mut table = String::from(
        "| Rust value | Column | Check | V: value's type | C: column's type | R: raw |\n|---|---|---|---|---|---|\n",
    );
    matrix(&c, cases(), &mut table).await;
    table.push_str(
        "\n| Under `TimeZone = 'UTC'` | Column | Check | V | C | R |\n|---|---|---|---|---|---|\n",
    );
    c.batch_execute("SET TimeZone = 'UTC'").await.unwrap();
    matrix(&c, utc_cases(), &mut table).await;
    c.batch_execute("SET TimeZone = 'Asia/Dhaka'")
        .await
        .unwrap();

    table.push_str(
        "\n| Raw SQL, `String` value | Value | text | unknown, binary | unknown, text (R) |\n|---|---|---|---|---|\n",
    );
    for r in RAW {
        let v = r.value.to_owned();
        let row = [
            raw(&c, r.sql, &v, r.want, Type::TEXT, false).await,
            raw(&c, r.sql, &v, r.want, Type::UNKNOWN, false).await,
            raw(&c, r.sql, &v, r.want, Type::UNKNOWN, true).await,
        ];
        table.push_str(&format!(
            "| `{}` | `{}` | {} |\n",
            r.sql.replace('|', "\\|"),
            r.value,
            row.join(" | ")
        ));
    }
    table.push_str(
        "\n| Raw SQL, `Vec<String>` value | Value | text[], binary (R) | unknown, text literal |\n|---|---|---|---|\n",
    );
    for r in RAW_LISTS {
        let v: Vec<String> = r.value.iter().map(|s| (*s).to_owned()).collect();
        let row = [
            raw(&c, r.sql, &v, r.want, Type::TEXT_ARRAY, false).await,
            raw(&c, r.sql, &v, r.want, Type::UNKNOWN, true).await,
        ];
        table.push_str(&format!(
            "| `{}` | `{:?}` | {} |\n",
            r.sql,
            r.value,
            row.join(" | ")
        ));
    }
    println!("Postgres {version}\n\n{table}");
    // `SPIKE_MATRIX=write` pins a new table.
    if std::env::var("SPIKE_MATRIX").as_deref() == Ok("write") {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/p09_matrix.expected");
        std::fs::write(path, &table).unwrap();
        return;
    }
    let expected = include_str!("p09_matrix.expected");
    assert_eq!(
        table, expected,
        "the matrix changed on Postgres {version}: see the table above"
    );
}
