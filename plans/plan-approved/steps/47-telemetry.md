# Step 47: Telemetry

- **Phase:** Tools
- **Depends on:** 14, 25
- **Can run beside:** 08, 23, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 46, 51, 52, 53, 54, 57, 58, 59
- **Crates:** `morm`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

OpenTelemetry-shaped spans and metrics, cheap enough to leave on.

## Deliverables

- The warning sink moved onto `tracing`
- `tracing` and `metrics` features, `span_sample`, `sample_scope`, `slow_query`, per-connection counters and histograms, the publishing task

## End-to-end check

Spans reach an OpenTelemetry Collector with the documented fields, metrics are scraped, and a build with both features off contains no telemetry code.

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

## Out of scope

- Exporter setup, which belongs to apps

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools

- **R-0053** `tracing`: sampled query spans with SQL text, duration, row count and pool wait time, compiled out when off (section 11). Never parameter values, because they hold passwords, tokens and personal data. Slow-query logging is left to Postgres, which also gives plans and per-query statistics; morm covers only what the server can't see, which is pool waits, network and decode time, and which code ran the query
- **R-0054** `metrics`: publishes pool and query metrics through the `metrics` crate on a timer, compiled out when off (section 11)

### Reading and writing / Deadlocks

- **R-0475** Postgres's deadlock detail is logged through `tracing`, with no parameter values.

### Performance and memory / Statements

- **R-0710** The `tracing` feature logs heavy churn.

### Performance and memory / Telemetry

**R-0728** morm has no vendor code. It emits OpenTelemetry-shaped data through `tracing` and `metrics`, so Datadog, Grafana, Honeycomb and in-house stacks work through exporters the app chooses, and morm depends on no exporter or vendor. Spans reach an agent through `tracing-opentelemetry` and OTLP. Metrics usually go through `metrics-exporter-prometheus`, scraped by the agent, because the `metrics` crate has no official OTLP exporter. Logs and database monitoring come from Postgres directly, as `app_suggestions.md` describes.

**R-0729** **Cheap by construction:**

| ID | Situation | Cost per query |
|---|---|---|
| R-0730 | Features compiled out | None |
| R-0731 | `tracing` compiled in, no subscriber | One relaxed atomic load |
| R-0732 | `tracing` on, query not sampled | That load, a thread-local counter, and two clock reads for `slow_query` |
| R-0733 | `tracing` on, query sampled | One span, with fields recorded only then |
| R-0734 | `metrics` on | Two clock reads and one uncontended bucket increment |

**R-0735** **Spans.** `span_sample` picks every Nth query with a thread-local counter, so sampling takes no lock and no shared atomic. The decision comes before any span exists, which is why this is cheaper than sampling only in the exporter.

- **R-0736** **Whole requests:** per-query sampling leaves gaps in a trace. `morm::sample_scope(keep, fut)` overrides `span_sample` for everything inside `fut` through a task-local, so middleware can pass the trace's own sampling decision and keep a request's queries together.
- **R-0737** **`slow_query`:** a query that wasn't sampled but ran over the limit is reported as a `tracing` event with its durations, attached to the current span. A span can't be started after the fact. The limit includes pool wait, which Postgres can't see, so this doesn't repeat the server's slow log.
- **R-0738** **Field names** follow the OpenTelemetry database conventions: `db.system.name`, `db.query.text`, `db.operation.name`, `db.collection.name`, `db.namespace`, plus `db.morm.tag` and `db.morm.pool_wait_us`. Query text holds only `$1` placeholders. `q_unchecked` and `sql_unchecked` text can contain values, so spans leave it out.

**R-0739** **Metrics.** Nothing is published from the query path. Each connection keeps plain counters and fixed-bucket duration histograms. A connection has one user at a time, so these are uncontended. One background task per pool adds them up every `metrics_interval` and publishes them, and a closing connection adds its totals to the pool first.

- **R-0740** Names follow the same conventions: `db.client.connection.count`, `db.client.connection.max`, `db.client.connection.pending_requests`, `db.client.connection.wait_time`, `db.client.operation.duration`.
- **R-0741** morm adds `db.morm.*` metrics for deadlocks, retries, lock timeouts, `shape_cache` churn and connections replaced for a stale generation.
- **R-0742** A shorter interval gives finer graphs but more data points, and the platform's bill rises with it. It adds no cost per query.

**R-0743** **No per-request SQL comments.** Trace-linking modes such as Datadog's `full` propagation add a `traceparent` comment to every statement. That makes every query text unique, so no statement could be reused. morm won't do it. `application_name` and `.tag()` give the fixed, service-level equivalent. A query still appears inside its request trace through the app-side span, and only the jump from a database sample to one exact trace is lost.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0106 | integration | `span_sample` at 0, 1 and between |
| T-0107 | integration | `sample_scope` |
| T-0108 | integration | `slow_query` events including pool wait |
| T-0109 | integration | metric totals surviving a closing connection |
