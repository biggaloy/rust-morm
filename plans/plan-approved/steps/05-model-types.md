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

- Type inference, the integer table with range CHECKs in metadata, `from_row`, `from_row_named`, `ProductRef`, capability markers on `F`, the conversion layer

## End-to-end check

A model with every inferred type decodes rows from a real Postgres table created by hand, borrowed and owned, and each compile-time rule is proven with `trybuild`.

## Out of scope

- Encoding exactness and `Decimal` (step 13)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Models / Type inference

**R-0185** `i16` → smallint, `i32` → integer, `i64` → bigint, `f32` → real, `f64` → double precision, `bool` → boolean, `String` → text, `Vec<u8>` → bytea, `DateTime<Utc>` / `Timestamp` → timestamptz, `NaiveDateTime` → timestamp, `NaiveDate` → date, `NaiveTime` → time, `Uuid` → uuid, `serde_json::Value` → jsonb, `Vec<T>` → `T[]`, `IpAddr` → inet, `morm::Range<T>` → the matching range type, such as `tstzrange` or `int8range`. `Option<T>` → nullable. Anything else requires an explicit `sql`, including `Decimal` and `Numeric`, which need their precision stated (section 8).

- **R-0189** **Integers Postgres has no exact type for** map to the smallest column that holds every value, with a generated range CHECK named `__CHK_{table}_{column}_range__`, so nothing outside the Rust type can be stored by any client:

| ID | Rust | Column | CHECK |
|---|---|---|---|
| R-0190 | `i8` | `smallint` | -128 to 127 |
| R-0191 | `u8` | `smallint` | 0 to 255 |
| R-0192 | `u16` | `integer` | 0 to 65,535 |
| R-0193 | `u32` | `bigint` | 0 to 4,294,967,295 |
| R-0194 | `u64` | `numeric(20,0)` | 0 to `u64::MAX` |
| R-0195 | `i128` | `numeric(39,0)` | `i128::MIN` to `i128::MAX` |
| R-0196 | `u128` | `numeric(39,0)` | 0 to `u128::MAX` |
| R-0197 | `NonZero*` | As the base type | Adds `<> 0` |
| R-0198 | `char` | `text` | `char_length = 1` |

- **R-0199** **`usize` and `isize` are compile errors,** because their size depends on the machine, so a value could fit on one build and not another. Use a fixed-width type.
- **R-0200** **A column must hold the whole Rust type.** An explicit `sql` can only widen: `u32` on `numeric` compiles, while `u64` on `bigint` or `i64` on `integer` is a compile error, so a value can't wrap or fail at runtime.
- **R-0201** **A wider column gets the range CHECK too,** so `i32` on `bigint` can't store a value the field can't read. If one still arrives, because the CHECK was removed outside morm, decoding returns `OutOfRange`, never a wrapped value.
- **R-0202** **`numeric` is larger and slower than `bigint`,** so `u64`, `i128` and `u128` columns and their indexes cost more. Use `i64` when values fit. `#[col(sql = "uuid")]` on `u128` is a compact alternative for ids and hashes: 16 fixed bytes, with no arithmetic in SQL.

### Models / Generated per model

- **R-0214** positional decoding (`from_row`), name-based decoding for raw queries (`from_row_named`), and a borrowed `ProductRef<'r>`

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0761 | A Rust type wider than its column, or `usize` on a column | Compile error |

### Risks, open items and later work / Later: field-level encryption

**R-0920** Encrypted columns, including per-user keys and rotation, are not designed yet. Nothing in 0.1 is built for them. These choices, which 0.1 needs anyway, keep them possible without breaking changes:

- **R-0921** `F` columns carry capability markers checked by the query API, so a column can refuse `.by()` or `set_expr`.
- **R-0922** Encode and decode go through one per-column conversion layer, so a column's Rust type can differ from its stored type. It exists only at compile time: generated code calls `ToSql` and `FromSql` directly through static dispatch, with no branch, pointer, allocation or per-row state. A benchmark against hand-written tokio-postgres decoding checks that it adds no cost.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0023 | trybuild | columns narrower than their Rust type |
| T-0024 | trybuild | `usize` and `isize` |
