# Parameter typing: design proposal (DH2)

For the owner's review. Design only: nothing in `plans/plan-approved/` is edited. It answers "Step 02, DH2, DH3 and DH5" in `INDEX.md` on `origin/main` (`0323967`), with `reviews/step-02-review-2.md` (DH2) and PR #2 at `632ab63` (`contracts.md`, `proofs.md` proof 9, R-1207 to R-1211, T-0278).

Every Postgres claim below was run on throwaway Postgres 18.6 and 17.11 servers in this worktree's `.claude/scratch/`, as a superuser and as an ordinary role (`app`). The two versions gave the same results except for OIDs. Every macro and scanner claim was run in a scratch Rust workspace on stable rustc 1.99.1, with `tokio-postgres` 0.7.18: a prototype `sql!` proc macro, the scanner, the runtime types, and a prototype of the CI check. Postgres 15 and 16 aren't installed here, so section 10 gives each building step a test on 15 to 18. Section 11 lists what was run. The servers are stopped and the scratch is deleted.

## 1. The design in short

- **Postgres never infers a parameter's type.** Every parameter morm sends is declared in the Parse message with a built-in type, its *carrier*, and the SQL text casts it, in the form `($1::citext)`. That holds for `sql!`, every builder, model inserts and updates, `UNNEST`, keyset pages, `.limit()`, and runtime fragments.
- **A parameter bound through a field** is cast to the field's column type, with any length, precision or scale removed. Its Rust value must fit that field without loss, checked at compile time. The conversion layer encodes it as the carrier.
- **A free parameter** is cast to its Rust type's own SQL type, through a new `SqlType` trait.
- **Types with no fixed OID** (enums, domains, composites, `citext`, extension types) travel as `text`, or a domain's base type, and are cast by name. No OID is looked up or sent, and a cached statement that touches the type's table survives a type swap.
- **`sql!`** writes parameters inline: `${value: Model::F.field}`, `${values: Model::F.field[]}` or `${value: RustType}`. `{Model::F.field}` writes a column name.
- **Mismatches are found in four layers:**
  1. `sql!` refuses at compile time what the text shows: a free value beside a column named through `F`, an untyped `$1`, a parameter inside quotes, and a cast that cuts or rounds.
  2. `check-sql` parses every registered `sql!` query on scratch, reads the analyzed parse tree Postgres sends back, and fails on any comparison where Postgres converted a column or a value. It fails too on any wrong shape, which is a Postgres error at Parse.
  3. `#[morm::test]` runs the same check on every runtime-built shape and every `sql!` query that a test runs.
  4. Everywhere else, a wrong shape is a Postgres type error, because every parameter has a type.
- **What none of them can see** is listed in section 8. The README states it loudly, with the rule to bind through the field.

There's no runtime round trip for typing. Every cast is a compile-time constant, or, for runtime fragments, text built once per shape.

## 2. How a parameter is sent

**The carrier and the cast.** Each parameter has two parts:

- its carrier, a built-in type it's encoded in (binary format, as today), declared in Parse;
- its cast, the type written into the SQL.

| Bound to | Carrier | Cast written |
|---|---|---|
| A built-in column type, such as `bigint` | That type (`int8`) | `($1::int8)` |
| `citext`, an enum, a composite, an extension type such as `ltree` | `text` | `($1::citext)`, `($1::"billing"."order_state")` |
| A domain | Its base type's carrier (`int4` for a domain over `integer`, `text` over `citext`) | The domain's name |
| `char(n)`, `varchar(n)`, `bit(n)`, `numeric(p,s)`, `timestamp(p)` | `bpchar`, `varchar`, `varbit`, `numeric`, `timestamp` | The type without its modifier: `bpchar`, `varchar`, `varbit`, `numeric`, `timestamp` |
| A list or an array column | The element carrier's array type (`text[]` for enums) | The cast with `[]` |

**Why the cast drops the length, precision or scale.** An explicit cast with one cuts or rounds silently, so `name = $1::varchar(5)` would compare a cut value:

- `'abcdefg'::varchar(5)` gives `abcde`, and so does `char(5)`.
- `1.005::numeric(12,2)` gives `1.01`.
- `'...00.123456'::timestamp(3)` gives `.123`.
- A bare `char` means `char(1)`, so `'abc'::char` gives `a`. A bare `bit` means `bit(1)`, so `'101'::bit` gives `1`.

That's why `char(n)` casts to `bpchar` and `bit(n)` to `varbit`. Without the modifier, the column's own check still applies on write:

- `VALUES ($1::varchar)` with `'abcdefg'` into `varchar(5)` fails with `22001`, and so does `$1::bpchar` into `char(5)`.
- `$1::varbit` `'101'` into `bit(8)` fails with `22026`.
- A domain's `CHECK` fails with `23514`.
- `numeric` is the one exception: `$1::numeric` into `numeric(12,2)` rounds `1.005` with no error, as any write does. R-0424 already refuses that value for a field-bound write, and check-sql reports a free one (section 6, rule T3).

The macro takes the cast from the field's `sql` with a small table: `char(n)`, `character(n)` and `nchar` go to `bpchar`; `varchar(n)` and `character varying(n)` to `varchar`; `bit(n)` and `bit varying(n)` to `varbit`; `numeric(p,s)` and `decimal` to `numeric`; `timestamp(p) with time zone` to `timestamptz`; `time(p)` and `timetz(p)` to `time` and `timetz`; any `interval ...` to `interval`; `float(1..24)` to `float4`, and other `float(p)` to `float8`. Any other name keeps its spelling, quoted parts included, with its parenthesised modifier dropped, so `vector(3)` casts to `vector`. A bare name with no modifier gets typmod `-1` in Postgres's grammar, unlimited, except the SQL-standard `char` and `bit`, which the table handles. The table is in the scratch prototype's tests, 28 cases.

**Why `(...)` around the cast.** A bare `$1::int8[]` before a subscript would parse `[1]` as part of the type name, and `$1::pair.n` as a schema-qualified type. With parentheses, every position I tried prepared and ran on 17.11 and 18.6:

- `($1::int8[])[1]` and `(($1::pt.pair).n)`
- `-($1::int8)`, and `($1::int8)::int4` after a cast of the user's own
- `LIMIT`, `OFFSET` and `FETCH FIRST`
- `IN (..)`, `BETWEEN`, `ARRAY[..]`, `ROW(..)`
- `COLLATE "C"`, `CAST(.. AS int4)`, `VARIADIC`, `AT TIME ZONE`, `NOT`, `IS NULL`

**Why a carrier, not `Type::UNKNOWN`.** I tried both forms. In each, Postgres takes the parameter's type from the cast. They differ when a type is swapped under a cached statement:

| Cached statement, then a type swap | `UNKNOWN` plus cast (or plain inference, as today) | `text` carrier plus cast |
|---|---|---|
| `WHERE s = $1::st`, old type dropped | `XX000` `cache lookup failed for type`, at Bind, on every run | Right answer: the table changed, so Postgres reparses and resolves `st` again |
| `= ANY($1::st[])`, and `INSERT .. VALUES ($2::st)` | `XX000` at Bind | Right answer, and the insert works |
| `WHERE s = $1::st`, old type kept as `st_old` | `42846` `cannot cast type st_old to st`. A label only the new type has gives `22P02` at Bind (plain inference: `22P02`, then `42883`) | Right answer, new labels included |
| A recreated domain or composite on the column | Not tested | Right answer |
| A statement whose only table has no column of the swapped type, such as `SELECT ($1::st)::text` | Same as the rows above | `XX000` once the old type is dropped, and `22P02` `invalid input value for enum sw.st_old` while it's kept |

So with the carrier, the only stale case left is a cached statement that casts to the type but touches no table holding it. Choice I covers it. The carrier also removes the text-format wrapper from `contracts.md`: every carrier is built-in, so every parameter goes in binary, and text bytes are the same in both formats. The `1234` read as `825373492` hazard needed `UNKNOWN` in binary, and it can't happen here.

**Indexes stay usable.** With `plan_cache_mode = force_generic_plan` on 20,000 rows, each of these used an index scan, on 17.11 and 18.6:

- `c = ($1::citext)` with a `text` carrier;
- `ch = ($1::bpchar)`, `m = ($1::pt.mood)` and `d = ($1::pt.posint)` (index condition `(d)::integer = (($1)::pt.posint)::integer`);
- `c = ANY(($1::citext[]))`;
- `b = ($1::int8)` and `b = ($1::int4)`.

The mismatches lost it: `c = ($1::text)` gave `Seq Scan`, `Filter: ((c)::text = $1)`, and `b = ($1::numeric)` gave `Filter: ((b)::numeric = $1)`.

**No round trip.** Casts name types, so nothing is resolved per query. The names are checked once per pool instead (section 4).

## 3. `sql!` (item 1)

**Syntax.** `sql!` takes one string literal. Outside strings, quoted identifiers, comments and dollar quotes, the macro's scanner (the morm-core scanner, section 7) reads three forms. `{` and `${` are never valid SQL there, so nothing is ambiguous:

| Form | Writes | Value checked at compile time |
|---|---|---|
| `{Order::F.vendor_id}` | The quoted column name, `"vendor_id"`, from the `F` constant | — |
| `${vendor_id: Order::F.vendor_id}` | `($n::int8)`, the column's cast | The value fits the field's Rust type without loss |
| `${ids: Order::F.id[]}` | `($n::int8[])` | A `Vec`, slice or array whose elements fit |
| `${since: NaiveDate}` | `($n::date)`, the type's `SqlType::CAST` | The value is that type, or a reference to it |

```rust
let rows: Vec<Sale> = db.sql(sql!(
    "SELECT day, sum(total) AS total FROM orders
     WHERE {Order::F.vendor_id} = ${vendor_id: Order::F.vendor_id}
       AND {Order::F.day} >= ${since: NaiveDate}
       AND {Order::F.state} = ANY(${states: Order::F.state[]})
     GROUP BY day"
)).limit(30).fetch().await?;
```

The text sent is `... WHERE "vendor_id" = ($1::int8) AND "day" >= ($2::date) AND "state" = ANY(($3::"order_state"[])) GROUP BY day`, plus R-0417's `LIMIT ($4::int8)`.

- **Where the value goes.** Each value sits at its place in the SQL, so values can't be misnumbered. The prototype ran 13 such queries through a prepared statement and through `query_typed`, on 17.11 and 18.6, with the right answer each time. They covered `citext`, `char(5)`, an `i32` on a `bigint` field, `= ANY` lists, an enum and an enum list, a `citext[]` column, a free enum, `Option<i64>` as `NULL`, a free `Vec<String>` in `array_length`, and `&str` in `concat`.
- **Lists.** `[]` after the field binds a list, cast to the column's type plus `[]`, for `= ANY(..)`, `<> ALL(..)` and `unnest(..)`. An array column, such as `tags: Vec<String>` on `citext[]`, binds a whole array with `${tags: M::F.tags}`. `[]` on an array column is a compile error, because a list of arrays isn't supported (prototype: a const panic naming it).
- **`Option` and `NULL`.** An `Option` value fits only a field whose type is an `Option`, so `NULL` is bound only through a nullable field, and a `NOT NULL` column can't be compared with a `NULL` that matches nothing. A non-`Option` value fits an `Option` field. A free `Option<T>` is cast as `T`.
- **Expressions.** Bind through the field when the expression keeps the column's type: `date_trunc('day', {O::F.at}) = ${t: O::F.at}`, or `{U::F.email} = ${e: U::F.email}` on `citext`. Use a free value with the expression's own type when it doesn't, as in `length({U::F.name}) > ${n: i32}`. check-sql reports either if it's in the wrong place. On `citext`, `lower(c) = $1::citext` found 0 rows for `'ALICE'`: `lower` returns `text`, so Postgres compared `text` with the value turned into `text`, and rule T2 reports it. `lower(c) = $1::text` is right, and passes.
- **A cast of your own** after a parameter is kept, as in `${s: String}::citext`, which morm writes as `($1::text)::citext`. An explicit cast states intent, so nothing reports it, except one with a length, precision or scale (sections 5 and 6).
- **Why every parameter's type is written.** `sql!` registers each query for check-sql in a static (R-0046). A static can't use a type that's only inferred from a local value, so the free value's SQL type has to be named in the macro. The prototype shows the limit: `${v: T}` inside a generic `fn f<T>` fails with `E0401` `can't use generic parameters from outer item`. A free parameter's type must be concrete, and the README says so. It also makes the text a compile-time constant: `const` concatenation of the literal pieces, the `F` names, the field casts and `SqlType::CAST`, on stable.
- **What `sql!` returns.** A `morm::Sql<'a>`: the static text and registration, and the borrowed values, each as its carrier, in a tuple with no boxing. The prototype boxed them for brevity. `db.sql()` and `db.raw::<T>()` take it (choice B). The registration holds the text, `file!()` and `line!()`, and each parameter's number, kind (field, field list or free), cast, model and column, and its `${..}` source.

## 4. Free parameters (item 2)

**The table.** `SqlType` gives each type its cast and carrier as `const`s. `Param` keeps object safety, for R-0723's `&dyn Param`, with methods that return them, and a blanket impl from `SqlType`.

| Rust type | Cast | Carrier |
|---|---|---|
| `bool` | `bool` | `bool` |
| `i16`, `i32`, `i64` | `int2`, `int4`, `int8` | the same |
| `f32`, `f64` | `float4`, `float8` | the same |
| `String`, `&str`, `Box<str>`, `Cow<str>` | `text` | `text` |
| `Vec<u8>`, `&[u8]` | `bytea` | `bytea` |
| `Decimal`, `morm::Numeric` | `numeric` | `numeric` |
| `Uuid` | `uuid` | `uuid` |
| `DateTime<Utc>`, `morm::Timestamp` | `timestamptz` | `timestamptz` |
| `NaiveDateTime`, `NaiveDate`, `NaiveTime` | `timestamp`, `date`, `time` | the same |
| `serde_json::Value` | `jsonb` | `jsonb` |
| `IpAddr` | `inet` | `inet` |
| `morm::Range<T>` | `int4range`, `int8range`, `numrange`, `tsrange`, `tstzrange` or `daterange`, by `T` | the same |
| `TextEnum` | `text` | `text` |
| `PgEnum`, `PgComposite` | Its name, as `#[pg_type]` or R-0976 gives it, quoted as morm's DDL quotes it | `text` |
| `PgDomain` over `T` | Its name | `T`'s carrier |
| `Vec<T>`, `&[T]` | `T`'s cast plus `[]` | `T`'s carrier array |
| `Option<T>`, `&T` | `T`'s | `T`'s |

Every other type is a compile error until it implements `SqlType`. That's DH3's column-type trait: a user type declares the SQL type it carries without loss, and text types may carry an unknown type only in text form.

**Names, not OIDs.** A cast names its type, so nothing is resolved per query. Postgres resolves the name when it parses the statement. For a prepared statement that's once, and again only when Postgres replans after a change. `to_regtype` resolved each form morm writes in one statement: `int8`, `citext`, `citext[]`, `bpchar`, `varchar`, `varbit`, `"pt"."mood"`, `"pt"."mood"[]`, `pt.posint`, `"pt"."Mixed Mood"`, `double precision`, `ltree` and `hstore`. An unknown name gave `NULL`, not an error, and a bare `mood` gave `NULL` with its schema off `search_path`. A `text` carrier cast to `ltree` and `hstore` worked, and bad input failed with `42601`. No extension type with a modifier, such as pgvector's `vector(3)`, is installed here, so `vector` is untested. The rule is the generic grammar one above.

**Checked once per pool.** R-1116's startup `check_schema` adds one array to its catalog statement: the cast name of every column it checks, plus every `PgEnum`, `PgDomain` and `PgComposite` the binary registers, resolved with `to_regtype` and compared with the column's `atttypid`, or checked to be not `NULL`. So a name that resolves to nothing, or to another schema's type of the same name through `search_path`, stops the pool at startup with `Error::SchemaMismatch`, naming the field, the cast and what it resolved to. It's the same statement, so there's no extra round trip. With `check_schema = "off"`, a bad name fails its first query with `42704`, never a wrong answer.
