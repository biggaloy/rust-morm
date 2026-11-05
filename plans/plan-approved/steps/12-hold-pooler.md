# Step 12: `db.hold()`, jobs and PgBouncer

- **Phase:** Runtime
- **Depends on:** 11
- **Can run beside:** 04, 05, 06, 07, 08, 09, 13, 14, 15, 16, 17, 18, 19, 22, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39
- **Crates:** `morm`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Session connections, and correct behaviour behind a pooler.

## Deliverables

- `db.hold()`, release on every exit path, `on_connect`, `listen`, `max_holds`, the pool side of `db.close()`, `pooler = "transaction"`

## End-to-end check

A held connection is released on every exit path and is never reused, and an app runs correctly behind PgBouncer with `hold` refused.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> ```toml
> [database]
> # Either a URL, or the separate fields below. Separate fields override URL parts.
> url = "${DATABASE_URL}"
> host = "/var/run/postgresql"     # Unix socket directory or hostname
> port = 5432
> name = "app"
> user = "app"                     # no password needed with peer auth on a Unix socket
> password = "${MORM_DB_PASSWORD}" # keep secrets in the environment
> sslmode = "disable"              # disable, prefer, require, verify-ca, verify-full
> target_session_attrs = "any"     # "read-write" finds the primary among several hosts
> connect_timeout = "5s"
> application_name = "auto"        # the binary's name; shows in pg_stat_activity
> search_path = "public"           # pinned, so another role can't shadow objects
> pooler = "none"                  # "transaction" when connecting through PgBouncer in that mode
>
> [database.migration]             # owner role, so the app can run with fewer privileges
> url = "${MORM_MIGRATION_URL}"
>
> [server]                         # optional: the database server, which SQL can't report
> ram = "8G"
> cores = 4
>
> [pool]
> min = "auto"                     # 2 x [server] cores, or of this machine if unset
> max = "auto"                     # 10 x min
> processes = 1                    # app processes sharing this database
> idle_timeout = "5m"              # shrink to 1 after this
> last_idle_timeout = "1h"         # then close the last one
> max_queries = 50000              # 0 = no limit
> max_lifetime = "off"
> hold_warn_after = "1h"
> wait_timeout = "5s"              # longest wait for a connection; can't be off
> max_holds = "auto"               # max / 4, at least 1
> lock_timeout = "5s"              # longest wait for one lock
> statement_timeout = "off"        # longest single statement
> statement_cache = 256            # prepared compile-time statements per connection
> shape_cache = 128                # prepared runtime-built statements per connection
> shutdown_timeout = "30s"         # db.close() waits this long
> plan_cache_mode = "auto"         # or force_custom_plan, force_generic_plan
> transaction_timeout = "30s"      # longest db.transaction
> idle_in_transaction_session_timeout = "1m"
> client_connection_check_interval = "5s"
> tx_warn_after = "1s"
>
> [pools.replica]                  # a read replica
> host = "db-replica.internal"
> read_only = true
>
> [pools.replica.server]           # a pool on a different server describes that server
> ram = "16G"
> cores = 8
>
> [pools.jobs]                     # optional named profile
> user = "app_jobs"                # any [database] field can be set here
> password = "${MORM_JOBS_PASSWORD}"
> min = 1
> max = 4
>
> [outbox]
> batch = 100                      # events claimed at once
> poll_interval = "1s"
> lease = "60s"                    # a crashed worker's events return after this
> max_attempts = 10
> retention = "7d"                 # how long delivered events are kept
>
> [telemetry]                      # used by the tracing and metrics features
> span_sample = 0.01               # share of queries that get a span: 0 = none, 1 = all
> slow_query = "250ms"             # always report slower queries, pool wait included; "off"
> metrics_interval = "10s"         # how often metrics are published
>
> [migrations]
> dir = "morm-migrations"
> min_postgres = 15
> scratch_url = "${MORM_SCRATCH_URL}"
> ```
>
> | `MORM_POOL` | Named pool this process opens, instead of `[pool]` |
>
> **Named pools** are full connection profiles. Any `[database]` field can be set in `[pools.<name>]`, so a job can log in as another role or reach another host. Unset connection fields come from `[database]`, and unset pool settings from `[pool]`, except `min`, which defaults to 1. A separate process can open only its own profile, so job settings live in the same file:
>
> ```rust
> let db = morm::Db::connect().await?;                       // reads morm.toml: [database], [pool]
> let db = morm::Db::connect_as("jobs").await?;              // reads morm.toml: [pools.jobs]
> let db = morm::Db::connect_to(url).min(10).max(90).await?; // or set values in code
> ```
>
> - Suggestions cover `[server]`, `[pool]` `min`, `max` and `processes`, plus `max_connections`, `shared_buffers`, `effective_cache_size`, `work_mem` and `maintenance_work_mem`.
>

## Out of scope

- Waiting for `after_commit` on shutdown (step 23)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Performance and memory / Behind a pooler

**R-0664** `morm::Pool` makes a pooler unnecessary for one process, but large fleets and managed Postgres often have PgBouncer in front. `pooler = "transaction"` says so:

- **R-0665** **No startup parameters.** PgBouncer rejects unknown ones by default. Falling back to session `SET` isn't safe here, because in transaction mode that setting stays on a server connection other clients will share. So set `lock_timeout`, `idle_in_transaction_session_timeout`, `client_connection_check_interval` and `search_path` on the role with `ALTER ROLE`.
- **R-0668** **Session features refuse.** `db.hold()` returns `PoolerMode`, since a session can't be kept.
- **R-0670** **Connection limits:** `SHOW max_connections` reports Postgres's limit, but the real ones are PgBouncer's `max_client_conn` and pool size, which morm can't see. So the startup adjustment are skipped, and `max` must be set explicitly: `auto` is refused.
- **R-0673** **Migrations must bypass the pooler.** The migrator holds a session advisory lock, so `MORM_MIGRATION_URL` has to point at Postgres directly.

### Performance and memory / Jobs and long-running work

**R-0675** Crons, background jobs and service workers use the pool like everything else. Every `db` call takes a connection and releases it, so a job that sleeps for hours holds nothing, and idle closing and reconnects need no code.

**R-0676**

```rust
loop {
    let batch = next_batch().await;    // holds no connection while waiting
    db.update_q::<Job>().by(F.id, &batch.id).set(F.done, true).run().await?;
}
```

| ID | Case | Use |
|---|---|---|
| R-0677 | Normal request, job or cron | `db` |
| R-0678 | Websocket query | `db`, per message |
| R-0679 | Several statements that must be atomic | `db.transaction(...)`, in small batches. A long transaction blocks vacuum and can hold locks |
| R-0680 | A very long query | `db`. It holds its slot until done, so size `max` for it |
| R-0681 | A heavy job in the same process as the web app | A named pool: `db.pool("jobs")` |
| R-0682 | `LISTEN`, temp tables, advisory locks, `SET` | `db.hold()` |
| R-0683 | Server push from Postgres to websockets | One `db.hold()` listener for the whole app, fanned out to the sockets. Not one per socket, and not needed for ordinary websockets |

### Performance and memory / `db.hold()`

**R-0684**

```rust
let held = db.hold().await?;
held.listen("orders").await?;
// ...
db.release(held).await;      // or held.release().await
```

**R-0685** `db.hold()` keeps one pool connection until released, and no limit or idle rule touches it. `release` takes ownership, so using `held` afterwards is a compile error. Holds count against `max`, and `max_holds` caps them so they can't starve queries.

**R-0686** **Release is guaranteed wherever the process survives,** and the server cleans up where it doesn't:

| ID | How it ends | What releases it |
|---|---|---|
| R-0687 | Normal release | `db.release(held)` |
| R-0688 | Forgot to release, early return, `?` error | Drop |
| R-0689 | Panic, or task cancelled | Drop, with `panic = "unwind"` |
| R-0690 | Process crash, `kill -9`, `panic = "abort"` | The OS closes the socket, and Postgres ends the session and releases its locks |
| R-0691 | Machine or network dies (TCP) | morm sets TCP keepalives, and the server clears the session after about a minute |
| R-0692 | Postgres restarts | Every session ends. The next use of `held` returns `ConnectionLost`, and the slot is freed |

- **R-0693** **Held connections are closed on release, never reused.** They can carry `LISTEN`, temp tables, `SET` values or advisory locks, and reusing one could leak that state into the next query. The slot is free immediately, and the pool opens a clean connection when needed.
- **R-0694** **Reconnecting:** `db.hold().on_connect(...)` re-applies session state after a lost connection. `held.listen()` channels re-subscribe automatically.
- **R-0695** **Forgotten holds:** `#[must_use]` warns if a guard is discarded straight away. Rust can't force an explicit `release`, so Drop is the safety net. `pool_stats()` lists each hold with its age and where it was created, and `hold_warn_after` logs a warning. It only warns, because long-running holds are allowed.

**R-0697** **Limit:** one app process has one budget, so several processes or hosts must be sized together against `max_connections`. One multi-threaded Tokio process is the recommended deployment.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0045 | integration | `db.hold()` release on every exit path |
| T-0046 | integration | `ConnectionLost` |
| T-0055 | integration | `max_holds` |
