# App and operations suggestions

These belong to the app or to operations, not to rust-morm. Anything morm handles itself is in `rust-morm-plan.md`.

## Database setup

| Area | What to do |
|---|---|
| **Roles** | Use a migration role that can change the schema, and an app role that can only read and write data. A bug or injection in the app then can't drop tables. morm's `MORM_MIGRATION_URL` supports this |
| **Timeouts** | Set `statement_timeout` for the app role, for example 30s. morm also has `statement_timeout` and `idle_in_transaction_session_timeout` settings, and a role-level value covers clients that aren't morm |
| **Collations** | A glibc upgrade can change how text sorts, which silently corrupts text indexes until they're rebuilt with `REINDEX`. ICU or `C` collations avoid this. After an OS upgrade, check collation version warnings |

## Memory

Each Postgres connection can use `work_mem` per sort or hash operation, so pool size × `work_mem` belongs in the server's memory budget. Raise it for the session of a heavy query, not globally.

## Deploys

- Run `morm-admin mgr migrate` as its own deploy step, before new code goes live, so a failed migration stops the deploy.
- Mark every local and staging database with `morm-admin mgr mode development`. Leave production unmarked, so it stays forward only.
- To undo a migration in production, ship `morm-admin mgr revert ID`, or a forward fix.
- On stop: stop accepting requests, then call `db.close()`, so running work finishes.

## Patterns for writes that must not be lost

- **Outbox handlers:** morm's outbox delivers at least once, so make every handler safe to run twice, using `e.id` to ignore repeats. Alert on failed events and on the age of the oldest pending one.
- **Idempotency keys:** when a connection drops during `COMMIT`, morm returns `CommitUnknown`, and you can't tell if the write happened. Have clients send a key, store it in a unique column in the same transaction, and return the stored result on a repeat.
- **Job queues:** a table with `.skip_locked()` lets many workers share jobs, so you may not need Redis or a separate queue.
- **Tenant isolation:** use row-level security with `pg_policy!`, and set the tenant per transaction with `tx.set_local`.

## Keeping tables healthy

| Area | What to do |
|---|---|
| **Vacuum and bloat** | Frequently updated tables collect dead rows, so tune autovacuum for hot tables. Long transactions block vacuum across the database. Monitor transaction ID wraparound |
| **Fast updates (HOT)** | Updates are much cheaper when no indexed column changes. Don't index columns that change constantly, such as counters or `last_seen_at`. Consider `fillfactor` 80–90 on update-heavy tables, with morm's `storage` option |
| **Large tables** | Partition time-series data, for example by month. Dropping an old partition is instant, while deleting the same rows takes hours |
| **Sequences** | Identity and `bigserial` values have gaps, because rolled-back inserts still use numbers. Never assume ids are contiguous |

## Availability

| Area | What to do |
|---|---|
| **Backups** | Use point-in-time recovery, with pgBackRest or WAL-G, and regularly test restores. A backup that has never been restored isn't proven |
| **Replicas** | Replication lag means users might not see their own writes when the next read goes to a replica. Send reads that follow a write to the primary |
| **Failover** | Run it through a tested tool, and rehearse it. morm discards connections to the old primary, and `target_session_attrs = "read-write"` finds the new one |
| **Major upgrades** | Plan for `pg_upgrade`, or logical replication for near-zero downtime |

## Monitoring

Use Postgres for slow queries, not the app. It gives you plans, per-query statistics and lock reports with nothing added to your binary:

```conf
shared_preload_libraries = 'pg_stat_statements,auto_explain'
log_min_duration_statement = 500ms
auto_explain.log_min_duration = 1s
auto_explain.log_analyze = off                  # on adds timing overhead to every query
log_lock_waits = on
log_temp_files = 0
log_parameter_max_length = 0                    # required, see below
auto_explain.log_parameter_max_length = 0       # required, see below
```

Changing `shared_preload_libraries` needs a restart. Then run `CREATE EXTENSION pg_stat_statements;` in each database. `auto_explain` needs no `CREATE EXTENSION`.

**Keep query values out of the logs.** Both parameter settings default to `-1`, full values. Any statement that gets logged, including every slow query, then writes its parameters to the log, so a slow `UPDATE` setting a password hash puts that hash in the file. Setting both to `0` keeps the SQL and drops the values. `morm-admin doctor` warns about this.

| Tool | Tells you |
|---|---|
| `log_min_duration_statement` | Each slow query |
| `auto_explain` | Why it was slow, through its plan |
| `pg_stat_statements` | What to optimize: calls, total and average time per query shape |
| `log_lock_waits` | Queries that waited longer than `deadlock_timeout` for a lock |
| `log_temp_files` | Queries that spilled to disk because `work_mem` was too small |

Watch `pg_stat_activity` for long transactions and lock waits, and track bloat and disk use.

Postgres can't see time spent outside it. morm's `pool_stats()` adds pool waits, deadlocks, retries and lock timeouts from the app's side, so watch pool wait time first when users report slowness and the server looks fine. `application_name` and `.tag()` show which app and which code ran a query.

## Observability platforms

Datadog, Grafana, Honeycomb and similar tools all fit the same way, and morm needs no vendor code:

```
morm spans    -> tracing -> tracing-opentelemetry -> OTLP exporter -> Datadog Agent or OTel Collector
morm metrics  -> metrics crate -> /metrics endpoint -> scraped by the agent
Postgres logs -> Agent tails the log file or journal
pg_stat_statements, pg_stat_activity -> Agent's database monitoring role
```

- The exporter and its setup live in your app, not in morm, so you can update those crates on your own schedule. For traces, use `tracing-opentelemetry` and `opentelemetry-otlp`.
- For metrics, use `metrics-exporter-prometheus` and let the agent scrape it. The `metrics` crate has no official OTLP exporter. `metrics-exporter-opentelemetry` is a third-party bridge if you need to push.
- Give the Agent its own role with `pg_monitor`, and keep `log_parameter_max_length = 0`, because shipped logs leave the server for a third party.
- Use `service` or fixed-comment trace modes only. A per-request `traceparent` comment, such as Datadog's `full` propagation mode, makes every query unique and defeats statement caching. morm doesn't add one.
- Keep `span_sample` low on busy services, and wrap requests in `morm::sample_scope` with the trace's own decision so a kept trace has all its queries.
- Raise `metrics_interval` if the bill matters more than fine graphs. It changes the data volume, not query speed.

## In-house observability

All of this is optional, and morm works the same without it. Run the OpenTelemetry Collector as the agent instead of writing your own, because it already handles batching, retries and buffering.

```
App: morm spans over OTLP, metrics scraped from /metrics
  |  localhost
  v
Agent on the same host (OTel Collector)
  |  also tails the Postgres log and reads pg_stat_statements and pg_stat_activity
  |  batches, samples, buffers, retries
  v
Separate telemetry storage
  v
Grafana
```

| Storage | Holds | Use when |
|---|---|---|
| ClickHouse | Logs, traces and metrics in one place | The default choice: one system to run, and it compresses this data well |
| Prometheus, Tempo and Loki | One store per data type | You already run Grafana's stack |
| A separate Postgres | Whatever you design tables for | Volumes are small; telemetry is a constant heavy write load |

- **Never store telemetry in the app's database,** or on its server. Every query would create a write, and an overloaded Postgres would lose its visibility just when you need it.
- **Telemetry must never block a query.** Keep the exporter's queue bounded and let it drop data when full.
- **Keep the agent outside the app, one per host.** If storage is down, the agent buffers and retries, and the app doesn't notice.
- **Set retention from day one,** for example 7 days of traces, 30 days of logs and a year of metrics. Otherwise this becomes your biggest disk problem.
- **Cost:** these tools are free to self-host. You pay for the server, its disk and the time to maintain it. Check each project's license before relying on it; Grafana, Tempo and Loki use AGPL v3.

Moving between this setup and a platform like Datadog is a change of exporter target and agent config, with no change to morm.
