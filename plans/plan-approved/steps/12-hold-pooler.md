# Step 12: `db.hold()`, jobs and PgBouncer

- **Phase:** Runtime
- **Depends on:** 11
- **Can run beside:** 04, 05, 06, 07, 08, 09, 13, 28, 29, 30, 31, 32, 33, 34, 35, 56
- **Crates:** `morm`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Session connections, and correct behaviour behind a pooler.

## Deliverables

- `db.hold()`, release on every exit path, `on_connect`, `listen`, `max_holds`, the pool side of `db.close()`, `pooler = "transaction"`, and CI's `pooler` job over PgBouncer 1.19.1, 1.20.1, 1.22 and 1.26.1 and Postgres 15 to 18, with its tests in `tests/tests/repo_policy.rs` and the README's "Contributing" section, and CI's `standby` job with a streaming hot standby of each of Postgres 15 to 18

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
> - **Shutdown:** `db.close()` stops new checkouts, then waits up to `shutdown_timeout` for running transactions, `after_commit` handlers and holds to finish, then closes every connection. Apps call it after they stop accepting requests, so a deploy doesn't roll back work or lose events.
>

## Out of scope

- Waiting for `after_commit` on shutdown (step 25)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Reading and writing / Deadlocks

- **R-0450** **Holds can't use up the pool.** `max_holds` defaults to a quarter of `max`, at least 1. `db.hold()` past it returns `PoolBusy`.

### Performance and memory / Behind a pooler

**R-0669** `morm::Pool` makes a pooler unnecessary for one process, but large fleets and managed Postgres often have PgBouncer in front. `pooler = "transaction"` says so:

- **R-0670** **Only tracked startup parameters, on PgBouncer 1.20 or later.** PgBouncer refuses a startup parameter it doesn't track, by default, so of morm's settings only two go at startup: `application_name`, as its own startup parameter, and `TimeZone = 'UTC'`, the one setting sent in `options`, as `-c TimeZone=UTC`. PgBouncer tracks both per client and sets them on every server connection the client gets, in transaction mode too, even after another client's `SET` on that connection, so the binary's name shows in `pg_stat_activity` and the role's `timezone` can stay as it is. PgBouncer accepts `options` from 1.20, so morm needs PgBouncer 1.20 or later. An older one refuses every connection with `unsupported startup parameter: options`, and morm turns that into `Error::Config`, from `Db::connect`, `connect_as`, `connect_to` and a pool `db.pool(name)` adds, quoting PgBouncer's message and naming the two fixes: upgrade PgBouncer to 1.20 or later, or add `options` to its `ignore_startup_parameters` and set the role's `timezone` to `UTC`, which the startup check then checks. It does so whatever `pooler` says, and with `pooler = "none"` it first says to set `pooler = "transaction"`. Falling back to session `SET` for the rest isn't safe here, because in transaction mode that setting stays on a server connection other clients will share. So set `lock_timeout`, `idle_in_transaction_session_timeout`, `client_connection_check_interval` and `search_path` on the role with `ALTER ROLE`. A pooler that drops `TimeZone`, such as PgBouncer 1.26.1 or later with `TimeZone` in `ignore_startup_parameters`, leaves sessions in the role's zone, and then a pool whose sessions report a zone other than UTC refuses to start, with an error naming the zone and saying to let the pooler pass `TimeZone` or to set the role's `timezone` to `UTC`. Owner decisions DJ2, DK1 and DK5.
- **R-0673** **Session features refuse.** `db.hold()` returns `PoolerMode`, since a session can't be kept.
- **R-0675** **Connection limits:** `SHOW max_connections` reports Postgres's limit, but the real ones are PgBouncer's `max_client_conn` and pool size, which morm can't see. So the startup adjustment are skipped, and `max` must be set explicitly: `auto` is refused.
- **R-0678** **Migrations must bypass the pooler.** The migrator holds a session advisory lock, so `MORM_MIGRATION_URL` has to point at Postgres directly.

**R-1238** **CI's `pooler` job runs every supported PgBouncer against every supported Postgres.** This step changes step 01's `ci.yml`, `start_pgbouncer.sh`, `pgbouncer.ini` and `tests/tests/repo_policy.rs`, so the pooler tests of this step and of later steps run on each pair:
- On every push and pull request, the job runs PgBouncer 1.20.1, 1.22 and 1.26.1, each in transaction mode, in front of each of Postgres 15, 16, 17 and 18. Each version comes from its own pinned source: 1.22 from the runner's apt sources, 1.26.1 from the image `start_pgbouncer.sh` already pins by digest, and 1.20.1 built from pgbouncer.org's `pgbouncer-1.20.1.tar.gz`, whose SHA-256 must be `24992cf557d73426d7048698dffc7b019e6364d4d8757ae2cf5e2471286a2088`. 1.20.1, not 1.20.0, which has a regression with `options` in `ignore_startup_parameters`.
- For the refusal cases, a second 1.26.1 instance runs with `ignore_startup_parameters = TimeZone`, in front of each of Postgres 15 to 18, so it drops the parameter; 1.26.1 is the first version in which that list wins over a tracked parameter, so 1.20.1 and 1.22 can't drop it. A PgBouncer 1.19.1, built the same way from `pgbouncer-1.19.1.tar.gz`, whose SHA-256 must be `58c3eff9bb72c18133b28e1f034fd59356ea76281c65e127432ca101c208a394`, runs two instances: one with `ignore_startup_parameters` empty, for the refusal of `options`, and one with `ignore_startup_parameters = options`, for the workaround that leaves the zone to the role. Each instance keeps one fixed config, so no test changes one at run time.
- A downloaded tarball whose SHA-256 isn't the one given is treated like a failed install: that version's cases are skipped, with a `::warning::` naming the PgBouncer version, the file, the expected hash and the one found.
- 1.19.1 and 1.20.1 are built and run inside a container from a Debian image pinned by digest, their build inputs (`libevent`, OpenSSL's headers, `pkg-config`, a C compiler and `make`, as in `build-essential`) installed there from Debian's archive, not the runner's Ubuntu mirror. PgBouncer refuses to run as root, so the container runs it as a non-root user, through `docker run --user`. So an Ubuntu mirror outage skips only 1.22, and a Debian archive outage skips only 1.19.1 and 1.20.1.
- A third 1.26.1 instance runs with `max_prepared_statements = 0`, in front of each of Postgres 15 to 18, the configuration DL3's caches-off mode supports on 1.21 and later, so a test can reach the failure EA guards against, the driver's own named statements reaching different server connections; morm's pools on it set `statement_cache = 0` and `shape_cache = 0`. Owner decision on EF4.
- Each instance's config sets `default_pool_size = 3` or more, `min_pool_size` the same and `server_round_robin = 1`, so a test's clients reach several server sessions, as production does. A pool of one hid a failure that only shows when a client's round trips reach different server connections (step 55, proof 16). Owner decision EA.
- Each instance has its own config. The 1.22 and 1.26.1 instances, apart from the third 1.26.1 instance, set `max_prepared_statements = 200` explicitly, because 1.24.0 changed its default from 0 to 200, so both versions run morm's statement caches the same way. The 1.19.1 and 1.20.1 configs leave the key out, and morm's pools on them set `statement_cache = 0` and `shape_cache = 0`.
- Each instance's startup runs `SHOW VERSION` on its admin console and fails the job unless it reports the version it stands for, as `SHOW VERSION` prints it: `PgBouncer 1.20.1`, `PgBouncer 1.19.1` and `PgBouncer 1.26.1` exactly, and a value starting `PgBouncer 1.22.` for apt's.
- The apt-to-image fallback ends, replacing step 01's decision of apt, one retry, then the pinned image. The 1.26.1 image is its own version now, so a 1.22 that can't be installed is skipped, never run on another version. If a version can't be installed in a run, after one retry, or its tarball's SHA-256 doesn't match, its cases are skipped, not failed. A skip you can see: `start_pgbouncer.sh` writes each version's result, `ran` or `skipped`, to `$GITHUB_OUTPUT`, writes the `::warning::` annotation naming the skipped PgBouncer version and the Postgres version, and ends with status 0 by finishing normally, with no `exit 0` line. Each version's test step carries one `if:` on that output, comparing with `!= 'skipped'`, as in `if: steps.pgbouncer.outputs.pgbouncer_1_22 != 'skipped'`, so GitHub shows the step as skipped when the script skipped that version, and a missing or misspelled output, which reads as an empty string, runs the step, whose tests then fail loudly with no PgBouncer to reach. A failed start step still stops the job, through the `success()` an `if:` implies. A PgBouncer that installs but doesn't answer, or reports another version, fails the job. `start_pgbouncer.sh`'s header says so, in place of "the pooler tests never run without PgBouncer".
- `repo_policy.rs`'s tests of this job, tagged with step 01's CI requirement, `integration_targets_each_version_and_transport`, `pooler_starts_pgbouncer_from_another_source_when_apt_fails` and `ci_runs_every_check_the_plan_names`, are rewritten for this requirement; the rewritten workflow test, tagged with this requirement, asserts that each per-version `if:` names the start step's `id` and reads an output name that the stubbed test sees the script write. Of its guard tests, `nothing_switches_a_job_or_step_off`'s `ALLOWED` list gains exactly those per-version `if:` lines, and nothing else; `no_file_under_github_hides_a_failure` doesn't change, and the script and workflow must pass it as it is. Its stubbed script test, with stubs among them for `sudo`, `apt-get`, `systemctl`, `pgbouncer`, `curl`, `docker`, `pg_isready`, `psql` (which runs `SHOW VERSION`) and the build, drives each version's paths, tagged with this requirement, and asserts what each writes to `$GITHUB_OUTPUT`: installed on the second try, it runs and writes `ran`; failing twice, it's skipped, writing `skipped` and a `::warning::` naming the PgBouncer and Postgres versions; a tarball with the wrong SHA-256 is skipped the same way, its `::warning::` also naming the file, the expected hash and the one found; in both, the script finishes with status 0; installed but not answering, or reporting another version, the job fails.
- This step writes a "Contributing" section at the end of `README.md`, holding this notice, which step 50 keeps:

  > **A green `pooler` job can have skipped a PgBouncer version.** When a version can't be installed in a run, after one retry, or its download doesn't match its pinned SHA-256, its cases are skipped, not failed, and the job writes a `::warning::` naming the PgBouncer and Postgres versions, such as `PgBouncer 1.22 skipped on Postgres 17: install failed`. Open the run's summary and read its annotations before trusting a green run. If a version was skipped, rerun the job; if it's skipped again, say so in the PR, and don't merge a change to pooler behaviour until a run has covered every version.

The workflow and the stubbed script test carry the tag. Owner decisions DK2, DL1 to DL4, DM1 and DM4; DN1 to DN3.

**R-1244** **CI's `standby` job.** This step adds a job to `ci.yml` that runs on every push and pull request, with a primary and a streaming hot standby of each of Postgres 15, 16, 17 and 18, the primary as the job's `postgres` service, so `job.services.postgres.id` names its container as in every other job, and the standby as a container from the same image, the standby made with `pg_basebackup -R`, with `hot_standby = on`, `hot_standby_feedback = off` and `max_standby_streaming_delay = 3s`, so a recovery conflict ends a session in seconds. It runs the tests behind a Cargo feature of their own, `standby`, with `required-features`, as `pooler` does, and later steps' standby tests run in it. It follows every rule `tests/tests/repo_policy.rs` checks, and the workflow carries the tag. Owner decision SG.

### Performance and memory / Jobs and long-running work

**R-0680** Crons, background jobs and service workers use the pool like everything else. Every `db` call takes a connection and releases it, so a job that sleeps for hours holds nothing, and idle closing and reconnects need no code.

**R-0681**

```rust
loop {
    let batch = next_batch().await;    // holds no connection while waiting
    db.update_q::<Job>().by(F.id, &batch.id).set(F.done, true).run().await?;
}
```

| ID | Case | Use |
|---|---|---|
| R-0682 | Normal request, job or cron | `db` |
| R-0683 | Websocket query | `db`, per message |
| R-0684 | Several statements that must be atomic | `db.transaction(...)`, in small batches. A long transaction blocks vacuum and can hold locks |
| R-0685 | A very long query | `db`. It holds its slot until done, so size `max` for it |
| R-0686 | A heavy job in the same process as the web app | A named pool: `db.pool("jobs")` |
| R-0687 | `LISTEN`, temp tables, advisory locks, `SET` | `db.hold()` |
| R-0688 | Server push from Postgres to websockets | One `db.hold()` listener for the whole app, fanned out to the sockets. Not one per socket, and not needed for ordinary websockets |

### Performance and memory / `db.hold()`

**R-0689**

```rust
let held = db.hold().await?;
held.listen("orders").await?;
// ...
db.release(held).await;      // or held.release().await
```

**R-0690** `db.hold()` keeps one pool connection until released, and no limit or idle rule touches it. `release` takes ownership, so using `held` afterwards is a compile error. Holds count against `max`, and `max_holds` caps them so they can't starve queries.

**R-0691** **Release is guaranteed wherever the process survives,** and the server cleans up where it doesn't:

| ID | How it ends | What releases it |
|---|---|---|
| R-0692 | Normal release | `db.release(held)` |
| R-0693 | Forgot to release, early return, `?` error | Drop |
| R-0694 | Panic, or task cancelled | Drop, with `panic = "unwind"` |
| R-0695 | Process crash, `kill -9`, `panic = "abort"` | The OS closes the socket, and Postgres ends the session and releases its locks |
| R-0696 | Machine or network dies (TCP) | morm sets TCP keepalives, and the server clears the session after about a minute |
| R-0697 | Postgres restarts, or a hot standby's replay ends the session over a recovery conflict | Every session ends, or that one does. The next use of `held` returns `ConnectionLost`, and the slot is freed. morm's connection task keeps the error that closed the connection, and when it was a `FATAL` with SQLSTATE `40001`, as replay sends a standby session that sits in a transaction holding a table it must change, `ConnectionLost` names the recovery conflict, because the session's state is gone. Owner decision SG |

- **R-0698** **Held connections are closed on release, never reused.** They can carry `LISTEN`, temp tables, `SET` values or advisory locks, and reusing one could leak that state into the next query. The slot is free immediately, and the pool opens a clean connection when needed.
- **R-0699** **Reconnecting:** `db.hold().on_connect(...)` re-applies session state after a lost connection. `held.listen()` channels re-subscribe automatically.
- **R-0700** **Forgotten holds:** `#[must_use]` warns if a guard is discarded straight away. Rust can't force an explicit `release`, so Drop is the safety net. `pool_stats()` lists each hold with its age and where it was created, and `hold_warn_after` logs a warning. It only warns, because long-running holds are allowed.

**R-0702** **Limit:** one app process has one budget, so several processes or hosts must be sized together against `max_connections`. One multi-threaded Tokio process is the recommended deployment.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0045 | integration | `db.hold()` release on every exit path |
| T-0046 | integration | `ConnectionLost` |
| T-0055 | integration | `max_holds` |
| T-0300 | integration | **Old PgBouncers and the two startup parameters, in the `pooler` job:** through PgBouncer 1.19.1's instance with `ignore_startup_parameters` empty, `Db::connect`, `connect_as`, `connect_to` and a pool added by `db.pool(name)` each return `Error::Config` quoting `unsupported startup parameter: options` and naming both fixes, before any of the app's statements, and with `pooler = "none"` saying first to set `pooler = "transaction"`; through its second instance, the one with `ignore_startup_parameters = options`, and the role's `timezone` at `UTC`, the pool starts and its sessions report `UTC`; through 1.20.1, 1.22 and 1.26.1 in front of each of Postgres 15 to 18, with the role at `Europe/Berlin`, every session shows the binary's `application_name` in `pg_stat_activity` and reports `TimeZone` as `UTC`. What happens when a version can't be installed, has a tarball with the wrong SHA-256, doesn't answer or reports another version is proven by the stubbed script test this step's CI requirement names, not here |
| T-0305 | integration | **A held connection ended by a recovery conflict, in the `standby` job, on Postgres 15–18:** a `db.hold()` connection on the standby, idle in a `REPEATABLE READ` transaction that read a table, is ended by replay of an `ALTER TABLE` on that table run on the primary, after `max_standby_streaming_delay`; its next use returns `ConnectionLost` naming the recovery conflict, and the slot is freed. A held connection idle outside a transaction, holding a prepared statement on the table, isn't ended, and its next run works |
