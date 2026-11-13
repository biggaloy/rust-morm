//! Shared helpers for step 55's proofs. Each proof is a binary under
//! `src/bin/` that prints what it saw as a table and exits non-zero when a
//! check fails.

pub mod stale;

use bytes::BytesMut;
use std::error::Error as StdError;
use std::sync::{Arc, Mutex};
use tokio_postgres::types::{FromSql, IsNull, Kind, ToSql, Type, to_sql_checked};
use tokio_postgres::{AsyncMessage, Client, Error, NoTls};

/// A connection, with the notices the server sent it, and the error that
/// closed it, such as a `FATAL` the server sent while it was idle.
pub struct Session {
    pub c: Client,
    notices: Arc<Mutex<Vec<String>>>,
    closed: Arc<Mutex<Option<String>>>,
}

impl Session {
    /// The notices received since the last call.
    pub fn take_notices(&self) -> Vec<String> {
        std::mem::take(&mut *self.notices.lock().unwrap())
    }
    /// Why the connection closed, if the server said.
    pub fn closed_by(&self) -> Option<String> {
        self.closed.lock().unwrap().clone()
    }
}

impl std::ops::Deref for Session {
    type Target = Client;
    fn deref(&self) -> &Client {
        &self.c
    }
}

pub async fn connect(dsn: &str) -> Result<Session, Error> {
    let (c, mut connection) = tokio_postgres::connect(dsn, NoTls).await?;
    let notices = Arc::new(Mutex::new(Vec::new()));
    let closed = Arc::new(Mutex::new(None));
    let sink = notices.clone();
    let why = closed.clone();
    tokio::spawn(async move {
        loop {
            match std::future::poll_fn(|cx| connection.poll_message(cx)).await {
                Some(Ok(AsyncMessage::Notice(n))) => sink.lock().unwrap().push(format!(
                    "{} {}: {}",
                    n.severity(),
                    n.code().code(),
                    n.message()
                )),
                Some(Ok(_)) => {}
                Some(Err(e)) => {
                    *why.lock().unwrap() = Some(show(&e));
                    break;
                }
                None => break,
            }
        }
    });
    Ok(Session { c, notices, closed })
}

/// Connects or stops the proof: every proof needs its servers.
pub async fn must_connect(dsn: &str) -> Session {
    connect(dsn)
        .await
        .unwrap_or_else(|e| panic!("connecting to {dsn}: {}", show(&e)))
}

pub fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} is not set"))
}

/// Replaces each run of four or more digits, such as an OID, with `<n>`, so
/// runs on different servers compare.
pub fn norm(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut digits = String::new();
    for ch in s.chars().chain(std::iter::once('\0')) {
        if ch.is_ascii_digit() {
            digits.push(ch);
            continue;
        }
        if digits.len() >= 4 {
            out.push_str("<n>");
        } else {
            out.push_str(&digits);
        }
        digits.clear();
        if ch != '\0' {
            out.push(ch);
        }
    }
    out
}

/// An error as `SQLSTATE message [routine]`, or the client's own error.
pub fn show(e: &Error) -> String {
    match e.as_db_error() {
        Some(db) => format!(
            "{} {} [{}]",
            db.code().code(),
            norm(db.message()),
            db.routine().unwrap_or("-")
        ),
        None => match e.source() {
            Some(src) => format!(
                "client: {} ({})",
                norm(&e.to_string()),
                norm(&src.to_string())
            ),
            None => format!("client: {}", norm(&e.to_string())),
        },
    }
}

/// The SQLSTATE of an error, or `client`.
pub fn code(e: &Error) -> String {
    e.as_db_error()
        .map(|db| db.code().code().to_owned())
        .unwrap_or_else(|| "client".to_owned())
}

/// An enum label, bound as a `text` carrier and decoded from any enum or from
/// `text`. It has no OID and no variant list: the cast in the SQL names the
/// type, so a type swap can't leave anything here stale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label(pub String);

pub fn label(s: &str) -> Label {
    Label(s.to_owned())
}

impl ToSql for Label {
    fn to_sql(
        &self,
        _: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn StdError + Sync + Send>> {
        out.extend_from_slice(self.0.as_bytes());
        Ok(IsNull::No)
    }
    fn accepts(ty: &Type) -> bool {
        *ty == Type::TEXT
    }
    to_sql_checked!();
}

impl<'a> FromSql<'a> for Label {
    fn from_sql(_: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn StdError + Sync + Send>> {
        Ok(Label(std::str::from_utf8(raw)?.to_owned()))
    }
    fn accepts(ty: &Type) -> bool {
        *ty == Type::TEXT || matches!(ty.kind(), Kind::Enum(_))
    }
}

/// What a proof prints: rows of a table, and the checks that failed.
#[derive(Default)]
pub struct Report {
    failures: Vec<String>,
}

impl Report {
    pub fn row(&mut self, cols: &[&str]) {
        println!("| {} |", cols.join(" | "));
    }
    pub fn header(&mut self, title: &str, cols: &[&str]) {
        println!();
        println!("### {title}");
        println!();
        println!("| {} |", cols.join(" | "));
        println!("|{}", "---|".repeat(cols.len()));
    }
    /// A check the proof requires: a failure fails the run.
    pub fn check(&mut self, name: &str, ok: bool, detail: &str) {
        let verdict = if ok { "pass" } else { "FAIL" };
        println!("| check | {name} | {verdict} | {detail} |");
        if !ok {
            self.failures.push(format!("{name}: {detail}"));
        }
    }
    pub fn note(&mut self, text: &str) {
        println!();
        println!("{text}");
    }
    /// Prints the failures and exits non-zero if there were any.
    pub fn finish(self, proof: &str) {
        println!();
        if self.failures.is_empty() {
            println!("{proof}: every check passed");
        } else {
            println!("{proof}: {} check(s) failed:", self.failures.len());
            for f in &self.failures {
                println!("- {f}");
            }
            std::process::exit(1);
        }
    }
}

/// Waits until a standby has replayed everything the primary has written.
pub async fn wait_replay(primary: &Client, standby: &Client) {
    let lsn: String = primary
        .query_one("SELECT pg_current_wal_lsn()::text", &[])
        .await
        .unwrap()
        .get(0);
    for _ in 0..600 {
        let done: bool = standby
            .query_one(
                "SELECT pg_last_wal_replay_lsn() >= (($1::text))::pg_lsn",
                &[&lsn],
            )
            .await
            .unwrap()
            .get(0);
        if done {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("the standby didn't replay {lsn} in 30 s");
}

pub async fn server_version(c: &Client) -> String {
    c.query_one("SHOW server_version", &[])
        .await
        .unwrap()
        .get::<_, String>(0)
}
