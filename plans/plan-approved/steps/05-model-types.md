# Step 05: Type inference and decoding

- **Phase:** Models
- **Depends on:** 04
- **Can run beside:** 10, 11, 12
- **Crates:** `morm-macros`, `morm`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Map Rust types to columns, and decode rows with no needless allocation.

## Deliverables

- Type inference, the integer table with range CHECKs in metadata, the compile error for `identity` on a non-integer column, `from_row`, `from_row_named`, `ProductRef`, `from_row_as`, capability markers on `F`, the conversion layer

## End-to-end check

A model with every inferred type decodes rows from a real Postgres table created by hand, borrowed and owned, except the types steps 13 and 56 own: `Decimal`, `Numeric`, `Range<T>`, `IpAddr`, enums, domains and composite types. Each compile-time rule is proven with `trybuild`.

## Out of scope

- Encoding exactness and `Decimal` (step 13)
- The decoders for a column whose type differs from the one the field was built against (step 13), and choosing them per statement (step 14). This step's `from_row_as` reads through whatever decoder the statement chose, and is tested here with the choice for matching types

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Models / Type inference

**R-0186** `i16` → smallint, `i32` → integer, `i64` → bigint, `f32` → real, `f64` → double precision, `bool` → boolean, `String` → text, `Vec<u8>` → bytea, `DateTime<Utc>` / `Timestamp` → timestamptz, `NaiveDateTime` → timestamp, `NaiveDate` → date, `NaiveTime` → time, `Uuid` → uuid, `serde_json::Value` → jsonb, `Vec<T>` → `T[]`, `IpAddr` → inet, `morm::Range<T>` → the matching range type, such as `tstzrange` or `int8range`. `Option<T>` → nullable. Anything else requires an explicit `sql`, including `Decimal` and `Numeric`, which need their precision stated (section 8).

- **R-0190** **Integers Postgres has no exact type for** map to the smallest column that holds every value, with a generated range CHECK named `__CHK_{table}_{column}_range__`, so nothing outside the Rust type can be stored by any client:

| ID | Rust | Column | CHECK |
|---|---|---|---|
| R-0191 | `i8` | `smallint` | -128 to 127 |
| R-0192 | `u8` | `smallint` | 0 to 255 |
| R-0193 | `u16` | `integer` | 0 to 65,535 |
| R-0194 | `u32` | `bigint` | 0 to 4,294,967,295 |
| R-0195 | `u64` | `numeric(20,0)` | 0 to `u64::MAX` |
| R-0196 | `i128` | `numeric(39,0)` | `i128::MIN` to `i128::MAX` |
| R-0197 | `u128` | `numeric(39,0)` | 0 to `u128::MAX` |
| R-0198 | `NonZero*` | As the base type | Adds `<> 0` |
| R-0199 | `char` | `text` | `char_length = 1` |

- **R-0200** **`usize` and `isize` are compile errors,** because their size depends on the machine, so a value could fit on one build and not another. Use a fixed-width type.
- **R-0201** **A column must hold the whole Rust type.** An explicit `sql` can only widen: `u32` on `numeric` compiles, while `u64` on `bigint` or `i64` on `integer` is a compile error, so a value can't wrap or fail at runtime.
- **R-0202** **A wider column gets the range CHECK too,** so `i32` on `bigint` can't store a value the field can't read. If one still arrives, because the CHECK was removed outside morm, decoding returns `Error::OutOfRange` naming the column, never a wrapped value.
- **R-0203** **`numeric` is larger and slower than `bigint`,** so `u64`, `i128` and `u128` columns and their indexes cost more. Use `i64` when values fit. `#[col(sql = "uuid")]` on `u128` is a compact alternative for ids and hashes: 16 fixed bytes, with no arithmetic in SQL.
- **R-1131** **`identity` needs an integer column.** `identity` on a field whose SQL type, from `sql` or inferred, isn't `smallint`, `integer` or `bigint`, under any of their names, is a compile error, so `u64`, `i128` and `u128` can't be identity keys. Postgres allows identity, and sequences, only on those three types: on Postgres 18.6 and 17.11, `CREATE TABLE`, `ALTER COLUMN ... TYPE` and `ADD GENERATED ... AS IDENTITY` each refused a `numeric` identity column with `identity column type must be smallint, integer, or bigint`, so no generated key can count past `bigint`'s maximum. Scratch's error is the backstop for a type the macro can't see.

### Models / Generated per model

- **R-0215** positional decoding (`from_row`), name-based decoding for raw queries (`from_row_named`), and a borrowed `ProductRef<'r>`

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0763 | A Rust type wider than its column, or `usize` on a column | Compile error |

### Risks, open items and later work / Later: field-level encryption

**R-0923** Encrypted columns, including per-user keys and rotation, are not designed yet. Nothing in 0.1 is built for them. These choices, which 0.1 needs anyway, keep them possible without breaking changes:

- **R-0924** `F` columns carry capability markers checked by the query API, so a column can refuse `.by()` or `set_expr`.
- **R-0925** Encode and decode go through one per-column conversion layer, so a column's Rust type can differ from its stored type. Generated code calls `ToSql` and `FromSql` directly through static dispatch, with no pointer or allocation. A field has one decoder for each column type its family accepts, as `check_schema` lists them, and which one a column uses is chosen once per statement from the column's real type when the statement is prepared, never per row. When every column has the type the binary was built against, the statement's rows go through `from_row`, and that path is unchanged: no branch, pointer, allocation or per-row state. Otherwise they go through `from_row_as`, which `#[model]` generates beside `from_row` and which reads each field through the decoder the statement chose. The choice is held by the statement, not the row, so that path adds one branch per value on the statement's choice and no per-row state. `from_row_named` and `ProductRef` work the same way. A parameter is always sent as the column type the binary was built against, and Postgres converts it to the column's real type. A benchmark against hand-written tokio-postgres decoding checks that the unchanged path adds no cost.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0023 | trybuild | columns narrower than their Rust type |
| T-0024 | trybuild | `usize` and `isize` |
| T-0237 | trybuild | `identity` on a `u64` field, and on a field whose `sql` is `numeric(20,0)`, `text` or `uuid`, each fail with a message naming the field and the three types identity allows, while `identity` on `smallint`, `integer`, `bigint`, `int4` and `int8` compiles |
