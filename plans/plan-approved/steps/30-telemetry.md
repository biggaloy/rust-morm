# Step 30: Telemetry

- **Phase:** Tools
- **Depends on:** 11, 18
- **Can run beside:** 28, 29
- **Crates:** `morm`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

OpenTelemetry-shaped spans and metrics, cheap enough to leave on.

## Deliverables

- `tracing` and `metrics` features, `[telemetry]`, `span_sample`, `sample_scope`, `slow_query`, per-connection counters and histograms, the publishing task

## End-to-end check

Spans reach an OpenTelemetry Collector with the documented fields, metrics are scraped, and a build with both features off contains no telemetry code.

## Out of scope

- Exporter setup, which belongs to apps

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools

- **R-0052** `tracing`: sampled query spans with SQL text, duration, row count and pool wait time, compiled out when off (section 11). Never parameter values, because they hold passwords, tokens and personal data. Slow-query logging is left to Postgres, which also gives plans and per-query statistics; morm covers only what the server can't see, which is pool waits, network and decode time, and which code ran the query
- **R-0053** `metrics`: publishes pool and query metrics through the `metrics` crate on a timer, compiled out when off (section 11)

### Performance and memory / Telemetry

**R-0677** morm has no vendor code. It emits OpenTelemetry-shaped data through `tracing` and `metrics`, so Datadog, Grafana, Honeycomb and in-house stacks work through exporters the app chooses, and morm depends on no exporter or vendor. Spans reach an agent through `tracing-opentelemetry` and OTLP. Metrics usually go through `metrics-exporter-prometheus`, scraped by the agent, because the `metrics` crate has no official OTLP exporter. Logs and database monitoring come from Postgres directly, as `app_suggestions.md` describes.

**R-0678** **Cheap by construction:**

| ID | Situation | Cost per query |
|---|---|---|
| R-0679 | Features compiled out | None |
| R-0680 | `tracing` compiled in, no subscriber | One relaxed atomic load |
| R-0681 | `tracing` on, query not sampled | That load, a thread-local counter, and two clock reads for `slow_query` |
| R-0682 | `tracing` on, query sampled | One span, with fields recorded only then |
| R-0683 | `metrics` on | Two clock reads and one uncontended bucket increment |

**R-0684** **Spans.** `span_sample` picks every Nth query with a thread-local counter, so sampling takes no lock and no shared atomic. The decision comes before any span exists, which is why this is cheaper than sampling only in the exporter.

- **R-0685** **Whole requests:** per-query sampling leaves gaps in a trace. `morm::sample_scope(keep, fut)` overrides `span_sample` for everything inside `fut` through a task-local, so middleware can pass the trace's own sampling decision and keep a request's queries together.
- **R-0686** **`slow_query`:** a query that wasn't sampled but ran over the limit is reported as a `tracing` event with its durations, attached to the current span. A span can't be started after the fact. The limit includes pool wait, which Postgres can't see, so this doesn't repeat the server's slow log.
- **R-0687** **Field names** follow the OpenTelemetry database conventions: `db.system.name`, `db.query.text`, `db.operation.name`, `db.collection.name`, `db.namespace`, plus `db.morm.tag` and `db.morm.pool_wait_us`. Query text holds only `$1` placeholders. `q_unchecked` and `sql_unchecked` text can contain values, so spans leave it out.

**R-0688** **Metrics.** Nothing is published from the query path. Each connection keeps plain counters and fixed-bucket duration histograms. A connection has one user at a time, so these are uncontended. One background task per pool adds them up every `metrics_interval` and publishes them, and a closing connection adds its totals to the pool first.

- **R-0689** Names follow the same conventions: `db.client.connection.count`, `db.client.connection.max`, `db.client.connection.pending_requests`, `db.client.connection.wait_time`, `db.client.operation.duration`.
- **R-0690** morm adds `db.morm.*` metrics for deadlocks, retries, lock timeouts and `shape_cache` churn.
- **R-0691** A shorter interval gives finer graphs but more data points, and the platform's bill rises with it. It adds no cost per query.

**R-0692** **No per-request SQL comments.** Trace-linking modes such as Datadog's `full` propagation add a `traceparent` comment to every statement. That makes every query text unique, so no statement could be reused. morm won't do it. `application_name` and `.tag()` give the fixed, service-level equivalent. A query still appears inside its request trace through the app-side span, and only the jump from a database sample to one exact trace is lost.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0101 | integration | `span_sample` at 0, 1 and between |
| T-0102 | integration | `sample_scope` |
| T-0103 | integration | `slow_query` events including pool wait |
| T-0104 | integration | metric totals surviving a closing connection |
