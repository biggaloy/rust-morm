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
| Pool | Our own `morm::Pool`, with a shared pipelined lane and an exclusive lane. No external pooler needed for a single process |
| Statement cache | Bounded per-connection LRU, with `query_typed` for dynamic SQL and overflow |
| Connection | Unix socket by default (`host=/var/run/postgresql`), the lowest latency and allows peer auth. TCP also supported |
| TLS | Optional cargo feature `tls` (rustls). Not compiled in unless enabled |
| Default primary key | `bigserial` (`base = "common"`), the fastest and smallest |
| Alternative primary key | UUIDv7 (`base = "common_uuid"`), generated in the app or by Postgres 18's `uuidv7()`. For ids in public URLs and offline clients |
| Custom primary key | `base = "bare"`: no columns added; you define your own single, composite, or no PK |
| Change tracking | None. There is no `save`. Updates name their columns, and never load a row first |
| Rows | Plain structs with public fields and zero per-row overhead |
| Field access control | Views at compile time, groups at runtime. `sudo` is a group with a reserved name |
| Validation | One mechanism: declarative rules and `clean`, generated inline, run only on write |
| Hooks | Trait with zero-cost default methods. Update and delete hooks receive only what the statement contains |
| JSON | Direct from a row, with no intermediate `Value` or map. Postgres-built JSON also available |
| Migration source of truth | Models and chain replayed in scratch databases, compared through the Postgres catalog. No snapshots |
| Admin | One tool, `morm-admin`, for everything administrative |
| Rust edition / MSRV | 2024 edition; MSRV at least 1.85, final value decided at first release |
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

- `tls`: rustls for TCP connections
- `chrono` (default) / `time`: timestamp types
- `uuid` (default): UUID columns and UUIDv7 generation
- `json` (default): `jsonb` and JSON output
- `decimal` (default): `numeric` via `rust_decimal`, which can't hold every `numeric` value (documented)
- `tracing`: spans and slow-query logs, compiled out when off. SQL text only, never parameter values
- `utoipa`: OpenAPI schemas

### `morm-admin`

```
morm-admin init [path] [--axum]       # create a project
morm-admin doctor                     # check Postgres, roles, scratch, versions
morm-admin import URL [--out DIR]     # generate models from a database
morm-admin scratch                    # check or create the scratch database
morm-admin mgr <command>              # migrations and schema work (section 10)
```

`init` creates the `morm-mgr` package, `morm-migrations/`, `.env.example`, a sample model, and a CI workflow. `--axum` adds the app template and a systemd unit.

Configuration: `DATABASE_URL`, `MORM_MIGRATION_URL` (an owner role for migrations, so the app can run with fewer privileges), `MORM_SCRATCH_URL`, `MORM_MIGRATIONS_DIR` (default `morm-migrations`).

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

### Bases

Defaults are the best choice for new tables. Anything else is declared explicitly, so models can match any existing table.

| Base | Adds |
|---|---|
| `base = "common"` | `id bigserial` PK, `created_at`, `updated_at` |
| `base = "common_uuid"` | `id uuid` PK (UUIDv7), `created_at`, `updated_at` |
| `base = "bare"` (or omitted) | Nothing. You define every column and your own PK: single, composite, or none |

PK-based helpers come through a `HasPk` trait, so a model without a PK simply doesn't have them: a compile error, not a runtime failure. To match an existing table, write for example `#[col(pk, sql = "serial")] id: i32`, with explicit index and constraint names.

### Struct options (`#[model(...)]`)

| Option | Meaning |
|---|---|
| `table` | table name (default: snake_case of the struct name) |
| `base` | `"common"`, `"common_uuid"`, or `"bare"` |
| `ordering` | default sort, `-` prefix for descending |
| `was` | previous table name (rename hint for migrations) |
| `indexes(...)` | list of indexes (section 7) |
| `extensions` | Postgres extensions the model needs |
| `hooks` | the model has its own `impl Hooks`; `hooks(tx)` also runs its statements in a transaction |
| `groups` | groups this model belongs to |
| `no_default` | don't generate `Default` |

### Field options (`#[col(...)]`)

| Option | Meaning |
|---|---|
| `sql` | SQL type; inferred from the Rust type when omitted |
| `default` | SQL default expression |
| `default_fn` | Rust function that supplies a default on insert |
| `pk` | primary key (with `base = "bare"`; several fields make a composite PK) |
| `unique`, `index`, `index(using = ..., ...)` | single-column shortcuts; `index = "hash,btree"` makes two |
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
| `help` | description, used in schemas |
| `auto_now` | set to `now()` on every update |
| `name` | column name override, for example `type_` with `name = "type"` |
| `was` | previous column name (rename hint) |

### Type inference

`i16` → smallint, `i32` → integer, `i64` → bigint, `f32` → real, `f64` → double precision, `bool` → boolean, `String` → text, `Vec<u8>` → bytea, `DateTime<Utc>` / `Timestamp` → timestamptz, `NaiveDateTime` → timestamp, `NaiveDate` → date, `NaiveTime` → time, `Uuid` → uuid, `serde_json::Value` → jsonb, `Decimal` → numeric, `Vec<T>` → `T[]`. `Option<T>` → nullable. `#[derive(PgEnum)]` maps a Rust enum to a Postgres enum; domains and composite types are also supported. Anything else requires an explicit `sql`.

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
- `Product::G.<group>` masks, when the model has groups
- an empty `impl Hooks` (unless `hooks` is set)
- schema metadata, compiled only into `morm-mgr`, with no model list to maintain

Compile-time checks: unknown columns in `ordering`, `indexes` and `include`; foreign keys whose types don't match; generic structs. Postgres checks `filter` and `expr` on scratch, because a macro can't parse SQL. Groups are `u64` masks, so a model with groups can have up to 64 fields. Other models have no field limit.

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

```rust
#[col(groups = "admin,mod")]  pub subscriptions: Vec<i32>,
#[col(groups = "staff")]      pub active_subscriptions: Vec<i32>,
#[col(sudo)]                  pub password: String,   // sugar for groups = "sudo"
```

`sudo` is an ordinary group with a reserved name.

- **Fields without a group** are open to everyone.
- **Compile time:** `#[view(pub Mod, groups = "mod")]` builds a view from ungrouped fields plus that group.
- **Runtime:** each group is a generated `u64` mask (`User::G.admin`). `db.groups(User::G.admin | User::G.mod)` or `db.sudo()` returns a scoped handle.
- **Runtime enforcement is on write and on output.** An `update` setting fields outside the mask fails, and `to_json_for(mask)` omits them. It's a single bit operation.
- `#[model(groups = "...")]` assigns a model to groups, and `Model::group_fields("mod")` lists its fields.

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

- Plain `unique` becomes a UNIQUE constraint. A unique index with `filter` or `expr` stays an index, because constraints can't be partial or use expressions.
- Deterministic names (`{table}_{cols}_{idx|key}`), with a short hash suffix when longer than 63 characters.
- `morm-admin mgr lint` warns about foreign key columns without an index, and redundant indexes such as a hash and a btree on one column.
- On existing tables, migrations build indexes concurrently by default (section 10).

---

## 8. Reading and writing

In the examples, `F` is short for the relevant model's `F`, such as `Product::F`.

### Connecting

```rust
let db = morm::Db::connect("host=/var/run/postgresql dbname=app user=app").await?;
```

### Get

```rust
db.get::<User>(5).await?;                                  // by PK, Option<User>
db.get_by::<User>(F.email, &email).await?;                 // any column
db.get_where::<Product>(F.price, "BETWEEN $1 AND $2", &[&5, &2000]).await?;   // any comparison
db.get_all_by::<Order>(F.user_id, &7).limit(50).await?;    // many rows

// several columns: one builder shared by get, update, delete, count, exists
db.get_q::<Hist>().by(F.owner_id, &1).by(F.app, &"chat").one().await?;
```

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
- `updated_at` and `auto_now` columns are set to `now()` in SQL.
- **Insert writes one static statement per model.** `Option` fields that are `None` and have a default use it, through `COALESCE($n::type, <default>)`.
- `upsert` holds a whole row, so it runs insert hooks.

### Delete

```rust
db.delete::<Product>(42).await?;                           // one row
db.delete_ids::<Product>(&[1, 2, 3]).await?;               // many ids, = ANY($1)
db.delete_by::<Product>(F.vendor_id, &7).await?;           // any column
db.delete_where::<Log>(F.created_at, "< $1", &[&cutoff]).await?;
db.delete_q::<Hist>().by(F.owner_id, &1).by(F.app, &"chat").any(F.key, &keys).run().await?;
```

Deletes return a count, and `.returning()` gives the rows back. A delete with no filter doesn't compile; `db.delete_all::<T>()` is the explicit form. The same rule applies to `db.update_all::<T>()`.

### Lists

```rust
let rows = db.select::<Product>()
    .filter()
    .qc(F.vendor_id, "=", &vendor_id)
    .q(" AND ").qc(F.price, "<", &max_price)
    .order()                         // model's default ordering
    .limit(50)
    .fetch_ref().await?;

let products: Vec<Product> = db.raw::<Product>("SELECT * FROM products WHERE attrs ? $1", &[&"color"])
    .limit(50).fetch().await?;
```

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

### Query safety

- `sql!` builds a `&'static str` from `F` constants and registers it. `morm-admin mgr check-sql` runs `PREPARE` on each against scratch and compares types. That gives checked raw SQL with no database at build time.
- `.q()` accepts only `&'static str`, so `format!`-ed input won't compile. `q_unchecked(String)` is the visible escape hatch.
- Typed errors: `Unique { constraint, field }`, `ForeignKey`, `NotNull`, `Check`, `Serialization`, `Deadlock`, `Conflict`, `PoolBusy`.

### Transactions

```rust
db.transaction(|tx| async move {
    tx.insert(&mut p).await?;
    Ok(())
}).isolation(Serializable).retry(3).await?;
```

Same API as `db`, with savepoints, retry on 40001 and 40P01, and per-transaction settings (`statement_timeout`, `SET LOCAL` for tenant RLS).

### JSON output and schemas

**Rule: data never passes through `serde_json::Value` or a map.** Each path has at most one conversion in each direction.

- `user.to_json()`, or `Serialize` from the `json` feature. `user.to_json_for(mask)` limits output to groups.
- `ProductRef` also serializes directly, so `fetch_ref` is the path for rows you process and then send.
- `jsonb` columns can decode as `&RawValue`, so stored JSON is embedded without being parsed and rewritten.
- **Input:** `Deserialize` on a view accepts only its writable fields, so `UserProfile` doubles as the safe request body. `insert` and `update` run `clean` and the rules. No separate input type.
- **Schema:** `User::json_schema(Up | Down)` as generated static strings, plus optional `utoipa` for OpenAPI.
- Formats are fixed and documented: `Decimal` as a string, timestamps as RFC 3339, `bytea` as base64, `None` as `null`, `jsonb` embedded as JSON.

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
- **`after_commit`** is for side effects (emails, queues, cache invalidation). It gets an owned enum: `Inserted(row)`, `Updated { changes, target }` or `Deleted { target }`. It's boxed only for models overriding it, runs only after commit, and is skipped on rollback. Errors there can't undo the commit.
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
| Add unique | `CREATE UNIQUE INDEX CONCURRENTLY`, then `ADD CONSTRAINT ... USING INDEX` |
| Add index | `CREATE INDEX CONCURRENTLY` (no-tx) |
| Set NOT NULL | Batched backfill, `CHECK NOT VALID`, `VALIDATE`, `SET NOT NULL`, drop the check |
| Add NOT NULL column | Add nullable, batched backfill, then as above |
| Lossy type change | New column, batched copy, swap (old column retired) |
| Widening type change | In place when no rewrite is needed |

New tables use plain DDL in one transaction. `--fast` allows the simple forms for small tables or maintenance windows.

### Down that can restore data

- **Retire instead of drop.** A dropped column is renamed `_morm_retired_<migration>_<col>`, with NOT NULL and defaults removed and dependent indexes dropped. A dropped table moves to the `_morm_retired` schema. Rollback restores them with their data.
- **`morm-admin mgr purge --older-than 30d`** does the real drop later, as a recorded operation. That migration's down then becomes irreversible.
- **Limit:** rows inserted after `up` have no value in a restored column. If it was NOT NULL, the decision must supply a `down_fill` value, or down refuses.
- **Irreversible migrations** must be declared. `rollback` refuses them without `--accept-data-loss`.
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
- **LIFO rollback,** plus `rollback --orphans` for another branch's migrations.

### Rolling deploys

Static column lists mean a dropped column breaks the old binary mid-deploy. Migrations carry `phase: expand | contract`, and `--phase` splits them. `contract` refuses while a required `#[col(deprecated)]` hasn't shipped. `db.check_schema()` compares a compile-time per-model fingerprint with the database at startup, so a mismatched binary fails at boot.

### Data migrations

SQL steps first, including `batch` steps, covered by checksums. Custom Rust data hooks are registered by migration id and use raw SQL only, since model types change. A hook needs a matching down hook, or the migration is declared irreversible.

### Commands

All are `morm-admin mgr <command>`:

```
makemigrations [--plan] [--json] [-n name] [--decide ID=OPT] [--decisions F] [--empty] [--fast]
migrate [--plan] [--json] [--yes] [--to ID] [--resume] [--phase expand|contract]
rollback [--last N | --to ID | --orphans] [--plan] [--yes] [--accept-data-loss]
status [--json]    verify [--json]    check    lint    check-sql
rehearse --url URL    purge --older-than D    baseline    rebase-chain
repair --reason R     refingerprint           sql ID
```

- `check`: offline CI check of chain, checksums, models.
- `verify`: live schema vs models and ledger; read-only and safe in production.
- `rehearse`: pending up → down → up on a clone or restored backup, with **real data**. The recommended last gate.
- `lint`: statement risk, plus index advice.
- `check-sql`: `PREPARE` and `EXPLAIN` on every registered query.
- `baseline`: adopts an existing database as migration zero, without running anything.

---

## 11. Performance and memory

### `morm::Pool`

`deadpool-postgres` gives each task exclusive use of a connection, so concurrency is capped at pool size, and its statement cache has no size limit. tokio-postgres can do better: its methods take `&self`, and concurrent requests on one connection pipeline automatically.

| Lane | Used for | Behaviour |
|---|---|---|
| **Shared** | Single statements outside a transaction | Many tasks use one connection at once, with no checkout. Requests go to the connection with the fewest in flight |
| **Exclusive** | Transactions, streams, `COPY`, per-query timeouts, slow statements | Normal checkout |

Each connection is a whole process on the Postgres side, so needing fewer of them saves real memory.

**Effect on query speed:**

- **A query doesn't execute any slower,** but a connection runs one query at a time, so anything queued waits. If 10,000 1ms queries truly arrive at the same instant on 8 connections, the last waits about 1.25s.
- **More connections wouldn't fix that.** Postgres can only run as many queries at once as it has CPU cores free, and hundreds of active connections slow it down through contention.
- **Against an exclusive pool of the same size, the shared lane should be as fast or faster on average:** no checkout wait, and no idle gap per network round trip.
- **The real cost is tail latency.** One slow query delays everything behind it on that connection, where an exclusive pool would have sent the next query to a free one.

Mitigations:

- **Adaptive routing:** each cached statement tracks its average duration in an atomic. Anything above a threshold (about 5ms) goes to the exclusive lane automatically. `.exclusive()` forces it by hand.
- **Cap in-flight per connection** (about 16), and route to the least loaded.
- **Sizing:** default pool size is about 2× Postgres CPU cores, higher for disk-heavy workloads.
- **Decision gate:** milestone 2 measures p50, p99 and throughput against deadpool at equal and larger sizes, with a mix of fast and slow queries. If p99 is worse under mixed load after these mitigations, the exclusive lane becomes the default and the shared lane an opt-in.

Other details:

- **Each statement runs in its own implicit transaction,** so one failure doesn't affect others on the connection.
- **Timeouts need the exclusive lane.** `CancelToken` cancels whatever is running on that connection, which on the shared lane could be someone else's query. `SET LOCAL` is exclusive too. Session-level `SET` isn't offered at all, so no state leaks.
- **Backpressure:** bounded wait queue with a deadline and a typed `PoolBusy` error, rather than unbounded queuing.
- **Lifecycle:** connection `max_lifetime`, reconnect with backoff, `ping()`, graceful `close()`, statement prewarm at startup.

**Limit:** this replaces a pooler for one process. Multiple app processes each hold their own connections, so the total is processes × pool size against `max_connections`. One multi-threaded Tokio process is the recommended deployment. A very large fleet of separate app hosts is the one case this design doesn't cover.

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
- Also: shared lane vs deadpool, `fetch_ref` vs `fetch` vs `fetch_json`, binary size, and RSS on both app and Postgres.

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
| Queries in a loop | Detector in debug builds and `#[morm::test]`: the same statement repeating in one task warns, and fails the test |
| Filtering on an unindexed column | `morm-admin mgr check-sql` runs `EXPLAIN` on every registered query |

The compiler can't see a query inside a loop, so that one is caught in tests and debug builds, not at compile time.

---

## 13. Testing and CI

Tests are written with the code: **a milestone isn't done until its tests pass in CI.**

- **Unit:** macro output, SQL generation, diff, naming.
- **Compile-fail (`trybuild`):** every compile-time guarantee, including hidden view fields, `HasPk`, unfiltered update and delete, update with no `.set()`, list with no limit, `self` absent in update and delete hooks, non-static `q()`.
- **Integration on real Postgres:** insert, every get, update and delete form, views, groups, hook inputs (`Changes`, `Target`), hooks order, the loop detector, `after_commit` on commit and skip on rollback, all list and JSON paths, pool lanes, routing, cancellation.
- **Property tests:** random models and data round-trip through insert, read and JSON.
- **Doc tests:** every README and doc example compiles and runs.
- **`#[morm::test]`:** gives each test a database cloned from a migrated template, for this project and its users.

Migration tests:

- **Fault injection:** kill the process at every step boundary, then prove `--resume` and `rollback` both reach the right state.
- **Property tests:** random schema A → B with data, including views, functions and triggers: up reaches B, down returns to A, preserved rows are intact.
- **Op tests:** every op's inverse, with and without data, and every dependency recreate path.
- **Concurrency:** rival migrators, migration under live traffic, lock-timeout retry.
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

4. **Performance rules:** never query in a loop, always limit lists, select through the narrowest view, keep transactions short with no network calls inside them, index what you filter on, use `.exclusive()` for known slow queries.
5. Models: bases, every `#[model]` and `#[col]` option in tables.
6. `get`, `insert`, `update`, `delete`, lists, transactions.
7. Views, groups, validation.
8. Hooks, with a table of what each receives and this line: "update and delete hooks never see a row unless you fetch it."
9. JSON output and schemas.
10. Pool: sizing and tuning.
11. Migrations: workflow, every command in one table, safety rules, triggers and functions.

---

## 15. Milestones

0. **Repo bootstrap:** workspace in `biggaloy/rust-morm`, CI workflows, README skeleton.
1. **Core models:** macro, bases, views, groups, declarative rules and `clean`, inference, `F`, compile-time checks.
2. **Runtime:** `morm::Pool` with both lanes and adaptive routing, bounded statement cache, insert, `get`/`update`/`delete` forms, query builder, loop detector, typed errors, transactions, all list and JSON paths. Benchmark the pool here against the decision gate. → **First publish** (`0.1.0-alpha`).
3. **Migration foundation:** `Schema` IR, catalog reader, diff, typed ops with inverses, scratch, proof pipeline.
4. **Migration execution:** ledger, steps, fingerprint gates, advisory lock, timeouts, preflight, crash recovery, fault-injection suite.
5. **Migration safety:** safe forms, batched backfills, retire and purge, decisions, `lint`, `rehearse`, expand/contract, `check_schema()`.
6. **Managed objects:** views, materialized views, functions, procedures, triggers, policies, grants, enums and domains, with dependency recreation.
7. **Hooks:** `Changes`, `Target`, all hooks, `validate`, `validate_changes`, `after_commit`, automatic transactions.
8. **`morm-admin`:** `init`, `doctor`, `import`, `scratch`, `morm-mgr` packaging, Axum template.
9. **Query safety:** `sql!`, `check-sql`, upsert, UNNEST and COPY, `#[morm::test]`.
10. **Remaining features:** UUIDv7, `tls`, schemas and `utoipa`, optimistic locking, tracing, typed FKs, settings, pipelined transactions.
11. **Benchmarks, hardening, full README, 0.1.0 release.**

---

## 16. Risks, open items and later work

### Risks

- **Managed objects:** functions, triggers and policies with exact inverses are the hardest engine work.
- **Shared-lane pool:** unproven until the milestone 2 benchmark. If it disappoints, the exclusive lane alone still works as a normal pool.
- **Needs proof early:** borrowed `ProductRef` decoding, the loop detector, macro feasibility for `#[view]`, and hook signatures.
- **Scratch databases:** `makemigrations` needs a Postgres where it can `CREATE DATABASE`.
- **Scope:** migrations alone outweigh the runtime.

### Open items

- Decide the final MSRV (at least 1.85).
- Check that `morm-migrate` and `morm-admin` are free on crates.io.

### Later

Read replicas, `LISTEN/NOTIFY`, keyset pagination, `schema = "x"`, partitioning, a `dynamic` registry for by-name admin CRUD, sensitive-column prompts in `import`.

### Not carried over from python-morm

- **`save`:** replaced by explicit `insert` and `update`.
- **`exclude_values_up/down`:** rarely useful, and it would make insert SQL vary per row. `Option` `None` already defers to the DB default, and `skip_json_if` covers output. It can come back if a real need appears.
- **`validator` and `modifier`:** replaced by the type system, native decoding, and declarative rules.
- **`pre_save` and `post_save`:** removed with `save`.
