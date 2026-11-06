# Step 13: Numbers and special types

- **Phase:** Runtime
- **Depends on:** 05
- **Can run beside:** 10, 11, 12, 29, 30
- **Crates:** `morm`, `morm-macros`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Exact encoding and decoding for every supported type, with no silent rounding.

## Deliverables

- `Decimal` decoding with no heap use, `morm::Numeric`, every integer path, `morm::Param` for every type, `Range<T>`, `IpAddr`, timestamps, `TextEnum`, `PgEnum` with OID-free binding, `#[pg_type]`, variant labels, domains and composite types

## End-to-end check

Every type round-trips through a real Postgres at its limits, and a value that can't be held exactly returns an error on read and on write.

## Out of scope

- JSON formats (step 23)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools

- **R-0049** `chrono` (default) / `time`: timestamp types
- **R-0050** `uuid` (default): UUID columns and UUIDv7 generation
- **R-0052** `decimal` (default): `Decimal` for `numeric` columns, through `rust_decimal` (section 8)

### Models / Type inference

**R-0187** `#[derive(TextEnum)]` maps a Rust enum to `text` with a CHECK constraint, the recommended form, because adding or removing a value is an ordinary migration.

**R-0188** `#[derive(PgEnum)]` maps to a Postgres enum, and on its own it only binds to a type that already exists. `#[pg_enum(name = "...")]` turns on full management, in step 35: adding, renaming, removing and reordering values. `TextEnum` stays the preferred form, because removing or reordering a Postgres enum value means replacing the whole type.

**R-0189** Domains and composite types are also supported.

- **R-0961** **Domains and composite types in Rust.** `#[derive(PgDomain)]` goes on a one-field struct, such as `pub struct Email(pub String);`, and `#[derive(PgComposite)]` on a struct with named fields, one field per attribute, in order. Each gives encoding, decoding and `morm::Param`, and a model column uses the Rust type directly, as it does a `PgEnum`. On its own, `PgDomain`, `PgComposite` or `PgEnum` only binds to a type that already exists: morm doesn't create or change that type. Step 35 adds the attributes that make morm manage one.
- **R-0207** **Timestamps:** Postgres stores microseconds and chrono nanoseconds, so a value can compare unequal after a round trip. `Timestamp::now()` truncates to microseconds, and `insert` fills database-set timestamps in place. Compare stored values, not values read from the clock.

### Reading and writing / Decimal

**R-0419** Postgres `numeric` goes up to 131,072 digits before the point, and also holds `NaN` and `Infinity`. No fixed-size Rust type covers that. `rust_decimal::Decimal` is 16 bytes, needs no heap and is `Copy`, but holds only 28 digits or so. Two types cover both cases:

| ID | Rust type | Holds | Use for |
|---|---|---|---|
| R-0420 | `Decimal` (`rust_decimal`) | Up to 28 significant digits | Money, quantities, rates |
| R-0421 | `morm::Numeric` | Every `numeric` value, `NaN` and `Infinity` included, with no arithmetic of its own | Larger values, and columns of unknown range |

**R-0422**

```rust
#[col(sql = "numeric(12,2)")] pub price: Decimal,   // fits, so checked at compile time
#[col(sql = "numeric(40,2)")] pub total: Numeric,   // Decimal here is a compile error
```

- **R-0423** **Never silently rounded on read.** `rust_decimal`'s own decoding rounds fractional digits it can't hold, and it allocates per value. morm decodes `Decimal` itself, with no heap use. A value that doesn't fit exactly returns `Error::Decimal`, as do `NaN` and `Infinity`.
- **R-0424** **Never silently rounded on write.** Postgres rounds to a column's scale without complaint, so `1.005` into `numeric(12,2)` would change. With the scale known from `sql`, a value with too many decimal places returns a validation error. Use `clean` when rounding is what you want.
- **R-0425** **Types are checked at compile time.** `Decimal` on `numeric(p,s)` with `p` or `s` above 28 is a compile error. `f32` or `f64` on `numeric` is a compile error, because floats can't hold decimal fractions exactly.
- **R-0426** **Unbounded `numeric`** with `Decimal` compiles, because existing tables have it, a value that doesn't fit fails at read time.
- **R-0430** **Speed:** `numeric` decoding is slower than integer decoding. When a column holds a fixed number of decimal places and speed matters most, `bigint` minor units (cents) are the fastest choice. The README says so.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0771 | `Decimal` on a `numeric` too large for it, or a float on `numeric` | Compile error |
| R-0772 | A rounded `numeric` value | Error on read and on write |

### Managed enums

- **R-0976** **Type names.** A bare `#[derive(PgEnum)]`, `#[derive(PgDomain)]` or `#[derive(PgComposite)]` binds to the snake_case of the Rust name, so `OrderState` binds to `order_state`. `#[pg_type(name = "...")]` overrides that name, and it doesn't turn on management.
- **R-0977** **Labels.** A variant's Postgres label is the snake_case of its name, so `AwaitingPayment` is `awaiting_payment`. `#[pg_enum(value = "...")]` on the variant overrides it. Attributes on a variant never turn on management: only `#[pg_enum(name = "...")]` on the type does.
- **R-0978** **Enum binding needs no OID.** morm writes `ToSql` and `FromSql` for `PgEnum` itself and doesn't use `postgres-derive`, whose exact variant check fails on every connection opened after a value is added. `accepts` compares no variants. `Param::pg_type` returns `Type::UNKNOWN`, so Postgres infers the type and a query never carries an OID that a swap could make stale. Array parameters use text format, because a binary array carries its element's OID. Labels are matched as bytes at compile time, with no heap use.
- **R-0979** **A label the binary doesn't know** returns `Error::UnknownEnumValue`, with the type and the label, for rows holding it and no others. `#[pg_enum(unknown)]` on one unit variant decodes any such label to that variant instead, and writing that variant returns `Error::Validation`.
- **R-0980** **Renamed labels.** `#[pg_enum(was = "old")]` on a variant decodes both labels to it and writes the current one, so a new binary reads rows from either side of a rename.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0031 | trybuild | `Decimal` and float on unsuitable `numeric` columns |
| T-0075 | integration | `OutOfRange` |
| T-0141 | integration | `#[derive(PgDomain)]` and `#[derive(PgComposite)]` round-tripping through columns and parameters on types created by hand |
| T-0142 | integration | **Plain `String` on a domain:** test whether a `String` field or parameter binds to a column of a text domain, and don't assume it. State the result in the PR. If it doesn't bind, `PgDomain` is the only way to use such a column, and its docs say so. |
| T-0148 | integration | **Enum binding survives type changes:** on an open connection and on one opened afterwards, `PgEnum` values and arrays bind and decode through `query_typed` and prepared statements after `ADD VALUE`, and through `query_typed` after a hand-run type swap. |
| T-0149 | integration | **Names and labels:** a bare derive binds to the snake_case name, `#[pg_type(name = "...")]` and `#[pg_enum(value = "...")]` override it, an unknown label returns `Error::UnknownEnumValue`, `#[pg_enum(unknown)]` decodes it and refuses to write, and `was` decodes both labels. |
| T-0150 | trybuild | a `PgEnum` with duplicate labels, an empty label, a label over 63 bytes, a variant that isn't a unit variant, or two `#[pg_enum(unknown)]` variants |
