# Step 15: Reads

- **Phase:** Runtime
- **Depends on:** 14, 07
- **Can run beside:** 08, 09, 12, 17, 28, 29, 30, 31, 32, 33, 34, 35
- **Crates:** `morm`, `morm-macros`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Every way of reading, and the shared `by`/`any` builder.

## Deliverables

- `get` forms, `select`, `filter`, `filter_cols`, every list and single-result call, `order_by_allowed`, `.custom_plan()`, `.tag()` on queries, replicas through `db.pool`

## End-to-end check

A test app reads the same rows through every call and gets identical data, and each compile-time rule fails under `trybuild`.

## Out of scope

- `db.sql`, `load_related`, `keyset` (step 16)
- `fetch_json` formats (step 23)
- Groups on a handle (step 26)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools / Configuration: `morm.toml`

**R-0115** **Read replicas** work through a named pool: `db.pool("replica").select::<Order>()...`. You choose which queries go there. A replica can lag behind the primary, so a read straight after a write belongs on `db`.

### Models / Type inference

- **R-0205** **`citext`** works on a `String` field with `sql = "citext"` and `extensions = "citext"`.

### Models / Generated per model

- **R-0216** `Product::F.alias("p")` works for JOINs, with prefix decoding for two models in one row.

### Reading and writing

**R-0331** In the examples, `F` is short for the relevant model's `F`, such as `Product::F`.

### Reading and writing / Get

**R-0334**

```rust
db.get::<User>(5).await?;                                  // by PK, Option<User>
db.get_by::<User>(F.email, &email).await?;                 // any column
db.get_where::<Product>(F.price, "BETWEEN $1 AND $2", &[&dec!(5), &dec!(2000)]).await?;   // any comparison
db.get_all_by::<Order>(F.user_id, &7).limit(50).await?;    // many rows

// several columns: one builder shared by get, update, delete, count, exists
db.get_q::<Hist>().by(F.owner_id, &1).by(F.app, &"chat").one().await?;
```

- **R-0336** `comp` is `&'static str`, so it can't come from a request.
- **R-0337** The statement is cached on first use, keyed by model, column and the `comp` pointer, so repeats allocate nothing.
- **R-0338** Composite PKs take tuples.

### Reading and writing / Lists

**R-0386**

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

- **R-0387** `.filter()` selects every down column: all the columns this model or view may read, without hidden, write-only or `deprecated` ones. It's the `.filter_cols(...)` default, since Rust has no default arguments.
- **R-0388** `.filter_cols((F.a, F.b))` selects only those. `fetch()` returns tuples of the columns' Rust types and `fetch_ref()` returns borrowed tuples, so a wrong type is a compile error, and a partial row can't pass for a full `Product`. `fetch_json`, `stream_json` and `fetch_raw` work the same way.
- **R-0389** Columns come from `F`, so a view can't select what it hides. Group rules on a `db.groups(mask)` handle are in section 5.
- **R-0390** The column list is cached like other statements, so repeats allocate nothing.
- **R-0391** **Sort columns from a request** can't be parameters, and `.q()` won't accept them. `.order_by_allowed(&input, &[F.name, F.age])` takes `"name"` or `"-age"`, and returns `InvalidOrder` for anything not on the list. `morm::quote_ident()` is only for names known at runtime in `q_unchecked` and `sql_unchecked`: it wraps a name in double quotes and doubles any quote inside it, so it can't break out. `F` columns are already quoted.
- **R-0392** `.custom_plan()` plans a query for its actual values, for skewed data where one customer has a million orders and most have ten. It runs through `query_typed`, so it needs no `SET` and no extra round trip. The `plan_cache_mode` setting does the same for a whole pool.

**R-0393** A list needs `.limit(n)`, `.all()` or `stream()`. Lists return one of:

| ID | Call | You get | Conversions | Use when |
|---|---|---|---|---|
| R-0394 | `fetch_json()` | JSON bytes built by Postgres | None in the app | The data goes straight to a client |
| R-0395 | `stream_json()` | The same, chunked | None in the app | Large responses, flat memory |
| R-0396 | `fetch_raw()` | Rows as Postgres sent them; columns read on demand | Only the columns you read | You need a few columns from each row |
| R-0397 | `fetch_ref()` | Borrowed `ProductRef<'r>` | Strings and `jsonb` borrowed from the row buffer, no per-field allocation | Reading and computing without keeping the data |
| R-0398 | `fetch()` | Owned `Product` | Row → struct | You keep or pass the data along |
| R-0399 | `stream()` | One row at a time | Row by row | Huge results |

**R-0400** `fetch_one`, `fetch_opt`, `fetch_val`, `exists` and `count` cover the single-result cases.

### Reading and writing / Query safety

- **R-0431** `.q()` accepts only `&'static str`, so `format!`-ed input won't compile. `q_unchecked(String)` is the visible escape hatch.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0750 | Building SQL from strings | Compile error (`q_unchecked` is the visible escape hatch) |
| R-0751 | A list with no limit | Compile error: requires `.limit(n)`, `.all()` or `stream()` |
| R-0759 | A request-supplied sort column | `.q()` refuses it; `order_by_allowed` checks it against a list |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0009 | trybuild | list with no limit |
| T-0021 | trybuild | non-static `q()` and `sql()` |
| T-0033 | integration | every get form |
| T-0035 | integration | views through every read |
| T-0063 | integration | `order_by_allowed` |
| T-0064 | integration | `.custom_plan()` |
| T-0079 | integration | models in several schemas |
| T-0083 | integration | `.limit()` sharing one statement across values |
| T-0105 | integration | `.tag()` in server logs and its character filtering |
