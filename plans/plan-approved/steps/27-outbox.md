# Step 27: Transactional outbox

- **Phase:** Runtime
- **Depends on:** 25
- **Can run beside:** 08, 23, 26, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 47, 51, 52, 53, 54, 57, 58, 59, 60
- **Crates:** `morm`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Events saved with their data and delivered at least once.

## Deliverables

- `tx.emit`, `tx.emit_keyed`, the worker, leases, retries, cleanup, the `outbox` feature, the table definition

## End-to-end check

Killing the process and worker at every point never loses an event, and repeats carry the same `e.id`.

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
> check_schema = "on"              # "off" starts against a schema that may not match: read the README first
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

## Out of scope

- The managed `pg_outbox!` form and `morm-admin outbox` (step 42); tests create the table directly

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools

- **R-0056** `outbox`: the transactional outbox (section 9), compiled out when off

### Hooks / Outbox: events that must not be lost

**R-0510** `after_commit` can lose an event if the process crashes after commit, because the data is saved and the event only exists in memory. The outbox saves the event in the same transaction as the data, so both exist or neither does.

**R-0511**

```rust
morm::pg_outbox!();                                    // declares the table, tracked by migrations

db.transaction(async |tx| {
    tx.insert(&mut order).await?;
    tx.emit("order_placed", &OrderPlaced { id: order.id }).await?;
    Ok(())
}).await?;

morm::outbox::worker(&db)                              // in the app, or its own process
    .on("order_placed", async |e: Event<OrderPlaced>| send_email(e.id, e.payload).await)
    .run().await?;
```

- **R-0512** **`emit` exists only on `tx`,** so an event can't be saved apart from its data. Hooks can reach it through `db.tx()` when they run in a transaction.
- **R-0513** **Delivery is at least once.** A worker can crash after sending and before recording it, so an event can arrive twice. `e.id` is stable across attempts, so handlers and receivers use it to ignore repeats.
- **R-0514** **Claims use a lease, not an open transaction.** A worker marks a batch as claimed with `FOR UPDATE SKIP LOCKED`, commits, then sends. Holding a transaction during network calls would hold locks and hit `transaction_timeout`. If a worker dies, its lease expires and the events are claimed again.
- **R-0515** **Failures retry with backoff.** After `max_attempts`, an event is marked `failed` and kept, never dropped. `pool_stats()` reports pending, failed and oldest-pending age from the worker's last poll.
- **R-0517** **Workers claim only event names they handle,** so an event with no handler waits during a deploy instead of burning its attempts. Its age shows in the oldest-pending figure. A payload that won't decode counts as a failed attempt.
- **R-0518** **Order isn't guaranteed by default,** because workers run in parallel. `tx.emit_keyed("order_placed", &order.id, &event)` delivers events with the same key in order, one at a time, which costs some throughput.
- **R-0519** **Workers poll,** using a partial index on pending events, so an idle check is one small query. `NOTIFY` isn't used, because it takes a global lock at commit, which limits write throughput.
- **R-0520** **Cleanup:** delivered events are deleted in batches after `retention`, so the table stays small.
- **R-0521** **Separate worker process:** use `Db::connect_as("jobs")`, as for other jobs.
- **R-0522** The app role needs read and write access on the table, so it's a normal managed table in the app's schema, not in `_morm`.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0129 | suite | **Fault injection:** kill the process and the worker at every point: before commit (no event), after commit (delivered), and after send before recording (delivered twice, same `e.id`). |
| T-0130 | suite | **Concurrency:** rival workers never hold one event at once while a lease is valid, expired leases return, and keyed events stay in order. |
| T-0131 | suite | **Failures:** backoff, `max_attempts`, `failed` events kept, and cleanup after `retention` |
| T-0134 | suite | **Fault injection runs in CI:** `ci.yml`'s `integration` job runs the T-0129 tests, and the PR links that job's log showing them pass. A run of zero tests doesn't count. These tests need Postgres, so they live in `tests/` (package `morm-tests`). The job runs that package with `cargo test --locked -p morm-tests`, which doesn't turn on `morm`'s `outbox` feature, so turn it on for that package. |
