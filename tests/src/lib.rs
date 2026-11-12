//! Test helper: reaches the Postgres server a test run targets.
//!
//! The target comes from the environment:
//!
//! - `MORM_TEST_TRANSPORT`: `tcp` (default), `unix` or `pgbouncer`.
//! - `MORM_TEST_DSN`: connect to this server, as a libpq keyword string.
//! - `MORM_TEST_PG_BIN`: with no `MORM_TEST_DSN`, start a throwaway server from
//!   the `initdb` and `pg_ctl` in this directory, or else from `PATH`. Setting it
//!   to each install in turn runs the tests on each version.
//! - `MORM_TEST_PG_MAJOR`: the major version the server must be.
//!
//! A server that can't be reached fails the test. Nothing is skipped.

use std::env;
use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use tokio_postgres::{Client, NoTls, SimpleQueryMessage};

/// How a test reaches the server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transport {
    Tcp,
    Unix,
    PgBouncer,
}

/// The server this test run targets.
#[derive(Debug)]
pub struct Target {
    pub transport: Transport,
    pub major: Option<u32>,
    pub dsn: String,
}

/// A server this process started, stopped when the process exits.
struct LocalServer {
    pg_ctl: PathBuf,
    dir: PathBuf,
}

static TARGET: OnceLock<Target> = OnceLock::new();
static LOCAL: OnceLock<LocalServer> = OnceLock::new();

/// The target for this run, starting a local server on first use if needed.
pub fn target() -> &'static Target {
    TARGET.get_or_init(|| {
        let transport = match env::var("MORM_TEST_TRANSPORT").as_deref() {
            Err(_) | Ok("tcp") => Transport::Tcp,
            Ok("unix") => Transport::Unix,
            Ok("pgbouncer") => Transport::PgBouncer,
            Ok(other) => panic!("MORM_TEST_TRANSPORT: unknown transport {other:?}"),
        };
        let major = env::var("MORM_TEST_PG_MAJOR")
            .ok()
            .map(|m| m.parse().expect("MORM_TEST_PG_MAJOR must be a number"));
        let dsn = env::var("MORM_TEST_DSN").unwrap_or_else(|_| start_local(transport));
        Target {
            transport,
            major,
            dsn,
        }
    })
}

/// Connects to the target.
pub async fn connect() -> Client {
    connect_to(&target().dsn).await
}

/// Connects to a server by its libpq keyword string.
pub async fn connect_to(dsn: &str) -> Client {
    let (client, connection) = tokio_postgres::connect(dsn, NoTls)
        .await
        .unwrap_or_else(|e| panic!("can't connect to the test server: {e}"));
    tokio::spawn(connection);
    client
}

/// The rows of a statement, over the simple protocol that PgBouncer's admin
/// console and transaction mode both accept.
pub async fn simple_rows(client: &Client, sql: &str) -> Vec<tokio_postgres::SimpleQueryRow> {
    client
        .simple_query(sql)
        .await
        .unwrap_or_else(|e| panic!("{sql}: {e}"))
        .into_iter()
        .filter_map(|message| match message {
            SimpleQueryMessage::Row(row) => Some(row),
            _ => None,
        })
        .collect()
}

/// Fails unless `client` really is on the version and transport this run targets.
pub async fn assert_on_target(client: &Client) {
    let target = target();
    if let Some(major) = target.major {
        let rows = simple_rows(client, "show server_version_num").await;
        let version: u32 = rows[0].get(0).unwrap().parse().unwrap();
        assert_eq!(version / 10_000, major, "wrong Postgres version");
    }
    if target.transport != Transport::PgBouncer {
        let rows = simple_rows(client, "select inet_server_addr() is null").await;
        let on_socket = rows[0].get(0) == Some("t");
        assert_eq!(
            on_socket,
            target.transport == Transport::Unix,
            "connected over the wrong transport"
        );
    }
}

fn start_local(transport: Transport) -> String {
    assert!(
        transport != Transport::PgBouncer,
        "the pgbouncer transport needs MORM_TEST_DSN"
    );
    let bin = env::var_os("MORM_TEST_PG_BIN").map(PathBuf::from);
    let tool = |name: &str| bin.as_deref().map_or(name.into(), |dir| dir.join(name));
    let dir = env::temp_dir().join(format!("morm-test-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let data = dir.join("data");
    let log = dir.join("postgres.log");
    let port = free_port();

    run(Command::new(tool("initdb"))
        .args(["--auth=trust", "--username=morm", "--encoding=UTF8", "-D"])
        .arg(&data));
    run(Command::new(tool("pg_ctl"))
        .args(["start", "-w", "-D"])
        .arg(&data)
        .arg("-l")
        .arg(&log)
        .arg("-o")
        .arg(format!(
            "-p {port} -k {} -c listen_addresses=127.0.0.1 -c fsync=off",
            dir.display()
        )));

    let server = LocalServer {
        pg_ctl: tool("pg_ctl"),
        dir: dir.clone(),
    };
    assert!(
        LOCAL.set(server).is_ok(),
        "the local server is started once"
    );
    assert_eq!(atexit(stop_local), 0, "can't register atexit");

    let host = match transport {
        Transport::Unix => dir.display().to_string(),
        _ => "127.0.0.1".to_owned(),
    };
    format!("host={host} port={port} user=morm dbname=postgres")
}

unsafe extern "C" {
    safe fn atexit(callback: extern "C" fn()) -> std::ffi::c_int;
}

extern "C" fn stop_local() {
    if let Some(server) = LOCAL.get() {
        let _ = Command::new(&server.pg_ctl)
            .args(["stop", "-w", "-m", "immediate", "-D"])
            .arg(server.dir.join("data"))
            .output();
        let _ = fs::remove_dir_all(&server.dir);
    }
}

fn free_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.local_addr().unwrap().port()
}

fn run(command: &mut Command) {
    let output = command
        .output()
        .unwrap_or_else(|e| panic!("can't run {command:?}: {e}"));
    assert!(
        output.status.success(),
        "{command:?} failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
