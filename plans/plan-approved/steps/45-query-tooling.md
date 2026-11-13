# Step 45: `sql!`, `check-sql`, test support and the loop detector

- **Phase:** Tools
- **Depends on:** 42
- **Can run beside:** 23, 39, 40, 43, 44, 51, 52, 53, 54, 58, 59, 60
- **Crates:** `morm`, `morm-macros`, `morm-admin`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Checked raw SQL with no database at build time, and test tooling for users.

## Deliverables

- `sql!`'s registration, `check-sql` with `PREPARE`, `EXPLAIN` and the typing pass, including hidden columns and unquoted mixed-case names, `#[morm::test]` with its typing check, the loop detector, `morm::allow_repeats`

## End-to-end check

A wrong column in `sql!` fails `check-sql`, a free value compared with a column of another type fails `check-sql` and a `#[morm::test]`, a query in a loop fails a `#[morm::test]`, and each exclusion passes.

## Out of scope

- The lock-order detector (step 49)
- `sql!`'s syntax, casts and compile-time refusals (step 16)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Models / Table and column names

- **R-0125** Hand-written SQL must then quote the name, and `check-sql` catches the mistake in registered queries.

### Views and groups / Views: restricted models

- **R-0235** `check-sql` flags queries selecting hidden columns.

### Reading and writing / Raw SQL, with no model

- **R-0416** `sql!` also registers it for `check-sql`.

### Reading and writing / Query safety

- **R-0431** `sql!` registers each query it builds, with its text, file and line, and each parameter's number, kind, cast, model and column and its `${..}` source. `morm-admin mgr check-sql` runs `PREPARE` on each against scratch, compares types, and checks every parameter's type in the analyzed parse tree. That gives checked raw SQL with no database at build time.
- **R-1231** **check-sql's typing pass.** Connected to the scratch database the chain migrated, `check-sql` loads three catalog facts once: each type's domain base, every cross-type btree operator with its family, and every function with a polymorphic argument. Once per connection it sets `debug_print_parse = on`, `debug_pretty_print = off` and `client_min_messages = log`, which an ordinary role may set, and `log_min_messages = panic` when the role is a superuser, so the trees reach the client and not the server's log; an ordinary role gets `42501` for that one and it's skipped. For each registered query it sends Parse with the query's carriers, as the app does, and a Postgres error there is a wrong shape, reported with its code and message. It reads the analyzed tree from the first `parse tree` notice, following a table column, a `Var` on a relation, through join and `GROUP BY` aliases, and treating as converted any `RelabelType`, `FuncExpr`, `CoerceViaIO`, `ArrayCoerceExpr` or `CoerceToDomain` with implicit-cast format, and applies four rules:
  - **T1, comparisons:** where a table column is on one side of an operator returning `boolean`, `IS DISTINCT FROM`, `NULLIF`, `IN`, `op ANY(..)`, a row comparison, `GREATEST`, `LEAST`, `CASE x WHEN` or a polymorphic function, it's a mismatch if Postgres converted the column, other than a domain to its base, `varchar` to `text` or `cidr` to `inet`; if it converted the value, other than those, a widening between integer types, an integer to `numeric`, or `float4` to `float8`; or if neither was converted but the operator is a cross-type btree operator outside the integer, float, text and network families, such as `date` against `timestamp`. A native operator on two types, such as `jsonb ? text`, isn't a mismatch. A comparison between two columns is checked too;
  - **T2:** a value bound through a field is implicitly converted anywhere, other than the safe forms above;
  - **T3:** a free value goes through an implicit conversion that changes it or what it means: one in a function (`FuncExpr` or `CoerceViaIO`) other than the safe widenings, or one that applies a precision or scale (`numeric`, `timestamp[tz]`, `time[tz]`, `interval`), which covers writes such as a `NaiveDateTime` into `timestamptz` or an `i64` into `integer`. A relabel such as `text` into `citext`, and a length check on `varchar(n)` or `char(n)`, aren't reported;
  - **T4:** an explicit cast with a length, precision or scale over a parameter.

  It reads only those node names and fields and ignores the rest, so a field a version adds changes nothing. It passes `--features`, `--all-features` and `--no-default-features` through to the `morm-mgr` build, and `morm-admin init`'s CI template runs it with the features the app ships with. Every finding prints before it exits with code `1`, each with the file and line, the `${..}` source, the rule, the table and column with its type, the value's type and the fix, such as `src/orders.rs:42: ${name: String} is text, compared with users.email (citext): bind it through the field, as ${name: User::F.email}`. Owner decision DH2, choice E.
- **R-1232** **`#[morm::test]` checks the SQL a test runs.** The first time a test process sees a runtime-built shape or a `sql!` query, `#[morm::test]` runs R-1231's typing pass on it, on one extra connection to the test database, and fails the test on any finding, with the same output. So a free `String` in `.q("email = $1")` against a `citext` column fails the test that runs it, and a query only compiled under `cfg(test)` is checked too. Owner decision DH2, choice H.

### Migrations / Commands

- **R-0620** `check-sql`: `PREPARE` and `EXPLAIN` on every registered query.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0785 | Queries in a loop | Detector in debug builds and `#[morm::test]`: the same statement repeating many times in a short window warns, and fails the test. `fetch_page`, `.in_batches`, streams, the outbox worker and retries don't count, and `morm::allow_repeats(fut)` marks an intended loop |
| R-0787 | Filtering on an unindexed column | `morm-admin mgr check-sql` runs `EXPLAIN` on every registered query |

**R-0788** The compiler can't see a query inside a loop, so that one is caught in tests and debug builds, not at compile time.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0040 | integration | the loop detector |
| T-0116 | suite | **`#[morm::test]`:** gives each test a database cloned from a migrated template, for this project and its users. |
| T-0294 | integration | **The typing pass, on Postgres 15–18:** at least 57 queries, as morm writes them with their carriers, each give their expected findings and no others, which pins the tree's node names and fields on each version. Reported: `citext = text` plainly, through a subquery, `= ANY(SELECT ..)`, a CTE, `trim()`, `coalesce`, `CASE WHEN`, a `VALUES` join, `= ANY(text[])`, `IN`, `GREATEST`, `HAVING` after `GROUP BY`, a row `=` and `>`, `IS DISTINCT FROM`, `NULLIF`, `CASE c WHEN`, `text = ANY(citext[])` and `array_position`; `char(5) = text`, `bigint = numeric`, `timestamp = timestamptz` both ways, `date = timestamp`, a domain over `citext` against `text`, `float8 = int8`; column against column in a `WHERE` and a `JOIN .. ON`; `lower(citext)` against a field-bound `citext` value (T2); free writes of `timestamp` into `timestamptz`, `numeric` and `float8` into `numeric(12,2)` and `int8` into `integer` (T3); `::varchar(3)` and `::char` on a parameter (T4). Not reported: each type against its own cast, `varchar = text`, `int8 = int4`, `numeric = int8`, `lower(c) = text`, `jsonb ? text`, `citext LIKE text`, a domain against its base type, `text` into `citext` and into `varchar(5)`, and `int4` into `bigint`. A wrong shape, such as `b = ($1::int8[])`, is reported with `42883`. Every finding prints before the exit with code `1`, the trees reach no server log under a superuser, and the PR states the cost per query |
| T-0295 | integration | **`#[morm::test]`'s check:** a test that runs `.q("email = $1")` with a free `String` against a `citext` column fails with the typing pass's finding, the same query bound through the field with `.qc` passes, a `sql!` query only compiled under `cfg(test)` is checked when a test runs it, and each shape is checked once per test process |
| T-0296 | integration | **The parameter matrix's free values are reported, on Postgres 15–18:** each free value from step 18's matrix whose answer differs from the field's, `text` against `citext` or `char(n)`, an instant against `timestamp`, a naive time against `timestamptz` and `jsonb` written into `json`, gets a finding from `check-sql` through `sql!`'s `${v: T}`, and from `#[morm::test]` through `.q()` and `set_expr`, while the same values bound through the field get none |
