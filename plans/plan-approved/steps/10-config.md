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
- `crates/morm-core/morm.toml`, a real config file with every setting present and set to its default, and its tests
- Every key and table the config structs don't read refused at load, naming the key and its table
- `sslmode` defaulting to `require`, applied to TCP connections only, with a Unix socket never negotiating TLS

## End-to-end check

An app connects from `morm.toml` alone, and from environment variables alone, over TCP, Unix socket and TLS, and every invalid config fails with a message naming the setting. The shipped `morm.toml`, copied next to the app unchanged, starts it exactly as no file does, `DATABASE_URL` included.

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
# An example. Every setting, at its default and with what it does, is in morm-core's shipped morm.toml
[database]
# Either a URL, or the separate fields below. Separate fields override URL parts.
url = "${DATABASE_URL}"
host = "/var/run/postgresql"     # Unix socket directory or hostname
port = 5432
name = "app"
user = "app"                     # no password needed with peer auth on a Unix socket
password = "${MORM_DB_PASSWORD}" # keep secrets in the environment
sslmode = "require"              # the default; TCP only, a Unix socket never uses TLS; disable, prefer, require, verify-ca, verify-full
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

**R-0062** Precedence: defaults, then `morm.toml`, then environment variables, then code. There is one config file, and a process that needs different values uses a named pool, an environment variable, or code. A URL is one value, and the separate fields override its parts wherever each is set, as R-0060 says: so a `host` written in `morm.toml` overrides the host of a `DATABASE_URL` from the environment, which R-1282 warns about, and `MORM_DB_HOST` overrides both, as R-0111 says. Owner decision: `init` writes a found socket loudly (ER2).

**R-1273** **morm ships a real `morm.toml`, with every setting at its default.** `crates/morm-core/morm.toml` is part of `morm-core`'s published package, because `morm-core` reads the config and cargo packages only the files inside a crate's folder. It's a working config file, not a template: copied next to an app unchanged, as `morm.toml`, it loads with no error and gives exactly the config no file gives, whatever environment variables are set. A user gets it from the README, which links to it and says to copy it next to the app and change only what they need, or from `morm-core`'s package source; `morm-admin init` writes it as a new project's `morm.toml`, as step 42 says, changing only the `host` and `port` lines a found socket needs, loudly. It lists every setting morm reads, grouped by table in R-0060's order: `[database]`, `[database.migration]`, `[server]`, `[pool]`, the `[pools.<name>]` profiles, `[outbox]`, `[telemetry]` and `[migrations]`. Its header comment says that every value in it is the default, so a line can be changed or deleted with nothing else lost; that morm refuses any key it doesn't read, in any table, so a misspelt or misplaced key stops the app at startup with `Error::Config`; and that this file is where to check a key's spelling and table. Each setting is one line, `key = value`, active apart from the commented-out examples below, with comments above it giving:

- what it does and when to change it, in a line or two;
- its default, its allowed values or range, and its environment variable, or that it has none;
- a loud `# WARNING:` line beside any value that turns a protection or a check off, saying what's lost and naming the README notice that explains it: `check_schema = "off"`, `stale_pending_expiry = "off"` or `"0s"`, `stale_pending_max = 0`, and `sslmode = "disable"` or `"prefer"`, which can leave a TCP connection unencrypted, naming the README notice on `sslmode`.

**What each line holds.**

- **A setting with a default** is set to it, the value the loader uses when the setting is absent. **R-0060's values are the defaults,** for every key it shows, apart from the ones it shows as examples: `[database]`'s `name`, `user` and `password`, the settings with no default, `[server]`'s `ram` and `cores`, and the keys of the commented-out profiles. Those are written as this list says, with R-0060's value in the comment, and T-0320 fails on any other key whose default isn't R-0060's value.
- **A connection field a URL can also carry** is written as a `${VAR}` placeholder naming its own environment variable, even when it has a default: in `[database]`, `host = "${MORM_DB_HOST}"`, `port = "${MORM_DB_PORT}"`, `name`, `user`, `password`, `sslmode`, `sslrootcert`, `sslcert`, `sslkey`, `target_session_attrs`, `connect_timeout` and `application_name` with R-0063's `MORM_DB_*` variables, and in `[database.migration]`, `user` and `password` with `MORM_MIGRATION_USER` and `MORM_MIGRATION_PASSWORD`. Its comment gives its default, if it has one, and R-0060's value as an example. Separate fields override URL parts, so an active value would silently override `DATABASE_URL`'s, but an unset variable leaves the field unset, as R-0061 says: the URL's parts win, and the default applies only where neither sets the field. So a copied file never moves an app off the host, port, database, user, password or `sslmode` its `DATABASE_URL` names. Of these fields, `host` and `port` have R-0102's defaults, `sslmode` has R-1277's, `target_session_attrs`, `connect_timeout` and `application_name` have R-0060's values as defaults, and `name`, `user`, `password` and the certificate paths have none.
- **A setting with no default,** such as `[database] url`, `[database.migration] url` or `[migrations] scratch_url`, is a `${VAR}` placeholder naming its variable, as R-0060 shows it; one with no default and no variable is written commented out, `# key = value`, with an example value.
- **The tables R-0060 shows as examples,** `[pools.replica]`, `[pools.replica.server]` and `[pools.jobs]`, show the shape of a pool profile and nothing more: each is written commented out, its lines as `# key = value` with R-0060's values and the table as `# [table]`, because an active profile adds a pool. Uncommenting one turns it on. `[server]` is active, its `ram` and `cores` written as `${MORM_SERVER_RAM}` and `${MORM_SERVER_CORES}`, so it describes no server until they're set.

A `${VAR}` placeholder can stand for a number or a boolean too, such as `port = "${MORM_DB_PORT}"`: after the variable is read, the value is parsed as that setting's type, and a value that doesn't parse stops loading with `Error::Config`, naming the setting and the variable. The stale-session settings say that `stale_cooldown_max` matters only behind a pooler, and what turning pending entries off costs.

**One table of settings.** The loader keeps one table of every setting it reads, each row holding the table, the key, the default, the environment variable and the allowed values or bounds, and its parsing, its validation and its unknown-key check, R-1275's, read that table, so the file, the loader and the tests can't disagree. A step that adds a setting adds its row and its line in `morm.toml` in the same PR, because T-0320 fails until it does. Owner decisions: the settings file is a real `morm.toml`, not a template, and unknown `morm.toml` keys are refused.

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

**R-1261** There's no `stale_first_failure_max` setting and no `MORM_POOL_STALE_FIRST_FAILURE_MAX`: straight to Postgres one epoch per pool decides which connections a stale-session failure may replace, so there are no first-failure records to bound, and config loading refuses the key, in whichever table it's in, with a message saying it was removed, as R-1275 says. Owner decision MA.

**R-0101** `MORM_POOL_*` variables apply to the pool this process opens at startup.

**R-0102** **Unix sockets:** the default `host` is `/var/run/postgresql`, the Ubuntu and Debian location, and the default `port` is `5432`. A `host` starting with `/` is a socket directory, and anything else is a hostname using TCP. The socket file is `<host>/.s.PGSQL.<port>`, so `port` still matters. Other systems set `host` to match Postgres's `unix_socket_directories`; Arch and Manjaro usually use `/run/postgresql`. If `host` is a full socket file path, morm takes the directory and port from it.

**R-0109** **`search_path`** is set on every connection, so another role can't shadow a table or function through a schema earlier in the path.

**R-1205** **Sessions always run in UTC.** Every connection sends `TimeZone = 'UTC'` as a startup parameter, so it adds no round trip, and it wins over the server's and the role's `TimeZone`, because a startup parameter wins over `ALTER ROLE ... SET timezone`. Nothing overrides it. So raw SQL that converts between `timestamp` and `timestamptz`, formats a time or truncates one to a day works in UTC, the zone in which morm's own encodings of a time bound through a field always work, and the two never disagree. Instants belong in `timestamptz`. Code that needs a user's local day or wall time passes the zone in its SQL, as in `date_trunc('day', {Order::F.at}, ${zone: String})` or `{Order::F.at} AT TIME ZONE ${zone: String}`. It goes in the `options` startup parameter, as `-c TimeZone=UTC`. With `pooler = "transaction"` it's still sent, the one setting sent in `options` there, beside `application_name`, because PgBouncer 1.20 and later tracks `TimeZone` per client and sets it on every server connection the client gets, as the pooler rules say, so the role's zone doesn't matter. The startup check still refuses a pool whose session reports another zone, for a pooler that drops the parameter. Owner decisions DG2, DH1, DJ2 and DK5; `proofs.md`, proof 9.

**R-1206** No setting or environment variable changes the session's zone: `morm.toml` has no `timezone`, and there's no `MORM_DB_TIMEZONE`. Owner decision DH1.

**R-1235** **A `timezone` key is refused.** A `timezone` key in any table of `morm.toml`, `[database]`, `[database.migration]`, `[pool]` and every `[pools.<name>]` profile among them, fails config loading with `Error::Config`, naming the key and the table it's in and saying morm's sessions always run in UTC, so a setting copied from another tool can't seem to work while it does nothing. `morm-core` loads the config, so `morm-admin` refuses it too. Owner decision DI2.

**R-1275** **Every key morm doesn't read is refused.** Config loading refuses any key the config structs don't read, in every table of `morm.toml`, `[database]`, `[database.migration]`, `[server]`, `[pool]`, every `[pools.<name>]` profile and its `[pools.<name>.server]`, `[outbox]`, `[telemetry]` and `[migrations]` among them, with `Error::Config` naming the key and the table it's in, so a misspelt key, such as `stale_cooldwn` or `wait_timout`, stops the app at startup instead of seeming to work while it does nothing. A table morm doesn't read, such as `[pol]`, is refused the same way, as an unknown key of the file. When a key morm reads in that table is close to the unknown one, the message names it, and it always says that the `morm.toml` morm ships, R-1273's, lists every setting. The keys the plan refuses by name keep their own messages wherever they appear, in any table, `[database]` and `[migrations]` as much as `[pool]` and every `[pools.<name>]`: `timezone`, R-1235's, and `stale_first_failure_max`, which says the setting was removed. This is where the scope of the removed-setting message is stated; step 14's settings and R-1261 point here. `morm-core` loads the config, so `morm-admin` and `morm-migrate` refuse the same file the same way. Environment variables aren't keys: a `MORM_*` variable morm doesn't read is ignored, because the `${VAR}` names a file interpolates, such as `MORM_JOBS_PASSWORD`, share the prefix. Owner decision on unknown `morm.toml` keys.

**R-0111** **Credentials:** `MORM_DB_*` values override the matching parts of `DATABASE_URL`, so a URL can hold the address while the password comes from `MORM_DB_PASSWORD_FILE`. Setting both `MORM_DB_PASSWORD` and `MORM_DB_PASSWORD_FILE` is an error. For a TCP host, `sslmode` values other than `disable` and `prefer`, the default `require` among them, need the `tls` feature, and morm refuses to start without it, with `Error::Config` naming `sslmode`, the `tls` feature and `sslmode = "disable"`; a Unix socket needs neither, as R-1277 says. `prefer` without `tls` can only connect unencrypted, so morm warns at startup, and it never connects unencrypted silently. Passwords are never logged, and they're masked in `Debug` output and error messages.

**R-1277** **`sslmode` defaults to `require`, and applies to TCP connections only.** With `sslmode` set nowhere, not in `morm.toml`, `MORM_DB_SSLMODE` or the URL, a TCP connection uses TLS, and a server that doesn't offer it is refused: the open fails with `Error::Connect` naming the host, saying the server doesn't offer TLS, and naming the README notice on `sslmode`, which says that a server reached over TCP without TLS needs `sslmode = "disable"` set explicitly. That holds for every connection morm opens, from the app, `morm-admin` and `morm-migrate` alike, PgBouncer over TCP included: every pool and profile, `[database.migration]`, `[migrations] scratch_url`, each `[migrations] pgbouncer_admin_urls` entry, step 59's, and `morm-admin`'s own, such as `doctor`, `scratch`, `outbox` and `import URL`. `morm-core`'s config resolves each connection's mode per host, so all three connect alike. `morm-admin` always carries TLS, as step 33 says, and so does `morm-migrate`, used on its own as a library too, as step 29 says, and the `morm-mgr` package `init` creates turns on `morm`'s `tls`, as step 42 says, so R-0111's refusal for want of the `tls` feature never comes from any of them, a migration's data hooks included. A connection built from `[database]`, directly or through a profile or `[database.migration]`, takes `sslmode` from wherever R-0062 says; one given a URL of its own and nothing else, `scratch_url`, an admin URL or `import`'s, takes it from that URL alone, so an admin URL over TCP to a PgBouncer without client TLS needs `?sslmode=disable`, as the README notice on `pgbouncer_admin_urls` says. A Unix socket, a `host` starting with `/`, never negotiates TLS, whatever `sslmode` says, the default `require` and `verify-full` included, and needs no `tls` feature, so local socket use works unchanged. Postgres offers no TLS on a Unix socket, and libpq ignores `sslmode` there, but `tokio-postgres` 0.7 sends its TLS request on a socket too and fails `require` with `server does not support TLS`, so morm gives the driver `SslMode::Disable` and no TLS connector for every socket host, and the mode and connector `sslmode` asks for only for a TCP host. A host list that mixes socket directories and TCP hosts is tried one host at a time, in its order, each with its own driver config, so each host gets its own mode, and `target_session_attrs` is checked on each as the driver checks it. morm's own test suites set `sslmode = "disable"` where they reach a server without TLS over TCP, such as CI's service containers and PgBouncer instances, so nothing in them leans on a weaker default. Owner decision: `sslmode` defaults to `require`; `proofs.md`, step 55's TLS evidence.

**R-1282** **A `[database]` value written in `morm.toml` that overrides `DATABASE_URL` is warned about.** Separate fields override a URL's parts, R-0062's rule, so a `host`, `port`, `name`, `user`, `password`, `sslmode` or any other field a URL can carry, written in `morm.toml`'s `[database]` as a value, not as a `${VAR}` placeholder, overrides that part of the `DATABASE_URL` a deployment sets. When config loading finds such a value in effect, with no `MORM_DB_*` variable overriding it, and the URL in effect came from `DATABASE_URL`, read directly or through the file's `url = "${DATABASE_URL}"`, and holds another value for that part, morm warns at startup through the warning sink, once per field at each load, naming the field, `morm.toml`, the file's value, the URL's value and the README notice on it, "A `host` or `port` written in `morm.toml` overrides `DATABASE_URL`"; for `password` it names the field and neither value. `[database.migration]`'s fields against `MORM_MIGRATION_URL` are warned about the same way. A `[pools.<name>]` profile's fields aren't, because a profile exists to differ from `[database]`, such as a replica's `host`. A URL with no value for that part, the same value, or a URL written in the file with `DATABASE_URL` unset gives no warning. `Db::connect`, `connect_as`, `morm-admin` and `morm-migrate` warn alike, because `morm-core` loads the config. Nothing is refused: the file's value wins, as R-0062 says. Owner decision: `init` writes a found socket loudly (ER2).

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
| T-0320 | suite | **The shipped `morm.toml` stays in step with the plan and loads as no file does:** a unit test in `morm-core`, run in `ci`'s workspace test job, reads `crates/morm-core/morm.toml`, R-0060's `toml` block from this step's file, `plans/plan-approved/steps/10-config.md`, and the loader's table of settings, R-1273's, reading a `# key = value` line as a commented-out setting and a `# [table]` line as a commented-out table: every key of R-0060's block is in the file, in the same table, active or commented out exactly where R-1273 says, an active setting with a default holding R-0060's value, or, where R-0060 shows another value than the default, giving R-0060's in its comment, a connection field holding the `${VAR}` placeholder of its own variable with R-0060's value in its comment, and the example tables holding R-0060's values; for every key of R-0060's block that R-1273 doesn't list as an example, the loader's table's default equals R-0060's value, so a default that drifts from the plan, such as `wait_timeout` at `"10s"`, fails it, naming the key and its table, and the table's default `port` is `5432`, R-0102's; every row of the loader's table, in every table, is in the file, a setting with a default set to the table's default, a connection field or a setting with no default as the placeholder of the variable the table names, its comment naming that variable, or none, and giving the default and the allowed values or range, compared with the table's, so a changed bound such as one of step 14's ceilings fails it; the file holds no key, active or commented out, that the table doesn't hold; the header comment states that every value is the default and that unknown keys are refused at startup; `check_schema`, `stale_pending_expiry`, `stale_pending_max` and `sslmode` each have a `# WARNING:` line, `sslmode`'s naming the README notice on `sslmode`; and loading the file gives the same config as loading no file, with no environment variable set, with each variable the file names set on its own, and with `DATABASE_URL` set to another host, port, database and user and `?sslmode=disable`, where both give the URL's host, port, name, user and `sslmode`, not the defaults and not R-0060's examples, and the same with `MORM_MIGRATION_URL` for `[database.migration]`. A row added to the loader's table or a key added to R-0060's block without the file, a changed default, range or variable, or a key left in the file after it's gone, such as `stale_first_failure_max`, fails it, naming the key and the table |
| T-0322 | integration | **Unknown keys are refused:** a misspelt key in each table, `[database]`, `[database.migration]`, `[server]`, `[pool]`, a `[pools.<name>]` profile, its `[pools.<name>.server]`, `[outbox]`, `[telemetry]` and `[migrations]`, and an unknown table such as `[pol]`, each make `Db::connect` and `connect_as` return `Error::Config` before any connection opens, naming the key and its table, naming the close key it was meant to be where there is one, such as `stale_cooldown` for `stale_cooldwn`, and pointing to the shipped `morm.toml`; `morm-admin` reading the same file stops with the same message, and so does `morm-migrate`'s config load, through its library entry point; a key that's right in one table but sits in another, such as `wait_timeout` in `[database]`, is refused too; `timezone` still gets R-1235's message, and `stale_first_failure_max` gets its removed-setting message in `[database]`, `[pool]`, a `[pools.<name>]` profile and `[migrations]` alike; a file holding every key the config structs read loads with no error; and an unread `MORM_*` environment variable, such as `MORM_JOBS_PASSWORD` used only through `${...}`, starts the app |
| T-0323 | integration | **`sslmode` defaults to `require`, over TCP only, on Postgres 15–18:** against a server with `ssl = on` and a certificate, with `sslmode` set nowhere and the `tls` feature on, a TCP connection reports `ssl` true in `pg_stat_ssl`; against one with `ssl = off`, the default refuses the TCP open with `Error::Connect` naming the host, saying the server doesn't offer TLS and naming the README notice on `sslmode`, while `sslmode = "disable"` in `morm.toml`, `MORM_DB_SSLMODE=disable` and `?sslmode=disable` in `DATABASE_URL` each connect with `ssl` false, and `prefer` with the `tls` feature off connects with R-0111's warning; over a Unix socket, against the server with `ssl = on`, `sslmode` unset, `"require"`, `"verify-ca"` and `"verify-full"` each connect with `ssl` false, with the `tls` feature on and off, where the driver given `require` on its own refuses with `server does not support TLS`; without the `tls` feature, a TCP host with `sslmode` unset stops the pool at start with `Error::Config` naming `sslmode`, the `tls` feature and `sslmode = "disable"`, while a socket host starts; a `[database.migration]` URL and a `[pools.<name>]` profile on a TCP host get the same default; a host list holding a socket directory and a TCP host, with `target_session_attrs = "read-write"`, reaches the socket host with `ssl` false and, when that server is read-only, the TCP host with `ssl` true; the same list with `sslmode` unset and the `tls` feature off stops the pool at start with `Error::Config` naming `sslmode`, the `tls` feature and `sslmode = "disable"`, as R-0111's rule for a TCP host says, while a list of socket directories alone starts; and the mode `morm-core`'s config resolves for `scratch_url` and for a URL given on its own is `require` for a TCP host and `disable` for a socket host with nothing set, and the URL's own `sslmode` when it has one, whatever `[database]` or `MORM_DB_SSLMODE` says |
| T-0325 | suite | **A `[database]` value written in `morm.toml` that overrides `DATABASE_URL` is warned about:** a unit test in `morm-core`, with the warning sink captured, loads a `morm.toml` holding `host = "/run/postgresql"` and `port = 5433` as values, with `DATABASE_URL` naming another host and port: the config holds the file's host and port, by R-0062's rule, and one warning per field names it, `morm.toml`, both values and the README notice on it; with `user` and `password` written as values and the URL holding others, the user's warning names both users and the password's names neither password; `[database.migration]`'s `user` written against a `MORM_MIGRATION_URL` holding another warns the same way; and there's no warning, and the config holds what R-0062 says, with the URL holding the same values, a URL with no port against the file's `port`, the fields as `${VAR}` placeholders, `MORM_DB_HOST` and `MORM_DB_PORT` set, which override the file, a `[pools.<name>]` profile's `host`, a `url` written in the file with `DATABASE_URL` unset, and the shipped `morm.toml` with any `DATABASE_URL` |
