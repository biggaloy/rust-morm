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

**Checked once per pool.** R-1116's startup `check_schema` adds one array to its catalog statement: the cast name of every column it checks, plus every `PgEnum`, `PgDomain` and `PgComposite` the binary registers, resolved with `to_regtype` and compared with the column's `atttypid`, or checked to be not `NULL`. So a name that resolves to nothing, or to another schema's type of the same name through `search_path`, stops the pool at startup with `Error::SchemaMismatch`, naming the field, the cast and what it resolved to. It's the same statement, so there's no extra round trip. With `check_schema = "off"`, R-1117 already accepts an unchecked schema. A name that resolves to nothing then fails its first query with `42704`.

## 5. What the macro refuses (item 3)

The macro has only the SQL text and the paths and types written in it. It runs the morm-core scanner (section 7), which drops comments and keeps strings, quoted identifiers and dollar quotes whole. Then it looks at the tokens next to each `${..}`. That's a few token patterns, with no SQL parser. Each refusal below is a compile error naming the `${..}` and the fix. Each was built in the prototype on rustc 1.99.1, 18 failing examples and 3 that compile:

| # | Refused | How it's seen | Prototype case |
|---|---|---|---|
| M1 | A positional `$1` | A parameter token outside strings and comments | `b = $1` |
| M2 | `${v}` with no `: field` or `: Type` | No top-level `:` that isn't `::` | `b = ${v}` |
| M3 | `${..}` inside a string, quoted identifier or dollar quote | The token's text holds `${` | `LIKE '%${q: String}%'` refused, with the fix `'%' \|\| ${q: String} \|\| '%'` |
| M4 | A free value beside a column named through `F` | The tokens on either side: `{M::F.c} op ${v: T}` or `${v: T} op {M::F.c}`, for `=`, `<>`, `!=`, `<`, `<=`, `>` and `>=`; `IS [NOT] DISTINCT FROM`; `[NOT] BETWEEN ${a} AND ${b}`; `[NOT] IN (.., ${v}, ..)`; `op ANY\|ALL\|SOME(${v})`; `SET {M::F.c} = ${v}`; and the matching position in `INSERT .. ({M::F.a}, ..) VALUES (.., ${v}, ..)` | `{Order::F.c} = ${name: String}`, the reverse with `<>`, `BETWEEN`, `IN`, `= ANY(${ids: Vec<i64>})`, the `VALUES` position, `IS NOT DISTINCT FROM` |
| M5 | A cast that cuts or rounds, after a parameter | `${..}::` or `CAST(${..} AS` followed by `varchar`, `character [varying]`, `char`, `bpchar`, `bit [varying]`, `varbit`, `numeric`, `decimal`, `timestamp[tz]`, `time[tz]`, `interval` or `float`, with `(`; or a bare `char`, `character` or `bit` | `${s: String}::varchar(3)`, `CAST(${s: String} AS char)` |
| M6 | A field-bound scalar inside `ANY(..)`, `ALL(..)` or `SOME(..)` | The token before `(` | `= ANY(${v: Order::F.b})` |
| M7 | A field-bound list beside a comparison operator, outside `ANY(..)` | The token before or after | `{Order::F.b} = ${ids: Order::F.b[]}` |

The type system refuses the rest at compile time:

- **A value that doesn't fit its field:** `i64` into an `i32` field fails with ``the trait bound `i64: Fits<i32>` is not satisfied``.
- **A free value of another type:** `${n: i64}` with an `i32` fails with ``the trait bound `i32: Is<i64>` is not satisfied``.
- **A list of an array column** is a const panic.
- **A missing field** fails with ``no field `nope` on type `OrderF` ``.
- **A generic free type** fails with `E0401`.

**Choice D** is M4's scope. It refuses even when the free value's type is the column's own. Binding through the field then follows the model when the column's `sql` changes, from `text` to `citext` for example, and gets the compile-time fit check.

**What the macro can't see:** a column written as a plain identifier (`email = ${e: String}`, which compiles), a column reached through an expression, a subquery or a CTE, and any column type at all. The macro never knows types, only which side is written through `F`. check-sql sees all of those.

## 6. The CI check (item 4)

**What runs.** `morm-admin mgr check-sql` (R-0431, R-0620) builds `morm-mgr` with the registry (R-0046) and connects to the scratch database the chain migrated. It loads three catalog facts once: each type's domain base, every cross-type btree operator with its family, and every function with a polymorphic argument. Then, for every registered query:

1. It sets `debug_print_parse = on`, `debug_pretty_print = off` and `client_min_messages = log`, once per connection. These are user settings: an ordinary role could set them, and the trees came back to it as `LOG` notices.
2. It sends Parse with the query's carriers, exactly as the app does. A Postgres error here is a wrong shape (`42883`, `42809`, `42804`), so it's a failure with the code and message. Examples: `b = ($1::int8[])` gives `operator does not exist: bigint = bigint[]`, and `ANY(($1::int8))` gives `op ANY/ALL (array) requires array on right side`.
3. It reads the analyzed parse tree from the first `parse tree` notice (`outfuncs` text) and applies T1 to T4 below.
4. It runs the `PREPARE`, result-type and `EXPLAIN` checks R-0431 and R-0620 already ask for.

**The rules.** A *table column* is a `Var` on a relation, followed through join and `GROUP BY` aliases. *Converted* means an implicit coercion node: `RelabelType`, `FuncExpr`, `CoerceViaIO`, `ArrayCoerceExpr` or `CoerceToDomain` with format `COERCE_IMPLICIT_CAST`. A value's type is its type before any implicit conversion.

- **T1, comparisons.** In a comparison with a table column on one side, a mismatch is any of:
  - Postgres converted the column, other than a domain to its base, `varchar` to `text` or `cidr` to `inet`;
  - Postgres converted the value, other than those, a widening between integer types, an integer to `numeric`, or `float4` to `float8`;
  - neither was converted, but the operator is a cross-type btree operator outside the integer, float, text and network families, such as `date` against `timestamp`.

  A comparison here is:
  - an operator that returns `boolean`, `IS DISTINCT FROM`, `NULLIF`, `IN`, or `op ANY(..)` with the column on either side, a scalar column on the left or an array column on the right;
  - a row comparison, `GREATEST`, `LEAST`, or `CASE x WHEN`;
  - a call to a polymorphic function, such as `array_position`.

  A native operator on two types, such as `jsonb ? text` or `citext LIKE text`, isn't a mismatch. A comparison between two columns is checked too, and reported with no parameter.
- **T2, a field-bound value converted.** Postgres implicitly converts a value bound through a field anywhere, other than the safe forms above. So the field was the wrong one for that place.
- **T3, a free value converted on its way somewhere.** A free value goes through an implicit conversion that changes the value or what it means: one in a function (`FuncExpr` or `CoerceViaIO`) other than the safe widenings, or one that applies a precision or scale (`numeric`, `timestamp[tz]`, `time[tz]`, `interval`). This covers writes: a `NaiveDateTime` into `timestamptz`, `f64` into `numeric`, an `i64` into `integer`, and a `Decimal` that `numeric(12,2)` would round. A relabel, such as `text` into a `citext` column, keeps the value, and a length check on `varchar(n)` or `char(n)` raises `22001` instead of cutting. Neither is reported.
- **T4, a cast that cuts or rounds.** An explicit cast with a length, precision or scale (a `FuncExpr` with explicit-cast format and a typmod argument) over a parameter. It's M5 again, for casts the macro's patterns miss.

**The evidence.** 57 queries, as morm would write them, with their carriers, each with the expected result, gave exactly that on 17.11 and 18.6. 36 were reported:

- `citext = text`, through a subquery, a CTE, `trim()`, `coalesce`, `CASE WHEN`, a `VALUES` join, `= ANY(text[])`, `IN`, `HAVING` after `GROUP BY` (Postgres 18's group RTE), a row `=` and a row `>`, `IS DISTINCT FROM`, `NULLIF`, `CASE c WHEN`, `text = ANY(citext[])` and `array_position`;
- `char(5) = text`, `bigint = numeric`, `timestamp = timestamptz` both ways, `date = timestamp`, a domain over `citext` against `text`, and `float8 = int8`;
- column against column, `citext = text`, in a `WHERE` and in a `JOIN .. ON`;
- `lower(citext) = citext value` (T2);
- free writes of `timestamp` into `timestamptz`, `numeric` and `float8` into `numeric(12,2)`, and `int8` into `integer` (T3);
- `::varchar(3)` and `::char` on a parameter (T4).

21 weren't reported, all of them right:

- `citext`, `char(5)`, an enum, a domain and `citext[]` each against their own cast;
- `varchar = text`, `int8 = int4`, `int4 = int8` and `numeric = int8`;
- `lower(c) = text`, `jsonb ? text`, `citext LIKE text` and `citext SIMILAR TO text`;
- `now() - interval`, a domain against its base type, and `text` into `citext` and into `varchar(5)`;
- `int4` into `bigint`, an `UPDATE` with field-bound values, a join with `date` and `timestamptz` bounds, and `substr(c, ..) = text` with `c || 'x'`.

**Finding every query.** `sql!` registers each query, and `db.sql()` and `db.raw()` take only `sql!` (choice B), so every fixed raw query is in the registry. `morm-mgr` sees what the build it's made from compiles. So `check-sql` passes `--features`, `--all-features` and `--no-default-features` through to that build, and `morm-admin init`'s CI template runs it with the features the app ships with. A query only under `cfg(test)` isn't in that build, so `#[morm::test]` checks it when a test runs it (choice H). Builder SQL is morm's own, and morm's suite tests it (section 10).

**How it fails.**

- Exit code `1`, the code for errors and drift.
- Each finding gives the file and line, the `${..}` source, the rule, the table and column with its type, the value's type, and the fix. For example: `src/orders.rs:42: ${name: String} is text, compared with users.email (citext): bind it through the field, as ${name: User::F.email}`.
- A wrong shape gives the Postgres code and message instead.
- Every finding prints before the exit, not just the first.

**The cost.** On local TCP, a five-parameter join cost 50 to 58 µs to Parse alone, and 313 to 413 µs per query with the tree and the rules. So 1,000 queries take about 0.4 s, on top of R-0620's `PREPARE` and `EXPLAIN`.

**Keeping the server log clean.** Postgres also writes each tree to its server log, about 5 KB per query here. When the scratch role is a superuser, check-sql sets `log_min_messages = panic` for its session. The trees still reached the client, and none reached the log. An ordinary role gets `42501` for that setting, so check-sql skips it, and the log grows by that much.

**The tree format.** check-sql reads only node names and these fields:

- `QUERY` `rtable` and `cteList`;
- `RANGETBLENTRY` `rtekind`, `joinaliasvars` and `groupexprs`;
- `VAR` `varno`, `varattno`, `vartype` and `varlevelsup`;
- `PARAM` `paramkind`, `paramid` and `paramtype`;
- the coercion nodes' `arg` or `args`, result type and format field;
- `OPEXPR`, `SCALARARRAYOPEXPR`, `DISTINCTEXPR` and `NULLIFEXPR` `opno` and `args`;
- `ROWCOMPAREEXPR` `largs` and `rargs`;
- `MINMAXEXPR` and `CASEEXPR` `args`, `CASEEXPR` `arg`, and `FUNCEXPR` `funcid` and `funcformat`.

It ignores everything else, so a field a version adds changes nothing. Every one of them was the same on 17.11 and 18.6. Step 45's test pins them on 15 to 18, as section 10 says.

**The alternative, choice E (B).** Describe each query with one parameter's cast removed, and compare the type Postgres infers with the cast. It needs no tree, but it can't see an inner scope. With the cast removed, Postgres inferred `text` for `c = (SELECT $1)`, `WITH q AS (SELECT $1 ..)`, `c = trim($1)`, `coalesce($1, $2)`, `CASE WHEN .. THEN $1` and a `VALUES` join, so those `citext` mismatches look right. It infers `citext` only for `c = $1`, `IN`, `BETWEEN` and `= ANY`. With a free `i64`, it would also report every position with no context, such as `SELECT ${v: i64}`, because Postgres infers `text` there.

## 7. Runtime-built SQL (item 5)

**The scanner (choice F, A).** R-0433's morm-core scanner already finds each `$n` in a fragment to renumber it. In the same pass, it writes the parameter's cast as `($k::cast)`. It follows Postgres's `scan.l` for everything that can hide a `$n`:

- standard strings with `''`, escape strings `E'..'` with backslashes, and `B'..'`, `X'..'`, `N'..'` and `U&'..'`;
- quoted identifiers with `""`, and `U&".."`;
- `--` comments, and nested `/* */` comments;
- dollar quotes `$$..$$` and `$tag$..$tag$`;
- identifiers that hold `$`, such as `a$1`;
- operator splitting, including the rule on a trailing `+` or `-`.

It refuses unterminated quotes and comments. The prototype's unit tests cover 16 ways a `$n` hides, operator splitting, unterminated input, renumbering and interpolation. One example: `price < $1 AND tag = '$1' AND x = ANY($2) -- $1`, numbered from 3, becomes `price < ($3::numeric) AND tag = '$1' AND x = ANY(($4::text[])) -- $1`.

**Which cast each slot gets:**

| Where | Bound | Cast |
|---|---|---|
| `.qc(F.col, op, &v)`, `.by`, `.any`, `.set`, `.version`, keyset bounds and cursors, `load_related`, `get_by`, `delete_by`, `update_each`, model inserts, updates and upserts, `UNNEST` arrays and R-0350's array literals | Through the field | The column's cast, from the `F` constant |
| `comp` in `get_where` and `delete_where` | Through the comp's field (choice G) | The column's cast. The values are `&[&dyn Fits<T>]`, so they're checked at compile time. A list in the comp, as in `<> ALL($1)`, gets `[]` |
| `set_expr`, `.q()` fragments, `q_unchecked`, `sql_unchecked` | Free | Each value's `Param::cast()` |
| `.limit(n)`, morm's own advisory locks, `set_config` for `tx.set_local` | Free, morm's own | `int8`. `set_local`'s value is cast to its own type and then `::text`, because `set_config` takes `text` |

`COPY` sends no parameters, so it needs no cast. The conversion layer encodes each column as today.

**What it costs.** The built text is cached as R-0715's built-SQL map and R-0705's `shape_cache` already cache it, keyed with the casts. A call site's value types are fixed, so its casts are too, and the scanner runs once per shape.

**What detects mismatches here:**

- A wrong shape is a Postgres error, because every parameter is typed.
- `#[morm::test]` runs check-sql's rules on each runtime-built shape the first time the test process sees it (choice H), so a free `String` in `.q("email = $1")` against `citext` fails the test that runs it.
- The builders' field-bound slots can't mismatch in the first place. Their casts come from the column.

**The alternative (choice F, B).** A fragment macro, `.q(frag!("price < ${max: Decimal}"))`, would give fragments M1 to M7 at compile time. But `comp`, `set_expr` and `.q()` would all change shape, and the scanner is still needed for `sql_unchecked`. Without a full statement, a fragment still can't be described on its own.

## 8. What can't be detected (item 6)

These are all that's left. Each is outside what Postgres shows morm for the statement it sends, or outside any build or test morm sees:

1. **Comparisons inside the database's own code.** A function, procedure, trigger or view body, and dynamic SQL run inside the database (`EXECUTE format(..)` in plpgsql). If `find_user(${e: String})` takes `text` and compares it with a `citext` column inside its body, the query's tree shows only a `text` argument.
2. **Runtime-built shapes that no test runs.** `.q()`, `set_expr`, `q_unchecked` and `sql_unchecked` get their casts, and Postgres errors on wrong shapes. But a silent mismatch, such as a free `String` against `citext`, is found only by a `#[morm::test]` that runs that shape.
3. **Code that neither the check-sql build nor a test compiles,** such as a query behind a feature or `cfg` that CI leaves off.
4. **Values compared inside JSON.** `jsonb` operators compare by JSON's rules, not SQL types: `attrs @> jsonb_build_object('n', ${n: String})` never matches a JSON number `5`.
5. **Your own explicit casts.** `${v: String}::citext`, or `col::text = ..`, says what you mean, so nothing reports it, except a cast with a length, precision or scale on a parameter (M5, T4).

The loud README notice, in R-1008's form, beside raw SQL:

> **Bind a value through its field whenever it meets a column.** morm writes every parameter's type into the SQL. A value bound through a field, as `${email: User::F.email}`, `.by(F.email, ..)` or `.qc(F.email, ..)`, gets the column's own type and can't mismatch. A free value, as `${email: String}`, `.q()` or `set_expr`, gets its Rust type's type, here `text`. If it's compared with a column of another type, Postgres converts one side, and the answer can be silently wrong: `text` against a `citext` column is case-sensitive, and `text` against `char(n)` minds trailing spaces. `sql!` refuses a free value beside a column it can see. `check-sql` and `#[morm::test]` report every other mismatch in the SQL morm sends. They can't see five places, so bind through the field there:
>
> - comparisons inside your functions, procedures, triggers and views, and dynamic SQL run inside the database;
> - runtime-built SQL that no `#[morm::test]` runs;
> - code that's left out of the build `check-sql` makes and out of your tests;
> - values compared inside JSON;
> - your own explicit casts.
