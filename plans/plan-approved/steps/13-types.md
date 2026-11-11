# Step 13: Numbers and special types

- **Phase:** Runtime
- **Depends on:** 05
- **Can run beside:** 10, 11, 12, 29
- **Crates:** `morm`, `morm-macros`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Exact encoding and decoding for every supported type, with no silent rounding.

## Deliverables

- `Decimal` decoding with no heap use, `morm::Numeric`, every integer path, `morm::Param` for these types, `Range<T>`, `IpAddr`, timestamps, and the decoders each type uses on another column type of its family

## End-to-end check

Every type round-trips through a real Postgres at its limits, and a value that can't be held exactly returns an error on read and on write.

## Out of scope

- `TextEnum`, `PgEnum`, `#[pg_type]`, domains and composite types (step 56)
- JSON formats (step 23)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools

- **R-0049** `chrono` (default) / `time`: timestamp types
- **R-0050** `uuid` (default): UUID columns and UUIDv7 generation
- **R-0052** `decimal` (default): `Decimal` for `numeric` columns, through `rust_decimal` (section 8)

### Models / Type inference

- **R-0207** **Timestamps:** Postgres stores microseconds and chrono nanoseconds, so a value can compare unequal after a round trip. `Timestamp::now()` truncates to microseconds, and `insert` fills database-set timestamps in place. Compare stored values, not values read from the clock.
- **R-1103** **Reading a column whose type changed.** A migration can change a column's type under a running binary, so each field also decodes every column type in its family, through the decoder for the column's real type, which step 14 chooses once per statement. These are the families, and there are no others: a field of a Rust integer type reads `smallint`, `integer`, `bigint` and `numeric(p,0)` with any `p`, and never a `numeric` with a scale or with no precision; `Decimal` reads `numeric` with any precision and scale, or with none, and `smallint`, `integer` and `bigint`, because every one of their values fits in its 28 digits; `morm::Numeric` reads `numeric` with any precision and scale, or with none, and `smallint`, `integer` and `bigint`, as `Decimal` does; `String` reads `text`, and `varchar` with any length or none; a `String` built against `char(n)` reads `char` with any length, through a decoder that trims trailing spaces back to `n`, because Postgres pads a `char` value with spaces to its length and treats them as insignificant, so on `char(4)` `'ab  '` reads as `'ab '`, as it did on `char(3)`, and a value with no spaces to trim, such as `'abcd'`, reads whole; `DateTime<Utc>` and `Timestamp` read `timestamptz` with any precision, and `NaiveDateTime` reads `timestamp` with any precision; and an array reads an array of its element's family. A field wider than the column reads every value losslessly. A field narrower than the column checks each value's range, and only a value that can't fit errors: an integer field returns `Error::OutOfRange` naming the column, never a wrapped value, and `Decimal` returns `Error::Decimal` for a value it can't hold exactly, past its 28 significant digits, as on unbounded `numeric`, and never on an integer column. `morm::Numeric`, and `String` on `text` and `varchar`, read every value whole. Nothing read is rounded, as R-0423 says. A write goes as the column type the field was built against, and Postgres converts it to the column's real type, so a value the column can't hold, such as a number with more digits before the point than it allows or a text longer than its length, gets the database's error for that write. Where Postgres would round or cut a write silently instead, to a lower scale, a lower timestamp precision or a shorter length that drops trailing spaces, step 41 never runs the change under a binary built for the old shape, and its `db.check_schema()` refuses such a binary that starts after the change.

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
- **R-0424** **Never silently rounded on write.** Postgres rounds to a column's scale without complaint, so `1.005` into `numeric(12,2)` would change. With the scale known from `sql`, a value with too many decimal places returns a validation error. Use `clean` when rounding is what you want. A migration that lowers a column's scale never runs under a binary built for the old scale, because step 41 holds it for `contract` or `--fast`, so the scale this check uses is never above the column's while that binary runs, and step 41's `check_schema`, which morm runs when each pool starts, refuses that binary if it starts after the change. Only a pool set to `check_schema = "off"` skips that, and then such a binary starts and Postgres rounds what it writes with no error.
- **R-0425** **Types are checked at compile time.** `Decimal` on `numeric(p,s)` with `p` or `s` above 28 is a compile error. `f32` or `f64` on `numeric` is a compile error, because floats can't hold decimal fractions exactly.
- **R-0426** **Unbounded `numeric`** with `Decimal` compiles, because existing tables have it, a value that doesn't fit fails at read time.
- **R-0430** **Speed:** `numeric` decoding is slower than integer decoding. When a column holds a fixed number of decimal places and speed matters most, `bigint` minor units (cents) are the fastest choice. The README says so.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0771 | `Decimal` on a `numeric` too large for it, or a float on `numeric` | Compile error |
| R-0772 | A rounded `numeric` value | Error on read and on write |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0031 | trybuild | `Decimal` and float on unsuitable `numeric` columns |
| T-0075 | integration | `OutOfRange` |
| T-0223 | integration | **Reading across a type change, on Postgres 15–18:** every integer field reads `smallint`, `integer`, `bigint` and `numeric(p,0)` columns at its own limits and theirs: a value in its range reads exactly, and one outside it returns `Error::OutOfRange` naming the column, never a wrapped value. `Decimal` reads every `numeric(6,2)` value exactly, and a `numeric(40,2)` or `numeric` value exactly up to 28 significant digits, and returns `Error::Decimal` for a 30-digit value of either, never a rounded one, while `morm::Numeric` reads it whole. `Decimal` and `morm::Numeric` read `smallint`, `integer` and `bigint` at their limits exactly. `String` reads `varchar(3)`, `varchar` and `text`. A `String` built against `char(3)` reads a `char(3)` value written as `'ab'` as `'ab '`, as Postgres sends it, and on `char(4)` trims trailing spaces back to 3, reading `'ab  '` as `'ab '`, `'    '` as `'   '` and `'abcd'` whole, and a value it writes as `'ab '` to `char(4)` reads back as `'ab '`. `DateTime<Utc>` reads `timestamptz(0)` and `timestamptz`, and `NaiveDateTime` reads `timestamp(3)`, each exactly. |
