//! Proof 9, rerun under DH2 (choice J): each R-0186 Rust type against each
//! column type an explicit `sql` can give it, inserted and compared, two ways:
//!
//! - **Field**: bound through the field. The SQL casts the parameter to the
//!   column's type with no modifier, `($1::bpchar)`, it's declared as that
//!   type's built-in carrier, and the conversion layer encodes the value.
//! - **Free**: cast to the Rust type's own SQL type, `($1::text)`, and
//!   declared as its carrier.
//!
//! Every cell runs through `query_typed` and through a prepared statement,
//! and shows one result when both agree. Beside each, the findings of a
//! small version of check-sql's typing pass (rules T1 to T3 of
//! `param-typing-design.md`, section 6), read from the analyzed parse tree
//! Postgres sends back under `debug_print_parse`: `-` for none.
//!
//! A cell is `ok`, `wrong: ..` (a silently different answer) or
//! `error <SQLSTATE>`. The session runs with `TimeZone = 'Asia/Dhaka'`
//! (+06), so a time that moves with the session shows, then the time rows
//! again under `UTC`. A free cell whose answer differs from the field's and
//! that no rule reports is marked `UNREPORTED`, and the test fails on it.
//! The table is pinned in `p09_matrix.expected`.

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use futures_util::StreamExt;
use morm::Param;
use morm::param::{Field, field, free};
use rust_decimal::Decimal;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tokio_postgres::types::{ToSql, Type};
use tokio_postgres::{AsyncMessage, Client, NoTls};
use uuid::Uuid;

type P = Box<dyn Param>;

fn p<T: Param + 'static>(v: T) -> P {
    Box::new(v)
}
fn s(v: &str) -> P {
    p(v.to_owned())
}
fn sv(v: &[&str]) -> P {
    p(v.iter().map(|x| (*x).to_owned()).collect::<Vec<String>>())
}
fn f(cast: &'static str, carrier: Type) -> Field {
    Field { cast, carrier }
}

struct Case {
    rust: &'static str,
    col: &'static str,
    field: Field,
    value: P,
    /// The column's output after inserting `value`, `None` for `NULL`.
    stored: Option<&'static str>,
    /// A stored literal to compare against, and the probes.
    fixture: Option<&'static str>,
    probes: Vec<Probe>,
}

struct Probe {
    what: &'static str,
    any: bool,
    field: Field,
    value: P,
    want: i64,
}

fn eq(what: &'static str, field: Field, value: P, want: i64) -> Probe {
    Probe {
        what,
        any: false,
        field,
        value,
        want,
    }
}
fn any(what: &'static str, field: Field, value: P, want: i64) -> Probe {
    Probe {
        what,
        any: true,
        field,
        value,
        want,
    }
}

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

#[allow(clippy::too_many_arguments)]
fn c(
    rust: &'static str,
    col: &'static str,
    field: Field,
    value: P,
    stored: Option<&'static str>,
    fixture: Option<&'static str>,
    probes: Vec<Probe>,
) -> Case {
    Case {
        rust,
        col,
        field,
        value,
        stored,
        fixture,
        probes,
    }
}

fn cases() -> Vec<Case> {
    use Type as T;
    let dec: Decimal = "12.5".parse().unwrap();
    let uuid_s = "01234567-89ab-cdef-0123-456789abcdef";
    let uuid_l = "'01234567-89ab-cdef-0123-456789abcdef'";
    vec![
        // Integers, floats, bool: the default columns and wider ones.
        c("i16", "smallint", f("int2", T::INT2), p(7i16), Some("7"), Some("7"), vec![eq("7", f("int2", T::INT2), p(7i16), 1)]),
        c("i16", "integer", f("int4", T::INT4), p(7i16), Some("7"), Some("7"), vec![eq("7", f("int4", T::INT4), p(7i16), 1)]),
        c("i16", "bigint", f("int8", T::INT8), p(7i16), Some("7"), Some("7"), vec![eq("7", f("int8", T::INT8), p(7i16), 1)]),
        c("i32", "integer", f("int4", T::INT4), p(7i32), Some("7"), Some("7"), vec![eq("7", f("int4", T::INT4), p(7i32), 1)]),
        c("i32", "bigint", f("int8", T::INT8), p(7i32), Some("7"), Some("7"), vec![eq("7", f("int8", T::INT8), p(7i32), 1)]),
        c("i64", "bigint", f("int8", T::INT8), p(7i64), Some("7"), Some("7"), vec![eq("7", f("int8", T::INT8), p(7i64), 1)]),
        c("f32", "real", f("float4", T::FLOAT4), p(1.5f32), Some("1.5"), Some("1.5"), vec![eq("1.5", f("float4", T::FLOAT4), p(1.5f32), 1)]),
        c("f32", "double precision", f("float8", T::FLOAT8), p(1.5f32), Some("1.5"), Some("1.5"), vec![eq("1.5", f("float8", T::FLOAT8), p(1.5f32), 1)]),
        c("f64", "double precision", f("float8", T::FLOAT8), p(-2.25f64), Some("-2.25"), Some("-2.25"), vec![eq("-2.25", f("float8", T::FLOAT8), p(-2.25f64), 1)]),
        c("bool", "boolean", f("bool", T::BOOL), p(true), Some("t"), Some("true"), vec![eq("true", f("bool", T::BOOL), p(true), 1)]),
        // Text: the default column and each one an explicit `sql` gives.
        c("String", "text", f("text", T::TEXT), s("Alice"), Some("Alice"), Some("'Alice'"), vec![
            eq("'Alice'", f("text", T::TEXT), s("Alice"), 1),
            eq("'alice'", f("text", T::TEXT), s("alice"), 0),
        ]),
        c("String", "citext", f("citext", T::TEXT), s("Alice"), Some("Alice"), Some("'Alice'"), vec![eq("'alice'", f("citext", T::TEXT), s("alice"), 1)]),
        c("&str", "citext", f("citext", T::TEXT), p("Alice"), Some("Alice"), Some("'Alice'"), vec![eq("'alice'", f("citext", T::TEXT), p("alice"), 1)]),
        c("String", "varchar(5)", f("varchar", T::VARCHAR), s("ab"), Some("ab"), Some("'ab'"), vec![
            eq("'ab'", f("varchar", T::VARCHAR), s("ab"), 1),
            eq("'ab '", f("varchar", T::VARCHAR), s("ab "), 0),
        ]),
        c("String, too long", "varchar(5)", f("varchar", T::VARCHAR), s("abcdef"), Some("error 22001"), None, vec![]),
        c("String", "char(5)", f("bpchar", T::BPCHAR), s("ab"), Some("ab   "), Some("'ab'"), vec![
            eq("'ab'", f("bpchar", T::BPCHAR), s("ab"), 1),
            eq("'ab '", f("bpchar", T::BPCHAR), s("ab "), 1),
        ]),
        c("&str", "char(5)", f("bpchar", T::BPCHAR), p("ab"), Some("ab   "), Some("'ab'"), vec![eq("'ab '", f("bpchar", T::BPCHAR), p("ab "), 1)]),
        c("String, too long", "char(5)", f("bpchar", T::BPCHAR), s("abcdef"), Some("error 22001"), None, vec![]),
        c("String", "json", f("json", T::TEXT), s(r#"{"b":1,"a":2}"#), Some(r#"{"b":1,"a":2}"#), None, vec![]),
        c("String", "jsonb", f("jsonb", T::TEXT), s(r#"{"b":1,"a":2}"#), Some(r#"{"a": 2, "b": 1}"#), Some(r#"'{"a":2,"b":1}'"#), vec![
            eq("text of the same object", f("jsonb", T::TEXT), s(r#"{"b":1,"a":2}"#), 1),
        ]),
        c("Vec<u8>", "bytea", f("bytea", T::BYTEA), p(vec![0u8, 1, 255]), Some(r"\x0001ff"), Some(r"'\x0001ff'"), vec![eq("bytes", f("bytea", T::BYTEA), p(vec![0u8, 1, 255]), 1)]),
        // Times: instants and wall times on both column types.
        c("DateTime<Utc>", "timestamptz", f("timestamptz", T::TIMESTAMPTZ), p(utc()), Some("2026-01-01 06:00:00+06"), Some("'2026-01-01 00:00:00+00'"), vec![eq("the same instant", f("timestamptz", T::TIMESTAMPTZ), p(utc()), 1)]),
        c("DateTime<Utc>", "timestamp", f("timestamp", T::TIMESTAMP), p(utc()), Some("2026-01-01 00:00:00"), Some("'2026-01-01 00:00:00'"), vec![eq("the same instant, as UTC", f("timestamp", T::TIMESTAMP), p(utc()), 1)]),
        c("SystemTime", "timestamptz", f("timestamptz", T::TIMESTAMPTZ), p(instant()), Some("2026-01-01 06:00:00+06"), Some("'2026-01-01 00:00:00+00'"), vec![eq("the same instant", f("timestamptz", T::TIMESTAMPTZ), p(instant()), 1)]),
        c("SystemTime", "timestamp", f("timestamp", T::TIMESTAMP), p(instant()), Some("2026-01-01 00:00:00"), Some("'2026-01-01 00:00:00'"), vec![eq("the same instant, as UTC", f("timestamp", T::TIMESTAMP), p(instant()), 1)]),
        c("NaiveDateTime", "timestamp", f("timestamp", T::TIMESTAMP), p(naive()), Some("2026-01-01 00:00:00"), Some("'2026-01-01 00:00:00'"), vec![eq("the same wall time", f("timestamp", T::TIMESTAMP), p(naive()), 1)]),
        c("NaiveDateTime", "timestamptz", f("timestamptz", T::TIMESTAMPTZ), p(naive()), Some("2026-01-01 06:00:00+06"), Some("'2026-01-01 00:00:00+00'"), vec![eq("the same wall time, as UTC", f("timestamptz", T::TIMESTAMPTZ), p(naive()), 1)]),
        c("NaiveDate", "date", f("date", T::DATE), p(naive().date()), Some("2026-01-01"), Some("'2026-01-01'"), vec![eq("the same date", f("date", T::DATE), p(naive().date()), 1)]),
        c("NaiveTime", "time", f("time", T::TIME), p(NaiveTime::from_hms_opt(12, 34, 56).unwrap()), Some("12:34:56"), Some("'12:34:56'"), vec![eq("the same time", f("time", T::TIME), p(NaiveTime::from_hms_opt(12, 34, 56).unwrap()), 1)]),
        c("Uuid", "uuid", f("uuid", T::UUID), p(uid()), Some(uuid_s), Some(uuid_l), vec![eq("the same uuid", f("uuid", T::UUID), p(uid()), 1)]),
        c("Value", "jsonb", f("jsonb", T::JSONB), p(json!({"b": 1, "a": 2})), Some(r#"{"a": 2, "b": 1}"#), Some(r#"'{"a":2,"b":1}'"#), vec![eq("the same object", f("jsonb", T::JSONB), p(json!({"b": 1, "a": 2})), 1)]),
        c("Value", "json", f("json", T::JSON), p(json!({"b": 1, "a": 2})), Some(r#"{"a":2,"b":1}"#), None, vec![]),
        c("IpAddr", "inet", f("inet", T::INET), p(ip()), Some("10.0.0.1"), Some("'10.0.0.1'"), vec![eq("the same address", f("inet", T::INET), p(ip()), 1)]),
        c("IpAddr", "cidr", f("cidr", T::CIDR), p(ip()), Some("10.0.0.1/32"), Some("'10.0.0.1/32'"), vec![eq("the same address", f("cidr", T::CIDR), p(ip()), 1)]),
        c("Decimal", "numeric(10,2)", f("numeric", T::NUMERIC), p(dec), Some("12.50"), Some("12.5"), vec![eq("12.5", f("numeric", T::NUMERIC), p(dec), 1)]),
        // Types with no fixed OID, cast by name: an enum, domains.
        c("String", "p9_mood", f("p9_mood", T::TEXT), s("happy"), Some("happy"), Some("'happy'"), vec![
            eq("'happy'", f("p9_mood", T::TEXT), s("happy"), 1),
            any("ANY {happy,sad}", f("p9_mood[]", T::TEXT_ARRAY), sv(&["happy", "sad"]), 1),
        ]),
        c("String, not a label", "p9_mood", f("p9_mood", T::TEXT), s("angry"), Some("error 22P02"), None, vec![]),
        c("Vec<String>", "p9_mood[]", f("p9_mood[]", T::TEXT_ARRAY), sv(&["happy", "sad"]), Some("{happy,sad}"), Some("'{happy,sad}'"), vec![eq("{happy,sad}", f("p9_mood[]", T::TEXT_ARRAY), sv(&["happy", "sad"]), 1)]),
        c("i32", "p9_posint", f("p9_posint", T::INT4), p(7i32), Some("7"), Some("7"), vec![eq("7", f("p9_posint", T::INT4), p(7i32), 1)]),
        c("i32, breaks the CHECK", "p9_posint", f("p9_posint", T::INT4), p(-1i32), Some("error 23514"), None, vec![]),
        c("String", "p9_email", f("p9_email", T::TEXT), s("Alice"), Some("Alice"), Some("'Alice'"), vec![eq("'alice'", f("p9_email", T::TEXT), s("alice"), 1)]),
        // NULL of each kind.
        c("Option<String> None", "citext", f("citext", T::TEXT), p(None::<String>), None, None, vec![]),
        c("Option<i32> None", "integer", f("int4", T::INT4), p(None::<i32>), None, None, vec![]),
        c("Option<DateTime<Utc>> None", "timestamp", f("timestamp", T::TIMESTAMP), p(None::<DateTime<Utc>>), None, None, vec![]),
        c("Option<Value> None", "json", f("json", T::JSON), p(None::<serde_json::Value>), None, None, vec![]),
        // Arrays as columns.
        c("Vec<String>", "text[]", f("text[]", T::TEXT_ARRAY), sv(&["Alice", "Bob"]), Some("{Alice,Bob}"), Some("'{Alice,Bob}'"), vec![eq("{Alice,Bob}", f("text[]", T::TEXT_ARRAY), sv(&["Alice", "Bob"]), 1)]),
        c("Vec<String>", "citext[]", f("citext[]", T::TEXT_ARRAY), sv(&["Alice", "Bob"]), Some("{Alice,Bob}"), Some("'{Alice,Bob}'"), vec![eq("{alice,bob}", f("citext[]", T::TEXT_ARRAY), sv(&["alice", "bob"]), 1)]),
        c("Vec<String>", "varchar(5)[]", f("varchar[]", T::VARCHAR_ARRAY), sv(&["Alice", "Bob"]), Some("{Alice,Bob}"), None, vec![]),
        c("Vec<String>", "char(5)[]", f("bpchar[]", T::BPCHAR_ARRAY), sv(&["Alice", "Bob"]), Some(r#"{Alice,"Bob  "}"#), Some("'{Alice,Bob}'"), vec![eq("{Alice,\"Bob \"}", f("bpchar[]", T::BPCHAR_ARRAY), sv(&["Alice", "Bob "]), 1)]),
        c("Vec<String>, empty", "citext[]", f("citext[]", T::TEXT_ARRAY), p(Vec::<String>::new()), Some("{}"), None, vec![]),
        c("Vec<Option<String>>", "citext[]", f("citext[]", T::TEXT_ARRAY), p(vec![Some(r"a\b".to_owned()), Some(String::new()), Some("{}".into()), Some("NULL".into()), Some("q\"uote, x".into()), None]), Some(r#"{"a\\b","","{}","NULL","q\"uote, x",NULL}"#), Some(r#"'{"a\\b","","{}","NULL","q\"uote, x",NULL}'"#), vec![
            eq("the same six elements", f("citext[]", T::TEXT_ARRAY), p(vec![Some(r"A\B".to_owned()), Some(String::new()), Some("{}".into()), Some("null".into()), Some("Q\"UOTE, X".into()), None]), 1),
        ]),
        c("Vec<i64>", "bigint[]", f("int8[]", T::INT8_ARRAY), p(vec![1i64, 2]), Some("{1,2}"), None, vec![]),
        c("Vec<Option<i32>>", "integer[]", f("int4[]", T::INT4_ARRAY), p(vec![Some(1i32), None]), Some("{1,NULL}"), None, vec![]),
        c("Vec<i16>", "integer[]", f("int4[]", T::INT4_ARRAY), p(vec![1i16, 2]), Some("{1,2}"), Some("'{1,2}'"), vec![eq("{1,2}", f("int4[]", T::INT4_ARRAY), p(vec![1i16, 2]), 1)]),
        // A list against a scalar column: `= ANY(..)`.
        c("Vec<String> in", "text", f("text", T::TEXT), s("Alice"), Some("Alice"), Some("'Alice'"), vec![any("ANY {Alice,x}", f("text[]", T::TEXT_ARRAY), sv(&["Alice", "x"]), 1)]),
        c("Vec<String> in", "citext", f("citext", T::TEXT), s("Alice"), Some("Alice"), Some("'Alice'"), vec![any("ANY {alice,x}", f("citext[]", T::TEXT_ARRAY), sv(&["alice", "x"]), 1)]),
        c("Vec<String> in", "char(5)", f("bpchar", T::BPCHAR), s("ab"), Some("ab   "), Some("'ab'"), vec![any("ANY {\"ab \"}", f("bpchar[]", T::BPCHAR_ARRAY), sv(&["ab "]), 1)]),
        c("Vec<i64> in", "bigint", f("int8", T::INT8), p(7i64), Some("7"), Some("7"), vec![any("ANY {7,8}", f("int8[]", T::INT8_ARRAY), p(vec![7i64, 8]), 1)]),
        c("Vec<i32> in", "bigint", f("int8", T::INT8), p(7i64), Some("7"), Some("7"), vec![any("ANY {7}", f("int8[]", T::INT8_ARRAY), p(vec![7i32]), 1)]),
        c("Vec<Uuid> in", "uuid", f("uuid", T::UUID), p(uid()), Some(uuid_s), Some(uuid_l), vec![any("ANY {the uuid}", f("uuid[]", T::UUID_ARRAY), p(vec![uid()]), 1)]),
        c("Vec<DateTime<Utc>> in", "timestamp", f("timestamp", T::TIMESTAMP), p(utc()), Some("2026-01-01 00:00:00"), Some("'2026-01-01 00:00:00'"), vec![any("ANY {the instant}", f("timestamp[]", T::TIMESTAMP_ARRAY), p(vec![utc()]), 1)]),
        c("Vec<String> in", "p9_mood", f("p9_mood", T::TEXT), s("sad"), Some("sad"), Some("'sad'"), vec![any("ANY {happy,sad}", f("p9_mood[]", T::TEXT_ARRAY), sv(&["happy", "sad"]), 1)]),
    ]
}

/// The pairs whose answer moves with the session's `TimeZone`, again under
/// `TimeZone = 'UTC'`, the zone morm's sessions always use (DH1).
fn utc_cases() -> Vec<Case> {
    use Type as T;
    vec![
        c("DateTime<Utc>", "timestamp", f("timestamp", T::TIMESTAMP), p(utc()), Some("2026-01-01 00:00:00"), Some("'2026-01-01 00:00:00'"), vec![eq("the same instant, as UTC", f("timestamp", T::TIMESTAMP), p(utc()), 1)]),
        c("SystemTime", "timestamp", f("timestamp", T::TIMESTAMP), p(instant()), Some("2026-01-01 00:00:00"), Some("'2026-01-01 00:00:00'"), vec![eq("the same instant, as UTC", f("timestamp", T::TIMESTAMP), p(instant()), 1)]),
        c("NaiveDateTime", "timestamptz", f("timestamptz", T::TIMESTAMPTZ), p(naive()), Some("2026-01-01 00:00:00+00"), Some("'2026-01-01 00:00:00+00'"), vec![eq("the same wall time, as UTC", f("timestamptz", T::TIMESTAMPTZ), p(naive()), 1)]),
        c("Vec<DateTime<Utc>> in", "timestamp", f("timestamp", T::TIMESTAMP), p(utc()), Some("2026-01-01 00:00:00"), Some("'2026-01-01 00:00:00'"), vec![any("ANY {the instant}", f("timestamp[]", T::TIMESTAMP_ARRAY), p(vec![utc()]), 1)]),
    ]
}

// ---- A small version of check-sql's typing pass --------------------------

/// A parsed `outfuncs` node tree.
#[derive(Debug, Clone)]
enum V {
    Node(String, Vec<(String, V)>),
    List(Vec<V>),
    Atom(String),
}

impl V {
    fn get(&self, k: &str) -> Option<&V> {
        match self {
            V::Node(_, fs) => fs.iter().find(|(n, _)| n == k).map(|(_, v)| v),
            _ => None,
        }
    }
    fn name(&self) -> &str {
        match self {
            V::Node(n, _) => n,
            _ => "",
        }
    }
    fn num(&self, k: &str) -> Option<u32> {
        match self.get(k) {
            Some(V::Atom(a)) => a.parse().ok(),
            _ => None,
        }
    }
    fn args(&self) -> Vec<&V> {
        match self.get("args") {
            Some(V::List(l)) => l.iter().collect(),
            _ => vec![],
        }
    }
}

fn tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut it = s.chars().peekable();
    let mut quoted = false;
    while let Some(ch) = it.next() {
        if quoted {
            cur.push(ch);
            if ch == '\\' {
                if let Some(n) = it.next() {
                    cur.push(n);
                }
            } else if ch == '"' {
                quoted = false;
            }
            continue;
        }
        match ch {
            '\\' => {
                if let Some(n) = it.next() {
                    cur.push(n);
                }
            }
            '"' if cur.is_empty() => {
                cur.push(ch);
                quoted = true;
            }
            '{' | '}' | '(' | ')' | '[' | ']' => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                out.push(ch.to_string());
            }
            c if c.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn parse(t: &[String], i: &mut usize) -> V {
    let tok = t[*i].clone();
    *i += 1;
    match tok.as_str() {
        "{" => {
            let name = t[*i].clone();
            *i += 1;
            let mut fs = Vec::new();
            while t[*i] != "}" {
                if let Some(k) = t[*i].strip_prefix(':') {
                    *i += 1;
                    let v = if t[*i] == "}" || t[*i].starts_with(':') {
                        V::Atom(String::new())
                    } else {
                        parse(t, i)
                    };
                    fs.push((k.to_owned(), v));
                } else {
                    // a datum's bytes after its length: skip
                    parse(t, i);
                }
            }
            *i += 1;
            V::Node(name, fs)
        }
        "(" | "[" => {
            let close = if tok == "(" { ")" } else { "]" };
            let mut l = Vec::new();
            while t[*i] != close {
                l.push(parse(t, i));
            }
            *i += 1;
            V::List(l)
        }
        _ => V::Atom(tok),
    }
}

/// What the rules need from the catalog, loaded once.
struct Catalog {
    domain_base: HashMap<u32, u32>,
    /// Cross-type btree operators outside the integer, float, text and
    /// network families.
    cross_ops: HashSet<u32>,
}

async fn catalog(c: &Client) -> Catalog {
    let mut domain_base = HashMap::new();
    for r in c
        .query("SELECT oid, typbasetype FROM pg_type WHERE typtype = 'd'", &[])
        .await
        .unwrap()
    {
        let (a, b): (u32, u32) = (r.get(0), r.get(1));
        domain_base.insert(a, b);
    }
    let mut cross_ops = HashSet::new();
    for r in c
        .query(
            "SELECT o.amopopr FROM pg_amop o JOIN pg_opfamily f ON f.oid = o.amopfamily
             JOIN pg_am a ON a.oid = f.opfmethod
             WHERE a.amname = 'btree' AND o.amoplefttype <> o.amoprighttype
               AND f.opfname NOT IN ('integer_ops', 'float_ops', 'text_ops', 'network_ops')",
            &[],
        )
        .await
        .unwrap()
    {
        cross_ops.insert(r.get::<_, u32>(0));
    }
    Catalog {
        domain_base,
        cross_ops,
    }
}

const IMPLICIT: u32 = 2; // COERCE_IMPLICIT_CAST

fn ty_of(n: &V) -> u32 {
    match n.name() {
        "VAR" => n.num("vartype"),
        "PARAM" => n.num("paramtype"),
        "CONST" => n.num("consttype"),
        "FUNCEXPR" => n.num("funcresulttype"),
        "OPEXPR" => n.num("opresulttype"),
        "CASETESTEXPR" => n.num("typeId"),
        _ => n.num("resulttype"),
    }
    .unwrap_or(0)
}

/// One implicit conversion: node kind, from type, to type, whether it
/// applies a typmod (a second argument), and for an array coercion, the
/// conversions it applies to each element.
struct Conv {
    kind: String,
    from: u32,
    to: u32,
    typmod: bool,
    elems: Vec<Conv>,
}

/// The conversions that change a value: an array coercion stands for its
/// element conversions.
fn flat(cs: &[Conv]) -> Vec<&Conv> {
    let mut out = Vec::new();
    for c in cs {
        if c.kind == "ARRAYCOERCEEXPR" {
            out.extend(flat(&c.elems));
        } else {
            out.push(c);
        }
    }
    out
}

/// Strips implicit conversions from the outside in, returning the node they
/// wrap and the conversions.
fn strip(n: &V) -> (&V, Vec<Conv>) {
    let mut convs = Vec::new();
    let mut cur = n;
    loop {
        let (fmt, inner) = match cur.name() {
            "RELABELTYPE" => (cur.num("relabelformat"), cur.get("arg")),
            "COERCEVIAIO" | "ARRAYCOERCEEXPR" => (cur.num("coerceformat"), cur.get("arg")),
            "COERCETODOMAIN" => (cur.num("coercionformat"), cur.get("arg")),
            "FUNCEXPR" => (cur.num("funcformat"), cur.args().first().copied()),
            _ => (None, None),
        };
        match (fmt, inner) {
            (Some(IMPLICIT), Some(inner)) => {
                let elems = match (cur.name(), cur.get("elemexpr")) {
                    ("ARRAYCOERCEEXPR", Some(e)) => strip(e).1,
                    _ => Vec::new(),
                };
                convs.push(Conv {
                    kind: cur.name().to_owned(),
                    from: ty_of(inner),
                    to: ty_of(cur),
                    typmod: cur.name() == "FUNCEXPR" && cur.args().len() > 1,
                    elems,
                });
                cur = inner;
            }
            _ => return (cur, convs),
        }
    }
}

const INTS: [u32; 3] = [21, 23, 20];

fn safe_both(cat: &Catalog, cv: &Conv) -> bool {
    cat.domain_base.get(&cv.from) == Some(&cv.to)
        || (cv.from == 1043 && cv.to == 25)
        || (cv.from == 650 && cv.to == 869)
        || cv.kind == "COERCETODOMAIN"
        || (cv.typmod && cv.from == cv.to)
}

fn safe_value(cat: &Catalog, cv: &Conv) -> bool {
    let widen = |a: u32, b: u32| {
        let ia = INTS.iter().position(|x| *x == a);
        let ib = INTS.iter().position(|x| *x == b);
        matches!((ia, ib), (Some(x), Some(y)) if x < y)
    };
    safe_both(cat, cv)
        || widen(cv.from, cv.to)
        || (INTS.contains(&cv.from) && cv.to == 1700)
        || (cv.from == 700 && cv.to == 701)
}

fn walk<'a>(n: &'a V, out: &mut Vec<&'a V>) {
    match n {
        V::Node(_, fs) => {
            out.push(n);
            for (_, v) in fs {
                walk(v, out);
            }
        }
        V::List(l) => l.iter().for_each(|v| walk(v, out)),
        V::Atom(_) => {}
    }
}

/// The rules over one analyzed query. `bound` says the parameter is bound
/// through a field (T2) rather than free (T3).
fn rules(cat: &Catalog, tree: &V, bound: bool) -> Vec<&'static str> {
    let mut found = Vec::new();
    let mut nodes = Vec::new();
    walk(tree, &mut nodes);
    for n in &nodes {
        // T1: comparisons with a table column on one side.
        if matches!(n.name(), "OPEXPR" | "SCALARARRAYOPEXPR") {
            let args = n.args();
            if args.len() != 2 {
                continue;
            }
            let (l, lc) = strip(args[0]);
            let (r, rc) = strip(args[1]);
            let (col, colc, val, valc) = if l.name() == "VAR" {
                (l, lc, r, rc)
            } else if r.name() == "VAR" {
                (r, rc, l, lc)
            } else {
                continue;
            };
            let _ = (col, val);
            let col_bad = flat(&colc).iter().any(|cv| !safe_both(cat, cv));
            let val_bad = flat(&valc).iter().any(|cv| !safe_value(cat, cv));
            let op_bad = colc.is_empty()
                && valc.is_empty()
                && n.num("opno").is_some_and(|o| cat.cross_ops.contains(&o));
            if col_bad || val_bad || op_bad {
                found.push("T1");
            }
        }
        // T2 and T3: the parameter on its way into a column or a function.
        if n.name() == "TARGETENTRY" {
            if let Some(e) = n.get("expr") {
                let (_, convs) = strip(e);
                for cv in flat(&convs) {
                    let reported = if bound {
                        !safe_value(cat, cv)
                    } else {
                        let changes = matches!(cv.kind.as_str(), "FUNCEXPR" | "COERCEVIAIO");
                        let precision = cv.typmod && [1700, 1114, 1184, 1083, 1266, 1186].contains(&cv.to);
                        (changes && !safe_value(cat, cv) && !(cv.typmod && cv.from == cv.to)) || precision
                    };
                    if reported {
                        found.push(if bound { "T2" } else { "T3" });
                    }
                }
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

/// A client whose `parse tree` notices are kept.
async fn notice_client() -> (Client, Arc<Mutex<Vec<String>>>) {
    let (c, mut conn) = tokio_postgres::connect(&app::dsn(), NoTls).await.unwrap();
    let notes = Arc::new(Mutex::new(Vec::new()));
    let keep = notes.clone();
    tokio::spawn(async move {
        let mut s = futures_util::stream::poll_fn(move |cx| conn.poll_message(cx));
        while let Some(m) = s.next().await {
            match m {
                Ok(AsyncMessage::Notice(n)) if n.message() == "parse tree:" => {
                    keep.lock().unwrap().push(n.detail().unwrap_or("").to_owned());
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });
    (c, notes)
}

struct Ctx {
    c: Client,
    notes: Arc<Mutex<Vec<String>>>,
    cat: Catalog,
}

/// How a failed statement shows in the table.
fn outcome(e: tokio_postgres::Error) -> String {
    match e.as_db_error() {
        Some(db) => format!("error {}", db.code().code()),
        None => format!("error {e}"),
    }
}

/// Runs `sql` with one parameter through `query_typed` and through a
/// prepared statement. Returns each run's rows or error, and the rules'
/// findings from the analyzed tree.
async fn run(
    x: &Ctx,
    sql: &str,
    b: &(dyn ToSql + Sync),
    ty: Type,
    bound: bool,
) -> (Vec<Result<Vec<tokio_postgres::Row>, String>>, String) {
    let a = x
        .c
        .query_typed(sql, &[(b, ty.clone())])
        .await
        .map_err(outcome);
    x.notes.lock().unwrap().clear();
    let prepared = x.c.prepare_typed(sql, &[ty]).await;
    let tree = x.notes.lock().unwrap().first().cloned();
    let found = match (&prepared, tree) {
        (Ok(_), Some(t)) => {
            let toks = tokens(&t);
            let tree = parse(&toks, &mut 0);
            let f = rules(&x.cat, &tree, bound);
            if f.is_empty() { "-".to_owned() } else { f.join(", ") }
        }
        _ => "-".to_owned(),
    };
    let b2 = match prepared {
        Ok(st) => x.c.query(&st, &[b]).await.map_err(outcome),
        Err(e) => Err(outcome(e)),
    };
    (vec![a, b2], found)
}

/// One cell's text from both runs.
fn cell<F: Fn(&[tokio_postgres::Row]) -> String>(
    rs: &[Result<Vec<tokio_postgres::Row>, String>],
    show: F,
) -> String {
    let each: Vec<String> = rs
        .iter()
        .map(|r| match r {
            Ok(rows) => show(rows),
            Err(e) => e.clone(),
        })
        .collect();
    if each[0] == each[1] {
        each[0].clone()
    } else {
        format!("{} / prepared: {}", each[0], each[1])
    }
}

async fn reset(x: &Ctx, col: &str, fixture: Option<&str>) {
    let mut ddl = format!("DROP TABLE IF EXISTS pg_temp.m; CREATE TEMP TABLE m (c {col});");
    if let Some(f) = fixture {
        ddl.push_str(&format!(" INSERT INTO m VALUES ({f});"));
    }
    x.c.batch_execute(&ddl).await.unwrap();
}

async fn insert(x: &Ctx, case: &Case, bound: bool) -> (String, String) {
    reset(x, case.col, None).await;
    let (cast, b, ty) = if bound {
        field(&*case.value, &case.field)
    } else {
        free(&*case.value)
    };
    let sql = format!("INSERT INTO m VALUES (($1::{cast}))");
    let (rs, found) = run(x, &sql, &b, ty.clone(), bound).await;
    // What each run stored, read back after it.
    let mut shown = Vec::new();
    for r in &rs {
        shown.push(r.as_ref().map(|_| ()).map_err(|e| e.clone()));
    }
    let rows = x
        .c
        .query(
            "SELECT CASE WHEN c IS NULL THEN NULL ELSE concat(c) END FROM m",
            &[],
        )
        .await
        .unwrap();
    let stored: Vec<Option<String>> = rows.iter().map(|r| r.get(0)).collect();
    let judge = |r: &Result<(), String>, got: Option<&Option<String>>| -> String {
        match r {
            Err(e) => {
                if Some(e.as_str()) == case.stored {
                    "ok".into()
                } else {
                    e.clone()
                }
            }
            Ok(()) => {
                let got = got.and_then(|g| g.as_deref());
                if got == case.stored {
                    "ok".into()
                } else {
                    format!("wrong: {}", got.unwrap_or("NULL"))
                }
            }
        }
    };
    // Rows land in order: the query_typed run first, then the prepared one.
    let mut it = stored.iter();
    let first = judge(&shown[0], if shown[0].is_ok() { it.next() } else { None });
    let second = judge(&shown[1], if shown[1].is_ok() { it.next() } else { None });
    let text = if first == second {
        first
    } else {
        format!("{first} / prepared: {second}")
    };
    (text, found)
}

async fn compare(x: &Ctx, case: &Case, fixture: &str, pr: &Probe, bound: bool) -> (String, String) {
    reset(x, case.col, Some(fixture)).await;
    let (cast, b, ty) = if bound {
        field(&*pr.value, &pr.field)
    } else {
        free(&*pr.value)
    };
    let sql = if pr.any {
        format!("SELECT count(*) FROM m WHERE c = ANY(($1::{cast}))")
    } else {
        format!("SELECT count(*) FROM m WHERE c = ($1::{cast})")
    };
    let (rs, found) = run(x, &sql, &b, ty, bound).await;
    let text = cell(&rs, |rows| {
        let n: i64 = rows[0].get(0);
        if n == pr.want {
            "ok".into()
        } else {
            format!("wrong: {n} rows")
        }
    });
    (text, found)
}

fn row(rust: &str, col: &str, check: &str, fld: (String, String), fr: (String, String)) -> String {
    let differs = fr.0 != fld.0 && !fr.0.starts_with("error");
    let rep = if differs && fr.1 == "-" {
        "UNREPORTED".to_owned()
    } else {
        fr.1
    };
    format!(
        "| {rust} | {col} | {check} | {} | {} | {} | {rep} |\n",
        fld.0, fld.1, fr.0
    )
}

async fn matrix(x: &Ctx, cases: Vec<Case>, table: &mut String) {
    for case in cases {
        let fld = insert(x, &case, true).await;
        let fr = insert(x, &case, false).await;
        table.push_str(&row(case.rust, case.col, "insert", fld, fr));
        if let Some(fx) = case.fixture {
            for pr in &case.probes {
                let fld = compare(x, &case, fx, pr, true).await;
                let fr = compare(x, &case, fx, pr, false).await;
                table.push_str(&row(case.rust, case.col, &format!("= {}", pr.what), fld, fr));
            }
        }
    }
}

/// Free values where Postgres has nothing to infer a type from: each is
/// cast to its own type, so these need no cast of the caller's own.
struct Raw {
    sql: &'static str,
    value: P,
    shown: &'static str,
    want: &'static str,
}

fn raws() -> Vec<Raw> {
    vec![
        Raw { sql: "SELECT ($1::text)", value: s("x"), shown: "x", want: "x" },
        Raw { sql: "SELECT (($1::text) IS NULL)::text", value: s("x"), shown: "x", want: "false" },
        Raw { sql: "SELECT concat(($1::text), 'x')", value: s("a"), shown: "a", want: "ax" },
        Raw { sql: "SELECT coalesce(($1::text), 'd')", value: s("x"), shown: "x", want: "x" },
        Raw { sql: "SELECT (($1::text))::int::text", value: s("12"), shown: "12", want: "12" },
        Raw { sql: "SELECT (1234::int4 = ($1::text))::text", value: s("1234"), shown: "1234", want: "error 42883" },
        Raw { sql: "SELECT (1234::int4 = ($1::int8))::text", value: p(1234i64), shown: "1234", want: "true" },
        Raw { sql: "SELECT array_length(($1::text[]), 1)::text", value: sv(&["a", "b"]), shown: "[a, b]", want: "2" },
        Raw { sql: "SELECT string_agg(x, ',') FROM unnest(($1::text[])) x", value: sv(&["a", "b"]), shown: "[a, b]", want: "a,b" },
        Raw { sql: "SELECT ('x'::text = ($1::text[]))::text", value: sv(&["a", "b"]), shown: "[a, b]", want: "error 42883" },
        Raw { sql: "SELECT ('x' = ANY(($1::text)))::text", value: s("x"), shown: "x", want: "error 42809" },
    ]
}

#[tokio::test]
async fn every_type_against_every_column_an_explicit_sql_gives() {
    let (c, notes) = notice_client().await;
    c.batch_execute(
        "CREATE EXTENSION IF NOT EXISTS citext;
         DROP TYPE IF EXISTS p9_mood CASCADE; DROP DOMAIN IF EXISTS p9_posint CASCADE;
         DROP DOMAIN IF EXISTS p9_email CASCADE;
         CREATE TYPE p9_mood AS ENUM ('happy', 'sad');
         CREATE DOMAIN p9_posint AS integer CHECK (VALUE > 0);
         CREATE DOMAIN p9_email AS citext;",
    )
    .await
    .unwrap();
    let cat = catalog(&c).await;
    c.batch_execute(
        "SET TimeZone = 'Asia/Dhaka'; SET debug_print_parse = on; SET debug_pretty_print = off;
         SET client_min_messages = log",
    )
    .await
    .unwrap();
    let x = Ctx { c, notes, cat };
    let version: String = x
        .c
        .query_one("SHOW server_version", &[])
        .await
        .unwrap()
        .get(0);

    let mut table = String::from(
        "| Rust value | Column | Check | Field | Field rules | Free | Free rules |\n|---|---|---|---|---|---|---|\n",
    );
    matrix(&x, cases(), &mut table).await;
    table.push_str(
        "\n| Under `TimeZone = 'UTC'` | Column | Check | Field | Field rules | Free | Free rules |\n|---|---|---|---|---|---|---|\n",
    );
    x.c.batch_execute("SET TimeZone = 'UTC'").await.unwrap();
    matrix(&x, utc_cases(), &mut table).await;

    table.push_str("\n| Free value, nothing to infer from | Value | Result |\n|---|---|---|\n");
    for r in raws() {
        let (_, b, ty) = free(&*r.value);
        let (rs, _) = run(&x, r.sql, &b, ty, false).await;
        let text = cell(&rs, |rows| {
            let got: Option<String> = rows[0].get(0);
            match got {
                Some(g) if g == r.want => "ok".into(),
                Some(g) => format!("wrong: {g}"),
                None => "wrong: NULL".into(),
            }
        });
        let text = if text == r.want { "ok".to_owned() } else { text };
        table.push_str(&format!(
            "| `{}` | `{}` | {} |\n",
            r.sql.replace('|', "\\|"),
            r.shown,
            text
        ));
    }
    x.c.batch_execute(
        "RESET debug_print_parse; RESET client_min_messages;
         DROP TYPE p9_mood CASCADE; DROP DOMAIN p9_posint CASCADE; DROP DOMAIN p9_email CASCADE",
    )
    .await
    .unwrap();
    println!("Postgres {version}\n\n{table}");
    assert!(
        !table.contains("UNREPORTED"),
        "a free value's answer differs from the field's and no rule reports it"
    );
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
