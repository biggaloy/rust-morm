# Step 16: Raw SQL, related rows and keyset pages

- **Phase:** Runtime
- **Depends on:** 15
- **Can run beside:** 08, 09, 17, 29, 30, 31, 32, 33, 34, 35
- **Crates:** `morm`, `morm-macros`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

The read paths beyond a model's own table.

## Deliverables

- `sql!`'s syntax, casts and compile-time refusals, `morm::Sql<'a, P>` and `Params`, `db.sql` and `db.raw` taking only `sql!` output, `sql_unchecked`, `#[derive(morm::Row)]`, `load_related`, `keyset`, `.limit` on raw statements

## End-to-end check

Raw SQL, related rows and keyset pages return correct rows on real Postgres, including pages under concurrent writes, and each compile-time rule fails under `trybuild`.

## Out of scope

- `sql!`'s registration and `check-sql` (step 45)
- `fetch_json` formats (step 23)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Reading and writing / Related rows

**R-0402** morm doesn't load relations automatically, but it makes the one-query form the easiest:

**R-0403**

```rust
let orders = db.get_all_by::<Order>(F.user_id, &7).limit(50).await?;
let items = db.load_related::<OrderItem>(F.order_id, &orders, |o| o.id).await?;

for order in &orders {
    let its_items = items.of(order.id);                // &[OrderItem], no query
}
```

**R-0404** It runs one `WHERE order_id = ANY($1)` query for the whole list. Rows are kept in one sorted `Vec`, so `of` returns a borrowed slice with no per-parent allocation. It's bounded by the parent list, so it doesn't need `.limit()`, and `.limit_each(n)` caps rows per parent. It takes a view, so hidden columns stay hidden. Each relation is one visible call, so nothing loads by surprise.

### Reading and writing / Pagination

**R-0405** `OFFSET 100000` reads and discards 100,000 rows on every page, so morm has no `.offset()` helper. Keyset pagination is equally fast at any depth:

**R-0406**

```rust
let page = db.select::<Order>().filter()
    .qc(F.user_id, "=", &user_id)
    .keyset((F.created_at.desc(), F.id.desc()))
    .after(cursor.as_deref())                    // None for the first page
    .limit(50)
    .fetch_page().await?;

page.rows;                                       // Vec<Order>
page.next;                                       // opaque cursor, None on the last page
```

- **R-0407** The last key must be unique, so rows can't be skipped, and without a unique column it's a compile error. Mixed ascending and descending columns are supported.
- **R-0408** **A nullable key column is a compile error.** `(a, b) > (1, 1)` is unknown when `b` is NULL, so that row would never appear on any page. Use a `NOT NULL` column.
- **R-0409** A cursor is base64 of the key values. They're decoded as typed parameters, so a tampered cursor returns `InvalidCursor` or a different page, never SQL. Cursors aren't a secret.
- **R-0410** Writes between pages can't shift rows, as they do with `OFFSET`.

### Reading and writing / Raw SQL, with no model

**R-0411** `db.sql()` runs any statement. No model, hooks, validation or ordering are involved.

**R-0412**

```rust
#[derive(morm::Row)]
struct Sale { day: NaiveDate, total: Decimal }

let q = sql!("SELECT day, sum(total) AS total FROM orders
              WHERE {Order::F.vendor_id} = ${vendor_id: Order::F.vendor_id} GROUP BY day");

let rows: Vec<Sale> = db.sql(&q).limit(30).fetch().await?;                   // named struct
let rows: Vec<(NaiveDate, Decimal)> = db.sql(&q).limit(30).fetch().await?;   // tuple
let body = db.sql(&q).limit(30).fetch_json().await?;                         // JSON bytes
let rows = db.sql(&q).limit(30).fetch_raw().await?;                          // rows as sent
let n: i64 = db.sql(sql!("SELECT count(*) FROM orders")).fetch_val().await?; // one value
let changed = db.sql(sql!("UPDATE orders SET seen = true WHERE {Order::F.id} = ${id: Order::F.id}")).run().await?;  // row count
```

- **R-0413** Every list and single-result call works here, including `fetch_ref`, `stream` and `stream_json`.
- **R-0414** `#[derive(morm::Row)]` decodes by column name, for any plain struct. Columns the struct lacks are ignored, and a missing column is an error.
- **R-0415** `db.sql()` and `db.raw()` take only what `sql!` returns, or a reference to it. `sql_unchecked(String, &[&dyn Param])` is the visible escape hatch, with the scanner's casts. Owner decision DH2, choice B.
- **R-1223** **`sql!`'s syntax.** `sql!` takes one string literal. Outside strings, quoted identifiers, comments and dollar quotes, which the `morm-core` scanner skips, it reads three forms, and neither `{` nor `${` is valid SQL there:

  | Form | Writes | Checked at compile time |
  |---|---|---|
  | `{Order::F.vendor_id}` | The quoted column name, `"vendor_id"`, from the `F` constant | — |
  | `${vendor_id: Order::F.vendor_id}` | `($n::int8)`, the column's cast | The value fits the field's Rust type without loss |
  | `${ids: Order::F.id[]}` | `($n::int8[])`, the column's list cast | A `Vec`, slice or array whose elements fit |
  | `${since: NaiveDate}` | `($n::date)`, the type's `SqlType` cast | The value is that type, or a reference to it |

  Each value sits where it's used, so values can't be misnumbered. `[]` binds a list for `= ANY(..)`, `<> ALL(..)` and `unnest(..)`; an array column binds a whole array without `[]`, and `[]` on one is a compile error. An `Option` value fits only a nullable field, and a free `Option<T>` is cast as `T`. A cast of your own after a value is kept, so `${s: String}::citext` is written as `($1::text)::citext`. A free value's type must be concrete, because the query is registered in a static, so `${v: T}` inside a generic `fn` fails with `E0401`. The text is a compile-time constant, built by `const` concatenation of the literal's pieces, the `F` names and the casts. `sql!` returns a `morm::Sql<'a, P>`: the text, its registration, and `P`, the query's own set of values, which the compiler fills in at each `sql!`: a tuple of borrows, each tagged with its carrier, with nothing boxed or allocated. `sql!` writes it as struct literals, so `let q = sql!(..)` keeps any temporary a value makes alive as long as `q`. A query can be stored in a variable, run several times, passed to a function and run there, and a function returning one writes `-> Sql<'a, impl Params>`. `db.sql()` and `db.raw()` are generic over `P`, so its values are encoded straight from the tuple. Owner decisions DH2, choice A, and DJ3.
- **R-1224** **What `sql!` refuses.** It runs the scanner and looks at the tokens beside each `${..}`, with no SQL parser, and each refusal is a compile error naming the `${..}` and the fix:
  - a positional `$1` outside strings and comments;
  - a `${v}` with no `: field` or `: Type`;
  - a `${..}` inside a string, quoted identifier or dollar quote, such as `LIKE '%${q: String}%'`, whose fix is `'%' || ${q: String} || '%'`;
  - a free value beside a column named through `F`, even of the column's own type (choice D): `{M::F.c} op ${v: T}` or the reverse, for `=`, `<>`, `!=`, `<`, `<=`, `>` and `>=`, `IS [NOT] DISTINCT FROM`, `[NOT] BETWEEN`, `[NOT] IN (..)`, `op ANY|ALL|SOME(..)`, `SET {M::F.c} = ${v}`, and the matching position in `INSERT .. ({M::F.a}, ..) VALUES (.., ${v}, ..)`;
  - a cast that cuts or rounds after a value: `::` or `CAST(.. AS` followed by `varchar`, `character [varying]`, `char`, `bpchar`, `bit [varying]`, `varbit`, `numeric`, `decimal`, `timestamp[tz]`, `time[tz]`, `interval` or `float` with `(`, or a bare `char`, `character` or `bit`;
  - a field-bound scalar inside `ANY(..)`, `ALL(..)` or `SOME(..)`;
  - a field-bound list beside a comparison operator, outside `ANY(..)`.

  The type system refuses the rest: a value that doesn't fit its field, a free value of another type than the one named, a list of an array column, a field that doesn't exist, and a generic free type. Owner decision DH2, choice D.
- **R-1225** **Related rows and pages bind through the field.** `load_related`'s `= ANY($1)` takes the parent keys through the related field, cast to its list type, and keyset bounds and decoded cursors bind through their key columns, cast to each column's type. `.limit(n)` is cast as `int8`. Owner decision DH2.
- **R-0417** **The list rule still applies.** `.limit(n)` appends a `LIMIT` on a new line, so a trailing `--` comment can't swallow it, and as a parameter, so different limits share one statement, and `.all()` runs it as written. `run()`, `fetch_val()`, `fetch_one()` and `fetch_opt()` don't need either.
- **R-0418** It uses the same bounded statement cache, and `db.raw::<T>()` remains for raw SQL decoding into a model or view.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0761 | Paging with `OFFSET` | No helper exists; `keyset` is the documented path |
| R-0774 | A nullable keyset column | Compile error |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0010 | trybuild | a nullable keyset column |
| T-0061 | integration | keyset pages under concurrent writes and with mixed sort directions |
| T-0062 | integration | tampered cursors |
| T-0066 | integration | `load_related` with `.limit_each` |
| T-0284 | trybuild | **`sql!` refusals:** each form R-1224 lists, 18 cases in all, among them a positional `$1`, `${v}` with no type, `LIKE '%${q: String}%'`, a free value beside an `F` column with `=`, `<>`, `BETWEEN`, `IN`, `= ANY`, `IS NOT DISTINCT FROM` and in a `VALUES` position, `::varchar(3)` and `CAST(.. AS char)` after a value, a field-bound scalar in `ANY(..)`, a field-bound list beside `=`, a value that doesn't fit its field, a free value of another type, `[]` on an array column and a missing field, plus `${v: T}` in a generic `fn` failing with `E0401`; and three that compile |
| T-0285 | integration | **`sql!` on Postgres 15–18:** queries with `citext`, `char(5)`, an `i32` on a `bigint` field, `= ANY` lists, an enum and an enum list, a `citext[]` column, a free enum, `Option<i64>` as `NULL`, a free `Vec<String>` in `array_length` and `&str` in `concat` give the right answer through a prepared statement and through `query_typed`, and the text sent holds each cast in its place |
| T-0299 | integration | **A stored query, on Postgres 15–18:** `let q = sql!(..)` holding a field-bound value, a free value, a free value whose expression makes a temporary, and a field-bound list, then run twice as `db.sql(&q)` and once as `db.sql(q)`, gives the right answer each time; so do a query passed to another `async fn` and run there, one returned from a function written `-> Sql<'a, impl Params>`, and one built inside `tokio::spawn`, borrowing a captured row, and awaited there twice; under a counting allocator, building the query and binding its values allocate nothing, and a run allocates no more than the same statement run straight on `tokio_postgres` with a borrowed slice of its values |
