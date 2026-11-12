# Step 13: Numbers and special types

- **Phase:** Runtime
- **Depends on:** 05
- **Can run beside:** 10, 11, 12, 29
- **Crates:** `morm`, `morm-macros`, `morm-core`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Exact encoding and decoding for every supported type, with no silent rounding.

## Deliverables

- `Decimal` decoding with no heap use, `morm::Numeric`, every integer path, `SqlType`, `morm::Param`, `Fits` and `FitsList` for these types, `Range<T>`, `IpAddr`, timestamps, and the decoders each type uses on another column type of its family
- The cast-name function in `morm-core`, the carriers, the conversion layer's encodings, and `#[model]`'s `SqlType` check on every field

## End-to-end check

Every type round-trips through a real Postgres at its limits, and a value that can't be held exactly returns an error on read and on write.

## Out of scope

- `TextEnum`, `PgEnum`, `#[pg_type]`, domains and composite types (step 56)
- JSON formats (step 23)
- `sql!` and the builders that bind through a field (steps 15 to 19): T-0278 runs straight on `tokio_postgres`

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools

- **R-0049** `chrono` (default) / `time`: timestamp types
- **R-0050** `uuid` (default): UUID columns and UUIDv7 generation
- **R-0052** `decimal` (default): `Decimal` for `numeric` columns, through `rust_decimal` (section 8)

### Models / Type inference

- **R-0207** **Timestamps:** Postgres stores microseconds and chrono nanoseconds, so a value can compare unequal after a round trip. `Timestamp::now()` truncates to microseconds, and `insert` fills database-set timestamps in place. Compare stored values, not values read from the clock.
- **R-1103** **Reading a column whose type changed.** A migration can change a column's type under a running binary, so each field also decodes every column type in its family, through the decoder for the column's real type, which step 14 chooses once per statement. These are the families, and there are no others: a field of a Rust integer type reads `smallint`, `integer`, `bigint` and `numeric(p,0)` with any `p`, and never a `numeric` with a scale or with no precision; `Decimal` reads `numeric` with any precision and scale, or with none, and `smallint`, `integer` and `bigint`, because every one of their values fits in its 28 digits; `morm::Numeric` reads `numeric` with any precision and scale, or with none, and `smallint`, `integer` and `bigint`, as `Decimal` does; `String` reads `text`, and `varchar` with any length or none; a `String` built against `char(n)` reads `char` with any length, through a decoder that trims trailing spaces back to `n`, because Postgres pads a `char` value with spaces to its length and treats them as insignificant, so on `char(4)` `'ab  '` reads as `'ab '`, as it did on `char(3)`, and a value with no spaces to trim, such as `'abcd'`, reads whole; `DateTime<Utc>` and `Timestamp` read `timestamptz` with any precision, and `NaiveDateTime` reads `timestamp` with any precision; and an array reads an array of its element's family. A field wider than the column reads every value losslessly. A field narrower than the column checks each value's range, and only a value that can't fit errors: an integer field returns `Error::OutOfRange` naming the column, never a wrapped value, and `Decimal` returns `Error::Decimal` for a value it can't hold exactly, past its 28 significant digits, as on unbounded `numeric`, and never on an integer column. `morm::Numeric`, and `String` on `text` and `varchar`, read every value whole. Nothing read is rounded, as R-0423 says. A write goes as the column type the field was built against, and Postgres converts it to the column's real type, so a value the column can't hold, such as a number with more digits before the point than it allows or a text longer than its length, gets the database's error for that write. Where Postgres would round or cut a write silently instead, to a lower scale, a lower timestamp precision or a shorter length that drops trailing spaces, step 41 never runs the change under a binary built for the old shape, and its `db.check_schema()` refuses such a binary that starts after the change.
- **R-1207** **A parameter bound through a field is cast to its column's type.** Where a parameter is bound through a field (builders, `F`, `sql!`'s `${v: M::F.field}`, model inserts and updates), it's cast in the SQL to the column's type, an explicit `sql` included, with no length, precision or scale, and declared as that type's built-in carrier, as `contracts.md` says. The conversion layer encodes the value as the carrier: a narrower integer as the wider column's integer, `f32` as `double precision`, an instant (`DateTime<Utc>`, `Timestamp`) on `timestamp` as its UTC wall time, a `NaiveDateTime` on `timestamptz` as that wall time in UTC, `IpAddr` on `cidr`, and `String` on `json` or `jsonb` as `text` cast to the column's type, each element alike for a list or an array. No answer depends on the session's `TimeZone`. An `Option` value binds only through a nullable field. Owner decisions DF1, DG1 and DH2; `proofs.md`, proof 9.
- **R-1208** **A field's column type is checked through `SqlType`.** `#[model]` accepts a field's column type, the inferred one or an explicit `sql`, only if the field's Rust type declares through `SqlType` that it carries that type without loss, in a const assertion, so a type alias resolves to its real type. A `sql` naming a type morm doesn't know, such as `ltree`, `hstore`, pgvector's `vector(3)` or a domain made by hand, is accepted only on a type with a text form (`String`, `&str`, and morm's enum, domain and composite types), and the value goes as a `text` carrier cast to that type. A Rust type that doesn't implement `SqlType` can't be a field, and morm's own wrappers, such as `Option`, `OrDefault` and `WriteOnly`, take their inner type's. Anything else is a compile error that names the field, its Rust type as written and the `sql`. A user type implements `SqlType` to declare its cast, its carrier and what it carries. Step 41's startup check compares each field's column type with the database's again. Owner decisions DG1 and DH3.
- **R-1209** **Free parameters.** A value that isn't bound through a field, a `${v: T}` in `sql!`, a `.q()` or `set_expr` value, or a `sql_unchecked` value, is cast to its Rust type's own SQL type, through `SqlType`, and declared as that type's built-in carrier:
  - `String` and `&str` as `text`, and a list as its element's array;
  - `serde_json::Value` as `jsonb`;
  - an enum or composite as `text`, cast to its name, and a domain as its base type, cast to its name.

  Postgres infers no type. So `$1 IS NULL`, `concat`, `array_length` and `unnest` need no cast of your own, and a list where one value is expected fails with `42883`. A free value compared with a column of another type is compared as Postgres compares those two types, such as `text` against `citext`, which is case-sensitive. check-sql and `#[morm::test]` report it. Owner decisions DF1 and DH2; `proofs.md`, proof 9.
- **R-1212** **The cast-name function.** `morm-core` turns a column type into the cast morm writes, with no length, precision or scale, because an explicit cast with one cuts or rounds silently: `'abcdefg'::varchar(5)` gives `abcde`, `1.005::numeric(12,2)` gives `1.01`, `'...00.123456'::timestamp(3)` gives `.123`, and a bare `char` means `char(1)`, so `'abc'::char` gives `a`. `char(n)`, `character(n)`, `nchar` and a bare `char` or `character` go to `bpchar`; `varchar(n)` and `character varying(n)` to `varchar`; `bit(n)`, `bit varying(n)` and a bare `bit` to `varbit`; `numeric(p,s)` and `decimal` to `numeric`; `timestamp(p)` to `timestamp` and `timestamp(p) with time zone` to `timestamptz`; `time(p)` and `timetz(p)` to `time` and `timetz`; any `interval ...` to `interval`; `float(1..24)` to `float4`, and other `float(p)` to `float8`. Any other name keeps its spelling, quoted parts included, with a parenthesised modifier dropped, so `vector(3)` casts to `vector`. The column's own check still applies on write: `($1::varchar)` into `varchar(5)` fails with `22001`, `($1::bpchar)` into `char(5)` too, and `($1::varbit)` into `bit(8)` with `22026`. `#[model]` and `sql!` call it at compile time. Owner decision DH2.
- **R-1213** **`SqlType` for every type morm maps.** Each gives its cast and carrier as `const`s, and the column types it carries without loss: `bool` as `bool`; `i16`, `i32` and `i64` as `int2`, `int4` and `int8`, each also carrying every wider integer column and `numeric`; every other integer type morm maps as the column it maps to; `f32` and `f64` as `float4` and `float8`, `f32` also carrying `float8`; `String`, `&str`, `Box<str>` and `Cow<str>` as `text`, carrying `varchar`, `bpchar`, `citext`, `json` and `jsonb`, with a text form; `Vec<u8>` and `&[u8]` as `bytea`; `Decimal` and `morm::Numeric` as `numeric`; `Uuid` as `uuid`; `DateTime<Utc>` and `morm::Timestamp` as `timestamptz`, also carrying `timestamp`; `NaiveDateTime` as `timestamp`, also carrying `timestamptz`; `NaiveDate` and `NaiveTime` as `date` and `time`; `serde_json::Value` as `jsonb`, also carrying `json`; `IpAddr` as `inet`, also carrying `cidr`; and `morm::Range<T>` as `int4range`, `int8range`, `numrange`, `tsrange`, `tstzrange` or `daterange`, by `T`. `Vec<T>` and `&[T]` take `T`'s cast with `[]` and `T`'s carrier's array, and `Option<T>` and `&T` take `T`'s. Every carrier is built in, so every parameter goes in binary format. `Param` comes from `SqlType` through a blanket impl. Owner decisions DH2 and DH3.
- **R-1214** **A value bound through a field must fit it.** `Fits<T>` says a value fits a field of type `T` without loss, and the conversion layer encodes it as the column's carrier: `T` itself and `&T`, `&str`, `Box<str>` and `Cow<str>` for `String`, `&[u8]` for `Vec<u8>`, and a narrower integer or `f32` for a wider field. An `Option` value fits only an `Option` field, so a `NULL` can't be handed to a comparison with a `NOT NULL` column, where it would match nothing, and a non-`Option` value fits an `Option` field. `FitsList<T>` does the same element by element, for a `Vec`, a slice or an array bound as a list. Anything else is a compile error, such as ``the trait bound `i64: Fits<i32>` is not satisfied``. Owner decision DH2, choice K.

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
| T-0278 | integration | **The parameter matrix, on Postgres 15–18:** with the session's `TimeZone` not UTC, straight on `tokio_postgres` through the conversion layer, `SqlType`, `Fits` and each parameter's carrier and cast, every type `#[model]` maps, `Timestamp`, `Range<T>` and `Numeric` included, against its default column and each column an explicit `sql` gives it (`citext`, `varchar(n)`, `char(n)`, `json`, `jsonb`, `timestamp` from an instant, `timestamptz` from a naive time, wider integers, `double precision` from `f32`, `cidr`, arrays), is bound through the field, inserted, and compared with `=` and `= ANY`, and gives the right answer through a prepared statement and through `query_typed`. Each free value of the same Rust type against the same columns gets Postgres's answer for its own cast, and it differs from the field's answer only for the pairs proof 9 pins: `text` against `citext` or `char(n)`, an instant against `timestamp`, a naive time against `timestamptz`, and `jsonb` written into `json`, which keeps the JSON value but not its text. `$1 IS NULL`, `concat`, `array_length` and `unnest` work free, and a list where one value is expected fails with `42883` |
| T-0279 | suite | **Unit: the cast-name function** on every form R-1212 lists, at least 28, including quoted and schema-qualified names and a modifier on an unknown type, and no cast it returns carries a length, precision or scale |
| T-0280 | trybuild | **Fields that don't fit:** an `i64` bound through an `i32` field, an `Option` value through a non-`Option` field, a field whose `sql` its type doesn't carry (`i32` on `text`, and `Vec<f32>` on `vector(3)`, an unknown type with no text form), the same through a type alias, and a field of a type with no `SqlType`, each a compile error naming the field |
