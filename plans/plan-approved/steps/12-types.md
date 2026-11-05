# Step 12: Numbers and special types

- **Phase:** Runtime
- **Depends on:** 04
- **Can run beside:** 05, 06, 07, 08, 09, 10, 11, 25, 26, 27, 28, 29
- **Crates:** `morm`, `morm-macros`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Exact encoding and decoding for every supported type, with no silent rounding.

## Deliverables

- `Decimal` decoding with no heap use, `morm::Numeric`, every integer path, `morm::Param` for every type, `Range<T>`, `IpAddr`, timestamps, `TextEnum`, `PgEnum`, domains and composite types, `InvalidText`

## End-to-end check

Every type round-trips through a real Postgres at its limits, and a value that can't be held exactly returns an error on read and on write.

## Out of scope

- JSON formats (step 21)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools

- **R-0048** `chrono` (default) / `time`: timestamp types
- **R-0049** `uuid` (default): UUID columns and UUIDv7 generation
- **R-0051** `decimal` (default): `Decimal` for `numeric` columns, through `rust_decimal` (section 8)

### Models / Type inference

**R-0185** `#[derive(TextEnum)]` maps a Rust enum to `text` with a CHECK constraint, the recommended form, because adding or removing a value is an ordinary migration.

**R-0186** `#[derive(PgEnum)]` maps to a Postgres enum, mainly to match existing tables, because removing a value from one is very hard.

**R-0187** Domains and composite types are also supported.

- **R-0205** **Timestamps:** Postgres stores microseconds and chrono nanoseconds, so a value can compare unequal after a round trip. `Timestamp::now()` truncates to microseconds, and `insert` fills database-set timestamps in place. Compare stored values, not values read from the clock.
- **R-0206** **NUL bytes:** Postgres rejects `\0` in text. morm doesn't scan every string for it, which would cost time on every write; Postgres's error becomes a typed `InvalidText`.

### Reading and writing / Decimal

**R-0409** Postgres `numeric` goes up to 131,072 digits before the point, and also holds `NaN` and `Infinity`. No fixed-size Rust type covers that. `rust_decimal::Decimal` is 16 bytes, needs no heap and is `Copy`, but holds only 28 digits or so. Two types cover both cases:

| ID | Rust type | Holds | Use for |
|---|---|---|---|
| R-0410 | `Decimal` (`rust_decimal`) | Up to 28 significant digits | Money, quantities, rates |
| R-0411 | `morm::Numeric` | Every `numeric` value, `NaN` and `Infinity` included, with no arithmetic of its own | Larger values, and columns of unknown range |

**R-0412**

```rust
#[col(sql = "numeric(12,2)")] pub price: Decimal,   // fits, so checked at compile time
#[col(sql = "numeric(40,2)")] pub total: Numeric,   // Decimal here is a compile error
```

- **R-0413** **Never silently rounded on read.** `rust_decimal`'s own decoding rounds fractional digits it can't hold, and it allocates per value. morm decodes `Decimal` itself, with no heap use. A value that doesn't fit exactly returns `Error::Decimal`, as do `NaN` and `Infinity`.
- **R-0414** **Never silently rounded on write.** Postgres rounds to a column's scale without complaint, so `1.005` into `numeric(12,2)` would change. With the scale known from `sql`, a value with too many decimal places returns a validation error. Use `clean` when rounding is what you want.
- **R-0415** **Types are checked at compile time.** `Decimal` on `numeric(p,s)` with `p` or `s` above 28 is a compile error. `f32` or `f64` on `numeric` is a compile error, because floats can't hold decimal fractions exactly.
- **R-0416** **Unbounded `numeric`** with `Decimal` compiles, because existing tables have it, a value that doesn't fit fails at read time.
- **R-0420** **Speed:** `numeric` decoding is slower than integer decoding. When a column holds a fixed number of decimal places and speed matters most, `bigint` minor units (cents) are the fastest choice. The README says so.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0750 | `Decimal` on a `numeric` too large for it, or a float on `numeric` | Compile error |
| R-0751 | A rounded `numeric` value | Error on read and on write |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0031 | trybuild | `Decimal` and float on unsuitable `numeric` columns |
| T-0075 | integration | `OutOfRange` |
| T-0076 | integration | `InvalidText` |
