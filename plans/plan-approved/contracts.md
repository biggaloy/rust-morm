# Contracts

What parallel steps share. Signatures here are the starting point, taken from the approved plan. Steps 02 and 55 correct them from the proofs, and step 03 freezes them as compiling Rust. After step 03, changing anything here follows "Changing the plan" in `rules.md`.

## Crates

| Crate | Depends on | Holds |
|---|---|---|
| `morm-core` | no async or database crates | Code several crates need: config loading (`morm.toml`, environment), the 63-byte name function, the connection budget function, and the manifest types |
| `morm-macros` | `morm-core` | Procedural macros |
| `morm` | `morm-macros`, `morm-core`, `tokio-postgres`, `deadpool` | Runtime |
| `morm-migrate` | `morm-core`, `tokio-postgres` | Migration engine. Doesn't depend on `morm` |
| `morm-admin` | `morm-migrate`, `morm-core` | The binary. Doesn't depend on the user's models |
| `morm-mgr` | generated per project; `morm` and the user's models | Prints the manifest as JSON, and runs a migration's Rust data hooks |

## Conventions

- Rust 2024, MSRV 1.99.1, Tokio only, PostgreSQL 15 and later only.
- Tool prefix `morm-`. Config `morm.toml`. Environment `MORM_*` and `DATABASE_URL`. Migrations in `morm-migrations/`. Ledger schema `_morm`.
- Exit codes: `0` ok, `1` error or drift, `2` refused by a guard, `3` open decisions. A run with both a refusal and an open decision exits `2`.
- Generated SQL always quotes names. Generated code uses static dispatch.
- Macro-generated items are public API: names below don't change without following `rules.md`.

## Shared types

```rust
// Implemented by #[model] and #[view]
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub trait Model: Sized + Send + Sync + 'static {
    type Pk: PkValue;                 // () for a model with no primary key
    type Global: MaskBits;            // the app's global mask, from morm::groups!
    type Scoped: MaskBits;            // scoped group mask, () when the model has none
    const TABLE: &'static str;
    const SELECT: &'static str;
    fn from_row(row: &tokio_postgres::Row) -> Result<Self>;
    fn from_row_named(row: &tokio_postgres::Row) -> Result<Self>;
    // A column whose type changed: reads each field through the decoder the
    // statement chose once, from the column's real type, when it was prepared
    fn from_row_as(row: &tokio_postgres::Row, decoders: &Decoders<Self>) -> Result<Self>;
}
// One statement's choice: each field's column position and the decoder for its real type,
// kept with the statement in its cache entry
pub struct Decoders<M: Model> { /* private */ }
pub trait HasPk: Model {}             // absent on models with no primary key
pub trait PkValue: Send + Sync + 'static {}   // one Param, or a tuple for a composite key

// Product::F.<field>. Cap is a zero-sized marker type generated for that one column, so
// every column is its own type (proof 8). Capability traits implemented on the marker
// let a column refuse .set(), .by() or set_expr.
// It carries the column's SQL type, including an explicit `sql`, as a ParamType, so a
// parameter bound to a known column (builders, F, sql!, model inserts and updates) is
// declared as the column's type, never the value's (DF1, proof 9)
pub struct Column<M, T, Cap> { /* name, ParamType, marker data */ }
pub enum ParamType {
    Builtin(tokio_postgres::types::Type),   // a built-in type, sent as its fixed OID
    NoOid,                                  // citext, an enum, any type with no fixed OID: Type::UNKNOWN
}

// A parameter: ToSql plus the type it's declared as when no column is known (raw SQL),
// because query_typed needs a type. String and &str return Type::UNKNOWN, so Postgres
// infers the type from the SQL; PgEnum returns Type::UNKNOWN too, so no OID is ever sent
// and a type swap can't leave one stale. Every other type returns its own type.
// A parameter declared Type::UNKNOWN that has a text form is sent in text format, so
// Postgres reads it as it reads a quoted literal: in binary, "1234" compared with an
// int4 reads as 825373492 (proof 9). The parameters go from &[&dyn Param] into
// query_typed_raw through an iterator of a small wrapper that sets each one's format:
// nothing is boxed or collected
pub trait Param: tokio_postgres::types::ToSql + Sync {
    fn pg_type(&self) -> tokio_postgres::types::Type;
    fn has_text(&self) -> bool;                         // String, &str, lists of them, PgEnum
    fn encode_text(&self, out: &mut bytes::BytesMut);   // the text form, for UNKNOWN
    fn is_null(&self) -> bool;
}
// Where an explicit `sql` gives a field a column its type's encoder doesn't take, such as
// sql = "bigint" on an i32 or "timestamp" on a DateTime<Utc>, is an open question to the
// owner (proofs.md, proof 9, question 1); step 03 freezes the answer

// Product::UK.<name>, and columns that are unique keys. Implemented for a unique
// column's Column type and for Key<M, K>, one per expression, partial or composite key
pub trait UniqueKey<M> { const CONFLICT: &'static str; }
pub struct Key<M, K>(/* marker */);

pub enum Patch<T> { Missing, Set(T) }
pub enum OrDefault<T> { Default, Set(T) }
pub struct WriteOnly<T>(/* private */);

pub struct Bits<const N: usize>([u64; N]);
// What generic code needs from a mask of any size: implemented for every Bits<N> and ().
// morm::groups! emits its mask as its own newtype over Bits<N>, not an alias, so a group
// from another groups! is a type error (proof 3)
pub trait MaskBits: Copy + Send + Sync + 'static { const EMPTY: Self; fn set(&mut self, bit: u16); fn has(&self, bit: u16) -> bool; }
pub struct Mask<M: Model> { global: M::Global, scoped: M::Scoped }
// morm::groups! emits `G` as both a struct, one GlobalGroup<Mask> field per group, and
// `pub const G: G`, so one `use crate::groups::G;` brings in both, plus
// `impl GlobalGroups for G { type Mask = Mask; }`. #[model] fills Model::Global from the
// G in scope: `type Global = <G as GlobalGroups>::Mask;`, and each group's bit from
// `G.<name>`. A model that names no global group gets `type Global = ()` (proof 3, DF5)
pub trait GlobalGroups { type Mask: MaskBits; }

// Connection-like: db, a held connection, or tx
pub trait Conn: Send + Sync { fn tx(&self) -> Option<&Tx>; /* every query entry point */ }

// Transactions (proof 4, DA, DF2). Every option goes before the closure, on one prefix
// builder, and the closure is always last, because a postfix modifier needs a named or
// boxed future that isn't Send for a borrowing closure on stable:
//   db.transaction(async |tx| ..)
//   db.isolation(Serializable).retry(3).transaction(async |tx| ..)
// db.isolation(..), db.retry(n) and each per-transaction setting start a TxOptions, and
// they chain in any order
impl Db {
    pub async fn transaction<F, T>(&self, f: F) -> Result<T> where F: AsyncFnOnce(&Tx) -> Result<T>;
    pub fn isolation(&self, i: Isolation) -> TxOptions<'_, Once>;
    pub fn retry(&self, n: u32) -> TxOptions<'_, Retry>;
    // and one starter per per-transaction setting: statement_timeout, lock_timeout,
    // transaction_timeout
}
pub struct TxOptions<'d, R> { /* db, isolation, settings, R */ }
pub struct Once;                 // no retry: the closure runs once and may move or mutate captures
pub struct Retry(/* n */);       // with retry: the closure must be Clone
impl TxOptions<'_, Once> {
    pub fn retry(self, n: u32) -> TxOptions<'_, Retry>;
    pub async fn transaction<F, T>(self, f: F) -> Result<T> where F: AsyncFnOnce(&Tx) -> Result<T>;
}
impl TxOptions<'_, Retry> {
    // An attempt that could still be retried runs a clone; the last allowed runs the original.
    // A closure mutating a capture holds a &mut, which isn't Clone, so it doesn't compile
    pub async fn transaction<F, T>(self, f: F) -> Result<T> where F: AsyncFnOnce(&Tx) -> Result<T> + Clone;
}
// Both TxOptions impls also have isolation(..) and each setting, returning Self

// Every hook has an empty default body, so an empty `impl Hooks` compiles away.
// #[model] emits an empty impl for every model, unless the model says #[model(hooks)];
// then #[morm::hooks] on the user's impl writes OVERRIDES, which has no default, so an
// impl without the attribute doesn't compile. #[morm::hooks] refuses a macro call among
// the impl's items and a hand-written OVERRIDES. Overrides<M> has no public constructor
// or constant and is typed by its model; the macros build it through a #[doc(hidden)]
// morm::__private::overrides(&[Hook::..]). Generic code asks M::OVERRIDES.has(Hook::..)
// and calls only those hooks (proof 5, DB, DF3)
pub struct Overrides<M: ?Sized>(/* private */);
pub enum Hook { Validate, PreInsert, PostInsert, ValidateChanges, PreUpdate, PostUpdate, PreDelete, PostDelete, AfterCommit }
pub trait Hooks: Model {
    const OVERRIDES: Overrides<Self>;
    fn validate(&self) -> Result<()> { Ok(()) }
    fn pre_insert(rows: &mut [Self], db: &impl Conn) -> impl Future<Output = Result<()>> + Send { async { Ok(()) } }
    fn post_insert(rows: &[Self], db: &impl Conn) -> impl Future<Output = Result<()>> + Send { async { Ok(()) } }
    fn validate_changes(changes: &Changes<Self>) -> Result<()> { Ok(()) }
    fn pre_update(changes: &mut Changes<Self>, target: &Target<Self>, db: &impl Conn) -> impl Future<Output = Result<()>> + Send { async { Ok(()) } }
    fn post_update(changes: &Changes<Self>, target: &Target<Self>, affected: u64, db: &impl Conn) -> impl Future<Output = Result<()>> + Send { async { Ok(()) } }
    fn pre_delete(target: &Target<Self>, db: &impl Conn) -> impl Future<Output = Result<()>> + Send { async { Ok(()) } }
    fn post_delete(target: &Target<Self>, affected: u64, db: &impl Conn) -> impl Future<Output = Result<()>> + Send { async { Ok(()) } }
    fn after_commit(event: Committed<Self>) -> impl Future<Output = Result<()>> + Send { async { Ok(()) } }
}
pub enum Target<M: Model> { Id(M::Pk), Ids(Vec<M::Pk>), Filter(/* condition */), All }
pub enum Change<'a, T> { Value(&'a T), Expr }
pub enum Committed<M: Model> {
    Inserted(Vec<M>),
    Copied { count: u64 },
    Updated { changes: Changes<M>, target: Target<M> },
    Upserted { inserted: u64, updated: u64 },
    Deleted { target: Target<M> },
}
pub enum Upserted { Inserted, Updated, Unchanged }
// db.upsert returns Upserted; db.upsert_many returns counts

pub enum Error {
    Unique { constraint: String, field: Option<&'static str> },
    ForeignKey, NotNull, Check, Serialization, Deadlock, LockTimeout,
    TransactionTimeout, TransactionAborted, CommitUnknown, NestedConnection,
    PoolerMode, NotFound, EmptyPatch, Conflict, Forbidden, Validation,
    PoolBusy, ConnectionLost, ReadOnly, OutOfRange, InvalidText,
    InvalidOrder, InvalidCursor, Decimal,
    SchemaChanged,                             // a stale statement or session inside a transaction or on a held connection, after a type swap
    UnknownEnumValue,                          // a label this binary's enum doesn't have, with the type and label
    SchemaMismatch,                            // check_schema's findings, from a pool's start or db.check_schema(), each naming the table, column, both definitions and what's wrong
    Config, Connect, ServerTooOld, Postgres,   // config, connecting, a server below 15, any other database error
    // payloads settled in step 03
}
```

## Defined in step 03

These are shared by steps that can run side by side, so each is written and frozen before any of them starts:

| Item | Lives in | Shared by |
|---|---|---|
| `Db`, `Tx` and every `Conn` entry point | `morm` | All runtime steps |
| `Changes<M>` | `morm` | Updates, hooks |
| The `by`/`any` filter builder | `morm` | Reads, updates |
| The internal query entry point, which runs SQL through the caches. It can carry a "don't count this" flag for the loop detector, so step 45's exclusions (R-0785) need no signature change (proof 11, DD) | `morm` | Pool, statements, every query step |
| Macro field IR: the parsed form of `#[model]`, `#[col]` and `#[view]` | `morm-macros` | All model steps |
| The conversion layer between a field and its column | `morm` | Models, types, reads, writes |
| The trait that gives a column its SQL type and, for an enum, its labels as a constant | `morm` | Models, types, manifest |
| `ModelMeta`, the metadata every model emits | `morm-core` | Models, manifest, migrations |
| The name function, with the 63-byte rule | `morm-core` | Macros, migrations |
| The connection budget function, implemented in step 11 | `morm-core` | Pool, `doctor`, `size` |
| `Timestamp`, re-exported `Uuid`, `Range<T>` as types | `morm` | Bases and inference, before `types` adds their encoding |
| The internal transaction with confirmed commit, implemented in step 17 for `insert_many` and chunked paths, before `db.transaction` exists | `morm` | Inserts, updates, upsert, transactions |
| `PoolStats`, with every counter the plan names, including connections replaced for a stale generation | `morm` | Pool, hold-pooler, locks, outbox, telemetry |
| The internal warning sink: stderr now, `tracing` when enabled | `morm-core` | Every step that warns |
| Expansion layout: `#[model]` calls one module per feature, so model steps add a module and don't edit one shared function | `morm-macros` | Model steps, `manifest`, `rolling` |
| The config structs | `morm-core` | `morm`, `morm-migrate`, `morm-admin` |
| The SQL text scanner | `morm-core` | Statements |

`Op`, the typed migration operation, is defined in step 30. Migration steps run one at a time, so it needs no earlier freeze.

## Between `morm-mgr` and `morm-admin`

Registration is proven in `proofs.md`, proof 10: `#[model]` calls a `macro_rules!` that `morm` defines under `cfg(morm_mgr)` and `cfg(not(morm_mgr))`, so user crates never see the cfg, and the generated `morm-mgr` names each crate with models as `use <crate> as _;`, because a crate only listed in `Cargo.toml` is dropped with its models.

`morm-mgr` prints one JSON manifest: `format`, then models (table, schema, columns, types, nullability, defaults, flags, indexes, constraints, enum labels, `was` hints, comments), managed objects, extensions, and registered `sql!` queries. Step 28 defines it, in `morm-core`. Everything else reads it, and it's versioned by `format`.

## `morm-admin` layout

Each command is its own module, registered in one table, so steps adding commands don't edit the same code. Step 31 creates this dispatcher. It uses no CLI framework.

## Approved key decisions

These are context for every step. Details live in the step files.

### 

**R-0001** A complete re-engineering of morm in Rust: define PostgreSQL models in code, track them with rock-solid up and down migrations, and write queries in plain SQL. There is no extra query language. This is a new design, not a port. python-morm is a reference for what real projects need, and nothing is kept just for compatibility.

- **R-0002** **Repo:** `biggaloy/rust-morm`
- **R-0003** **Crates:** `morm`, `morm-macros`, `morm-migrate`, `morm-admin`.
- **R-0004** **Database:** PostgreSQL 15 and later only.
- **R-0005** **Naming:** everything in the project uses the `morm-` prefix.

### Goals (in priority order)

- **R-0006** **Highest raw performance:** minimal work per query; everything that can be decided at compile time is.
- **R-0007** **Lowest memory footprint:** no per-row overhead, no unnecessary allocations, few dependencies, small binary, few Postgres connections.
- **R-0008** **Rock-solid up and down migrations:** proven before they're written, checked when they run, recoverable from any failure.
- **R-0009** **Hard to misuse:** the cheap, correct way is the obvious way, and costly mistakes fail at compile time or in tests.
- **R-0010** **morm's ergonomics:** models in code, automatic migrations, raw SQL for queries.

**R-0011** Non-goals: other databases, a query language that replaces SQL, runtime reflection, automatic or lazy relation loading, implicit soft-delete filters, automatic change tracking.

### Key decisions

| ID | Topic | Decision |
|---|---|---|
| R-0012 | Driver | `tokio-postgres` |
| R-0013 | Pool | `morm::Pool`: the `deadpool` core crate with our own connection manager. Dynamic size, one exclusive checkout per query |
| R-0014 | Configuration | `morm.toml`, shared by the app and `morm-admin`, so settings need no Rust |
| R-0015 | Long-running work | Jobs and crons use the pool normally. `db.hold()` is only for session state |
| R-0016 | Deadlocks and commits | Commits are confirmed, a second connection on a task is refused, pool and lock waits are finite, and single statements retry once on a reported deadlock (section 8) |
| R-0017 | Statement cache | Two bounded per-connection LRUs: `statement_cache` for compile-time SQL, `shape_cache` for SQL built at runtime, so a client's choice of fields can't evict fixed queries. `query_typed` for dynamic SQL and overflow |
| R-0018 | Connection | Unix socket by default (`host=/var/run/postgresql`), the lowest latency and allows peer auth. TCP also supported |
| R-0019 | TLS | Optional cargo feature `tls` (rustls). Not compiled in unless enabled. Compiling it in costs binary size and build time; RAM and CPU are only spent on connections that use TLS |
| R-0020 | Default primary key | `bigint GENERATED BY DEFAULT AS IDENTITY` (`base = "common"`), the fastest and smallest. Same size and speed as `bigserial`, and the app role needs no separate sequence grant |
| R-0021 | Alternative primary key | UUIDv7 (`base = "common_uuid"`), generated in the app or by Postgres 18's `uuidv7()`. For ids in public URLs and offline clients |
| R-0022 | Custom primary key | `base = "bare"`: no columns added; you define your own single, composite, or no PK |
| R-0023 | Base models | Built-in `common`, `common_uuid`, `pk_only`, `pk_uuid_only`, `bare`, plus your own with `#[morm::base]` |
| R-0024 | Change tracking | None. There is no `save`. Updates name their columns, and never load a row first |
| R-0025 | Rows | Plain structs with public fields and zero per-row overhead |
| R-0026 | Field access control | Views at compile time, groups at runtime. Global groups mean the same everywhere and apply to fields and whole models; model-scoped groups apply to one model's fields. `sudo` is a built-in global group |
| R-0027 | Validation | One mechanism: declarative rules and `clean`, generated inline, run only on write |
| R-0028 | Hooks | Trait with zero-cost default methods. Update and delete hooks receive only what the statement contains |
| R-0029 | JSON | Direct from a row, with no intermediate `Value` or map. Postgres-built JSON also available |
| R-0030 | Production migrations | Forward only by default. A database is treated as production until marked otherwise, and `revert` undoes a migration with a new forward migration |
| R-0031 | Reliable events | A built-in transactional outbox: `tx.emit` saves an event with the data, and a worker delivers it at least once |
| R-0032 | Migration source of truth | Models and chain replayed in scratch databases, compared through the Postgres catalog. No snapshots |
| R-0033 | Admin | One tool, `morm-admin`, for everything administrative |
| R-0034 | Rust edition / MSRV | 2024 edition; MSRV 1.99.1, the current stable. Moves up to the latest stable until the first release |
| R-0035 | Decimal | `numeric` maps to `Decimal` where it fits exactly and to `morm::Numeric` otherwise. Values are never silently rounded |
| R-0036 | Names | morm's `__UNQ_`, `__IDX_`, `__FK_` convention, shortened with a hash at Postgres's 63-byte limit |
| R-0037 | License | BSD-3-Clause (same as python-morm) |
