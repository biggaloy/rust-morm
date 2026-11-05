# Step 04: `#[model]`, names and types

- **Phase:** Models
- **Depends on:** 03
- **Can run beside:** 09, 10, 11
- **Crates:** `morm-macros`, `morm`, `morm-core`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Generate everything a model needs at compile time, with no runtime cost, from a plain struct.

## Deliverables

- `#[model]`, every struct and field option assigned here, `F`, generated SQL constants, `from_row`, `from_row_named`, `ProductRef`, `Default`, `new`, `HasPk`
- Name rules, type inference, the integer table with range CHECKs in metadata, `db_set`, `insert_only`, `flatten`, capability markers and the conversion layer

## End-to-end check

A model using every option expands to the expected SQL, and rows decode from a real Postgres table created by hand. All compile-time rules are proven with `trybuild`.

## Out of scope

- Indexes and bases (step 05)
- Running queries through `db`

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Models

**R-0118**

```rust
use morm::{model, Timestamp};

#[model(
    table = "products",
    base = "common",                 // adds id (identity bigint pk), created_at, updated_at
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

### Models / Table and column names

**R-0119** Generated SQL always quotes every table and column name, so reserved words like `order` and `type` work as they are.

**R-0120** **Names are lowercase only.** Postgres folds unquoted names to lowercase but keeps quoted ones exactly, so `userID` in hand-written SQL means `userid`, while `"userID"` is a different column. With lowercase names, quoted and unquoted always match, and the trap can't happen.

- **R-0121** An uppercase letter in `table` or `name` is a compile error. The defaults are already lowercase snake_case.
- **R-0122** morm can't treat everything as lowercase, because `"userID"` and `userid` are separate names in Postgres, and folding one would point at a column that doesn't exist.
- **R-0123** `keep_case` is the exception for existing tables, as in `#[col(name = "userID", keep_case)]`.
- **R-0126** Generated constraint and index names keep their `__UNQ_` style, because only morm uses them.
- **R-0127** A table or column name over 63 bytes is a compile error, because Postgres would silently cut it.

### Models / Struct options (`#[model(...)]`)

| ID | Option | Meaning |
|---|---|---|
| R-0144 | `table` | table name (default: snake_case of the struct name); lowercase only unless `keep_case` is set |
| R-0145 | `keep_case` | allow uppercase in an existing table's name |
| R-0148 | `ordering` | default sort, `-` prefix for descending |
| R-0149 | `was` | previous table name (rename hint for migrations) |
| R-0155 | `no_default` | don't generate `Default` |

### Models / Field options (`#[col(...)]`)

| ID | Option | Meaning |
|---|---|---|
| R-0159 | `sql` | SQL type; inferred from the Rust type when omitted |
| R-0160 | `default` | SQL default expression |
| R-0161 | `default_fn` | Rust function that supplies a default on insert |
| R-0162 | `pk` | primary key (with `base = "bare"`; several fields make a composite PK) |
| R-0165 | `not_null` | `Option<T>` field that is still NOT NULL in the DB (filled by a default) |
| R-0167 | `expr` | read-only computed column; excluded from writes and migrations |
| R-0168 | `flatten` | embed a reusable column set |
| R-0169 | `generated`, `identity`, `check` | generated column, identity column, check constraint. A generated column is never written, and `.set()` on it is a compile error |
| R-0171 | `deprecated` | removes the field from reads and writes while the column remains (rolling deploys) |
| R-0177 | `help` | description, used in schemas; defaults to the field's `///` doc comment |
| R-0178 | `auto_now` | set to `now()` on every update |
| R-0179 | `db_set` | Postgres always sets it on insert: the field's value is ignored, and the result is filled in place. Bases set it on `created_at` and `updated_at` |
| R-0180 | `insert_only` | written by `insert`, then never changed: left out of `upsert` updates, and `.set()`, `set_expr`, `update_each`, `.update_only` or a `patch` naming it is a compile error. Bases set it on `created_at`, and custom bases use it the same way |
| R-0181 | `name` | column name override, for example `type_` with `name = "type"`; lowercase only unless `keep_case` is set |
| R-0182 | `keep_case` | allow uppercase in an existing column's name |
| R-0183 | `was` | previous column name (rename hint) |

### Models / Type inference

**R-0184** `i16` → smallint, `i32` → integer, `i64` → bigint, `f32` → real, `f64` → double precision, `bool` → boolean, `String` → text, `Vec<u8>` → bytea, `DateTime<Utc>` / `Timestamp` → timestamptz, `NaiveDateTime` → timestamp, `NaiveDate` → date, `NaiveTime` → time, `Uuid` → uuid, `serde_json::Value` → jsonb, `Vec<T>` → `T[]`, `IpAddr` → inet, `morm::Range<T>` → the matching range type, such as `tstzrange` or `int8range`. `Option<T>` → nullable. Anything else requires an explicit `sql`, including `Decimal` and `Numeric`, which need their precision stated (section 8).

- **R-0188** **Integers Postgres has no exact type for** map to the smallest column that holds every value, with a generated range CHECK named `__CHK_{table}_{column}_range__`, so nothing outside the Rust type can be stored by any client:

| ID | Rust | Column | CHECK |
|---|---|---|---|
| R-0189 | `i8` | `smallint` | -128 to 127 |
| R-0190 | `u8` | `smallint` | 0 to 255 |
| R-0191 | `u16` | `integer` | 0 to 65,535 |
| R-0192 | `u32` | `bigint` | 0 to 4,294,967,295 |
| R-0193 | `u64` | `numeric(20,0)` | 0 to `u64::MAX` |
| R-0194 | `i128` | `numeric(39,0)` | `i128::MIN` to `i128::MAX` |
| R-0195 | `u128` | `numeric(39,0)` | 0 to `u128::MAX` |
| R-0196 | `NonZero*` | As the base type | Adds `<> 0` |
| R-0197 | `char` | `text` | `char_length = 1` |

- **R-0198** **`usize` and `isize` are compile errors,** because their size depends on the machine, so a value could fit on one build and not another. Use a fixed-width type.
- **R-0199** **A column must hold the whole Rust type.** An explicit `sql` can only widen: `u32` on `numeric` compiles, while `u64` on `bigint` or `i64` on `integer` is a compile error, so a value can't wrap or fail at runtime.
- **R-0200** **A wider column gets the range CHECK too,** so `i32` on `bigint` can't store a value the field can't read. If one still arrives, because the CHECK was removed outside morm, decoding returns `OutOfRange`, never a wrapped value.
- **R-0201** **`numeric` is larger and slower than `bigint`,** so `u64`, `i128` and `u128` columns and their indexes cost more. Use `i64` when values fit. `#[col(sql = "uuid")]` on `u128` is a compact alternative for ids and hashes: 16 fixed bytes, with no arithmetic in SQL.
- **R-0204** **`citext`** works on a `String` field with `sql = "citext"` and `extensions = "citext"`.

### Models / Plain structs, no `save`

**R-0207** There is no `save`. It would force either change tracking or full-row writes, and both are costs nobody asked for. Models are plain structs with public fields, and rows carry **zero** extra bytes.

**R-0208**

```rust
let mut p = Product { vendor_id: 1, sku: "A-1".into(), name: "Lamp".into(), ..Default::default() };
db.insert(&mut p).await?;        // fills id and timestamps in place
```

- **R-0209** `p.price = dec!(34)` is allowed, because it only changes your local copy. Nothing reaches the database without an explicit `update`.
- **R-0210** **Forgotten required fields:** `..Default::default()` hides them, so `no_default` removes `Default` when you want the compiler to insist. `Product::new(...)` takes required fields as arguments.

### Models / Generated per model

- **R-0212** compile-time SQL constants (`SELECT`, `GET`, `INSERT`, `DELETE`, `RETURNING`, `ORDER BY`)
- **R-0213** positional decoding (`from_row`), name-based decoding for raw queries (`from_row_named`), and a borrowed `ProductRef<'r>`
- **R-0214** `Product::F.<field>`: typed, quoted column references, checked at compile time; `Product::F.alias("p")` for JOINs, with prefix decoding for two models in one row
- **R-0215** `Default` (unless `no_default`), `Product::new(...)`, `Debug`, `Serialize`, `json_schema()`. `Deserialize` only as described in section 5
- **R-0218** an empty `impl Hooks` (unless `hooks` is set)
- **R-0219** schema metadata, compiled only into `morm-mgr`, with no model list to maintain

**R-0220** Compile-time checks: unknown columns in `ordering`, `indexes` and `include`; foreign keys whose types don't match; generic structs. Postgres checks `filter` and `expr` on scratch, because a macro can't parse SQL. Group limits are in section 5.

### Reading and writing / Insert and update

- **R-0358** **A non-`Option` field is always sent,** so generated `Default` and `new` must agree with its SQL default, or `active: bool` with `default = "true"` would quietly insert `false` through `..Default::default()`. The macro reads literal defaults (numbers, `true`, `false`, quoted strings) into `Default`. A non-literal default such as `gen_random_uuid()` on a non-`Option` field is a compile error, unless the field is `db_set`, the model has `no_default`, or it's written as `Option<T>` with `not_null`.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0742 | A Rust type wider than its column, or `usize` on a column | Compile error |
| R-0754 | A generated `Default` disagreeing with a SQL default | Literals are read into `Default`; other defaults on non-`Option` fields are compile errors |

### Risks, open items and later work / Later: field-level encryption

**R-0901** Encrypted columns, including per-user keys and rotation, are not designed yet. Nothing in 0.1 is built for them. These choices, which 0.1 needs anyway, keep them possible without breaking changes:

- **R-0902** `F` columns carry capability markers checked by the query API, so a column can refuse `.by()` or `set_expr`.
- **R-0903** Encode and decode go through one per-column conversion layer, so a column's Rust type can differ from its stored type. It exists only at compile time: generated code calls `ToSql` and `FromSql` directly through static dispatch, with no branch, pointer, allocation or per-row state. A benchmark against hand-written tokio-postgres decoding checks that it adds no cost.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0001 | suite | Unit: macro output |
| T-0006 | trybuild | `HasPk` |
| T-0011 | trybuild | a non-literal default on a non-`Option` field |
| T-0022 | trybuild | uppercase names without `keep_case` |
| T-0023 | trybuild | columns narrower than their Rust type |
| T-0024 | trybuild | `usize` and `isize` |
