# rust-morm: plan

A complete re-engineering of morm in Rust: define PostgreSQL models in code, track them with rock-solid up and down migrations, and write queries in plain SQL. There is no extra query language. This is a new design, not a port. python-morm is a reference for what real projects need, and nothing is kept just for compatibility.

- **Repo:** `biggaloy/rust-morm`
- **Crates:** `morm`, `morm-macros`, `morm-migrate`, `morm-admin`. The first two names were free on crates.io as of 2026-11-04; the other two still need checking.
- **Database:** PostgreSQL only.
- **Naming:** everything in the project uses the `morm-` prefix.

---

## 1. Goals (in priority order)

1. **Highest raw performance:** minimal work per query; everything that can be decided at compile time is.
2. **Lowest memory footprint:** no per-row overhead, no unnecessary allocations, few dependencies, small binary, few Postgres connections.
3. **Rock-solid up and down migrations:** proven before they're written, checked when they run, recoverable from any failure.
4. **Hard to misuse:** the cheap, correct way is the obvious way, and costly mistakes fail at compile time or in tests.
5. **morm's ergonomics:** models in code, automatic migrations, raw SQL for queries.

Non-goals: other databases, a query language that replaces SQL, runtime reflection, relation loading, implicit soft-delete filters, automatic change tracking.

---

## 2. Key decisions

| Topic | Decision |
|---|---|
| Driver | `tokio-postgres` |
| Pool | `morm::Pool`: the `deadpool` core crate with our own connection manager. Dynamic size, one exclusive checkout per query |
| Configuration | `morm.toml`, shared by the app and `morm-admin`, so settings need no Rust |
| Long-running work | Jobs and crons use the pool normally. `db.hold()` is only for session state |
| Deadlocks and commits | Commits are confirmed, a second connection on a task is refused, pool and lock waits are finite, and single statements retry once on a reported deadlock (section 8) |
| Statement cache | Bounded per-connection LRU, with `query_typed` for dynamic SQL and overflow |
| Connection | Unix socket by default (`host=/var/run/postgresql`), the lowest latency and allows peer auth. TCP also supported |
| TLS | Optional cargo feature `tls` (rustls). Not compiled in unless enabled. Compiling it in costs binary size and build time; RAM and CPU are only spent on connections that use TLS |
| Default primary key | `bigserial` (`base = "common"`), the fastest and smallest |
| Alternative primary key | UUIDv7 (`base = "common_uuid"`), generated in the app or by Postgres 18's `uuidv7()`. For ids in public URLs and offline clients |
| Custom primary key | `base = "bare"`: no columns added; you define your own single, composite, or no PK |
| Base models | Built-in `common`, `common_uuid`, `pk_only`, `pk_uuid_only`, `bare`, plus your own with `#[morm::base]` |
| Change tracking | None. There is no `save`. Updates name their columns, and never load a row first |
| Rows | Plain structs with public fields and zero per-row overhead |
| Field access control | Views at compile time, groups at runtime. Global groups mean the same everywhere and apply to fields and whole models; model-scoped groups apply to one model's fields. `sudo` is a built-in global group |
| Validation | One mechanism: declarative rules and `clean`, generated inline, run only on write |
| Hooks | Trait with zero-cost default methods. Update and delete hooks receive only what the statement contains |
| JSON | Direct from a row, with no intermediate `Value` or map. Postgres-built JSON also available |
| Production migrations | Forward only by default. A database is treated as production until marked otherwise, and `revert` undoes a migration with a new forward migration |
| Migration source of truth | Models and chain replayed in scratch databases, compared through the Postgres catalog. No snapshots |
| Admin | One tool, `morm-admin`, for everything administrative |
| Rust edition / MSRV | 2024 edition; MSRV 1.99.1, the current stable. Moves up to the latest stable until the first release |
| Decimal | `numeric` maps to `Decimal` where it fits exactly and to `morm::Numeric` otherwise. Values are never silently rounded |
| Names | morm's `__UNQ_`, `__IDX_`, `__FK_` convention, shortened with a hash at Postgres's 63-byte limit |
| License | BSD-3-Clause (same as python-morm) |

---

## 3. Crates and tools

```
rust-morm/
├── crates/
│   ├── morm/            # runtime: pool, queries, hooks, JSON
│   ├── morm-macros/     # #[model], #[view], pg_* macros
│   ├── morm-migrate/    # migration engine
│   └── morm-admin/      # the admin binary
├── examples/
│   └── app/             # Axum + morm example
├── tests/               # integration tests against real Postgres
├── .github/workflows/
└── README.md
```

| Name | Kind | Job |
|---|---|---|
| `morm` | crate | Runtime |
| `morm-macros` | crate | Procedural macros |
| `morm-migrate` | crate | Migration engine, used by `morm-admin` |
| `morm-admin` | installed binary | All admin work |
| `morm-mgr` | small package in each project | Links your models and prints their schema and queries as JSON. Nothing else |

**How it fits together:** `morm-admin` holds the engine. When a command needs the models, it builds and runs `morm-mgr` to get the manifest. Commands that don't need models (`migrate`, `rollback`, `status`, `purge`) run from `morm-migrations/` alone, so a production host needs `morm-admin` and no Rust toolchain. The app binary carries no migration code or schema metadata.

Cargo features of `morm`:

- `tls`: rustls for TCP connections. The connector and its certificates load only when `sslmode` needs them, so with it compiled in and unused, the cost is binary size and build time. Unix sockets never use TLS
- `chrono` (default) / `time`: timestamp types
- `uuid` (default): UUID columns and UUIDv7 generation
- `json` (default): `jsonb` and JSON output
- `decimal` (default): `Decimal` for `numeric` columns, through `rust_decimal` (section 8)
- `tracing`: one span per query with SQL text, duration, row count and pool wait time, plus slow-query logs, compiled out when off. Never parameter values, because they hold passwords, tokens and personal data
- `utoipa`: OpenAPI schemas

### `morm-admin`

```
morm-admin init [path] [--axum]       # create a project
morm-admin doctor                     # check Postgres, roles, scratch, versions
morm-admin import URL [--out DIR]     # generate models from a database
morm-admin scratch                    # check or create the scratch database
morm-admin mgr <command>              # migrations and schema work (section 10)
```

`init` creates the `morm-mgr` package, `morm-migrations/`, `morm.toml`, `.env.example`, a sample model, and a CI workflow. `--axum` adds the app template and a systemd unit.

### Configuration: `morm.toml`

TOML is the Rust standard, with comments, real types and sections, and no indentation traps. Settings need no Rust code.

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

[database.migration]             # owner role, so the app can run with fewer privileges
url = "${MORM_MIGRATION_URL}"

[pool]
min = "auto"                     # 2 x cores of this machine
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
statement_cache = 256            # prepared statements per connection
shutdown_timeout = "30s"         # db.close() waits this long
plan_cache_mode = "auto"         # or force_custom_plan, force_generic_plan
transaction_timeout = "30s"      # longest db.transaction
idle_in_transaction_session_timeout = "1m"
client_connection_check_interval = "5s"
tx_warn_after = "1s"

[pools.replica]                  # a read replica
host = "db-replica.internal"
read_only = true

[pools.jobs]                     # optional named profile
user = "app_jobs"                # any [database] field can be set here
password = "${MORM_JOBS_PASSWORD}"
min = 1
max = 4

[migrations]
dir = "morm-migrations"
scratch_url = "${MORM_SCRATCH_URL}"
```

Precedence: defaults, then `morm.toml`, then environment variables, then code. There is one config file, and a process that needs different values uses a named pool, an environment variable, or code.

**Environment variables:**

| Variable | Overrides |
|---|---|
| `MORM_CONFIG` | Path to the config file (default `morm.toml`) |
| `DATABASE_URL` | Full database URL |
| `MORM_DB_HOST`, `MORM_DB_PORT` | Host or Unix socket directory, and port |
| `MORM_DB_NAME` | Database name |
| `MORM_DB_USER` | User |
| `MORM_DB_PASSWORD` | Password |
| `MORM_DB_PASSWORD_FILE` | File holding the password, for systemd credentials and secret mounts |
| `MORM_DB_SSLMODE` | `disable`, `prefer`, `require`, `verify-ca`, `verify-full` |
| `MORM_DB_SSLROOTCERT` | CA certificate path |
| `MORM_DB_SSLCERT`, `MORM_DB_SSLKEY` | Client certificate and key paths |
| `MORM_DB_TARGET_SESSION_ATTRS`, `MORM_DB_CONNECT_TIMEOUT` | `target_session_attrs`, `connect_timeout` |
| `MORM_DB_APPLICATION_NAME`, `MORM_DB_SEARCH_PATH` | `application_name`, `search_path` |
| `MORM_MIGRATION_URL` | Full owner-role URL for migrations |
| `MORM_MIGRATION_USER`, `MORM_MIGRATION_PASSWORD` | Owner-role login, using the same host and database |
| `MORM_MIGRATION_PASSWORD_FILE` | File holding the owner-role password |
| `MORM_SCRATCH_URL` | Scratch database URL |
| `MORM_MIGRATIONS_DIR` | Migrations directory |
| `MORM_POOL` | Named pool this process opens, instead of `[pool]` |
| `MORM_POOL_MIN`, `MORM_POOL_MAX` | `min`, `max` |
| `MORM_POOL_PROCESSES` | `processes` |
| `MORM_POOL_IDLE_TIMEOUT`, `MORM_POOL_LAST_IDLE_TIMEOUT` | `idle_timeout`, `last_idle_timeout` |
| `MORM_POOL_MAX_QUERIES`, `MORM_POOL_MAX_LIFETIME` | `max_queries`, `max_lifetime` |
| `MORM_POOL_HOLD_WARN_AFTER` | `hold_warn_after` |
| `MORM_POOL_WAIT_TIMEOUT`, `MORM_POOL_MAX_HOLDS` | `wait_timeout`, `max_holds` |
| `MORM_POOL_LOCK_TIMEOUT`, `MORM_POOL_TRANSACTION_TIMEOUT` | `lock_timeout`, `transaction_timeout` |
| `MORM_POOL_IDLE_IN_TRANSACTION_SESSION_TIMEOUT` | `idle_in_transaction_session_timeout` |
| `MORM_POOL_CLIENT_CONNECTION_CHECK_INTERVAL` | `client_connection_check_interval` |
| `MORM_POOL_TX_WARN_AFTER` | `tx_warn_after` |
| `MORM_POOL_STATEMENT_TIMEOUT`, `MORM_POOL_PLAN_CACHE_MODE` | `statement_timeout`, `plan_cache_mode` |
| `MORM_POOL_STATEMENT_CACHE`, `MORM_POOL_SHUTDOWN_TIMEOUT` | `statement_cache`, `shutdown_timeout` |

`MORM_POOL_*` variables apply to the pool this process opens at startup.

**Unix sockets:** the default `host` is `/var/run/postgresql`, the Ubuntu and Debian location. A `host` starting with `/` is a socket directory, and anything else is a hostname using TCP. The socket file is `<host>/.s.PGSQL.<port>`, so `port` still matters. Other systems set `host` to match Postgres's `unix_socket_directories`; Arch and Manjaro usually use `/run/postgresql`. If `host` is a full socket file path, morm takes the directory and port from it. `morm-admin doctor` reports the exact socket path it tries, and warns if `host` no longer matches a live socket.

**Detection:** `morm-admin init` finds the socket and writes it into `morm.toml` and `.env.example`, so most projects never set it by hand:

1. Look for `.s.PGSQL.<port>` files in `/var/run/postgresql`, `/run/postgresql` and `/tmp`, plus any path given by `--socket-dir`. The filename also gives the port.
2. If several servers are found, ask which to use. Without a terminal, take the default directory if it's among them, otherwise exit with code `3` and list the choices.
3. If none is found, keep the default and print a warning, with the TCP settings as a commented-out alternative.
4. Once connected, compare with `SHOW unix_socket_directories` and warn on a mismatch.

**`search_path`** is set on every connection, so another role can't shadow a table or function through a schema earlier in the path. `lint` warns about a `SECURITY DEFINER` function with no `SET search_path` of its own.

**Credentials:** `MORM_DB_*` values override the matching parts of `DATABASE_URL`, so a URL can hold the address while the password comes from `MORM_DB_PASSWORD_FILE`. Setting both `MORM_DB_PASSWORD` and `MORM_DB_PASSWORD_FILE` is an error. `sslmode` values other than `disable` and `prefer` need the `tls` feature, and morm refuses to start without it, so it never silently connects unencrypted. Passwords are never logged, and they're masked in `Debug` output and error messages. `morm-admin doctor` checks that credentials work and warns if `morm.toml` contains a literal password.

**Named pools** are full connection profiles. Any `[database]` field can be set in `[pools.<name>]`, so a job can log in as another role or reach another host. Unset connection fields come from `[database]`, and unset pool settings from `[pool]`, except `min`, which defaults to 1. A separate process can open only its own profile, so job settings live in the same file:

```rust
let db = morm::Db::connect_as("jobs").await?;   // opens only [pools.jobs]
```

`MORM_POOL=jobs` with plain `Db::connect()` does the same without a code change. `db.pool("jobs")` is different: it adds that pool alongside the default one in a process that has both.

**Read replicas** work through a named pool: `db.pool("replica").select::<Order>()...`. You choose which queries go there. A replica can lag behind the primary, so a read straight after a write belongs on `db`. `read_only = true` makes a write on that pool fail with `ReadOnly` at once, sets `default_transaction_read_only`, and exempts it from the `25006` discard rule.

**`morm-admin import`** is a simple starting point for any existing database. It reads the catalog only, never rows, and writes model files that match the schema exactly: types, nullability, defaults, indexes, constraints and their names. Nothing is "improved". `morm-admin mgr baseline` then records that state as migration zero. Views, hooks and validation are added by hand afterwards.

---

## 4. Models

```rust
use morm::{model, Timestamp};

#[model(
    table = "products",
    base = "common",                 // adds id (bigserial pk), created_at, updated_at
    ordering = "name, -price",       // default ORDER BY
    was = "items",                   // rename hint: table used to be called "items"
    hooks,                           // this model implements Hooks itself
    extensions = "pg_trgm",
    indexes(
        index(cols = "vendor_id, sku", unique),
        index(cols = "name", using = "gin", ops = "gin_trgm_ops"),
        index(cols = "vendor_id", filter = "deleted_at IS NULL"),
    )
)]
pub struct Product {
    #[col(fk = Vendor::id, on_delete = "cascade", index)]
    pub vendor_id: i64,

    #[col(sql = "varchar(64)", len = "1..=64")]
    pub sku: String,

    #[col(sql = "varchar(255)", was = "title", clean = "trim_name")]
    pub name: String,

    #[col(sql = "numeric(12,2)", default = "0", min = 0)]
    pub price: rust_decimal::Decimal,

    #[col(default = "0", min = 0)]
    pub stock: i32,

    #[col(default = "true")]
    pub active: bool,

    pub attrs: Option<serde_json::Value>,          // jsonb, nullable

    #[col(expr = "price * stock", sql = "numeric")]  // computed in SELECT, not stored
    pub stock_value: rust_decimal::Decimal,

    pub deleted_at: Option<Timestamp>,
}
```

### Table and column names

Generated SQL always quotes every table and column name, so reserved words like `order` and `type` work as they are.

**Names are lowercase only.** Postgres folds unquoted names to lowercase but keeps quoted ones exactly, so `userID` in hand-written SQL means `userid`, while `"userID"` is a different column. With lowercase names, quoted and unquoted always match, and the trap can't happen.

- An uppercase letter in `table` or `name` is a compile error. The defaults are already lowercase snake_case.
- morm can't treat everything as lowercase, because `"userID"` and `userid` are separate names in Postgres, and folding one would point at a column that doesn't exist.
- `keep_case` is the exception for existing tables, as in `#[col(name = "userID", keep_case)]`. `import` adds it automatically. Hand-written SQL must then quote the name, and `check-sql` catches the mistake in registered queries.
- Generated constraint and index names keep their `__UNQ_` style, because only morm uses them.
- A table or column name over 63 bytes is a compile error, because Postgres would silently cut it.

### Bases

Defaults are the best choice for new tables. Anything else is declared explicitly, so models can match any existing table.

| Base | Adds |
|---|---|
| `base = "common"` | `id bigserial` PK, `created_at`, `updated_at` |
| `base = "common_uuid"` | `id uuid` PK (UUIDv7), `created_at`, `updated_at` |
| `base = "pk_only"` | `id bigserial` PK only |
| `base = "pk_uuid_only"` | `id uuid` PK (UUIDv7) only |
| `base = "bare"` (or omitted) | Nothing. You define every column and your own PK: single, composite, or none |
| `base = MyBase` | Your own base, below |

PK-based helpers come through a `HasPk` trait, so a model without a PK simply doesn't have them: a compile error, not a runtime failure. To match an existing table, write for example `#[col(pk, sql = "serial")] id: i32`, with explicit index and constraint names.

**Custom bases** are ordinary structs:

```rust
#[morm::base(base = "common_uuid")]             // a base can extend another base
pub struct Tenanted {
    #[col(fk = Tenant::id, index)]
    pub tenant_id: Uuid,
    #[col(groups = "staff")]
    pub created_by: Option<Uuid>,
}

#[model(table = "invoices", base = Tenanted)]
pub struct Invoice { pub total: Decimal }       // invoice.tenant_id is a plain field
```

- The fields become real fields of the model, with every `#[col]` option, so `invoice.id` works and there is no nested struct or runtime cost.
- A macro can't read another struct's fields, so `#[morm::base]` generates a `macro_rules!` macro holding its fields, and `#[model]` calls it to get them. Bases work across crates, and they carry indexes and `ordering`.
- A field defined in both the base and the model is a compile error.
- The built-in bases use this same mechanism.
- `flatten` remains for a nested reusable struct, as in `order.shipping.city`.

### Struct options (`#[model(...)]`)

| Option | Meaning |
|---|---|
| `table` | table name (default: snake_case of the struct name); lowercase only unless `keep_case` is set |
| `keep_case` | allow uppercase in an existing table's name |
| `schema` | Postgres schema, as in `schema = "billing"`. Generated SQL then uses `"billing"."invoices"`, and migrations create the schema. Omitted, the table resolves through the pinned `search_path` |
| `base` | `"common"`, `"common_uuid"`, `"pk_only"`, `"pk_uuid_only"`, `"bare"`, or a `#[morm::base]` type |
| `ordering` | default sort, `-` prefix for descending |
| `was` | previous table name (rename hint for migrations) |
| `indexes(...)` | list of indexes (section 7) |
| `extensions` | Postgres extensions the model needs |
| `hooks` | the model has its own `impl Hooks`; `hooks(tx)` also runs its statements in a transaction |
| `groups`, `sudo` | global groups for the whole model (section 5) |
| `scoped_groups` | declares groups that exist only on this model (section 5) |
| `no_default` | don't generate `Default` |
| `checks(...)` | table-level CHECK constraints, tracked by migrations: `checks(name = "ends_after_starts", sql = "ends_at > starts_at")` |
| `storage` | table storage settings, tracked by migrations: `storage = "fillfactor=90"` |

### Field options (`#[col(...)]`)

| Option | Meaning |
|---|---|
| `sql` | SQL type; inferred from the Rust type when omitted |
| `default` | SQL default expression |
| `default_fn` | Rust function that supplies a default on insert |
| `pk` | primary key (with `base = "bare"`; several fields make a composite PK) |
| `unique`, `index`, `index(using = ..., ...)` | single-column shortcuts; `index = "hash,btree"` makes two. Rules in section 7 |
| `not_null` | `Option<T>` field that is still NOT NULL in the DB (filled by a default) |
| `fk = Type::field`, `on_delete`, `on_update` | typed foreign key, checked at compile time; actions: `cascade`, `restrict`, `set null`, `set default`, `no action` |
| `expr` | read-only computed column; excluded from writes and migrations |
| `flatten` | embed a reusable column set |
| `generated`, `identity`, `check` | generated column, identity column, check constraint |
| `version` | optimistic locking; a stale update fails with `Conflict` |
| `deprecated` | removes the field from reads and writes while the column remains (rolling deploys) |
| `groups`, `sudo` | field access groups (section 5) |
| `min`, `max`, `len`, `choices`, `clean`, `db_check` | validation (section 6) |
| `skip_json_if` | omit a value from JSON output when a function returns true |
| `json_string` | write an integer as a JSON string (section 8) |
| `help` | description, used in schemas |
| `auto_now` | set to `now()` on every update |
| `name` | column name override, for example `type_` with `name = "type"`; lowercase only unless `keep_case` is set |
| `keep_case` | allow uppercase in an existing column's name |
| `was` | previous column name (rename hint) |

### Type inference

`i16` → smallint, `i32` → integer, `i64` → bigint, `f32` → real, `f64` → double precision, `bool` → boolean, `String` → text, `Vec<u8>` → bytea, `DateTime<Utc>` / `Timestamp` → timestamptz, `NaiveDateTime` → timestamp, `NaiveDate` → date, `NaiveTime` → time, `Uuid` → uuid, `serde_json::Value` → jsonb, `Vec<T>` → `T[]`. `Option<T>` → nullable. `#[derive(TextEnum)]` maps a Rust enum to `text` with a CHECK constraint, the recommended form, because adding or removing a value is an ordinary migration. `#[derive(PgEnum)]` maps to a Postgres enum, mainly to match existing tables, because removing a value from one is very hard. Domains and composite types are also supported. Anything else requires an explicit `sql`, including `Decimal` and `Numeric`, which need their precision stated (section 8).

- **Unsigned integers:** Postgres has none, so `u8` → smallint, `u16` → integer and `u32` → bigint, each with a generated range CHECK. `u64` doesn't fit `bigint`, so it's a compile error: use `i64`, or `Numeric` with an explicit `sql`.
- **Timestamps:** Postgres stores microseconds and chrono nanoseconds, so a value can compare unequal after a round trip. `Timestamp::now()` truncates to microseconds, and `insert` fills database-set timestamps in place. Compare stored values, not values read from the clock.
- **NUL bytes:** Postgres rejects `\0` in text. morm doesn't scan every string for it, which would cost time on every write; Postgres's error becomes a typed `InvalidText`.

### Plain structs, no `save`

There is no `save`. It would force either change tracking or full-row writes, and both are costs nobody asked for. Models are plain structs with public fields, and rows carry **zero** extra bytes.

```rust
let mut p = Product { vendor_id: 1, sku: "A-1".into(), name: "Lamp".into(), ..Default::default() };
db.insert(&mut p).await?;        // fills id and timestamps in place
```

- `p.price = dec!(34)` is allowed, because it only changes your local copy. Nothing reaches the database without an explicit `update`.
- **Forgotten required fields:** `..Default::default()` hides them, so `no_default` removes `Default` when you want the compiler to insist. `Product::new(...)` takes required fields as arguments.

### Generated per model

- compile-time SQL constants (`SELECT`, `GET`, `INSERT`, `DELETE`, `RETURNING`, `ORDER BY`)
- positional decoding (`from_row`), name-based decoding for raw queries (`from_row_named`), and a borrowed `ProductRef<'r>`
- `Product::F.<field>`: typed, quoted column references, checked at compile time; `Product::F.alias("p")` for JOINs, with prefix decoding for two models in one row
- `Default` (unless `no_default`), `Product::new(...)`, `Serialize`, `Deserialize`, `json_schema()`
- `Product::G.<group>` masks, when the model has scoped groups
- an empty `impl Hooks` (unless `hooks` is set)
- schema metadata, compiled only into `morm-mgr`, with no model list to maintain

Compile-time checks: unknown columns in `ordering`, `indexes` and `include`; foreign keys whose types don't match; generic structs. Postgres checks `filter` and `expr` on scratch, because a macro can't parse SQL. Group limits are in section 5.

---

## 5. Views and groups

### Views: restricted models

A view is a restricted model over the same table. Views are declared on the full definition, because an attribute macro can't read another struct's fields.

```rust
#[model(table = "users", base = "common")]
#[view(pub User,
       read_hide  = "password, expires, is_super_user",
       write_hide = "is_super_user, expires")]
#[view(pub UserProfile, groups = "profile")]
pub(in crate::auth) struct UserAdmin { /* every column */ }
```

- Each view is a real generated plain struct with its own columns, `F`, `SELECT` constant and decoder.
- **Hidden means absent.** `user.password` and `User::F.password` are compile errors, and JSON output can't include the field.
- **Write-only works:** `read_hide` without `write_hide` generates a `WriteOnly<T>` field, which can be set but has no reader, `Debug` or `Serialize`.
- **Read-only works:** `write_hide` fields can't be used in `.set()` (compile error) and are ignored on JSON input.
- **Privilege follows Rust visibility.** With `UserAdmin` private to `crate::auth`, the compiler proves no other module touches passwords.
- **Narrowing is one-way.** `From<UserAdmin> for User` exists; nothing goes the other way.
- **Raw SQL stays safe.** Named decoding ignores columns a view lacks, and `check-sql` flags queries selecting hidden columns.
- **Inserts:** a view that hides a required column with no default can't be inserted.
- Only the definition is migrated. `clean` and declarative rules apply to every view, and `impl Hooks` is per type.

### Groups: granular field access

There are two kinds of group:

| Kind | Declared | Applies to | Mask |
|---|---|---|---|
| **Global** | Once, with `morm::groups!` | Models and fields, everywhere. The same name is the same group on every model | `G.admin` |
| **Model-scoped** | On one model, with `scoped_groups` | That model's fields only | `User::G.billing` |

```rust
// src/groups.rs: one declaration for the whole app
morm::groups!(admin, staff, moderator);         // sudo is always included

// src/users/mod.rs
use crate::groups::G;

#[model(table = "users", base = "common", scoped_groups = "billing, support")]
pub struct UserAdmin {
    #[col(groups = "admin,moderator")]  pub subscriptions: Vec<i32>,   // global
    #[col(groups = "billing")]          pub card_token: String,        // scoped to this model
    #[col(groups = "staff,support")]    pub notes: String,             // one of each
    #[col(sudo)]                        pub password: String,          // sugar for groups = "sudo"
}

#[model(table = "audit_log", base = "common", groups = "staff")]       // global, whole model
pub struct AuditLog { /* ... */ }
```

`sudo` is an ordinary global group with a reserved name, always present, so it works with no declaration.

**Declaring global groups:** call `morm::groups!` once per app, in any module. It's plain Rust, not `morm.toml`, so names are checked by the compiler. It generates a constant `G`, with one field per group numbered in the order listed, and `sudo` as bit 0. Models find `G` through a normal `use`. If models span several crates, put the call in a crate they all depend on.

- **Resolving names:** a name listed in the model's `scoped_groups` is scoped. Any other name is global and resolves to `G`, so a typo is a compile error. A scoped name hiding a global one is a compile error too.
- **Access:** a field with no groups is open to everyone. A grouped field needs the caller to be in at least one of its groups.
- **Whole model:** `#[model(groups = "staff")]` puts every field without its own `groups` into that group. `#[model(sudo)]` does the same with `sudo`. Only global groups work here, and a field's own `groups` takes priority.
- **Compile time:** `#[view(pub Moderator, groups = "moderator")]` builds a view from ungrouped fields plus that group. Views take both kinds.
- **Enforcement at runtime** is on write and on output. An `update` setting fields outside the caller's groups fails, and `to_json_for(mask)` omits them.
- `Model::groups()` lists the model's groups, and `Model::group_fields("moderator")` lists a group's fields.

### Masks

```rust
let db = db.groups(G.admin | G.staff);                  // works on every model
let db = db.sudo();                                     // same as db.groups(G.sudo)
let db = db.groups(G.staff | User::G.billing);          // works on User only
let mask = G::from_names(&claims.roles)?;               // roles from a token or a table
```

- **Structure:** bits live in an opaque `Bits<N>`, a plain `[u64; N]` array. A group number `n` is bit `n % 64` of word `n / 64`, and a check ANDs the arrays word by word.
- **Size is counted by the macros, with nothing to configure.** `morm::groups!` counts the global groups and `#[model]` counts its scoped groups, then each sets `N` to the fewest words that fit. Up to 64 groups use `[u64; 1]`, 65 to 128 use `[u64; 2]`, and so on. Global and scoped sizes are separate, and a model with no scoped groups stores no scoped bits.
- **Generic code:** stable Rust can't write `[u64; Self::N]` in generic code, so each model names its mask as an associated type (`type Scoped = Bits<2>`), and `db.groups()` is generic over the mask.
- **Limits:** none fixed, for groups or fields.
- **Cost:** a mask is held by value in the handle with no allocation, 8 bytes per word. Each grouped field has an entry of the same size in a static table, and rows carry no extra bytes. Each extra word adds a fraction of a nanosecond to a check, and the milestone 2 benchmark gives real figures. Group code is monomorphized per mask type, which slightly lengthens compile times.
- **Only grouped fields are checked.** A field without groups generates no check. `.set()` checks once per call, and `to_json_for` works out its field list once per mask, so output cost doesn't grow with the number of rows.
- **Scoped bits are per model,** so a mask containing them is typed `Mask<User>`, and using it on another model is a compile error. A caller with scoped groups on two models uses two handles. A mask of only global groups works everywhere.
- **Bit numbers aren't stable.** They change when the list changes, so store group names, never mask values, and convert with `from_names`. An unknown name is an error.

---

## 6. Validation

One mechanism, instead of python-morm's overlapping `validator`, `modifier` and `_clean_*`. The compiler replaces type checks, native decoding replaces JSON modifiers, and the rest is declarative rules or `clean`.

```rust
#[col(min = 1, max = 100)]            pub qty: i32,      // declarative, generated inline
#[col(len = "3..=65")]                pub username: String,
#[col(choices = "admin,user,system")] pub kind: String,
#[col(clean = "hash_password")]       pub password: String,  // fn(T) -> Result<T>
```

- `Hooks::validate(&self)` handles rules across fields on insert, and `Hooks::validate_changes(&Changes<Self>)` on update.
- A rule that needs the whole row on update must be a `CHECK` constraint, or its hook fetches the row visibly.
- **No function pointers per field.** Everything is static and inlined, and a field without rules generates no code.
- **Runs only on write,** and on updates only for values passed to `.set()`. Reads are never validated.
- **Runs once,** in order: pre hooks, then `clean` and declarative rules, then `validate`, then SQL.
- `db_check` also emits a declarative rule as a real `CHECK` constraint.

---

## 7. Indexes

Declared in `indexes(...)` on the struct, or with field shortcuts.

| Option | Meaning | Default |
|---|---|---|
| `cols` | columns, each with optional `ASC`/`DESC`, `NULLS FIRST`/`NULLS LAST` | required unless `expr` |
| `expr` | expression index, e.g. `lower(email)` | none |
| `unique` | unique index | off |
| `nulls_not_distinct` | with `unique`, treat NULLs as equal, so only one NULL is allowed | off |
| `using` | `btree`, `hash`, `gin`, `gist`, `brin`, `spgist` | `btree` |
| `ops` | operator class, e.g. `jsonb_path_ops`, `gin_trgm_ops` | none |
| `filter` | `WHERE` clause (partial index) | none |
| `include` | extra non-key columns (covering index) | none |
| `name` | explicit name | generated |

```rust
indexes(
    index(cols = "category_id, created_at"),
    index(cols = "price DESC NULLS LAST, id"),
    index(cols = "sku", using = "hash"),
    index(cols = "attrs", using = "gin", ops = "jsonb_path_ops"),
    index(cols = "created_at", using = "brin"),
    index(cols = "email", unique, filter = "deleted_at IS NULL"),
    index(expr = "lower(email)", unique),
    index(cols = "vendor_id", include = "price, stock"),
    index(name = "products_search_idx", cols = "name, sku"),
)
```

Rules:

- **`unique` gives both a constraint and an index.** Postgres backs a UNIQUE constraint with a unique index of the same name, so morm creates one object that serves as both. This applies to `#[col(unique)]` and to `index(cols = "a, b", unique)`, and `include` is kept on the constraint. A unique index with `filter` or `expr` stays an index only, because constraints can't be partial or use expressions.
- **Duplicates are compile errors.** The macro sees every declaration, so it rejects a second btree index with the same columns in the same order, as `#[col(unique, index)]`, `#[col(unique)]` plus `index(cols = "email")`, a repeated `indexes(...)` entry, or an index matching the primary key. Inherited base fields are included. Indexes differing in `using`, `ops`, `filter`, `expr` or `include` are allowed, and `lint` reports the ones that look redundant.
- `morm-admin mgr lint` warns about foreign key columns without an index, and redundant indexes such as a hash and a btree on one column, or an index on `a` when one on `a, b` exists.
- On existing tables, migrations build indexes concurrently by default (section 10).

### Names

The convention is python-morm's, so models can match its databases without explicit names:

| Object | Name |
|---|---|
| Unique on one column | `__UNQ_{table}_{column}__` |
| Unique on several columns | `__UNQ_{table}_{name}__` |
| Index on one column | `__IDX_{table}_{column}_{method}__` |
| Index on several columns | `__IDX_{table}_{name}_{method}__` |
| Foreign key | `__FK_{table}_{column}__` |
| Check | `__CHK_{table}_{name}__` (new; python-morm has no convention for it) |

`{name}` is the `name` you give, or the column names joined by `_`. A primary key keeps the Postgres name `{table}_pkey`.

- **63-byte limit:** Postgres silently cuts longer names, so two long names could collide. morm counts bytes, not characters. A longer name is cut and ends in 8 hex characters of its full SHA-256, as `__IDX_{start of name}_{hash}__`, so it stays at 63 bytes, is stable across builds and still shows the table.
- A generated name used twice in one model, for example two partial indexes on one column, is a compile error asking for an explicit `name`.
- An explicit `name` is used as written. Over 63 bytes is a compile error.
- `morm-admin import` keeps existing names, as explicit `name` values where they differ.

---

## 8. Reading and writing

In the examples, `F` is short for the relevant model's `F`, such as `Product::F`.

### Connecting

```rust
let db = morm::Db::connect().await?;                       // reads morm.toml: [database], [pool]
let db = morm::Db::connect_as("jobs").await?;              // reads morm.toml: [pools.jobs]
let db = morm::Db::connect_to(url).min(10).max(90).await?; // or set values in code
```

`connect` honours `MORM_POOL`, so a process can switch profile without a code change. `connect_as` names the profile in code and fails if it doesn't exist. Both return a builder, so `.min()` and `.max()` work there too.

### Get

```rust
db.get::<User>(5).await?;                                  // by PK, Option<User>
db.get_by::<User>(F.email, &email).await?;                 // any column
db.get_where::<Product>(F.price, "BETWEEN $1 AND $2", &[&5, &2000]).await?;   // any comparison
db.get_all_by::<Order>(F.user_id, &7).limit(50).await?;    // many rows

// several columns: one builder shared by get, update, delete, count, exists
db.get_q::<Hist>().by(F.owner_id, &1).by(F.app, &"chat").one().await?;
```

- `tx.get_for_update::<T>(id)` locks the row while reading it, for read-then-write logic. It exists only on `tx`.
- `comp` is `&'static str`, so it can't come from a request.
- The statement is cached on first use, keyed by model, column and the `comp` pointer, so repeats allocate nothing.
- Composite PKs take tuples.

### Insert and update

```rust
db.insert(&mut p).await?;                                  // INSERT ... RETURNING, fills p in place
db.insert_many(&rows).await?;                              // UNNEST, COPY above a threshold
db.upsert(&mut p, F.sku).await?;                           // ON CONFLICT

db.update::<Product>(42).set(F.price, dec!(34)).run().await?;       // no fetch needed
db.update_q::<Product>().by(F.vendor_id, &7).set(F.active, false).run().await?;
db.update_each::<Product>(&rows, &[F.price]).await?;                // per-row values, one statement
db.update::<Product>(42).set_expr(F.stock, "stock - $1", &n).run().await?;  // atomic math in SQL
```

- An update never loads a row, and it writes exactly the columns named. Only `.returning()` brings rows back.
- **`update` and `delete` by primary key return `NotFound` when no row matched,** so a mistyped id can't pass as success. `.allow_missing()` returns a count, for deletes that may have already happened. `update_q`, `update_each` and the other filtered and bulk forms return a count, because zero rows is a normal result for a filter.
- `updated_at` and `auto_now` columns are set to `now()` in SQL.
- **Lost updates:** under Read Committed, two requests can read `stock = 10` and both write `9`. Use `set_expr` for arithmetic, `#[col(version)]` for optimistic locking (`Conflict` on a stale write), or `tx.get_for_update` when logic must see the current row.
- **`now()` is the transaction's start time,** so every row in a long transaction gets the same `updated_at`. Use `clock_timestamp()` in `set_expr` when the actual time matters.
- **Insert writes one static statement per model.** `Option` fields that are `None` and have a default use it, through `COALESCE($n::type, <default>)`.
- `upsert` holds a whole row, so it runs insert hooks. Conflict targets can be one column or a tuple, as in `(F.vendor_id, F.sku)`.

### Delete

```rust
db.delete::<Product>(42).await?;                           // one row
db.delete_ids::<Product>(&[1, 2, 3]).await?;               // many ids, = ANY($1)
db.delete_by::<Product>(F.vendor_id, &7).await?;           // any column
db.delete_where::<Log>(F.created_at, "< $1", &[&cutoff]).await?;
db.delete_q::<Hist>().by(F.owner_id, &1).by(F.app, &"chat").any(F.key, &keys).run().await?;
```

Filtered and bulk deletes return a count, and `.returning()` gives the rows back. A delete with no filter doesn't compile; `db.delete_all::<T>()` is the explicit form. The same rule applies to `db.update_all::<T>()`.

**Batches:** one statement over millions of rows holds locks for a long time, delays replicas and causes bloat. `.in_batches(n)` on `update_q`, `delete_q`, `delete_where`, `update_all` and `delete_all` works through the rows by primary key:

```rust
db.delete_where::<Log>(F.created_at, "< $1", &[&cutoff]).in_batches(5000).pause_ms(50).await?;
```

Each batch is its own statement and commits on its own, so the whole operation isn't atomic, it can stop partway, and running it again continues. It needs a primary key, and it exists only on `db`, not `tx`. Hooks run once per batch. It returns the total count.

### Lists

```rust
let rows = db.select::<Product>()
    .filter()
    .qc(F.vendor_id, "=", &vendor_id)
    .q(" AND ").qc(F.price, "<", &max_price)
    .order()                         // model's default ordering
    .limit(50)
    .fetch_ref().await?;

// only some columns; the row type comes from the columns
let rows: Vec<(i64, String)> = db.select::<Product>()
    .filter_cols((F.id, F.name))
    .qc(F.vendor_id, "=", &vendor_id)
    .limit(50)
    .fetch().await?;

let products: Vec<Product> = db.raw::<Product>("SELECT * FROM products WHERE attrs ? $1", &[&"color"])
    .limit(50).fetch().await?;
```

- `.filter()` selects every down column: all the columns this model or view may read, without hidden, write-only or `deprecated` ones. It's the `.filter_cols(...)` default, since Rust has no default arguments.
- `.filter_cols((F.a, F.b))` selects only those. `fetch()` returns tuples of the columns' Rust types and `fetch_ref()` returns borrowed tuples, so a wrong type is a compile error, and a partial row can't pass for a full `Product`. `fetch_json`, `stream_json` and `fetch_raw` work the same way.
- Columns come from `F`, so a view can't select what it hides. On a `db.groups(mask)` handle, a column outside the mask is an error.
- The column list is cached like other statements, so repeats allocate nothing.

- **Sort columns from a request** can't be parameters, and `.q()` won't accept them. `.order_by_allowed(&input, &[F.name, F.age])` takes `"name"` or `"-age"`, and returns `InvalidOrder` for anything not on the list. `morm::quote_ident()` is only for names known at runtime in `q_unchecked` and `sql_unchecked`: it wraps a name in double quotes and doubles any quote inside it, so it can't break out. `F` columns are already quoted.
- `.custom_plan()` plans a query for its actual values, for skewed data where one customer has a million orders and most have ten. It runs through `query_typed`, so it needs no `SET` and no extra round trip. The `plan_cache_mode` setting does the same for a whole pool.

A list needs `.limit(n)`, `.all()` or `stream()`. Lists return one of:

| Call | You get | Conversions | Use when |
|---|---|---|---|
| `fetch_json()` | JSON bytes built by Postgres | None in the app | The data goes straight to a client |
| `stream_json()` | The same, chunked | None in the app | Large responses, flat memory |
| `fetch_raw()` | Rows as Postgres sent them; columns read on demand | Only the columns you read | You need a few columns from each row |
| `fetch_ref()` | Borrowed `ProductRef<'r>` | Strings and `jsonb` borrowed from the row buffer, no per-field allocation | Reading and computing without keeping the data |
| `fetch()` | Owned `Product` | Row → struct | You keep or pass the data along |
| `stream()` | One row at a time | Row by row | Huge results |

`fetch_one`, `fetch_opt`, `fetch_val`, `exists` and `count` cover the single-result cases.

### Pagination

`OFFSET 100000` reads and discards 100,000 rows on every page, so morm has no `.offset()` helper. Keyset pagination is equally fast at any depth:

```rust
let page = db.select::<Order>().filter()
    .qc(F.user_id, "=", &user_id)
    .keyset((F.created_at.desc(), F.id.desc()))
    .after(cursor.as_deref())                    // None for the first page
    .limit(50)
    .fetch_page().await?;

page.rows;                                       // Vec<Order>
page.next;                                       // opaque cursor, None on the last page
```

- The last key must be unique, so rows can't be skipped, and without a unique column it's a compile error. Mixed ascending and descending columns are supported.
- A cursor is base64 of the key values. They're decoded as typed parameters, so a tampered cursor returns `InvalidCursor` or a different page, never SQL. Cursors aren't a secret.
- Writes between pages can't shift rows, as they do with `OFFSET`.

### Raw SQL, with no model

`db.sql()` runs any statement. No model, hooks, validation or ordering are involved.

```rust
#[derive(morm::Row)]
struct Sale { day: NaiveDate, total: Decimal }

let q = sql!("SELECT day, sum(total) AS total FROM orders WHERE vendor_id = $1 GROUP BY day");

let rows: Vec<Sale> = db.sql(q, &[&vendor_id]).limit(30).fetch().await?;     // named struct
let rows: Vec<(NaiveDate, Decimal)> = db.sql(q, &[&vendor_id]).limit(30).fetch().await?;  // tuple
let body = db.sql(q, &[&vendor_id]).limit(30).fetch_json().await?;           // JSON bytes
let rows = db.sql(q, &[&vendor_id]).limit(30).fetch_raw().await?;            // rows as sent
let n: i64 = db.sql("SELECT count(*) FROM orders", &[]).fetch_val().await?;  // one value
let changed = db.sql("UPDATE orders SET seen = true WHERE id = $1", &[&id]).run().await?;  // row count
```

- Every list and single-result call works here, including `fetch_ref`, `stream` and `stream_json`.
- `#[derive(morm::Row)]` decodes by column name, for any plain struct. Columns the struct lacks are ignored, and a missing column is an error.
- The SQL must be `&'static str`, as with `.q()`, and `sql_unchecked(String)` is the visible escape hatch. `sql!` also registers it for `check-sql`.
- **The list rule still applies.** `.limit(n)` appends `LIMIT n` to your statement, and `.all()` runs it as written. `run()`, `fetch_val()`, `fetch_one()` and `fetch_opt()` don't need either.
- It uses the same bounded statement cache, and `db.raw::<T>()` remains for raw SQL decoding into a model or view.

### Decimal

Postgres `numeric` goes up to 131,072 digits before the point, and also holds `NaN` and `Infinity`. No fixed-size Rust type covers that. `rust_decimal::Decimal` is 16 bytes, needs no heap and is `Copy`, but holds only 28 digits or so. Two types cover both cases:

| Rust type | Holds | Use for |
|---|---|---|
| `Decimal` (`rust_decimal`) | Up to 28 significant digits | Money, quantities, rates |
| `morm::Numeric` | Every `numeric` value, `NaN` and `Infinity` included, with no arithmetic of its own | Larger values, and columns of unknown range |

```rust
#[col(sql = "numeric(12,2)")] pub price: Decimal,   // fits, so checked at compile time
#[col(sql = "numeric(40,2)")] pub total: Numeric,   // Decimal here is a compile error
```

- **Never silently rounded on read.** `rust_decimal`'s own decoding rounds fractional digits it can't hold, and it allocates per value. morm decodes `Decimal` itself, with no heap use. A value that doesn't fit exactly returns `Error::Decimal`, as do `NaN` and `Infinity`.
- **Never silently rounded on write.** Postgres rounds to a column's scale without complaint, so `1.005` into `numeric(12,2)` would change. With the scale known from `sql`, a value with too many decimal places returns a validation error. Use `clean` when rounding is what you want.
- **Types are checked at compile time.** `Decimal` on `numeric(p,s)` with `p` or `s` above 28 is a compile error. `f32` or `f64` on `numeric` is a compile error, because floats can't hold decimal fractions exactly.
- **Unbounded `numeric`** with `Decimal` compiles, because existing tables have it, but `lint` warns, and a value that doesn't fit fails at read time. `import` writes `Decimal` for bounded columns that fit, and `Decimal` with an explicit `sql = "numeric"` for unbounded ones.
- **JSON:** always a string, such as `"34.50"`, because a JSON number would lose digits in JavaScript clients. `Numeric` follows the same rule.
- **Speed:** `numeric` decoding is slower than integer decoding. When a column holds a fixed number of decimal places and speed matters most, `bigint` minor units (cents) are the fastest choice. The README says so.

### Query safety

- `sql!` builds a `&'static str` from `F` constants and registers it. `morm-admin mgr check-sql` runs `PREPARE` on each against scratch and compares types. That gives checked raw SQL with no database at build time.
- `.q()` accepts only `&'static str`, so `format!`-ed input won't compile. `q_unchecked(String)` is the visible escape hatch.
- Typed errors: `Unique { constraint, field }`, `ForeignKey`, `NotNull`, `Check`, `Serialization`, `Deadlock`, `LockTimeout`, `TransactionTimeout`, `TransactionAborted`, `CommitUnknown`, `NestedConnection`, `NotFound`, `Conflict`, `PoolBusy`, `ConnectionLost`, `ReadOnly`, `InvalidOrder`, `InvalidCursor`, `Decimal`.

### Transactions

```rust
db.transaction(|tx| async move {
    tx.insert(&mut p).await?;
    Ok(())
}).await?;

db.transaction(async |tx| {                       // runs again on 40001 or 40P01
    let mut order = Order { user_id, ..Default::default() };   // built inside
    tx.insert(&mut order).await?;
    Ok(())
}).isolation(Serializable).retry(3).await?;
```

Same API as `db`, with savepoints and per-transaction settings (`statement_timeout`, `lock_timeout`, `transaction_timeout`, `SET LOCAL` for tenant RLS).

**A commit is confirmed, never assumed.** After a failed statement, Postgres turns `COMMIT` into `ROLLBACK` with no error, and tokio-postgres's `commit()` returns `Ok` either way, so a swallowed error would look like a saved transaction. Two checks prevent that:

- Any Postgres error marks the transaction as failed. Further use returns that error, and a closure returning `Ok` still gets it.
- `COMMIT` is sent as `SELECT 1; COMMIT` in one message. In a failed transaction the `SELECT` errors and Postgres skips the `COMMIT`, so this also catches errors morm didn't see, with no extra round trip. morm then rolls back and returns `TransactionAborted`.
- `after_commit` fires only after a confirmed commit. If the connection is lost during `COMMIT`, the outcome is unknown, so the result is `CommitUnknown`, with no retry and no `after_commit`.

**Savepoints** clear the failed mark, except after a deadlock or serialization error, which always fails the whole transaction. Rolling back to a savepoint keeps earlier locks, so a deadlock could repeat, and a serialization failure can't succeed on the same snapshot.

**`.retry(n)`** is available only for a closure that can run again (`AsyncFn`), so a closure that changes a captured row doesn't compile. Rows are built inside the closure, so each attempt starts fresh and `pre_insert` can't run twice on one row, for example hashing a password twice. Each attempt starts with no queued `after_commit` events. Between attempts the transaction rolls back and the connection is released during the jittered backoff.

### Deadlocks

There are two kinds. In **Postgres**, two transactions lock the same rows in opposite order, and Postgres kills one with `40P01`, by default after `deadlock_timeout` (1s). In the **pool**, tasks holding a connection wait for another one while every connection is taken, and Postgres can't see that.

**Pool deadlocks:**

- **A second connection on the same task is refused.** Using `db` inside a transaction closure, or inside a `stream()` loop, returns `NestedConnection` immediately. This can't be a compile error, because closures can capture `db`, so it's a task-local flag costing a few nanoseconds per query.
- `db.extra_conn()` allows deliberate cases, such as an audit row that must survive a rollback.
- **Pool waits are always finite.** `wait_timeout` (default 5s) can't be turned off, and it ends every case the flag can't see: `tokio::spawn`, `join!`, and a task holding an app `Mutex` while waiting for a connection when another holds a connection and waits for that `Mutex`.
- **Holds can't use up the pool.** `max_holds` defaults to a quarter of `max`, at least 1. `db.hold()` past it returns `PoolBusy`.
- `after_commit` runs after the connection is released, so it can't cause this.

**Postgres deadlocks:**

```rust
db.transaction(async |tx| {
    tx.lock::<Account>(&[from, to]).await?;             // same order every time
    tx.update::<Account>(from).set_expr(F.balance, "balance - $1", &amount).run().await?;
    tx.update::<Account>(to).set_expr(F.balance, "balance + $1", &amount).run().await?;
    Ok(())
}).retry(3).await?;

db.update_q::<Job>().any(F.id, &ids).set(F.done, true).lock_ordered().run().await?;
tx.select::<Job>().filter().qc(F.state, "=", &"queued").limit(10).skip_locked().fetch().await?;
tx.advisory_lock(&["billing", "reports"]).await?;
```

- **Bulk writes are sorted.** `insert_many`, `upsert` batches, `update_each` and `delete_ids` sort input by key, skipping sorted input. Inserts lock in that order. For updates and deletes it usually holds, but the planner chooses the order, so a hash join can differ. `.lock_ordered()` is the guarantee.
- **`.lock_ordered()`** on `update_q`, `delete_q`, `update_each` and `delete_ids` locks rows in key order in the same statement, with a CTE and no extra round trip. It adds query work, so it's opt-in, and the benchmark shows how much.
- **`tx.lock::<T>(&ids)`** runs `SELECT ... WHERE pk = ANY($1) ORDER BY pk FOR NO KEY UPDATE`. `NO KEY` means child inserts aren't blocked by foreign key checks.
- **One byte order everywhere.** App sorting and `ORDER BY` must agree, so text keys use `COLLATE "C"`, which matches Rust's byte order. Numbers and UUIDs already agree.
- **Foreign keys:** a child insert takes `FOR KEY SHARE` on the parent. A normal update of the parent doesn't conflict with that. Deleting the parent, or changing a key column it's referenced by, does, and that can deadlock, so those paths need `tx.lock` first.
- **`tx.advisory_lock(&keys)`** serializes work across tables when row order isn't practical, and it releases at the end of the transaction. Keys are the first 8 bytes of the SHA-256 of each string, a documented algorithm, because services must compute the same key across builds. Keys are sorted before locking.
- **`.skip_locked()`** on a list adds `FOR UPDATE SKIP LOCKED`, so queue workers skip claimed rows. It exists only on `tx`.

**Automatic retry:** a single statement outside a transaction retries once when Postgres reports `40P01` or `40001`, or when it failed before being sent. That's the complete list: a lost connection after sending has an unknown outcome, and a stream that has delivered rows never retries. Retry covers only the SQL, so hooks don't run again and `RETURNING` fills the row only on success. `hooks(tx)` statements are transactions, so they follow `.retry(n)`.

**Limiting the damage:**

| Setting | Default | Effect |
|---|---|---|
| `lock_timeout` | 5s | One lock wait past this returns `LockTimeout` |
| `transaction_timeout` | 30s | A `db.transaction` past this returns `TransactionTimeout` |
| `idle_in_transaction_session_timeout` | 1m | Postgres ends a transaction left open and idle |
| `client_connection_check_interval` | 5s | Postgres stops a running query when the client has gone |
| `tx_warn_after` | 1s | Logs a slow transaction, usually a network call inside it |

- `lock_timeout`, `idle_in_transaction_session_timeout` and `client_connection_check_interval` are connection startup parameters, so they add no round trip. All exist on every supported version, which matters because an unknown startup parameter fails the connection.
- **`transaction_timeout` covers only `db.transaction`.** Postgres's setting also applies to single statements and would break long queries, so it's sent as `SET LOCAL`, pipelined with `BEGIN`, on Postgres 17+. It ends the connection, and the pool replaces it. Postgres 15 and 16 use a client-side timer, which cancels the query and discards the connection.
- `.lock_timeout("30s")` and `.transaction_timeout("5m")` raise a limit for one transaction.
- **Cancelled requests:** when a future is dropped mid-query, as when a client disconnects, a queued rollback isn't enough, because the running query keeps its locks. morm spawns a cancel using the same TLS settings, and discards the connection. `client_connection_check_interval` is the backstop if the runtime is shutting down.
- **Visibility:** `pool_stats()` counts deadlocks, retries, lock timeouts and `NestedConnection` errors. Postgres's deadlock detail is logged through `tracing`, with no parameter values.

**Lock-order detector:** debug builds and `#[morm::test]` record the order each transaction touches tables, including foreign key locks that can conflict, such as cascades. If one path goes `accounts → orders` and another `orders → accounts`, it reports the cycle, even if no deadlock happened. Unregistered raw SQL is invisible to it, and table-level tracking can give false alarms, so it warns by default.

### JSON output and schemas

**Rule: data never passes through `serde_json::Value` or a map.** Each path has at most one conversion in each direction.

- `user.to_json()`, or `Serialize` from the `json` feature. `user.to_json_for(mask)` limits output to groups.
- `ProductRef` also serializes directly, so `fetch_ref` is the path for rows you process and then send.
- `jsonb` columns can decode as `&RawValue`, so stored JSON is embedded without being parsed and rewritten.
- **Input:** `Deserialize` on a view accepts only its writable fields, so `UserProfile` doubles as the safe request body. `insert` and `update` run `clean` and the rules. No separate input type.
- **Schema:** `User::json_schema(Up | Down)` as generated static strings, plus optional `utoipa` for OpenAPI.
- **Large integers:** JavaScript loses digits above 2^53, so `#[col(json_string)]` writes an integer as a JSON string, on every JSON path.
- Formats are fixed and documented: `Decimal` as a string, timestamps as RFC 3339, `bytea` as base64, `None` as `null`, `jsonb` embedded as JSON.
- **`fetch_json()` on a model or view follows the same formats.** Postgres would write `numeric` as a JSON number and `bytea` as hex, so generated SQL converts those columns. `db.sql(...).fetch_json()` has no model to go by, so Postgres's own formats apply there, and your SQL can cast.

`fetch_json()` isn't automatically fastest: it moves the CPU work into Postgres, where it occupies a connection. Benchmarks compare it with `fetch_ref()`, and the docs state the result.

---

## 9. Hooks

Hook signatures show exactly what's available. **Update and delete hooks take no `self`, so they never see a row unless they fetch it.** Nothing is ever fetched automatically.

```rust
impl Hooks for Product {
    // Insert: you built the whole row, so hooks get the whole row
    fn validate(&self) -> Result<()>;
    async fn pre_insert(&mut self, db: &impl Conn) -> Result<()>;
    async fn post_insert(&self, db: &impl Conn) -> Result<()>;   // includes id and defaults

    // Update: only what was set, and what was targeted
    fn validate_changes(changes: &Changes<Self>) -> Result<()>;
    async fn pre_update(changes: &mut Changes<Self>, target: &Target<Self>, db: &impl Conn) -> Result<()>;
    async fn post_update(changes: &Changes<Self>, target: &Target<Self>, affected: u64, db: &impl Conn) -> Result<()>;

    // Delete: only what was targeted
    async fn pre_delete(target: &Target<Self>, db: &impl Conn) -> Result<()>;
    async fn post_delete(target: &Target<Self>, affected: u64, db: &impl Conn) -> Result<()>;

    async fn after_commit(event: Event<Self>) -> Result<()>;
}
```

`Changes<Self>` holds only the columns passed to `.set()`:

```rust
match changes.get(F.price) {
    Some(Change::Value(price)) => { /* set in this update */ }
    Some(Change::Expr)         => { /* set_expr: the value exists only in SQL */ }
    None                       => { /* not in this update; the row's value is unknown */ }
}
changes.set(F.slug, slug);   // a pre hook can add or change a value
```

`Target<Self>` says which rows are affected, and every variant must be handled: `Id(id)`, `Ids(ids)`, `Filter(f)` (unknown rows, only the condition is known), or `All`.

### Order

| Operation | Order |
|---|---|
| `insert` | `pre_insert` → `clean` and rules → `validate` → **INSERT** → `post_insert` → *(commit)* → `after_commit` |
| `update` | `pre_update` → `clean` and rules → `validate_changes` → **UPDATE** → `post_update` → *(commit)* → `after_commit` |
| `delete` | `pre_delete` → **DELETE** → `post_delete` → *(commit)* → `after_commit` |

### Rules

- **A hook needing the full row** fetches it itself, so the query is visible in the code.
- **Hooks run once per statement,** never per row, so a bulk update can't become thousands of hook calls.
- **`update_each`:** `pre_update` runs once with `Target::Ids` and only the column list. Per-row values go through `clean` and the declarative rules.
- **SQL is built after pre hooks,** so changes made there apply. A pre hook can cancel the operation by returning an error.
- **Post hooks** run on the same connection or transaction.
- **`after_commit`** is for side effects (emails, queues, cache invalidation). It gets an owned enum: `Inserted(row)`, `Updated { changes, target }` or `Deleted { target }`. It's boxed only for models overriding it, runs only after commit, and is skipped on rollback. Errors there can't undo the commit. It runs in the app process, so a crash right after commit loses it. Events that must not be lost go in a table in the same transaction.
- **Automatic transaction:** a model with `hooks(tx)` has each statement wrapped in a transaction. Others stay at one round trip.
- **Raw SQL doesn't run hooks;** this is documented.
- **Zero cost:** static dispatch, and empty default hooks compile away.
- **Axum-safe:** signatures use `-> impl Future<Output = Result<()>> + Send` underneath, and users can still write `async fn`.

---

## 10. Migrations

### Principles

1. Never trust generated SQL until it has run.
2. Check the real database before and after every migration.
3. Every failure has a defined state and a way forward, including a killed process.
4. Down is a first-class product: dropped data is kept where possible.
5. Nothing destructive is guessed.

### What's managed

**Everything an application's schema depends on, from v1:** tables, columns (type, collation, nullability, default, identity, generated, comment), constraints (including deferrable and validated state), indexes, enums, domains, composite types, sequences, extensions, **database views, materialized views, functions, procedures, triggers, RLS policies, and grants** on managed objects.

Tables and other objects with no definition in code are never changed. If one depends on a change, that's a decision with evidence.

### Triggers, functions and other objects

They live next to the model they belong to. Objects are ordinary Rust items, auto-registered with no central list. Short ones go inline. Long ones go in a real `.sql` file beside the module, so editors highlight them and they can be pasted into `psql`.

```
src/users/
├── mod.rs                  # models, plus the object declarations
└── sql/
    ├── touch_updated_at.sql
    └── active_users.sql
```

```rust
// src/users/mod.rs
#[model(table = "users", base = "common")]
pub struct User { /* ... */ }

morm::pg_function!(touch_updated_at, file = "sql/touch_updated_at.sql");
morm::pg_trigger!(users_touch, on = User, "BEFORE UPDATE FOR EACH ROW EXECUTE FUNCTION touch_updated_at()");
morm::pg_policy!(own_rows, on = User, "USING (id = current_setting('app.user')::bigint)");
morm::pg_procedure!(archive_users, file = "sql/archive_users.sql");
morm::pg_grant!(on = User, "SELECT, UPDATE TO app_rw");

#[pg_view(file = "sql/active_users.sql")]          // read-only model over a database view
pub struct ActiveUser { pub id: i64, pub email: String }
```

- `file` paths are relative to the Rust file, like `include_str!`, so editing the SQL triggers a rebuild.
- `on = User` is a real type reference, so a typo fails to compile and table renames carry through.
- A `.sql` file holds the complete `CREATE` statement, plain Postgres. morm has no SQL parser: it runs the statement on scratch and reads what was created from the catalog.
- **Order is automatic.** Kinds apply in a fixed order (types, tables, functions, views, triggers, policies, grants). Within a kind, statements retry until all resolve, then `pg_depend` gives the true order.
- **These definitions never reach the app binary.** They're compiled only into `morm-mgr`.
- Macros also generate a typed caller where the signature allows, for example `db.call(archive_users, &days)`.
- Roles are cluster-wide, so they aren't created. `morm-admin scratch` makes placeholder roles so policies and grants can run on scratch, and `doctor` checks the real ones exist.

**Dependencies are handled by the planner.** A column type change under a view normally fails. Because morm owns the view, the planner reads `pg_depend`, drops the dependent objects, alters the column, and recreates them in order, with an exact inverse. It uses `CREATE OR REPLACE` when safe, and drop and recreate when a signature or result shape changes. Materialized views rebuild as a planned step with their indexes and a refresh.

### Source of truth

No snapshots. `makemigrations` uses a scratch Postgres (`MORM_SCRATCH_URL`) and never touches staging or production:

1. Replay the migration chain into temporary database A.
2. Create the models and objects in temporary database B.
3. Read both through the same catalog queries (`pg_get_viewdef`, `pg_get_functiondef`, `pg_get_triggerdef` and so on) into a typed `Schema` IR, so Postgres normalizes everything.
4. Diff into typed operations.

Column position isn't compared.

### Typed operations

The diff yields `Op` values, not SQL strings. Each declares:

| Property | Use |
|---|---|
| forward SQL | up step |
| inverse op | down exists **by construction** |
| lock level, rewrite, scan | `lint`, `--plan`, ordering |
| transaction mode | `tx`, `no-tx`, `batch` |
| data effect | `none`, `retires`, `lossy`, `irreversible` |
| precondition | catalog check that makes re-running safe |

The planner topologically sorts ops (foreign key cycles are resolved by adding constraints last), takes table locks in a consistent order to avoid deadlocks, and groups ops into **steps**, each with its own transaction boundary.

### Safe forms are the default on existing tables

| Change | Generated steps |
|---|---|
| Add FK or CHECK | `ADD ... NOT VALID`, then `VALIDATE` in a **separate** transaction (one transaction would hold the lock through the scan) |
| Add unique or primary key | `CREATE UNIQUE INDEX CONCURRENTLY`, then `ADD CONSTRAINT ... USING INDEX` |
| Add index | `CREATE INDEX CONCURRENTLY` (no-tx) |
| Set NOT NULL | Batched backfill, `CHECK NOT VALID`, `VALIDATE`, `SET NOT NULL`, drop the check |
| Add NOT NULL column | Add nullable, batched backfill, then as above |
| Add column with a volatile default, such as `gen_random_uuid()` | Add with no default, `SET DEFAULT` so new rows get it, then batched backfill. A constant default is instant and stays one step |
| Type change that is lossy or rewrites the table, such as `int` to `bigint` | New column, batched copy, swap (old column retired) |
| Widening type change | In place when no rewrite is needed |

New tables use plain DDL in one transaction. `--fast` allows the simple forms for small tables or maintenance windows.

### Down that can restore data

- **Retire instead of drop.** A dropped column is renamed `_morm_retired_<migration>_<col>`, with NOT NULL and defaults removed and dependent indexes dropped. A dropped table moves to the `_morm_retired` schema. Rollback restores them with their data.
- **`morm-admin mgr purge --older-than 30d`** does the real drop later, as a recorded operation. That migration's down then becomes irreversible.
- **Limit:** rows inserted after `up` have no value in a restored column. If it was NOT NULL, the decision must supply a `down_fill` value, or down refuses.
- **Irreversible migrations** must be declared. `rollback` refuses them without `--accept-data-loss`.

### Production is forward only

Rolling back after new data has been written can fail or lose that data, so production moves forward only. `down.sql` is still proven for every migration, because `revert` and development depend on it.

- **Every database is production until marked otherwise.** The mode is stored in `_morm`, not in an environment variable, so a wrong setting on a host can't turn it off, and a backup restores with it. Forgetting to mark a database leaves it safe.
- `morm-admin mgr mode development` marks a local or staging database, and `mode` shows the current value. Every change goes in `_morm.history`.
- **On a production database,** `rollback` and `rehearse` exit with code `2`. To rehearse on a clone of production, mark the clone first.
- **`morm-admin mgr revert ID`** is the way to undo in production. It creates a new forward migration from that migration's down, runs the full proof on scratch, and needs the same decisions, such as `down_fill`. Retired columns and tables come back with their data, and the chain stays linear, so every database has the same history.
- **A failed migration can always be cleaned up.** `migrate --resume` is the first choice. `rollback --failed` undoes the completed steps of a migration that never finished, because no release depends on it yet.
- **Emergency override:** `rollback --production --reason "..."` works but is logged to `_morm.history`, and it still needs `--accept-data-loss` where that applies.
- Retired columns are a tracked state, so `verify` doesn't report them as drift.

### Decisions

Every rename, retire, purge, lossy change and missing backfill is a decision with an id, evidence (type match, name similarity, row presence, dependents, size) and options. They're answered interactively, by `--decide ID=OPT`, or by file. `--json` suits CI and agents. `was` is a pre-answered rename, ignored once the old name is gone.

Exit codes: `0` ok, `1` error or drift, `2` refused by a guard, `3` open decisions.

### Proof before writing

Files are written only after all of these pass on scratch:

1. `up` reaches the models exactly.
2. `down` returns to the previous schema exactly.
3. `up` again gives the same fingerprint.
4. **With data:** synthetic rows per column type (NULLs, boundaries, empties) go in first, and preserved-column checksums must match after up → down.
5. `lint` passes, or each finding is explicitly accepted.

### Files and ledger

```
morm-migrations/
└── 0007_add_products/
    ├── up.sql      # steps split by: -- morm:step tx | no-tx | batch size=5000
    ├── down.sql
    └── meta.json   # id, parent, phase, pg_major, schema_before, schema_after,
                    # checksum, decisions, data_effect
```

`id` is permanent and `parent` forms a chain, so merged branches are detected (`diverged`) and fixed with `rebase-chain`. The directory number is cosmetic. Ledger, in its own `_morm` schema:

- `_morm.migrations`: what's applied, with exact up and down SQL, so rollback works when files are gone.
- `_morm.steps`: per-step state (`pending/running/done/failed`) and batch cursors.
- `_morm.history`: append-only log of every action, with user, host, duration and tool version.

### Runtime guarantees

- **Fingerprint gate.** Live schema must match `schema_before`, or the run refuses. It must match `schema_after` before commit, or the transaction rolls back. Rollback checks the reverse.
- **Postgres versions.** Catalog text can differ between majors, so `pg_major` is recorded. On a mismatch, the expected state is rebuilt by replaying the chain on the target's version in a temporary database. Without `CREATEDB` it refuses unless `--skip-fingerprint` is given. `refingerprint` handles upgrades.
- **One migrator:** session advisory lock on a dedicated connection. If that connection is lost, the run stops.
- **Lock safety:** `lock_timeout` on every step with bounded retry and backoff, plus per-step `statement_timeout`.
- **Preflight:** privileges, long-running transactions, invalid indexes, checksums, table sizes.
- **Crash recovery:** preconditions make steps safe to re-run. `status` reports the exact step, `migrate --resume` continues, `rollback` undoes only completed steps, failed concurrent indexes are rebuilt, and batches resume from their cursor.
- **Tamper refusal:** edited or missing applied migrations are refused. `repair --reason "..."` re-signs and logs it.
- **LIFO rollback,** plus `rollback --orphans` for another branch's migrations, on development databases.

### Rolling deploys

Static column lists mean a dropped column breaks the old binary mid-deploy. Migrations carry `phase: expand | contract`, and `--phase` splits them. `contract` refuses while a required `#[col(deprecated)]` hasn't shipped. A rename decision offers an expand/contract form for live tables: add the new column, keep both in sync with a generated trigger, backfill in batches, and drop the old column and trigger in `contract`. `db.check_schema()` compares a compile-time per-model fingerprint with the database at startup, so a mismatched binary fails at boot.

### Data migrations

SQL steps first, including `batch` steps, covered by checksums. Custom Rust data hooks are registered by migration id and use raw SQL only, since model types change. A hook needs a matching down hook, or the migration is declared irreversible.

### Commands

All are `morm-admin mgr <command>`:

```
makemigrations [--plan] [--json] [-n name] [--decide ID=OPT] [--decisions F] [--empty] [--fast]
migrate [--plan] [--json] [--yes] [--to ID] [--resume] [--phase expand|contract]
rollback [--last N | --to ID | --orphans | --failed] [--plan] [--yes] [--accept-data-loss] [--production --reason R]
revert ID [--plan] [--decide ID=OPT]    mode [development | production]    squash --to ID
status [--json]    verify [--json]    check    lint    check-sql
rehearse --url URL    purge --older-than D    baseline    rebase-chain
repair --reason R     refingerprint           sql ID
```

- `check`: offline CI check of chain, checksums, models.
- `verify`: live schema vs models and ledger; read-only and safe in production.
- `rehearse`: pending up → down → up on a clone or restored backup, with **real data**. The recommended last gate.
- `lint`: statement risk, in the style of `squawk`, plus index advice. With `--url` it reads real table sizes and reports risky operations only on tables big enough to matter.
- `check-sql`: `PREPARE` and `EXPLAIN` on every registered query.
- `baseline`: adopts an existing database as migration zero, without running anything.
- `squash --to ID`: replaces the chain up to `ID` with one baseline generated by replaying it. A database that has applied all of them just continues, and one partway through is refused until it catches up. Run it only when every database has passed `ID`.

**Run migrations as a deploy step,** before new code goes live, so a failed migration stops the deploy. The app binary has no migration code, so it can't migrate on start.

---

## 11. Performance and memory

### `morm::Pool`

`morm::Pool` is the generic `deadpool` crate with our own connection manager, not `deadpool-postgres`, whose statement cache has no size limit. `deadpool` provides the tested parts: waiting with timeouts, closing and resizing. morm provides the rest:

- **Statement cache:** a bounded LRU on each connection.
- **Health check:** `client.is_closed()` at checkout, with no extra query, so checkout adds no round trip.
- **Errors:** a pool wait timeout becomes a typed `PoolBusy`.
- **Sizing:** warm start, idle shrinking and limit checks, below.

Each query takes a connection, runs, and releases it. Connections are not shared between tasks, so one slow query only holds its own connection, a dropped connection fails only its own request, and timeouts work normally. Tasks can still pipeline several queries within one checkout.

Everything goes through `db`, so the pool can be replaced later without changing application code.

### Dynamic size

| Setting | Default | Meaning |
|---|---|---|
| `min` | 2 × cores | Connections opened at startup |
| `max` | 10 × `min` | The ceiling |
| `idle_timeout` | 5m | Idle connections close, down to 1 |
| `last_idle_timeout` | 1h | The last connection closes too, so an idle app can reach 0 |
| `max_queries` | 50000 | Replace a connection after this many queries; 0 disables it |
| `max_lifetime` | off | Replace a connection after this age |

- **`min` is not a floor.** It only sets the warm start. Connections reopen on demand, and the first query after a full idle period pays the connection cost.
- **Limits never interrupt work.** `max_queries` and `max_lifetime` are checked only when a connection returns to the pool.
- **Custom rules:** `Db::connect().recycle_when(|c| c.queries > 50_000 || c.age > hours(6))`.
- **Cores:** the default uses the machine running the app, because morm can't see a remote server's core count. If the database is elsewhere, set `min` by hand.
- **Failover:** on a pool that isn't `read_only`, a connection that returns `25006` (read-only transaction, after a primary switch) or an admin shutdown error is discarded, not reused. With several hosts and `target_session_attrs = "read-write"`, replacements find the new primary.
- **Queues:** a request that can't get a connection waits up to `wait_timeout`, then returns `PoolBusy`. Waits are never infinite.
- **Separate processes:** a service job, cron or background worker running as its own process must not start a pool with `min` above 1. It should use `Db::connect_as("jobs")`, `MORM_POOL=jobs`, or `MORM_POOL_MIN=1`, because the default of 2 × cores would open connections it doesn't need. The pool still grows on demand.

### Sizing guidance

- Connections needed ≈ queries per second × seconds each query holds a connection. For example, 2,000 queries/s × 5ms needs about 10.
- Only as many queries run at once as Postgres has free CPU cores. Extra connections help when queries wait on disk, locks or a slow network. When everything is CPU-bound, they only move the queue into Postgres and add contention and memory.
- `db.pool_stats()` reports wait time, busy connections and holds. If tasks wait while Postgres CPU isn't maxed out, raise `max`. If Postgres CPU is maxed out, a bigger pool won't help, so fix the queries or add cores.

### `max_connections`

At startup, morm reads `max_connections` and the reserved slots, then computes:

```
budget = (max_connections - reserved - safety margin) / processes
```

If `min` or `max` exceed the budget, including explicit values, morm lowers them and emits a warning, through `tracing` when enabled and stderr otherwise. `morm-admin doctor` runs the same check.

```
morm: pool max 160 lowered to 92 (max_connections=100, reserved=3, margin=5)
```

morm can't see other apps on the same server, which is what the margin is for, and `processes` is set by hand. When several separate processes share a server, Postgres is the real limit: on "too many clients", morm retries with a short backoff before returning the error.

### Jobs and long-running work

Crons, background jobs and service workers use the pool like everything else. Every `db` call takes a connection and releases it, so a job that sleeps for hours holds nothing, and idle closing and reconnects need no code.

```rust
loop {
    let batch = next_batch().await;    // holds no connection while waiting
    db.update_q::<Job>().by(F.id, &batch.id).set(F.done, true).run().await?;
}
```

| Case | Use |
|---|---|
| Normal request, job or cron | `db` |
| Websocket query | `db`, per message |
| Several statements that must be atomic | `db.transaction(...)`, in small batches. A long transaction blocks vacuum and can hold locks |
| A very long query | `db`. It holds its slot until done, so size `max` for it |
| A heavy job in the same process as the web app | A named pool: `db.pool("jobs")` |
| `LISTEN`, temp tables, advisory locks, `SET` | `db.hold()` |
| Server push from Postgres to websockets | One `db.hold()` listener for the whole app, fanned out to the sockets. Not one per socket, and not needed for ordinary websockets |

### `db.hold()`

```rust
let held = db.hold().await?;
held.listen("orders").await?;
// ...
db.release(held).await;      // or held.release().await
```

`db.hold()` keeps one pool connection until released, and no limit or idle rule touches it. `release` takes ownership, so using `held` afterwards is a compile error. Holds count against `max`, and `max_holds` caps them so they can't starve queries.

**Release is guaranteed wherever the process survives,** and the server cleans up where it doesn't:

| How it ends | What releases it |
|---|---|
| Normal release | `db.release(held)` |
| Forgot to release, early return, `?` error | Drop |
| Panic, or task cancelled | Drop, with `panic = "unwind"` |
| Process crash, `kill -9`, `panic = "abort"` | The OS closes the socket, and Postgres ends the session and releases its locks |
| Machine or network dies (TCP) | morm sets TCP keepalives, and the server clears the session after about a minute |
| Postgres restarts | Every session ends. The next use of `held` returns `ConnectionLost`, and the slot is freed |

- **Held connections are closed on release, never reused.** They can carry `LISTEN`, temp tables, `SET` values or advisory locks, and reusing one could leak that state into the next query. The slot is free immediately, and the pool opens a clean connection when needed.
- **Reconnecting:** `db.hold().on_connect(...)` re-applies session state after a lost connection. `held.listen()` channels re-subscribe automatically.
- **Forgotten holds:** `#[must_use]` warns if a guard is discarded straight away. Rust can't force an explicit `release`, so Drop is the safety net. `pool_stats()` lists each hold with its age and where it was created, and `hold_warn_after` logs a warning. It only warns, because long-running holds are allowed.
- **Shutdown:** `db.close()` stops new checkouts, then waits up to `shutdown_timeout` for running transactions, `after_commit` handlers and holds to finish, then closes every connection. Apps call it after they stop accepting requests, so a deploy doesn't roll back work or lose events.

**Limit:** one app process has one budget, so several processes or hosts must be sized together against `max_connections`. One multi-threaded Tokio process is the recommended deployment.

### Statements

- Bounded per-connection LRU. Evicted statements are closed when dropped, which also frees Postgres memory.
- Dynamic SQL and overflow go through `query_typed`, which needs no prepare round trips.
- `cached plan must not change result type` (0A000): evict and re-prepare once outside a transaction, typed error inside.

### Query path

- SQL for select, get, delete and insert is a compile-time `&'static str`.
- `= ANY($1)` instead of `IN` lists, so list length doesn't create statement variants.
- Bulk insert via `UNNEST` (one stable statement, no 65,535-parameter limit), and `COPY` above a threshold.
- Positional decoding; borrowed parameters (`&dyn ToSql`), not boxed.
- `BEGIN`, statement and `COMMIT` pipelined to cut auto-transaction round trips.
- Rows carry no per-row metadata.
- No dynamic dispatch in hot paths.
- Minimal dependencies and features; no CLI framework in `morm-admin`.

### Build and benchmarks

- Release profile for benches and examples, recommended in the docs: `lto = "fat"`, `codegen-units = 1`, `strip = true`, `panic = "unwind"` (so a panicking handler doesn't kill the server). A library's profile doesn't reach its users.
- Criterion benchmarks in CI against raw tokio-postgres, SQLx, SeaORM and Diesel.
- Also: `morm::Pool` vs `deadpool-postgres`, pool sizes under mixed fast and slow queries, `fetch_ref` vs `fetch` vs `fetch_json`, morm's `Decimal` decoding vs `rust_decimal`'s, binary size and RSS with `tls` compiled in but unused, and RSS on both app and Postgres.

---

## 12. Hard to misuse

| Mistake | Caught by |
|---|---|
| `update` or `delete` with no filter | Compile error (`update_all` and `delete_all` are explicit) |
| `update` with no `.set()` | Compile error |
| Forgetting `.run()` | `#[must_use]` warning, `-D warnings` in CI |
| Building SQL from strings | Compile error (`q_unchecked` is the visible escape hatch) |
| A list with no limit | Compile error: requires `.limit(n)`, `.all()` or `stream()` |
| Reading a hidden field | Compile error |
| Using `db` inside a transaction or stream loop | `NestedConnection` at runtime; `db.extra_conn()` is the visible escape hatch |
| A request-supplied sort column | `.q()` refuses it; `order_by_allowed` checks it against a list |
| Paging with `OFFSET` | No helper exists; `keyset` is the documented path |
| Rolling back a production database | Refused, exit code `2`, unless marked `development`; `revert` is the forward path |
| `u64` on a column | Compile error |
| A stale cached statement after a migration | Re-prepared automatically outside a transaction |
| Swallowing an error, then committing | `TransactionAborted` at runtime |
| Updating or deleting a missing id | `NotFound` at runtime |
| `.retry()` on a closure that changes captured rows | Compile error |
| Locks taken in opposite order across transactions | Lock-order detector in debug builds and tests |
| `tx.lock`, `advisory_lock` or `.skip_locked()` outside a transaction | Compile error |
| A duplicate of a unique, primary key or plain index | Compile error |
| `Decimal` on a `numeric` too large for it, or a float on `numeric` | Compile error |
| A rounded `numeric` value | Error on read and on write |
| Queries in a loop | Detector in debug builds and `#[morm::test]`: the same statement repeating in one task warns, and fails the test |
| Filtering on an unindexed column | `morm-admin mgr check-sql` runs `EXPLAIN` on every registered query |

The compiler can't see a query inside a loop, so that one is caught in tests and debug builds, not at compile time.

---

## 13. Testing and CI

Tests are written with the code: **a milestone isn't done until its tests pass in CI.**

- **Unit:** macro output, SQL generation, diff, naming.
- **Compile-fail (`trybuild`):** every compile-time guarantee, including hidden view fields, `HasPk`, unfiltered update and delete, update with no `.set()`, list with no limit, `self` absent in update and delete hooks, non-static `q()` and `sql()`, uppercase names without `keep_case`, `u64` columns, unknown group names, scoped groups on `#[model(groups)]`, a scoped mask on another model, duplicate indexes, duplicate base fields, over-long and colliding names, `Decimal` and float on unsuitable `numeric` columns.
- **Integration on real Postgres:** insert, every get, update and delete form, views, groups, hook inputs (`Changes`, `Target`), hooks order, the loop detector, `after_commit` on commit and skip on rollback, all list and JSON paths, pool growth and idle shrinking, `max_connections` adjustment, `db.hold()` release on every exit path, `ConnectionLost`, cancellation, forced deadlocks in both kinds, swallowed errors never reporting a commit, `CommitUnknown`, savepoint behaviour after deadlocks, single-statement and transaction retry limits, every timeout, cancelled queries releasing their locks, `max_holds`, `.lock_ordered()` and bulk writes under concurrent load, advisory key stability, `skip_locked` with rival workers, `.in_batches` stopping and resuming, keyset pages under concurrent writes and with mixed sort directions, tampered cursors, `order_by_allowed`, `.custom_plan()`, connections discarded on `25006`, unsigned ranges, `InvalidText`, `json_string` on every path, `db.close()` finishing in-flight work, models in several schemas.
- **Property tests:** random models and data round-trip through insert, read and JSON.
- **Doc tests:** every README and doc example compiles and runs.
- **`#[morm::test]`:** gives each test a database cloned from a migrated template, for this project and its users.

Migration tests:

- **Fault injection:** kill the process at every step boundary, then prove `--resume` and `rollback` both reach the right state.
- **Property tests:** random schema A → B with data, including views, functions and triggers: up reaches B, down returns to A, preserved rows are intact.
- **Op tests:** every op's inverse, with and without data, and every dependency recreate path.
- **Concurrency:** rival migrators, migration under live traffic, lock-timeout retry.
- **Forward only:** `rollback` and `rehearse` refused on production mode, the default on a new database, and `revert` restoring retired data.
- **Squash:** a squashed chain gives the same schema as the full chain, and a database partway through is refused.
- **Full chain:** all up → all down → all up on Postgres 15–18.
- **Fuzzing:** step parser and `meta.json`.

### GitHub Actions (`.github/workflows/`)

| Workflow | When | Runs |
|---|---|---|
| `ci.yml` | Every push and PR | `fmt`, `clippy -D warnings`, unit, `trybuild`, doc tests, integration on Postgres 15–18 (TCP and Unix socket), MSRV build, feature combinations, migration suite including fault injection |
| `nightly.yml` | Daily | Long property tests, fuzzing, full chain, `cargo audit` and `cargo deny` |
| `bench.yml` | Weekly and on demand | Benchmarks, compared with the last run; tracks binary size and RSS |
| `release.yml` | On tag | Tests, publish crates, build `morm-admin` binaries |

Every merge to `main` requires `ci.yml` to pass. The repo is private, so Actions minutes are limited: the Postgres matrix runs fully on `main` and PRs, while pushes to other branches use one version.

---

## 14. README.md

One compact, complete file at the repo root. It must be enough to use morm efficiently without other docs, and it must be explicit about doing the right thing. It's included as crate docs (`#![doc = include_str!("../README.md")]`), so CI compiles every example and it can't go stale.

1. What it is and why: a few lines, plus benchmark numbers.
2. Quick start: install, `morm-admin init`, first model, first query.
3. **Pick the right call**, near the top:

| You want to | Use | Not |
|---|---|---|
| Send the data on unchanged | `fetch_json`, `stream_json` | `fetch`, then serialize |
| Read a few columns | `fetch_raw` | `fetch` |
| Read and compute, no changes | `fetch_ref` | `fetch` |
| Keep or pass data around | `fetch` | |
| Change known fields | `update` | Fetching the row first |
| Set one value on many rows | `update_q` | A loop of `update` |
| Set different values on many rows | `update_each` | A loop of `update` |
| Do math on a value | `set_expr` | Read, compute, write |
| Fetch related rows | `= ANY` or a JOIN | A query per row |
| Add many rows | `insert_many` | A loop of `insert` |
| Handle huge results | `stream` | `.all()` |

4. **Performance rules:** never query in a loop, always limit lists, select through the narrowest view, keep transactions short with no network calls inside them, index what you filter on, release every `db.hold()`, use `tx` and never `db` inside a transaction, no network calls or app `Mutex` waits inside one, build rows inside a retried closure, lock several rows with `tx.lock`, use `.lock_ordered()` on contended bulk updates, put side effects in `after_commit`.
5. **Extra calls, one line each.** Every opt-in call, escape hatch and override gets one line saying when to use it. A public option with no line here is a documentation bug, and CI compares this table with the public API.

| Call | Use this when |
|---|---|
| `.allow_missing()` | You will not handle a 0-row error |
| `#[col(json_string)]` | JavaScript clients read this integer, and it can pass 2^53 |
| `schema = "..."` | A model's table lives outside the default schema |
| `mgr mode development` | This is a local or staging database, and you want `rollback` |
| `mgr revert ID` | You must undo a migration in production |
| `rollback --production` | `revert` can't work and you accept the risk; a reason is required |
| `mgr squash --to ID` | The chain has grown long, and every database has passed `ID` |
| `keep_case` | You're matching an existing mixed-case table or column |
| `tx.get_for_update` | Your logic must see the current row before changing it, and `set_expr` can't express it |
| `#[col(version)]` | Users edit a row over time, and a stale save must fail, not overwrite |
| `.in_batches(n)` | You change so many rows that one statement would hold locks too long, and it needn't be atomic |
| `.keyset(...)` | You page through a list that can go deep |
| `.order_by_allowed(...)` | A client chooses the sort column |
| `morm::quote_ident` | You use a dynamic table or column name in `q_unchecked` or `sql_unchecked` |
| `.custom_plan()` | A query is slow only for some values, because your data is skewed |
| `nulls_not_distinct` | A unique column may hold only one NULL |
| `#[derive(PgEnum)]` | You're matching an existing Postgres enum; use `TextEnum` otherwise |
| `storage = "fillfactor=90"` | A table gets many updates to columns with no index |
| `.returning()` | You need the changed rows back |
| `.all()` | You want every row, and you know the result is small |
| `set_expr` | The new value depends on the current one, such as `stock - 1` |
| `.retry(n)` | You want the transaction to run again after a deadlock, and you build its rows inside the closure |
| `.isolation(...)` | You read rows and write based on them, and nothing may change them in between |
| `.lock_timeout(...)` | A transaction will wait on locks for longer than 5s, such as a large batch |
| `.transaction_timeout(...)` | A transaction will run for longer than 30s, such as a large batch |
| `tx.lock(&ids)` | You will change several rows that other requests also change, such as a money transfer |
| `.lock_ordered()` | You run bulk updates or deletes that can touch the same rows at the same time |
| `.skip_locked()` | Your workers take jobs from a queue and shouldn't wait for each other |
| `tx.advisory_lock(&keys)` | Only one process may do this work at a time, such as a billing run |
| `db.extra_conn()` | You need a write to survive a rollback, such as an audit row |
| `db.hold()` | You need `LISTEN`, temp tables, session advisory locks or `SET` |
| `db.pool("replica")` | You run heavy reads, such as reports, that can be slightly out of date |
| `db.pool("jobs")` | You run heavy jobs in the web app's process, and they must not take its connections |
| `db.groups(mask)`, `db.sudo()` | Your callers may see or change different fields |
| `Numeric` | Your values can exceed 28 digits, or be `NaN` or `Infinity` |
| `q_unchecked`, `sql_unchecked` | You must build SQL at runtime, and you will handle injection safety yourself |
| `recycle_when` | You need your own rule for replacing connections |
| `no_default` | You want a compile error when a required field is missed |
| `--fast` | The table is small, or you're in a maintenance window |
| `--accept-data-loss` | You've read the plan, and you accept losing that data |
| `--skip-fingerprint` | Your role can't create a temporary database, and you accept a weaker check |

6. Models: bases, every `#[model]` and `#[col]` option in tables.
7. `get`, `insert`, `update`, `delete`, lists, `filter_cols`, pagination, batches, lost updates, raw SQL with `db.sql()`, transactions, and which of `Decimal`, `Numeric` or `bigint` to use.
8. Views, groups, validation.
9. Hooks, with a table of what each receives and this line: "update and delete hooks never see a row unless you fetch it."
10. JSON output and schemas.
11. Pool and `morm.toml`: every setting, sizing, and the rule for long-running work: "use `db` for everything, including jobs and crons, and use `db.hold()` only when you need session state." It states plainly that service jobs, crons and background workers running as their own process must not spawn a pool with `min` above 1, and shows `connect_as`, `MORM_POOL` and `MORM_POOL_MIN`. It includes the full environment variable table. It also shows the jobs table, the `db.hold()` lifecycle, and when a websocket needs a listener: "release when you're done, and rely on Drop as the safety net, never as the plan."
12. Migrations: workflow, every command in one table, safety rules, production being forward only with `revert`, running as a deploy step, triggers and functions.
13. Security: TLS settings, encrypting disks and backups (outside morm's job, but expected in production), and password hashing with `clean`.

---

## 15. Milestones

0. **Repo bootstrap:** workspace in `biggaloy/rust-morm`, CI workflows, README skeleton.
1. **Core models:** macro, `schema`, unsigned types, bases including `#[morm::base]`, index and name rules, views, global and model-scoped groups, declarative rules and `clean`, inference, `F`, compile-time checks.
2. **Runtime:** `morm.toml` and environment variables, `morm::Pool` on the `deadpool` core (start with a quick proof-of-concept), dynamic size, named pools, `max_connections` adjustment, `db.hold()`, bounded statement cache, insert, `get`/`update`/`delete` forms, query builder, `filter_cols`, `db.sql()` and `#[derive(morm::Row)]`, `keyset`, `.in_batches`, `order_by_allowed`, `.custom_plan()`, `get_for_update`, failover handling, `Decimal` and `Numeric`, loop detector, typed errors, transactions, deadlock measures, all list and JSON paths. → **First publish** (`0.1.0-alpha`).
3. **Migration foundation:** `Schema` IR, catalog reader, diff, typed ops with inverses, scratch, proof pipeline.
4. **Migration execution:** ledger, steps, fingerprint gates, advisory lock, timeouts, preflight, crash recovery, fault-injection suite.
5. **Migration safety:** safe forms, batched backfills, retire and purge, production mode, `revert`, `squash`, decisions, `lint`, `rehearse`, expand/contract, `check_schema()`.
6. **Managed objects:** views, materialized views, functions, procedures, triggers, policies, grants, enums and domains, with dependency recreation.
7. **Hooks:** `Changes`, `Target`, all hooks, `validate`, `validate_changes`, `after_commit`, automatic transactions.
8. **`morm-admin`:** `init`, `doctor`, `import`, `scratch`, `morm-mgr` packaging, Axum template.
9. **Query safety:** `sql!`, `check-sql`, upsert, UNNEST and COPY, `#[morm::test]`.
10. **Remaining features:** UUIDv7, `tls`, schemas and `utoipa`, optimistic locking, tracing, typed FKs, settings, pipelined transactions.
11. **Benchmarks, lock-order detector, hardening, full README, 0.1.0 release.**

---

## 16. Risks, open items and later work

### Risks

- **Managed objects:** functions, triggers and policies with exact inverses are the hardest engine work.
- **Needs proof early:** borrowed `ProductRef` decoding, the loop detector, macro feasibility for `#[view]` and `#[morm::base]`, macro-counted group mask sizes on stable Rust, `AsyncFn` closures for `.retry()`, and hook signatures.
- **Scratch databases:** `makemigrations` needs a Postgres where it can `CREATE DATABASE`.
- **Scope:** migrations alone outweigh the runtime.

### Open items

- Check that `morm-migrate` and `morm-admin` are free on crates.io.

### Later

Shared reads: a small opt-in lane that pipelines primary-key reads on a few connections, added only if a benchmark shows real gains with no worse p99 under mixed load. Also partitioning, a `dynamic` registry for by-name admin CRUD, sensitive-column prompts in `import`, and a provided transactional outbox (table and worker) for events that must survive a crash between commit and `after_commit`.

### Confirmed for after 0.1

Features we will build are in `later-defo-build.plan`: automatic read routing so far.

### Later: field-level encryption

Encrypted columns, including per-user keys and rotation, are not designed yet. Nothing in 0.1 is built for them. These choices, which 0.1 needs anyway, keep them possible without breaking changes:

- `F` columns carry capability markers checked by the query API, so a column can refuse `.by()` or `set_expr`.
- Encode and decode go through one per-column conversion layer, so a column's Rust type can differ from its stored type. It exists only at compile time: generated code calls `ToSql` and `FromSql` directly through static dispatch, with no branch, pointer, allocation or per-row state. A benchmark against hand-written tokio-postgres decoding checks that it adds no cost.

### Not carried over from python-morm

- **`save`:** replaced by explicit `insert` and `update`.
- **`exclude_values_up/down`:** rarely useful, and it would make insert SQL vary per row. `Option` `None` already defers to the DB default, and `skip_json_if` covers output. It can come back if a real need appears.
- **`validator` and `modifier`:** replaced by the type system, native decoding, and declarative rules.
- **`pre_save` and `post_save`:** removed with `save`.
