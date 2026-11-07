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

- `db.sql`, `db.raw`, `#[derive(morm::Row)]`, `load_related`, `keyset`, `.limit` on raw statements

## End-to-end check

Raw SQL, related rows and keyset pages return correct rows on real Postgres, including pages under concurrent writes, and each compile-time rule fails under `trybuild`.

## Out of scope

- `sql!` (step 45)
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

let q = sql!("SELECT day, sum(total) AS total FROM orders WHERE vendor_id = $1 GROUP BY day");

let rows: Vec<Sale> = db.sql(q, &[&vendor_id]).limit(30).fetch().await?;     // named struct
let rows: Vec<(NaiveDate, Decimal)> = db.sql(q, &[&vendor_id]).limit(30).fetch().await?;  // tuple
let body = db.sql(q, &[&vendor_id]).limit(30).fetch_json().await?;           // JSON bytes
let rows = db.sql(q, &[&vendor_id]).limit(30).fetch_raw().await?;            // rows as sent
let n: i64 = db.sql("SELECT count(*) FROM orders", &[]).fetch_val().await?;  // one value
let changed = db.sql("UPDATE orders SET seen = true WHERE id = $1", &[&id]).run().await?;  // row count
```

- **R-0413** Every list and single-result call works here, including `fetch_ref`, `stream` and `stream_json`.
- **R-0414** `#[derive(morm::Row)]` decodes by column name, for any plain struct. Columns the struct lacks are ignored, and a missing column is an error.
- **R-0415** The SQL must be `&'static str`, as with `.q()`, and `sql_unchecked(String)` is the visible escape hatch.
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
