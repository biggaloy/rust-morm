# Step 11: `morm::Pool`

- **Phase:** Runtime
- **Depends on:** 10
- **Can run beside:** 04, 05, 06, 07, 08, 09, 13, 28, 29, 30, 31, 32, 33, 34, 35
- **Crates:** `morm`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

The pool, with sizing, budget and failure rules.

## Deliverables

- `morm::Pool` on `deadpool`, dynamic size, idle shrinking, recycling, `recycle_when`, the shared `max_connections` budget through the `morm-core` function, version refusal, failover, `pool_stats()`, an internal query entry point that later steps build on

## End-to-end check

`SELECT 1` through every pool type under load grows and shrinks the pool, a Postgres restart is survived, and a pool over budget is lowered with a warning.

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

- Statement caches (step 14)
- `db.hold()` and pooler rules (step 12)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Runtime guarantees

- **R-0602** `morm` refuses to connect to a server below 15.

### Performance and memory / `morm::Pool`

**R-0623** `morm::Pool` is the generic `deadpool` crate with our own connection manager, not `deadpool-postgres`, whose statement cache has no size limit. `deadpool` provides the tested parts: waiting with timeouts, closing and resizing. morm provides the rest:

- **R-0625** **Health check:** `client.is_closed()` at checkout, with no extra query, so checkout adds no round trip.
- **R-0626** **Errors:** a pool wait timeout becomes a typed `PoolBusy`.
- **R-0627** **Sizing:** warm start, idle shrinking and limit checks, below.

**R-0628** Each query takes a connection, runs, and releases it. Connections are not shared between tasks, so one slow query only holds its own connection, a dropped connection fails only its own request, and timeouts work normally.

**R-0630** Everything goes through `db`, so the pool can be replaced later without changing application code.

### Performance and memory / Dynamic size

| ID | Setting | Default | Meaning |
|---|---|---|---|
| R-0631 | `min` | 2 × cores | Connections opened at startup |
| R-0632 | `max` | 10 × `min` | The ceiling |
| R-0633 | `idle_timeout` | 5m | Idle connections close, down to 1 |
| R-0634 | `last_idle_timeout` | 1h | The last connection closes too, so an idle app can reach 0 |
| R-0635 | `max_queries` | 50000 | Replace a connection after this many queries; 0 disables it |
| R-0636 | `max_lifetime` | off | Replace a connection after this age |

- **R-0637** **`min` is not a floor.** It only sets the warm start. Connections reopen on demand, and the first query after a full idle period pays the connection cost.
- **R-0638** **Limits never interrupt work.** `max_queries` and `max_lifetime` are checked only when a connection returns to the pool.
- **R-0639** **Custom rules:** `Db::connect().recycle_when(|c| c.queries > 50_000 || c.age > hours(6))`.
- **R-0640** **Cores:** Postgres doesn't report its core count over SQL. The default uses `[server] cores`, or the machine running the app when that's unset. If the database is elsewhere, set `[server]`.
- **R-0641** **Failover:** on a pool that isn't `read_only`, a connection that returns `25006` (read-only transaction, after a primary switch) or an admin shutdown error is discarded, not reused. With several hosts and `target_session_attrs = "read-write"`, replacements find the new primary.
- **R-0642** **Queues:** a request that can't get a connection waits up to `wait_timeout`, then returns `PoolBusy`. Waits are never infinite.
- **R-0643** **Separate processes:** a service job, cron or background worker running as its own process must not start a pool with `min` above 1. It should use `Db::connect_as("jobs")`, `MORM_POOL=jobs`, or `MORM_POOL_MIN=1`, because the default of 2 × cores would open connections it doesn't need. The pool still grows on demand.

### Performance and memory / Sizing guidance

- **R-0644** Connections needed ≈ queries per second × seconds each query holds a connection. For example, 2,000 queries/s × 5ms needs about 10.
- **R-0645** Only as many queries run at once as Postgres has free CPU cores. Extra connections help when queries wait on disk, locks or a slow network. When everything is CPU-bound, they only move the queue into Postgres and add contention and memory.
- **R-0646** `db.pool_stats()` reports wait time, busy, idle and waiting connections, and holds. These are plain atomic counters and always on, because a request waiting for a connection is invisible to Postgres: the server sees fast queries while users see slow responses. If tasks wait while Postgres CPU isn't maxed out, raise `max`. If Postgres CPU is maxed out, a bigger pool won't help, so fix the queries or add cores.

### Performance and memory / `max_connections`

**R-0647** At startup, morm reads `max_connections` and the reserved slots, then computes:

**R-0648**

```
budget = (max_connections - reserved - safety margin) / processes
```

**R-0649** Pools open in one process share the budget, so `db.pool("jobs")` beside the default pool can't double it. If `min` or `max` exceed the budget, including explicit values, morm lowers them and emits a warning, through `tracing` when enabled and stderr otherwise.

**R-0651**

```
morm: pool max 160 lowered to 92 (max_connections=100, reserved=3, margin=5)
```

**R-0652** The default `max` is deliberately generous. On a larger server, raise `max_connections` in Postgres to match, and `morm-admin size` suggests the value. morm can't see other apps on the same server, which is what the margin is for, and `processes` is set by hand. When several separate processes share a server, Postgres is the real limit: on "too many clients", morm retries with a short backoff before returning the error.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0043 | integration | pool growth and idle shrinking |
| T-0044 | integration | `max_connections` adjustment |
| T-0065 | integration | connections discarded on `25006` |
| T-0086 | integration | two pools sharing the connection budget |
