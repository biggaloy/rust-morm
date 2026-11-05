# Step 09: Config, connecting and the pool

- **Phase:** Runtime
- **Depends on:** 03
- **Can run beside:** 04, 10, 21
- **Crates:** `morm`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

`morm.toml`, environment variables and `morm::Pool`, with every sizing, safety and failure rule.

## Deliverables

- Config loading, `Db::connect`, `connect_as`, `connect_to`, TLS, sockets, named pools, replicas
- `morm::Pool` on `deadpool`, dynamic size, recycling, `max_connections` budget shared by pools, failover, `db.hold()`, `db.close()`, `pool_stats()`
- `pooler = "transaction"` behaviour

## End-to-end check

An app starts from `morm.toml` alone, runs `SELECT 1` through every pool type over TCP and socket, grows and shrinks under load, survives a Postgres restart, and shuts down cleanly. The same flow passes behind PgBouncer.

## Out of scope

- Statement caches (step 11)
- Deadlock rules (step 16)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools

- **R-0047** `tls`: rustls for TCP connections. The connector and its certificates load only when `sslmode` needs them, so with it compiled in and unused, the cost is binary size and build time. Unix sockets never use TLS

### Crates and tools / Configuration: `morm.toml`

**R-0058** TOML is the Rust standard, with comments, real types and sections, and no indentation traps. Settings need no Rust code.

**R-0059**

```toml
[database]
# Either a URL, or the separate fields below. Separate fields override URL parts.
url = "${DATABASE_URL}"
host = "/var/run/postgresql"     # Unix socket directory or hostname
port = 5432
name = "app"
user = "app"                     # no password needed with peer auth on a Unix socket
password = "${MORM_DB_PASSWORD}" # keep secrets in the environment
sslmode = "disable"              # disable, prefer, require, verify-ca, verify-full
target_session_attrs = "any"     # "read-write" finds the primary among several hosts
connect_timeout = "5s"
application_name = "auto"        # the binary's name; shows in pg_stat_activity
search_path = "public"           # pinned, so another role can't shadow objects
pooler = "none"                  # "transaction" when connecting through PgBouncer in that mode

[database.migration]             # owner role, so the app can run with fewer privileges
url = "${MORM_MIGRATION_URL}"

[server]                         # optional: the database server, which SQL can't report
ram = "8G"
cores = 4

[pool]
min = "auto"                     # 2 x [server] cores, or of this machine if unset
max = "auto"                     # 10 x min
processes = 1                    # app processes sharing this database
idle_timeout = "5m"              # shrink to 1 after this
last_idle_timeout = "1h"         # then close the last one
max_queries = 50000              # 0 = no limit
max_lifetime = "off"
hold_warn_after = "1h"
wait_timeout = "5s"              # longest wait for a connection; can't be off
max_holds = "auto"               # max / 4, at least 1
lock_timeout = "5s"              # longest wait for one lock
statement_timeout = "off"        # longest single statement
statement_cache = 256            # prepared compile-time statements per connection
shape_cache = 128                # prepared runtime-built statements per connection
shutdown_timeout = "30s"         # db.close() waits this long
plan_cache_mode = "auto"         # or force_custom_plan, force_generic_plan
transaction_timeout = "30s"      # longest db.transaction
idle_in_transaction_session_timeout = "1m"
client_connection_check_interval = "5s"
tx_warn_after = "1s"

[pools.replica]                  # a read replica
host = "db-replica.internal"
read_only = true

[pools.replica.server]           # a pool on a different server describes that server
ram = "16G"
cores = 8

[pools.jobs]                     # optional named profile
user = "app_jobs"                # any [database] field can be set here
password = "${MORM_JOBS_PASSWORD}"
min = 1
max = 4

[outbox]
batch = 100                      # events claimed at once
poll_interval = "1s"
lease = "60s"                    # a crashed worker's events return after this
max_attempts = 10
retention = "7d"                 # how long delivered events are kept

[telemetry]                      # used by the tracing and metrics features
span_sample = 0.01               # share of queries that get a span: 0 = none, 1 = all
slow_query = "250ms"             # always report slower queries, pool wait included; "off"
metrics_interval = "10s"         # how often metrics are published

[migrations]
dir = "morm-migrations"
min_postgres = 15
scratch_url = "${MORM_SCRATCH_URL}"
```

**R-0060** A missing `morm.toml` isn't an error unless `MORM_CONFIG` names it, so a deployment can use environment variables alone. An unset `${VAR}` leaves that field unset, and if a required value ends up missing, startup fails and names the variable.

**R-0061** Precedence: defaults, then `morm.toml`, then environment variables, then code. There is one config file, and a process that needs different values uses a named pool, an environment variable, or code.

**R-0062** **Environment variables:**

| ID | Variable | Overrides |
|---|---|---|
| R-0063 | `MORM_CONFIG` | Path to the config file (default `morm.toml`) |
| R-0064 | `DATABASE_URL` | Full database URL |
| R-0065 | `MORM_DB_HOST`, `MORM_DB_PORT` | Host or Unix socket directory, and port |
| R-0066 | `MORM_DB_NAME` | Database name |
| R-0067 | `MORM_DB_USER` | User |
| R-0068 | `MORM_DB_PASSWORD` | Password |
| R-0069 | `MORM_DB_PASSWORD_FILE` | File holding the password, for systemd credentials and secret mounts |
| R-0070 | `MORM_DB_SSLMODE` | `disable`, `prefer`, `require`, `verify-ca`, `verify-full` |
| R-0071 | `MORM_DB_SSLROOTCERT` | CA certificate path |
| R-0072 | `MORM_DB_SSLCERT`, `MORM_DB_SSLKEY` | Client certificate and key paths |
| R-0073 | `MORM_DB_TARGET_SESSION_ATTRS`, `MORM_DB_CONNECT_TIMEOUT` | `target_session_attrs`, `connect_timeout` |
| R-0074 | `MORM_DB_APPLICATION_NAME`, `MORM_DB_SEARCH_PATH` | `application_name`, `search_path` |
| R-0075 | `MORM_DB_POOLER` | `pooler` |
| R-0076 | `MORM_MIGRATION_URL` | Full owner-role URL for migrations |
| R-0077 | `MORM_MIGRATION_USER`, `MORM_MIGRATION_PASSWORD` | Owner-role login, using the same host and database |
| R-0078 | `MORM_MIGRATION_PASSWORD_FILE` | File holding the owner-role password |
| R-0079 | `MORM_SCRATCH_URL` | Scratch database URL |
| R-0080 | `MORM_MIGRATIONS_DIR` | Migrations directory |
| R-0081 | `MORM_OUTBOX_BATCH`, `MORM_OUTBOX_POLL_INTERVAL` | `batch`, `poll_interval` |
| R-0082 | `MORM_OUTBOX_LEASE`, `MORM_OUTBOX_MAX_ATTEMPTS`, `MORM_OUTBOX_RETENTION` | `lease`, `max_attempts`, `retention` |
| R-0083 | `MORM_SERVER_RAM`, `MORM_SERVER_CORES` | `[server]` `ram`, `cores` |
| R-0084 | `MORM_TELEMETRY_SPAN_SAMPLE`, `MORM_TELEMETRY_SLOW_QUERY` | `span_sample`, `slow_query` |
| R-0085 | `MORM_TELEMETRY_METRICS_INTERVAL` | `metrics_interval` |
| R-0086 | `MORM_POOL` | Named pool this process opens, instead of `[pool]` |
| R-0087 | `MORM_POOL_MIN`, `MORM_POOL_MAX` | `min`, `max` |
| R-0088 | `MORM_POOL_PROCESSES` | `processes` |
| R-0089 | `MORM_POOL_IDLE_TIMEOUT`, `MORM_POOL_LAST_IDLE_TIMEOUT` | `idle_timeout`, `last_idle_timeout` |
| R-0090 | `MORM_POOL_MAX_QUERIES`, `MORM_POOL_MAX_LIFETIME` | `max_queries`, `max_lifetime` |
| R-0091 | `MORM_POOL_HOLD_WARN_AFTER` | `hold_warn_after` |
| R-0092 | `MORM_POOL_WAIT_TIMEOUT`, `MORM_POOL_MAX_HOLDS` | `wait_timeout`, `max_holds` |
| R-0093 | `MORM_POOL_LOCK_TIMEOUT`, `MORM_POOL_TRANSACTION_TIMEOUT` | `lock_timeout`, `transaction_timeout` |
| R-0094 | `MORM_POOL_IDLE_IN_TRANSACTION_SESSION_TIMEOUT` | `idle_in_transaction_session_timeout` |
| R-0095 | `MORM_POOL_CLIENT_CONNECTION_CHECK_INTERVAL` | `client_connection_check_interval` |
| R-0096 | `MORM_POOL_TX_WARN_AFTER` | `tx_warn_after` |
| R-0097 | `MORM_POOL_STATEMENT_TIMEOUT`, `MORM_POOL_PLAN_CACHE_MODE` | `statement_timeout`, `plan_cache_mode` |
| R-0098 | `MORM_POOL_STATEMENT_CACHE`, `MORM_POOL_SHAPE_CACHE` | `statement_cache`, `shape_cache` |
| R-0099 | `MORM_POOL_SHUTDOWN_TIMEOUT` | `shutdown_timeout` |

**R-0100** `MORM_POOL_*` variables apply to the pool this process opens at startup.

**R-0101** **Unix sockets:** the default `host` is `/var/run/postgresql`, the Ubuntu and Debian location. A `host` starting with `/` is a socket directory, and anything else is a hostname using TCP. The socket file is `<host>/.s.PGSQL.<port>`, so `port` still matters. Other systems set `host` to match Postgres's `unix_socket_directories`; Arch and Manjaro usually use `/run/postgresql`. If `host` is a full socket file path, morm takes the directory and port from it. `morm-admin doctor` reports the exact socket path it tries, and warns if `host` no longer matches a live socket.

**R-0107** **`search_path`** is set on every connection, so another role can't shadow a table or function through a schema earlier in the path. `lint` warns about a `SECURITY DEFINER` function with no `SET search_path` of its own.

**R-0108** **Credentials:** `MORM_DB_*` values override the matching parts of `DATABASE_URL`, so a URL can hold the address while the password comes from `MORM_DB_PASSWORD_FILE`. Setting both `MORM_DB_PASSWORD` and `MORM_DB_PASSWORD_FILE` is an error. `sslmode` values other than `disable` and `prefer` need the `tls` feature, and morm refuses to start without it. `prefer` without `tls` can only connect unencrypted, so morm warns at startup, and it never connects unencrypted silently. Passwords are never logged, and they're masked in `Debug` output and error messages. `morm-admin doctor` checks that credentials work, warns if `morm.toml` contains a literal password, and warns if Postgres can log query values: `log_parameter_max_length` and `auto_explain.log_parameter_max_length` default to full values, which puts anything sent as a parameter, including `secret` columns, in the server log.

**R-0109** **Named pools** are full connection profiles. Any `[database]` field can be set in `[pools.<name>]`, so a job can log in as another role or reach another host. Unset connection fields come from `[database]`, and unset pool settings from `[pool]`, except `min`, which defaults to 1. A separate process can open only its own profile, so job settings live in the same file:

**R-0110**

```rust
let db = morm::Db::connect_as("jobs").await?;   // opens only [pools.jobs]
```

**R-0111** `MORM_POOL=jobs` with plain `Db::connect()` does the same without a code change. `db.pool("jobs")` is different: it adds that pool alongside the default one in a process that has both.

**R-0112** **Read replicas** work through a named pool: `db.pool("replica").select::<Order>()...`. You choose which queries go there. A replica can lag behind the primary, so a read straight after a write belongs on `db`. `read_only = true` makes a write on that pool fail with `ReadOnly` at once, sets `default_transaction_read_only`, and exempts it from the `25006` discard rule.

### Reading and writing / Connecting

**R-0315**

```rust
let db = morm::Db::connect().await?;                       // reads morm.toml: [database], [pool]
let db = morm::Db::connect_as("jobs").await?;              // reads morm.toml: [pools.jobs]
let db = morm::Db::connect_to(url).min(10).max(90).await?; // or set values in code
```

**R-0316** `connect` honours `MORM_POOL`, so a process can switch profile without a code change. `connect_as` names the profile in code and fails if it doesn't exist. Both return a builder, so `.min()` and `.max()` work there too.

### Performance and memory / `morm::Pool`

**R-0581** `morm::Pool` is the generic `deadpool` crate with our own connection manager, not `deadpool-postgres`, whose statement cache has no size limit. `deadpool` provides the tested parts: waiting with timeouts, closing and resizing. morm provides the rest:

- **R-0582** **Statement caches:** two bounded LRUs on each connection, below.
- **R-0583** **Health check:** `client.is_closed()` at checkout, with no extra query, so checkout adds no round trip.
- **R-0584** **Errors:** a pool wait timeout becomes a typed `PoolBusy`.
- **R-0585** **Sizing:** warm start, idle shrinking and limit checks, below.

**R-0586** Each query takes a connection, runs, and releases it. Connections are not shared between tasks, so one slow query only holds its own connection, a dropped connection fails only its own request, and timeouts work normally. Queries joined on one `tx` are pipelined on its connection.

**R-0587** Everything goes through `db`, so the pool can be replaced later without changing application code.

### Performance and memory / Dynamic size

| ID | Setting | Default | Meaning |
|---|---|---|---|
| R-0588 | `min` | 2 × cores | Connections opened at startup |
| R-0589 | `max` | 10 × `min` | The ceiling |
| R-0590 | `idle_timeout` | 5m | Idle connections close, down to 1 |
| R-0591 | `last_idle_timeout` | 1h | The last connection closes too, so an idle app can reach 0 |
| R-0592 | `max_queries` | 50000 | Replace a connection after this many queries; 0 disables it |
| R-0593 | `max_lifetime` | off | Replace a connection after this age |

- **R-0594** **`min` is not a floor.** It only sets the warm start. Connections reopen on demand, and the first query after a full idle period pays the connection cost.
- **R-0595** **Limits never interrupt work.** `max_queries` and `max_lifetime` are checked only when a connection returns to the pool.
- **R-0596** **Custom rules:** `Db::connect().recycle_when(|c| c.queries > 50_000 || c.age > hours(6))`.
- **R-0597** **Cores:** Postgres doesn't report its core count over SQL. The default uses `[server] cores`, or the machine running the app when that's unset. If the database is elsewhere, set `[server]`.
- **R-0598** **Failover:** on a pool that isn't `read_only`, a connection that returns `25006` (read-only transaction, after a primary switch) or an admin shutdown error is discarded, not reused. With several hosts and `target_session_attrs = "read-write"`, replacements find the new primary.
- **R-0599** **Queues:** a request that can't get a connection waits up to `wait_timeout`, then returns `PoolBusy`. Waits are never infinite.
- **R-0600** **Separate processes:** a service job, cron or background worker running as its own process must not start a pool with `min` above 1. It should use `Db::connect_as("jobs")`, `MORM_POOL=jobs`, or `MORM_POOL_MIN=1`, because the default of 2 × cores would open connections it doesn't need. The pool still grows on demand.

### Performance and memory / Sizing guidance

- **R-0601** Connections needed ≈ queries per second × seconds each query holds a connection. For example, 2,000 queries/s × 5ms needs about 10.
- **R-0602** Only as many queries run at once as Postgres has free CPU cores. Extra connections help when queries wait on disk, locks or a slow network. When everything is CPU-bound, they only move the queue into Postgres and add contention and memory.
- **R-0603** `db.pool_stats()` reports wait time, busy, idle and waiting connections, and holds. These are plain atomic counters and always on, because a request waiting for a connection is invisible to Postgres: the server sees fast queries while users see slow responses. If tasks wait while Postgres CPU isn't maxed out, raise `max`. If Postgres CPU is maxed out, a bigger pool won't help, so fix the queries or add cores.

### Performance and memory / `max_connections`

**R-0604** At startup, morm reads `max_connections` and the reserved slots, then computes:

**R-0605**

```
budget = (max_connections - reserved - safety margin) / processes
```

**R-0606** Pools open in one process share the budget, so `db.pool("jobs")` beside the default pool can't double it. If `min` or `max` exceed the budget, including explicit values, morm lowers them and emits a warning, through `tracing` when enabled and stderr otherwise. `morm-admin doctor` runs the same check.

**R-0607**

```
morm: pool max 160 lowered to 92 (max_connections=100, reserved=3, margin=5)
```

**R-0608** The default `max` is deliberately generous. On a larger server, raise `max_connections` in Postgres to match, and `morm-admin size` suggests the value. morm can't see other apps on the same server, which is what the margin is for, and `processes` is set by hand. When several separate processes share a server, Postgres is the real limit: on "too many clients", morm retries with a short backoff before returning the error.

### Performance and memory / Behind a pooler

**R-0624** `morm::Pool` makes a pooler unnecessary for one process, but large fleets and managed Postgres often have PgBouncer in front. `pooler = "transaction"` says so:

- **R-0625** **No startup parameters.** PgBouncer rejects unknown ones by default. Falling back to session `SET` isn't safe here, because in transaction mode that setting stays on a server connection other clients will share. So set `lock_timeout`, `idle_in_transaction_session_timeout`, `client_connection_check_interval` and `search_path` on the role with `ALTER ROLE`. `doctor` checks that they're there.
- **R-0626** **`SET LOCAL` still works,** because it ends with the transaction.
- **R-0627** **Session features refuse.** `db.hold()` returns `PoolerMode`, since a session can't be kept. `tx.advisory_lock` is unaffected, because it ends with the transaction.
- **R-0628** **Connection limits:** `SHOW max_connections` reports Postgres's limit, but the real ones are PgBouncer's `max_client_conn` and pool size, which morm can't see. So the startup adjustment and the `max_connections` check in `size` are skipped, and `max` must be set explicitly: `auto` is refused.
- **R-0629** **Prepared statements** need PgBouncer 1.21+ with `max_prepared_statements` set. On older versions, set `statement_cache = 0` and `shape_cache = 0`, so everything runs through `query_typed`.
- **R-0630** **Migrations must bypass the pooler.** The migrator holds a session advisory lock, so `MORM_MIGRATION_URL` has to point at Postgres directly, and `doctor` warns if it doesn't.

### Performance and memory / Jobs and long-running work

**R-0631** Crons, background jobs and service workers use the pool like everything else. Every `db` call takes a connection and releases it, so a job that sleeps for hours holds nothing, and idle closing and reconnects need no code.

**R-0632**

```rust
loop {
    let batch = next_batch().await;    // holds no connection while waiting
    db.update_q::<Job>().by(F.id, &batch.id).set(F.done, true).run().await?;
}
```

| ID | Case | Use |
|---|---|---|
| R-0633 | Normal request, job or cron | `db` |
| R-0634 | Websocket query | `db`, per message |
| R-0635 | Several statements that must be atomic | `db.transaction(...)`, in small batches. A long transaction blocks vacuum and can hold locks |
| R-0636 | A very long query | `db`. It holds its slot until done, so size `max` for it |
| R-0637 | A heavy job in the same process as the web app | A named pool: `db.pool("jobs")` |
| R-0638 | `LISTEN`, temp tables, advisory locks, `SET` | `db.hold()` |
| R-0639 | Server push from Postgres to websockets | One `db.hold()` listener for the whole app, fanned out to the sockets. Not one per socket, and not needed for ordinary websockets |

### Performance and memory / `db.hold()`

**R-0640**

```rust
let held = db.hold().await?;
held.listen("orders").await?;
// ...
db.release(held).await;      // or held.release().await
```

**R-0641** `db.hold()` keeps one pool connection until released, and no limit or idle rule touches it. `release` takes ownership, so using `held` afterwards is a compile error. Holds count against `max`, and `max_holds` caps them so they can't starve queries.

**R-0642** **Release is guaranteed wherever the process survives,** and the server cleans up where it doesn't:

| ID | How it ends | What releases it |
|---|---|---|
| R-0643 | Normal release | `db.release(held)` |
| R-0644 | Forgot to release, early return, `?` error | Drop |
| R-0645 | Panic, or task cancelled | Drop, with `panic = "unwind"` |
| R-0646 | Process crash, `kill -9`, `panic = "abort"` | The OS closes the socket, and Postgres ends the session and releases its locks |
| R-0647 | Machine or network dies (TCP) | morm sets TCP keepalives, and the server clears the session after about a minute |
| R-0648 | Postgres restarts | Every session ends. The next use of `held` returns `ConnectionLost`, and the slot is freed |

- **R-0649** **Held connections are closed on release, never reused.** They can carry `LISTEN`, temp tables, `SET` values or advisory locks, and reusing one could leak that state into the next query. The slot is free immediately, and the pool opens a clean connection when needed.
- **R-0650** **Reconnecting:** `db.hold().on_connect(...)` re-applies session state after a lost connection. `held.listen()` channels re-subscribe automatically.
- **R-0651** **Forgotten holds:** `#[must_use]` warns if a guard is discarded straight away. Rust can't force an explicit `release`, so Drop is the safety net. `pool_stats()` lists each hold with its age and where it was created, and `hold_warn_after` logs a warning. It only warns, because long-running holds are allowed.
- **R-0652** **Shutdown:** `db.close()` stops new checkouts, then waits up to `shutdown_timeout` for running transactions, `after_commit` handlers and holds to finish, then closes every connection. Apps call it after they stop accepting requests, so a deploy doesn't roll back work or lose events.

**R-0653** **Limit:** one app process has one budget, so several processes or hosts must be sized together against `max_connections`. One multi-threaded Tokio process is the recommended deployment.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0038 | integration | pool growth and idle shrinking |
| T-0039 | integration | `max_connections` adjustment |
| T-0040 | integration | `db.hold()` release on every exit path |
| T-0041 | integration | `ConnectionLost` |
| T-0050 | integration | `max_holds` |
| T-0060 | integration | connections discarded on `25006` |
| T-0073 | integration | `db.close()` finishing in-flight work |
| T-0081 | integration | two pools sharing the connection budget |
| T-0099 | integration | every `pooler = "transaction"` rule and the skipped limit check behind a real PgBouncer |
