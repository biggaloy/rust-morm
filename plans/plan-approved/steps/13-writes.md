# Step 13: Writes

- **Phase:** Runtime
- **Depends on:** 11, 06
- **Can run beside:** 12
- **Crates:** `morm`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Every way of writing except upsert, including bulk paths.

## Deliverables

- `insert`, `insert_many` on every path, `insert_stream`, `.explicit_ids()`, `OrDefault`, `update`, `update_q`, `update_each`, `.apply()`, `set_expr`, `.version()`, `delete` forms, `.in_batches`, `.pause_ms`

## End-to-end check

A million rows go through each `insert_many` path, atomically or in batches, and defaults, arrays, `.apply()` states and version conflicts behave as specified on real Postgres.

## Out of scope

- Hooks (step 18)
- Upsert (step 14)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Models / Field options (`#[col(...)]`)

| ID | Option | Meaning |
|---|---|---|
| R-0164 | `version` | optimistic locking; a stale update fails with `Conflict` (section 8) |

### Views and groups / Public input and output

- **R-0237** **`.apply()` acts as one `.set()` per field present,** so hooks, `clean`, rules, `Changes<Self>` and group masks work unchanged, and `clean` runs only on fields that were sent. It's also available on `update_q`, and it satisfies the compile-time "no `.set()`" check.
- **R-0238** **Server values win.** In `.set(F.updated_by, uid).apply(body)`, a column in both uses the `.set()` value, so a client can't override your code.
- **R-0239** **An empty patch returns `EmptyPatch`** and sends no `UPDATE`. A no-op can't tell whether the row exists, so success could hide a `NotFound`, and `updated_at` must not change when nothing did.
- **R-0240** `.allow_empty()` accepts an empty patch. By primary key it checks that the row exists, so `NotFound` still works, and it writes nothing. On `update_q` it returns a count of 0.
- **R-0241** Only the columns sent are written. One fixed `CASE WHEN` statement would break that rule and fire every `UPDATE OF column` trigger. Section 11 explains how the varying SQL stays cheap.

### Reading and writing / Insert and update

**R-0322**

```rust
db.insert(&mut p).await?;                                  // INSERT ... RETURNING, fills p in place
db.insert_many(&mut rows).await?;                          // UNNEST, COPY above a threshold
db.upsert(&mut p, F.sku).await?;                           // ON CONFLICT

db.update::<Product>(42).set(F.price, dec!(34)).run().await?;       // no fetch needed
db.update_q::<Product>().by(F.vendor_id, &7).set(F.active, false).run().await?;
db.update_each::<Product>(&rows, &[F.price]).await?;                // per-row values, one statement
db.update::<Product>(42).set_expr(F.stock, "stock - $1", &n).run().await?;  // atomic math in SQL
```

- **R-0323** An update never loads a row, and it writes exactly the columns named. Only `.returning()` brings rows back.
- **R-0324** **Large inserts need no chunking from you.** `insert_many` picks the path by size. A statement has no row limit, because `UNNEST` sends one array per column, not one parameter per value.

| ID | Rows | Path | Notes |
|---|---|---|---|
| R-0325 | Small | `UNNEST` | One statement, one round trip |
| R-0326 | Large | `COPY`, streamed | Fastest, with flat memory on both sides |
| R-0327 | Large, with `.returning()` or an `upsert` | `UNNEST` in chunks, one transaction | `COPY` can't return rows or handle conflicts. Each Postgres value is capped at 1GB, so chunks stay well below it |
| R-0328 | Large, with `None` on some rows of a defaulted column | `UNNEST` in chunks, one per pattern of present columns, one transaction | `COPY` can't choose a default per row |

- **R-0329** `insert_many` is atomic on every path: all rows or none, in one transaction.
- **R-0330** **`.returning()` doesn't promise input order,** because rows are sorted by key and grouped by pattern, so match returned rows by a key.
- **R-0331** **Array columns:** `UNNEST` flattens nested arrays, so two rows holding `text[]` values would become four. A `Vec<T>` column is sent as a `text[]` of array literals and cast back per row (`x::text[]`), which keeps one statement and works with `.returning()`, `upsert` and `update_each`. These columns use the text format, so the escaping must be exact, and property tests cover quoted `"NULL"` strings, real NULLs, quotes, commas and empty arrays. `COPY` handles arrays directly.
- **R-0332** **Explicit ids:** a normal insert always leaves an identity column to Postgres, whatever the field holds. `.explicit_ids()` on `insert` and `insert_many` writes the ids you give, adds `OVERRIDING SYSTEM VALUE` for `GENERATED ALWAYS`, and moves the sequence past them in the same transaction.
- **R-0333** It takes the two-argument `pg_advisory_xact_lock(<morm namespace>, <sequence oid>)`, which can't collide with the hashed keys of `tx.advisory_lock`, so two imports can't overwrite each other's position, then calls `setval(seq, max(id))` only when `max(id) >= last_value`. It's `>=` because a sequence that was never used has `last_value = 1` with nothing taken, and the condition means a stale read can't move the sequence backwards.
- **R-0334** It needs `SELECT` and `UPDATE` on the sequence, unlike normal identity inserts, so give those grants only to the role that imports.
- **R-0335** A normal insert can still collide with an imported id above the sequence until that transaction finishes, and the advisory lock doesn't stop normal inserts. Run imports while the table is quiet.
- **R-0336** An id set without `.explicit_ids()` is ignored, which is almost always a mistake, so debug builds warn.
- **R-0337** `.in_batches(n)` on `insert_many` commits every `n` rows, for bulk loads that shouldn't be one huge transaction with long locks and a large WAL burst. It isn't atomic, and running it again needs `upsert` or a filter to skip rows already loaded.
- **R-0338** `db.insert_stream::<T>(stream)` takes rows from a `Stream`, through `COPY`, so 100 million rows never sit in app memory.
- **R-0339** **`update` and `delete` by primary key return `NotFound` when no row matched,** so a mistyped id can't pass as success. `.allow_missing()` returns a count, for deletes that may have already happened. `update_q`, `update_each` and the other filtered and bulk forms return a count, because zero rows is a normal result for a filter.
- **R-0340** `updated_at` and `auto_now` columns are set to `now()` in SQL.
- **R-0341** **Request-driven updates** use a `patch` view and `.apply()` (section 5), which needs no conditional `.set()` calls.
- **R-0342** **`#[col(version)]`:** an `update` or `delete` by primary key must call `.version(v)`, which adds `AND version = $n` and returns `Conflict` when no row matches, or `.ignore_version()`. Leaving out both is a compile error. Every update increments the column, including `.apply()` and `upsert`. A PATCH handler passes the version the client read, for example from `If-Match`, because `patch` can't list it.
- **R-0343** **Lost updates:** under Read Committed, two requests can read `stock = 10` and both write `9`. Use `set_expr` for arithmetic, `#[col(version)]` for optimistic locking (`Conflict` on a stale write), or `tx.get_for_update` when logic must see the current row.
- **R-0344** **`now()` is the transaction's start time,** so every row in a long transaction gets the same `updated_at`. Use `clock_timestamp()` in `set_expr` when the actual time matters.
- **R-0345** **Defaults stay in the database.** A field that is `None` on a column with a default is left out of the statement, so a migration changing a default takes effect on running binaries, and identity and `GENERATED ALWAYS` columns work. A model with no such fields has one compile-time statement. Otherwise there is one statement per pattern of present columns, usually one or two, held in `shape_cache`. `insert_many` groups rows by pattern, in one transaction.
- **R-0346** **A non-`Option` field is always sent,** so generated `Default` and `new` must agree with its SQL default, or `active: bool` with `default = "true"` would quietly insert `false` through `..Default::default()`. The macro reads literal defaults (numbers, `true`, `false`, quoted strings) into `Default`. A non-literal default such as `gen_random_uuid()` on a non-`Option` field is a compile error, unless the field is `db_set`, the model has `no_default`, or it's written as `Option<T>` with `not_null`.
- **R-0347** **Base columns:** a bigint `id` is an `i64`, 0 until inserted, and identity columns are never sent. A uuid `id` is a `Uuid`: left nil, `insert` generates a UUIDv7 in the app and fills it in, so the statement doesn't change. `created_at` and `updated_at` are `db_set`.
- **R-0348** **Writing NULL over a default:** `None` on a nullable column with a default means "use the default". When you need a real NULL there, declare the field `morm::OrDefault<Option<T>>`, with the states `Default` and `Set(v)`.

### Reading and writing / Delete

**R-0354**

```rust
db.delete::<Product>(42).await?;                           // one row
db.delete_ids::<Product>(&[1, 2, 3]).await?;               // many ids, = ANY($1)
db.delete_by::<Product>(F.vendor_id, &7).await?;           // any column
db.delete_where::<Log>(F.created_at, "< $1", &[&cutoff]).await?;
db.delete_q::<Hist>().by(F.owner_id, &1).by(F.app, &"chat").any(F.key, &keys).run().await?;
```

**R-0355** Filtered and bulk deletes return a count, and `.returning()` gives the rows back. A delete with no filter doesn't compile; `db.delete_all::<T>()` is the explicit form. The same rule applies to `db.update_all::<T>()`.

**R-0356** **Batches:** one statement over millions of rows holds locks for a long time, delays replicas and causes bloat. `.in_batches(n)` on `update_q`, `delete_q`, `delete_where`, `update_all` and `delete_all` works through the rows by primary key, and it works on `insert_many` too (above):

**R-0357**

```rust
db.delete_where::<Log>(F.created_at, "< $1", &[&cutoff]).in_batches(5000).pause_ms(50).await?;
```

**R-0358** Each batch is its own statement and commits on its own, so the whole operation isn't atomic, it can stop partway, and running it again continues. It needs a primary key, and it exists only on `db`, not `tx`. Hooks run once per batch. It returns the total count.

### Performance and memory / Query path

- **R-0670** `= ANY($1)` instead of `IN` lists, so list length doesn't create statement variants.
- **R-0671** Bulk insert via `UNNEST` (one stable statement, no 65,535-parameter limit), and `COPY` above a threshold, whose value comes from the benchmark.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0697 | `update` or `delete` with no filter | Compile error (`update_all` and `delete_all` are explicit) |
| R-0698 | `update` with no `.set()` | Compile error |
| R-0699 | Forgetting `.run()` | `#[must_use]` warning, `-D warnings` in CI |
| R-0701 | A list with no limit | Compile error: requires `.limit(n)`, `.all()` or `stream()` |
| R-0705 | A client sending an empty update | `EmptyPatch` at runtime |
| R-0715 | Updating or deleting a missing id | `NotFound` at runtime |
| R-0725 | Forgetting to pass a `version` | Compile error: `.version(v)` or `.ignore_version()` is required |
| R-0732 | Explicit ids leaving the sequence behind | A normal insert ignores the id; `.explicit_ids()` moves the sequence |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0004 | trybuild | unfiltered update and delete |
| T-0005 | trybuild | update with no `.set()` |
| T-0009 | trybuild | a missing `.version()` |
| T-0013 | trybuild | every write path on an `insert_only` column |
| T-0029 | integration | insert |
| T-0030 | integration | every get, update and delete form |
| T-0054 | integration | `.in_batches` stopping and resuming |
| T-0055 | integration | every `insert_many` path at a million rows |
| T-0064 | integration | `patch` missing, null and value states on nullable and NOT NULL columns |
| T-0065 | integration | `EmptyPatch` and `.allow_empty()` |
| T-0066 | integration | `.set()` winning over `.apply()` |
| T-0077 | integration | `.version()` conflicts |
| T-0082 | integration | array columns on every `insert_many` and `update_each` path with property tests on escaping for each element type |
| T-0090 | integration | concurrent `.explicit_ids()` runs and fresh sequences |
| T-0092 | integration | `.explicit_ids()` with both identity kinds and with sequences ahead of the data |
| T-0093 | integration | changed defaults reaching old binaries |
| T-0094 | integration | `OrDefault` |
