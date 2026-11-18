# Step 10: Config, TLS and connecting

- **Phase:** Runtime
- **Depends on:** 03
- **Can run beside:** 04, 05, 06, 07, 08, 09, 13, 28, 56
- **Crates:** `morm-core`, `morm`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

`morm.toml` and environment variables, in `morm-core` so `morm-migrate` and `morm-admin` read the same config, plus connecting over sockets, TCP and TLS.

## Deliverables

- Config loading in `morm-core`, every key and variable, `[database.migration]`, `[server]`, named pool profiles and replicas as config
- `Db::connect`, `connect_as`, `connect_to`, sockets, TLS, `search_path`, startup parameters
- `crates/morm-core/morm.example.toml`, the commented settings template, and its test
- Every key and table the config structs don't read refused at load, naming the key and its table

## End-to-end check

An app connects from `morm.toml` alone, and from environment variables alone, over TCP, Unix socket and TLS, and every invalid config fails with a message naming the setting.

## Out of scope

- The pool (step 11)
- `init` socket detection (step 42)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools

- **R-0048** `tls`: rustls for TCP connections. The connector and its certificates load only when `sslmode` needs them, so with it compiled in and unused, the cost is binary size and build time. Unix sockets never use TLS

### Crates and tools / Configuration: `morm.toml`

**R-0059** TOML is the Rust standard, with comments, real types and sections, and no indentation traps. Settings need no Rust code.

**R-0060**

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
check_schema = "on"              # "off" starts against a schema that may not match: read the README first
stale_cooldown = "60s"           # one window for pool-wide stale repairs, the epoch, the pooler cool-down; 1s to 10m
stale_cooldown_max = 64          # cool-down entries per pool, behind a pooler only; 1 to 4096
stale_pending_expiry = "60s"     # how long a pending entry keeps a stale statement fresh; "off", or "0s" to 10m
stale_pending_max = 64           # pending entries per pool; 0 (off) to 4096

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

**R-0061** A missing `morm.toml` isn't an error unless `MORM_CONFIG` names it, so a deployment can use environment variables alone. An unset `${VAR}` leaves that field unset, and if a required value ends up missing, startup fails and names the variable.

**R-0062** Precedence: defaults, then `morm.toml`, then environment variables, then code. There is one config file, and a process that needs different values uses a named pool, an environment variable, or code.

**R-1273** **A commented settings template ships with morm.** `crates/morm-core/morm.example.toml`, part of `morm-core`'s published package because `morm-core` reads the config, lists every setting morm reads from `morm.toml`, grouped by table in R-0060's order: `[database]`, `[database.migration]`, `[server]`, `[pool]`, the `[pools.<name>]` profiles, `[outbox]`, `[telemetry]` and `[migrations]`. Its header comment says that morm refuses any key it doesn't read, in any table, so a misspelt or misplaced key stops the app at startup with `Error::Config`, and that this file is where to check a key's spelling and table. Each setting is one line, `key = value`, with comments above it giving:

- what it does and when to change it, in a line or two;
- its default, its allowed values or range, and its environment variable, or that it has none;
- a loud `# WARNING:` line beside any value that turns a protection or a check off, saying what's lost and naming the README notice that explains it, such as `check_schema = "off"`, `stale_pending_expiry = "off"` or `"0s"`, `stale_pending_max = 0`, and `sslmode` values that don't encrypt a TCP connection.

Every key of R-0060's block appears in the template, in the same table, with R-0060's value. The tables R-0060 shows as examples rather than defaults, `[server]`, `[pools.replica]`, `[pools.replica.server]` and `[pools.jobs]`, are written commented out, each line as `# key = value` and each table as `# [table]`, and so is every setting with no default that R-0060 doesn't show, so loading the template unchanged, with no environment variable set, gives the same config as no file at all; uncommenting a line turns it on. The stale-session settings carry step 14's ranges, say that `stale_cooldown_max` matters only behind a pooler, and say what turning pending entries off costs. A step that adds a setting adds it to the template in the same PR, because T-0320 fails until it does. Owner decision on the stale-session design: a commented settings template, kept in step with the plan by a test.

**R-0063** **Environment variables:**

| ID | Variable | Overrides |
|---|---|---|
| R-0064 | `MORM_CONFIG` | Path to the config file (default `morm.toml`) |
| R-0065 | `DATABASE_URL` | Full database URL |
| R-0066 | `MORM_DB_HOST`, `MORM_DB_PORT` | Host or Unix socket directory, and port |
| R-0067 | `MORM_DB_NAME` | Database name |
| R-0068 | `MORM_DB_USER` | User |
| R-0069 | `MORM_DB_PASSWORD` | Password |
| R-0070 | `MORM_DB_PASSWORD_FILE` | File holding the password, for systemd credentials and secret mounts |
| R-0071 | `MORM_DB_SSLMODE` | `disable`, `prefer`, `require`, `verify-ca`, `verify-full` |
| R-0072 | `MORM_DB_SSLROOTCERT` | CA certificate path |
| R-0073 | `MORM_DB_SSLCERT`, `MORM_DB_SSLKEY` | Client certificate and key paths |
| R-0074 | `MORM_DB_TARGET_SESSION_ATTRS`, `MORM_DB_CONNECT_TIMEOUT` | `target_session_attrs`, `connect_timeout` |
| R-0075 | `MORM_DB_APPLICATION_NAME`, `MORM_DB_SEARCH_PATH` | `application_name`, `search_path` |
| R-0076 | `MORM_DB_POOLER` | `pooler` |
| R-0077 | `MORM_MIGRATION_URL` | Full owner-role URL for migrations |
| R-0078 | `MORM_MIGRATION_USER`, `MORM_MIGRATION_PASSWORD` | Owner-role login, using the same host and database |
| R-0079 | `MORM_MIGRATION_PASSWORD_FILE` | File holding the owner-role password |
| R-0080 | `MORM_SCRATCH_URL` | Scratch database URL |
| R-0081 | `MORM_MIGRATIONS_DIR` | Migrations directory |
| R-0082 | `MORM_OUTBOX_BATCH`, `MORM_OUTBOX_POLL_INTERVAL` | `batch`, `poll_interval` |
| R-0083 | `MORM_OUTBOX_LEASE`, `MORM_OUTBOX_MAX_ATTEMPTS`, `MORM_OUTBOX_RETENTION` | `lease`, `max_attempts`, `retention` |
| R-0084 | `MORM_SERVER_RAM`, `MORM_SERVER_CORES` | `[server]` `ram`, `cores` |
| R-0085 | `MORM_TELEMETRY_SPAN_SAMPLE`, `MORM_TELEMETRY_SLOW_QUERY` | `span_sample`, `slow_query` |
| R-0086 | `MORM_TELEMETRY_METRICS_INTERVAL` | `metrics_interval` |
| R-0087 | `MORM_POOL` | Named pool this process opens, instead of `[pool]` |
| R-0088 | `MORM_POOL_MIN`, `MORM_POOL_MAX` | `min`, `max` |
| R-0089 | `MORM_POOL_PROCESSES` | `processes` |
| R-0090 | `MORM_POOL_IDLE_TIMEOUT`, `MORM_POOL_LAST_IDLE_TIMEOUT` | `idle_timeout`, `last_idle_timeout` |
| R-0091 | `MORM_POOL_MAX_QUERIES`, `MORM_POOL_MAX_LIFETIME` | `max_queries`, `max_lifetime` |
| R-0092 | `MORM_POOL_HOLD_WARN_AFTER` | `hold_warn_after` |
| R-0093 | `MORM_POOL_WAIT_TIMEOUT`, `MORM_POOL_MAX_HOLDS` | `wait_timeout`, `max_holds` |
| R-0094 | `MORM_POOL_LOCK_TIMEOUT`, `MORM_POOL_TRANSACTION_TIMEOUT` | `lock_timeout`, `transaction_timeout` |
| R-0095 | `MORM_POOL_IDLE_IN_TRANSACTION_SESSION_TIMEOUT` | `idle_in_transaction_session_timeout` |
| R-0096 | `MORM_POOL_CLIENT_CONNECTION_CHECK_INTERVAL` | `client_connection_check_interval` |
| R-0097 | `MORM_POOL_TX_WARN_AFTER` | `tx_warn_after` |
| R-0098 | `MORM_POOL_STATEMENT_TIMEOUT`, `MORM_POOL_PLAN_CACHE_MODE` | `statement_timeout`, `plan_cache_mode` |
| R-0099 | `MORM_POOL_STATEMENT_CACHE`, `MORM_POOL_SHAPE_CACHE` | `statement_cache`, `shape_cache` |
| R-0100 | `MORM_POOL_SHUTDOWN_TIMEOUT` | `shutdown_timeout` |
| R-1118 | `MORM_POOL_CHECK_SCHEMA` | `check_schema` |
| R-1259 | `MORM_POOL_STALE_COOLDOWN`, `MORM_POOL_STALE_COOLDOWN_MAX` | `stale_cooldown`, `stale_cooldown_max` |
| R-1260 | `MORM_POOL_STALE_PENDING_EXPIRY`, `MORM_POOL_STALE_PENDING_MAX` | `stale_pending_expiry`, `stale_pending_max` |

**R-1261** There's no `stale_first_failure_max` setting and no `MORM_POOL_STALE_FIRST_FAILURE_MAX`: straight to Postgres one epoch per pool decides which connections a stale-session failure may replace, so there are no first-failure records to bound, and config loading refuses the key with a message saying it was removed, as R-1275 says. Owner decision MA.

**R-0101** `MORM_POOL_*` variables apply to the pool this process opens at startup.

**R-0102** **Unix sockets:** the default `host` is `/var/run/postgresql`, the Ubuntu and Debian location. A `host` starting with `/` is a socket directory, and anything else is a hostname using TCP. The socket file is `<host>/.s.PGSQL.<port>`, so `port` still matters. Other systems set `host` to match Postgres's `unix_socket_directories`; Arch and Manjaro usually use `/run/postgresql`. If `host` is a full socket file path, morm takes the directory and port from it.

**R-0109** **`search_path`** is set on every connection, so another role can't shadow a table or function through a schema earlier in the path.

**R-1205** **Sessions always run in UTC.** Every connection sends `TimeZone = 'UTC'` as a startup parameter, so it adds no round trip, and it wins over the server's and the role's `TimeZone`, because a startup parameter wins over `ALTER ROLE ... SET timezone`. Nothing overrides it. So raw SQL that converts between `timestamp` and `timestamptz`, formats a time or truncates one to a day works in UTC, the zone in which morm's own encodings of a time bound through a field always work, and the two never disagree. Instants belong in `timestamptz`. Code that needs a user's local day or wall time passes the zone in its SQL, as in `date_trunc('day', {Order::F.at}, ${zone: String})` or `{Order::F.at} AT TIME ZONE ${zone: String}`. It goes in the `options` startup parameter, as `-c TimeZone=UTC`. With `pooler = "transaction"` it's still sent, the one setting sent in `options` there, beside `application_name`, because PgBouncer 1.20 and later tracks `TimeZone` per client and sets it on every server connection the client gets, as the pooler rules say, so the role's zone doesn't matter. The startup check still refuses a pool whose session reports another zone, for a pooler that drops the parameter. Owner decisions DG2, DH1, DJ2 and DK5; `proofs.md`, proof 9.

**R-1206** No setting or environment variable changes the session's zone: `morm.toml` has no `timezone`, and there's no `MORM_DB_TIMEZONE`. Owner decision DH1.

**R-1235** **A `timezone` key is refused.** A `timezone` key in any table of `morm.toml`, `[database]`, `[database.migration]`, `[pool]` and every `[pools.<name>]` profile among them, fails config loading with `Error::Config`, naming the key and the table it's in and saying morm's sessions always run in UTC, so a setting copied from another tool can't seem to work while it does nothing. `morm-core` loads the config, so `morm-admin` refuses it too. Owner decision DI2.

**R-1275** **Every key morm doesn't read is refused.** Config loading refuses any key the config structs don't read, in every table of `morm.toml`, `[database]`, `[database.migration]`, `[server]`, `[pool]`, every `[pools.<name>]` profile and its `[pools.<name>.server]`, `[outbox]`, `[telemetry]` and `[migrations]` among them, with `Error::Config` naming the key and the table it's in, so a misspelt key, such as `stale_cooldwn` or `wait_timout`, stops the app at startup instead of seeming to work while it does nothing. A table morm doesn't read, such as `[pol]`, is refused the same way, as an unknown key of the file. When a key morm reads in that table is close to the unknown one, the message names it, and it always says that `morm.example.toml` lists every setting. The keys the plan refuses by name keep their own messages: `timezone`, R-1235's, and `stale_first_failure_max`, which says the setting was removed, as step 14 says. `morm-core` loads the config, so `morm-admin` and `morm-migrate` refuse the same file the same way. Environment variables aren't keys: a `MORM_*` variable morm doesn't read is ignored, because the `${VAR}` names a file interpolates, such as `MORM_JOBS_PASSWORD`, share the prefix. Owner decision on unknown `morm.toml` keys.

**R-0111** **Credentials:** `MORM_DB_*` values override the matching parts of `DATABASE_URL`, so a URL can hold the address while the password comes from `MORM_DB_PASSWORD_FILE`. Setting both `MORM_DB_PASSWORD` and `MORM_DB_PASSWORD_FILE` is an error. `sslmode` values other than `disable` and `prefer` need the `tls` feature, and morm refuses to start without it. `prefer` without `tls` can only connect unencrypted, so morm warns at startup, and it never connects unencrypted silently. Passwords are never logged, and they're masked in `Debug` output and error messages.

**R-0113** **Named pools** are full connection profiles. Any `[database]` field can be set in `[pools.<name>]`, so a job can log in as another role or reach another host. Unset connection fields come from `[database]`, and unset pool settings from `[pool]`, except `min`, which defaults to 1. A separate process can open only its own profile, so job settings live in the same file:

**R-0114**

```rust
let db = morm::Db::connect_as("jobs").await?;   // opens only [pools.jobs]
```

**R-0115** `MORM_POOL=jobs` with plain `Db::connect()` does the same without a code change. `db.pool("jobs")` is different: it adds that pool alongside the default one in a process that has both.

### Reading and writing / Connecting

**R-0333**

```rust
let db = morm::Db::connect().await?;                       // reads morm.toml: [database], [pool]
let db = morm::Db::connect_as("jobs").await?;              // reads morm.toml: [pools.jobs]
let db = morm::Db::connect_to(url).min(10).max(90).await?; // or set values in code
```

**R-0334** `connect` honours `MORM_POOL`, so a process can switch profile without a code change. `connect_as` names the profile in code and fails if it doesn't exist. Both return a builder, so `.min()` and `.max()` work there too.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0298 | integration | **A `timezone` key is refused:** `timezone` in `[database]`, in `[database.migration]`, in `[pool]`, and in a `[pools.<name>]` profile, each make `Db::connect` and `connect_as` return `Error::Config` before any connection opens, naming the key and its table and saying sessions always run in UTC; `morm-admin` reading the same file stops with the same message; and a session opened with no such key reports `TimeZone` as `UTC` on a server whose own zone isn't UTC |
| T-0320 | suite | **The settings template stays in step with the plan:** a unit test in `morm-core`, run in `ci`'s workspace test job, reads `crates/morm-core/morm.example.toml` and R-0060's `toml` block from this step's file, `plans/plan-approved/steps/10-config.md`, reading a `# key = value` line as a commented-out setting and a `# [table]` line as a commented-out table: every key of R-0060's block is in the template, in the same table, with the same value, commented out exactly where R-1273 says; every key the config structs read, in every table, is in the template, with the default the loader uses and the environment variable the loader reads for it named in its comment, or none; the template holds no key, active or commented out, that the config structs don't read; the header comment states that unknown keys are refused at startup; each setting has a comment giving its default and its allowed values or range; `check_schema`, `stale_pending_expiry`, `stale_pending_max` and `sslmode` each have a `# WARNING:` line; and loading the template with no environment variable set gives the same config as loading no file. A key added to the config structs or to R-0060's block without the template, a changed default, or a key left in the template after it's gone, such as `stale_first_failure_max`, fails it, naming the key and the table |
| T-0322 | integration | **Unknown keys are refused:** a misspelt key in each table, `[database]`, `[database.migration]`, `[server]`, `[pool]`, a `[pools.<name>]` profile, its `[pools.<name>.server]`, `[outbox]`, `[telemetry]` and `[migrations]`, and an unknown table such as `[pol]`, each make `Db::connect` and `connect_as` return `Error::Config` before any connection opens, naming the key and its table, naming the close key it was meant to be where there is one, such as `stale_cooldown` for `stale_cooldwn`, and pointing to `morm.example.toml`; `morm-admin` reading the same file stops with the same message; a key that's right in one table but sits in another, such as `wait_timeout` in `[database]`, is refused too; `timezone` still gets R-1235's message and `stale_first_failure_max` its removed-setting message; a file holding every key the config structs read loads with no error; and an unread `MORM_*` environment variable, such as `MORM_JOBS_PASSWORD` used only through `${...}`, starts the app |
