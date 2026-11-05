# Step 14: Reads

- **Phase:** Runtime
- **Depends on:** 13, 06
- **Can run beside:** 07, 08, 11, 15, 25, 26, 27, 28, 29
- **Crates:** `morm`, `morm-macros`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Every way of reading, and the shared `by`/`any` builder.

## Deliverables

- `get` forms, `select`, `filter`, `filter_cols`, every list and single-result call, `db.sql`, `db.raw`, `#[derive(morm::Row)]`, `load_related`, `keyset`, `order_by_allowed`, `.custom_plan()`, `.tag()` on queries

## End-to-end check

A test app reads the same rows through every call and gets identical data, and each compile-time rule fails under `trybuild`.

## Out of scope

- `fetch_json` formats (step 21)
- Groups on a handle (step 23)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Reading and writing

**R-0326** In the examples, `F` is short for the relevant model's `F`, such as `Product::F`.

### Reading and writing / Get

**R-0329**

```rust
db.get::<User>(5).await?;                                  // by PK, Option<User>
db.get_by::<User>(F.email, &email).await?;                 // any column
db.get_where::<Product>(F.price, "BETWEEN $1 AND $2", &[&dec!(5), &dec!(2000)]).await?;   // any comparison
db.get_all_by::<Order>(F.user_id, &7).limit(50).await?;    // many rows

// several columns: one builder shared by get, update, delete, count, exists
db.get_q::<Hist>().by(F.owner_id, &1).by(F.app, &"chat").one().await?;
```

- **R-0331** `comp` is `&'static str`, so it can't come from a request.
- **R-0332** The statement is cached on first use, keyed by model, column and the `comp` pointer, so repeats allocate nothing.
- **R-0333** Composite PKs take tuples.

### Reading and writing / Lists

**R-0378**

```rust
let rows = db.select::<Product>()
    .filter()
    .qc(F.vendor_id, "=", &vendor_id)
    .q(" AND ").qc(F.price, "<", &max_price)
    .order()                         // model's default ordering
    .limit(50)
    .fetch_ref().await?;

// only some columns; the row type comes from the columns
let rows: Vec<(i64, String)> = db.select::<Product>()
    .filter_cols((F.id, F.name))
    .qc(F.vendor_id, "=", &vendor_id)
    .limit(50)
    .fetch().await?;

let products: Vec<Product> = db.raw::<Product>("SELECT * FROM products WHERE attrs ? $1", &[&"color"])
    .limit(50).fetch().await?;
```

- **R-0379** `.filter()` selects every down column: all the columns this model or view may read, without hidden, write-only or `deprecated` ones. It's the `.filter_cols(...)` default, since Rust has no default arguments.
- **R-0380** `.filter_cols((F.a, F.b))` selects only those. `fetch()` returns tuples of the columns' Rust types and `fetch_ref()` returns borrowed tuples, so a wrong type is a compile error, and a partial row can't pass for a full `Product`. `fetch_json`, `stream_json` and `fetch_raw` work the same way.
- **R-0381** Columns come from `F`, so a view can't select what it hides. Group rules on a `db.groups(mask)` handle are in section 5.
- **R-0382** The column list is cached like other statements, so repeats allocate nothing.
- **R-0383** **Sort columns from a request** can't be parameters, and `.q()` won't accept them. `.order_by_allowed(&input, &[F.name, F.age])` takes `"name"` or `"-age"`, and returns `InvalidOrder` for anything not on the list. `morm::quote_ident()` is only for names known at runtime in `q_unchecked` and `sql_unchecked`: it wraps a name in double quotes and doubles any quote inside it, so it can't break out. `F` columns are already quoted.
- **R-0384** `.custom_plan()` plans a query for its actual values, for skewed data where one customer has a million orders and most have ten. It runs through `query_typed`, so it needs no `SET` and no extra round trip. The `plan_cache_mode` setting does the same for a whole pool.

**R-0385** A list needs `.limit(n)`, `.all()` or `stream()`. Lists return one of:

| ID | Call | You get | Conversions | Use when |
|---|---|---|---|---|
| R-0386 | `fetch_json()` | JSON bytes built by Postgres | None in the app | The data goes straight to a client |
| R-0387 | `stream_json()` | The same, chunked | None in the app | Large responses, flat memory |
| R-0388 | `fetch_raw()` | Rows as Postgres sent them; columns read on demand | Only the columns you read | You need a few columns from each row |
| R-0389 | `fetch_ref()` | Borrowed `ProductRef<'r>` | Strings and `jsonb` borrowed from the row buffer, no per-field allocation | Reading and computing without keeping the data |
| R-0390 | `fetch()` | Owned `Product` | Row → struct | You keep or pass the data along |
| R-0391 | `stream()` | One row at a time | Row by row | Huge results |

**R-0392** `fetch_one`, `fetch_opt`, `fetch_val`, `exists` and `count` cover the single-result cases.

### Reading and writing / Related rows

**R-0393** morm doesn't load relations automatically, but it makes the one-query form the easiest:

**R-0394**

```rust
let orders = db.get_all_by::<Order>(F.user_id, &7).limit(50).await?;
let items = db.load_related::<OrderItem>(F.order_id, &orders, |o| o.id).await?;

for order in &orders {
    let its_items = items.of(order.id);                // &[OrderItem], no query
}
```

**R-0395** It runs one `WHERE order_id = ANY($1)` query for the whole list. Rows are kept in one sorted `Vec`, so `of` returns a borrowed slice with no per-parent allocation. It's bounded by the parent list, so it doesn't need `.limit()`, and `.limit_each(n)` caps rows per parent. It takes a view, so hidden columns stay hidden. Each relation is one visible call, so nothing loads by surprise.

### Reading and writing / Pagination

**R-0396** `OFFSET 100000` reads and discards 100,000 rows on every page, so morm has no `.offset()` helper. Keyset pagination is equally fast at any depth:

**R-0397**

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

- **R-0398** The last key must be unique, so rows can't be skipped, and without a unique column it's a compile error. Mixed ascending and descending columns are supported.
- **R-0399** **A nullable key column is a compile error.** `(a, b) > (1, 1)` is unknown when `b` is NULL, so that row would never appear on any page. Use a `NOT NULL` column.
- **R-0400** A cursor is base64 of the key values. They're decoded as typed parameters, so a tampered cursor returns `InvalidCursor` or a different page, never SQL. Cursors aren't a secret.
- **R-0401** Writes between pages can't shift rows, as they do with `OFFSET`.

### Reading and writing / Raw SQL, with no model

**R-0402** `db.sql()` runs any statement. No model, hooks, validation or ordering are involved.

**R-0403**

```rust
#[derive(morm::Row)]
struct Sale { day: NaiveDate, total: Decimal }

let q = sql!("SELECT day, sum(total) AS total FROM orders WHERE vendor_id = $1 GROUP BY day");

let rows: Vec<Sale> = db.sql(q, &[&vendor_id]).limit(30).fetch().await?;     // named struct
let rows: Vec<(NaiveDate, Decimal)> = db.sql(q, &[&vendor_id]).limit(30).fetch().await?;  // tuple
let body = db.sql(q, &[&vendor_id]).limit(30).fetch_json().await?;           // JSON bytes
let rows = db.sql(q, &[&vendor_id]).limit(30).fetch_raw().await?;            // rows as sent
let n: i64 = db.sql("SELECT count(*) FROM orders", &[]).fetch_val().await?;  // one value
let changed = db.sql("UPDATE orders SET seen = true WHERE id = $1", &[&id]).run().await?;  // row count
```

- **R-0404** Every list and single-result call works here, including `fetch_ref`, `stream` and `stream_json`.
- **R-0405** `#[derive(morm::Row)]` decodes by column name, for any plain struct. Columns the struct lacks are ignored, and a missing column is an error.
- **R-0406** The SQL must be `&'static str`, as with `.q()`, and `sql_unchecked(String)` is the visible escape hatch. `sql!` also registers it for `check-sql`.
- **R-0407** **The list rule still applies.** `.limit(n)` appends a `LIMIT` on a new line, so a trailing `--` comment can't swallow it, and as a parameter, so different limits share one statement, and `.all()` runs it as written. `run()`, `fetch_val()`, `fetch_one()` and `fetch_opt()` don't need either.
- **R-0408** It uses the same bounded statement cache, and `db.raw::<T>()` remains for raw SQL decoding into a model or view.

### Reading and writing / Query safety

- **R-0422** `.q()` accepts only `&'static str`, so `format!`-ed input won't compile. `q_unchecked(String)` is the visible escape hatch.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0730 | Building SQL from strings | Compile error (`q_unchecked` is the visible escape hatch) |
| R-0731 | A list with no limit | Compile error: requires `.limit(n)`, `.all()` or `stream()` |
| R-0739 | A request-supplied sort column | `.q()` refuses it; `order_by_allowed` checks it against a list |
| R-0740 | Paging with `OFFSET` | No helper exists; `keyset` is the documented path |
| R-0753 | A nullable keyset column | Compile error |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0009 | trybuild | list with no limit |
| T-0010 | trybuild | a nullable keyset column |
| T-0021 | trybuild | non-static `q()` and `sql()` |
| T-0033 | integration | every get form |
| T-0035 | integration | views through every read |
| T-0061 | integration | keyset pages under concurrent writes and with mixed sort directions |
| T-0062 | integration | tampered cursors |
| T-0063 | integration | `order_by_allowed` |
| T-0064 | integration | `.custom_plan()` |
| T-0066 | integration | `load_related` with `.limit_each` |
| T-0079 | integration | models in several schemas |
| T-0083 | integration | `.limit()` sharing one statement across values |
| T-0105 | integration | `.tag()` in server logs and its character filtering |
