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
- `_morm` is owned by the migrating role and holds the database's statement generation in its comment, `morm statement generation <n>`, which any role reads from `pg_description` with no grant; a database with no `_morm` or no such comment reads as `0` (SE). Every migration `tx` step ends, in its own transaction, with the reset `CREATE OPERATOR _morm.### (LEFTARG = int4, RIGHTARG = int4, FUNCTION = int4eq); DROP OPERATOR _morm.### (int4, int4); COMMENT ON SCHEMA _morm IS 'morm statement generation <n>'`, `<n>` one more than before, which marks every cached plan in every session invalid at commit (SA, step 33). Behind a pooler with the statement caches on, the text of each cached statement starts with `/* morm g<n> */`, or `/* morm g<n>.<salt> */` after DDL made outside morm (SD, step 14).
- Enum labels starting `_morm_` are reserved: a retired enum type's labels are renamed `_morm_retired_<label>` (SA, step 38).
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
// The macro writes the column's typing as constants, with const fn accessors, so sql! can
// concatenate them at compile time (DH2, proof 9): cast and cast_list, the column's type,
// an explicit `sql` included, with no length, precision or scale and then with [] (char(n)
// as bpchar, bit(n) as varbit, an enum, domain, composite or citext by its name, quoted as
// morm's DDL quotes it); and carrier and list_carrier, the built-in types the parameter is
// declared as in Parse and encoded as (text for an enum, composite, citext or extension
// type; a domain's base type's carrier for a domain). A parameter bound through a field
// (builders, F, sql!'s ${v: M::F.field}, model inserts and updates) is written into the
// SQL as ($n::cast) and declared as the carrier, so Postgres never infers its type
pub struct Column<M, T, Cap> { /* name, cast, cast_list, carrier, list_carrier, marker data */ }

// A Rust type's own SQL type (DH2, DH3): the cast morm writes for a free parameter, and the
// built-in carrier it's declared and encoded as. Every type with no fixed OID (PgEnum,
// PgDomain, PgComposite, and arrays of them) travels as text, or a domain as its base
// type's carrier, and is cast to its name, so no OID is ever looked up or sent and a type
// swap can't leave one stale. Every other type uses its own built-in type.
// It's also DH3's column-type trait: CARRIES lists the column types (cast names) a field
// of this type can sit on without loss, each encoded by the conversion layer as that
// column's carrier, and a type with TEXT_FORM (String, &str, Box<str>, Cow<str>, and a list
// of one) may also sit on a SQL type morm doesn't know, carried as text. PgEnum, PgDomain,
// PgComposite and TextEnum have no text form and sit only on their own types (DJ5). Every
// integer type, unsigned and NonZero* included, carries every wider integer column and
// numeric, and u128 also uuid; Vec<T> carries each of T's types with [] (DJ1, R-1213).
// #[model] checks a field's `sql` against them in a const
// assertion, so a type alias resolves, and the error names the field, its type and the
// `sql`. A user type implements it to be a field or a free parameter (steps 13 and 56)
pub trait SqlType {
    const CAST: &'static str;
    const CAST_LIST: &'static str;
    const CARRIER: tokio_postgres::types::Type;
    const LIST_CARRIER: tokio_postgres::types::Type;
    const CARRIES: &'static [&'static str];
    const TEXT_FORM: bool;
    // EA: a result column of this type is read as "col"::text, or ::text[] for a list, in every
    // column list morm writes, so the driver never looks its type up with named statements of
    // its own, which fail behind PgBouncer with no prepared statements. True for PgEnum,
    // PgComposite, a PgDomain over one of them and a list of any of these; #[model] and #[view]
    // also read as text a TEXT_FORM field whose sql names a type morm doesn't know (step 56)
    const READ_AS_TEXT: bool;
}
// A parameter, object safe for &dyn Param in runtime fragments (R-0723), with a blanket
// impl from SqlType: Option<T> and &T take T's cast and carrier, Vec<T> and &[T] T's list
// ones. Every carrier is built in, so every parameter goes in binary format, from
// &[&dyn Param] into query_typed_raw or a prepared statement through an iterator, with
// nothing boxed or collected
pub trait Param: tokio_postgres::types::ToSql + Sync {
    fn cast(&self) -> &'static str;
    fn carrier(&self) -> tokio_postgres::types::Type;
    fn is_null(&self) -> bool;
}
// Proof 13 (step 55) and EA: a PgEnum's ToSql accepts only the text carrier and writes the
// label; its FromSql accepts only text, because every enum result column is read as ::text,
// and never reads a variant list. A PgComposite's ToSql writes its text form into a text
// carrier, and its FromSql parses Postgres's text form, (a,b): nothing between delimiters is
// NULL, "" an empty string, a backslash escapes the next character, "" inside quotes is one
// ", each position and length is checked, and bad input is an error, never a panic (EE9).
// Lists are Vec<T> with the text[] carrier, read as ::text[]. tokio-postgres never sees an
// enum or composite result column, so it never looks one up (R-1241, R-1242, R-1243)
// A value fits a field of type T without loss, and is encoded as that column's carrier.
// An Option value fits only an Option field, so NULL is bound only through a nullable
// field; a non-Option value fits an Option field (K). FitsList<T> does the same element
// by element, for a list bound with [] (sql!'s ${v: M::F.field[]}, .any). Both are object
// safe and Sync (DJ4): comp's values are &[&dyn Fits<T>] (R-1222, R-1227), held across an
// .await in a future that must be Send under tokio::spawn
pub trait Fits<T>: Sync {
    fn encode(&self, carrier: &tokio_postgres::types::Type, out: &mut bytes::BytesMut)
        -> std::result::Result<tokio_postgres::types::IsNull, Box<dyn std::error::Error + Sync + Send>>;
}
pub trait FitsList<T>: Sync { /* the same, element by element, as the column's list carrier */ }
// What sql! returns (DJ3): the static text with every cast written in, its registration,
// and P, that query's own set of values, a tuple the compiler fills in at each sql!, each
// value a borrow tagged with its carrier (through its field by Fits, free by SqlType), so
// nothing is boxed or allocated. sql! writes it as nested struct literals (hence the
// #[doc(hidden)] public fields), so `let q = sql!(..)` keeps any temporary a value makes
// alive as long as q. A query can be stored in a variable, passed around and run
// elsewhere, or run several times; a function returning one writes
// `-> Sql<'a, impl Params>`. db.sql() and db.raw() are generic over P and take a Sql or a
// reference to one (B). sql_unchecked(String, &[&dyn Param]) is the visible escape hatch,
// with the scanner's casts. Proven on rustc 1.99.1 (proofs.md, DJ3)
pub struct Sql<'a, P> { /* #[doc(hidden)] pub text, registration, params: P, PhantomData<&'a ()> */ }
// One sql! query's values: implemented for flat tuples of carrier-tagged borrows, from 0 to
// 64 values; sql! refuses a 65th at compile time, naming the limit (DK6). Send + Sync
// (DK3): Sync so a &Sql held across an .await keeps the future Send, and Send so a Sql
// taken by value does, as Conn's entry points, declared + Send for every P, need under
// &impl Conn in a hook. bind() hands each value to query_typed_raw or a prepared statement
// from an array iterator, with nothing collected (64 values proven on rustc 1.99.1, DK6)
pub trait Params: Send + Sync {
    fn bind(&self) -> impl ExactSizeIterator<Item = (&(dyn tokio_postgres::types::ToSql + Sync), tokio_postgres::types::Type)> + Send;
}
// The conversion layer (Defined in step 03, below) encodes a value bound through a field
// as that column's carrier where the value's own encoder doesn't take it (DG1): a narrower
// integer as the wider column's, f32 as float8, an instant (DateTime<Utc>, Timestamp) on
// timestamp as its UTC wall time, a NaiveDateTime on timestamptz as UTC, IpAddr on cidr,
// String on json or jsonb as a text carrier cast to the column's type, each element alike
// for a list or an array. Sessions always run with TimeZone = 'UTC', sent through PgBouncer
// in transaction mode too (DJ2, step 12), and nothing overrides it (DH1, step 10; a
// `timezone` key is Error::Config, DI2), and the startup check refuses a
// pool whose session reports another zone (DI1, step 41), so raw SQL converts times as
// these encodings do. Step 03 freezes the
// layer's shape; step 13 builds the conversions and the refusal (R-1207, R-1208, T-0278)

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
impl<'d> TxOptions<'d, Once> {
    pub fn retry(self, n: u32) -> TxOptions<'d, Retry>;
    pub async fn transaction<F, T>(self, f: F) -> Result<T> where F: AsyncFnOnce(&Tx) -> Result<T>;
}
impl TxOptions<'_, Retry> {
    // An attempt that could still be retried runs a clone; the last allowed runs the original.
    // A closure mutating a capture holds a &mut, which isn't Clone, so it doesn't compile
    pub async fn transaction<F, T>(self, f: F) -> Result<T> where F: AsyncFnOnce(&Tx) -> Result<T> + Clone;
}
// Both TxOptions impls also have isolation(..) and each setting, returning Self

// Inside a transaction (DH5). tx has every Conn query entry point, and these:
impl Tx {
    // A savepoint: released when the closure returns Ok, rolled back to when it returns an
    // error (R-0441). There's no TxOptions on tx: no tx.isolation(..) or tx.retry(n), so
    // both are compile errors, because the isolation level is fixed before a transaction's
    // first query, and a savepoint can't recover from 40001 or 40P01
    pub async fn transaction<F, T>(&self, f: F) -> Result<T> where F: AsyncFnOnce(&Tx) -> Result<T>;
    // Each setting is immediate, one SET LOCAL, and holds for the rest of the transaction,
    // through savepoints that are released; rolling back to a savepoint opened before it
    // undoes it, as Postgres does: tx.lock_timeout(d).await?. On 15 and 16,
    // transaction_timeout moves morm's client-side timer instead (R-0471)
    pub async fn statement_timeout(&self, d: Duration) -> Result<()>;
    pub async fn lock_timeout(&self, d: Duration) -> Result<()>;
    pub async fn transaction_timeout(&self, d: Duration) -> Result<()>;
    // set_config(name, (value cast to its own type)::text, true), for RLS
    pub async fn set_local(&self, name: &'static str, value: &dyn Param) -> Result<()>;
}

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
                                               // Serialization also: a FATAL 40001 recovery conflict on a hot standby that
                                               // ended the connection inside a transaction, before COMMIT (SG, step 20)
    TransactionTimeout, TransactionAborted, CommitUnknown, NestedConnection,
    PoolerMode, NotFound, EmptyPatch, Conflict, Forbidden, Validation,
    PoolBusy, ConnectionLost, ReadOnly, OutOfRange, InvalidText,
                                               // ConnectionLost on a held connection names a recovery conflict when one ended it (SG, step 12)
    InvalidOrder, InvalidCursor, Decimal,
    SchemaChanged,                             // a stale statement or session inside a transaction or on a held connection, after a type swap
                                               // or a column type change, recognised by SQLSTATE with the routine, an identifier or an
                                               // untranslated elog message, never translated text (SC, R-1245): 0A000, the three plpgsql
                                               // 42804 routines, XX000 cache lookup, and 22P02 from enum_in naming a _morm_retired_ type
                                               // or another enum than the statement's cast (proofs 14 to 17)
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
| The internal query entry point, which runs SQL through the caches. It can carry a "don't count this" flag for the loop detector, so step 45's exclusions (R-0785) need no signature change (proof 11, DD). Behind a pooler with the caches on, the text it prepares for a cached statement carries the statement generation's marker (SD) | `morm` | Pool, statements, every query step |
| Macro field IR: the parsed form of `#[model]`, `#[col]` and `#[view]` | `morm-macros` | All model steps |
| The conversion layer between a field and its column | `morm` | Models, types, reads, writes |
| `SqlType`, the trait that gives a column and a free parameter its cast, carrier and the column types it carries without loss (DH2, DH3), and, for an enum, its labels as a constant; `Param`, `Fits`, `FitsList`, `Sql<'a, P>` and `Params` | `morm` | Models, types, statements, reads, writes, manifest |
| The cast-name function: a column type to its cast, with no length, precision or scale (`char(n)` to `bpchar`, `bit(n)` to `varbit`), which `#[model]` and `sql!` call at compile time | `morm-core` | Macros, types, statements, `check_schema` |
| `ModelMeta`, the metadata every model emits | `morm-core` | Models, manifest, migrations |
| The name function, with the 63-byte rule | `morm-core` | Macros, migrations |
| The connection budget function, implemented in step 11 | `morm-core` | Pool, `doctor`, `size` |
| `Timestamp`, re-exported `Uuid`, `Range<T>` as types | `morm` | Bases and inference, before `types` adds their encoding |
| The internal transaction with confirmed commit, implemented in step 17 for `insert_many` and chunked paths, before `db.transaction` exists | `morm` | Inserts, updates, upsert, transactions |
| `PoolStats`, with every counter the plan names, including connections replaced for a stale generation, stale statements retried unnamed behind a pooler, and the pool's current marker, `g<n>` or `g<n>.<salt>` (SD) | `morm` | Pool, hold-pooler, locks, outbox, telemetry |
| The internal warning sink: stderr now, `tracing` when enabled | `morm-core` | Every step that warns |
| Expansion layout: `#[model]` calls one module per feature, so model steps add a module and don't edit one shared function | `morm-macros` | Model steps, `manifest`, `rolling` |
| The config structs | `morm-core` | `morm`, `morm-migrate`, `morm-admin` |
| The SQL text scanner, which renumbers each `$n` in a fragment and writes its cast as `($n::cast)` (DH2) | `morm-core` | Statements, `sql!` |

`Op`, the typed migration operation, is defined in step 30. Migration steps run one at a time, so it needs no earlier freeze.

## Between `morm-mgr` and `morm-admin`

Registration is proven in `proofs.md`, proof 10: `#[model]` calls a `macro_rules!` that `morm` defines under `cfg(morm_mgr)` and `cfg(not(morm_mgr))`, so user crates never see the cfg, and the generated `morm-mgr` names each crate with models as `use <crate> as _;`, because a crate only listed in `Cargo.toml` is dropped with its models.

`morm-mgr` prints one JSON manifest: `format`, then models (table, schema, columns, types, nullability, defaults, flags, indexes, constraints, enum labels, `was` hints, comments), managed objects, extensions, and registered `sql!` queries, each with its text, file and line, and for each parameter its number, kind (field, field list or free), cast, model and column, and its `${..}` source (DH2). Step 28 defines it, in `morm-core`. Everything else reads it, and it's versioned by `format`.

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
