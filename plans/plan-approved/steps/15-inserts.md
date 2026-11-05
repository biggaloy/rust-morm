# Step 15: Inserts

- **Phase:** Runtime
- **Depends on:** 13, 06
- **Can run beside:** 07, 08, 11, 14, 21, 25, 26, 27, 28, 29
- **Crates:** `morm`, `morm-macros`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Every way of adding rows, including bulk paths.

## Deliverables

- `insert`, `insert_many` on every path, `insert_stream`, `.in_batches` on inserts, `.explicit_ids()`, `OrDefault`, statements per pattern, array columns

## End-to-end check

A million rows go through each `insert_many` path, atomically or in batches, and defaults, arrays and explicit ids behave as specified on real Postgres.

## Out of scope

- Hooks (step 22)
- Upsert (step 17)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Reading and writing / Insert and update

**R-0334**

```rust
db.insert(&mut p).await?;                                  // INSERT ... RETURNING, fills p in place
db.insert_many(&mut rows).await?;                          // UNNEST, COPY above a threshold
db.upsert(&mut p, F.sku).await?;                           // ON CONFLICT

db.update::<Product>(42).set(F.price, dec!(34)).run().await?;       // no fetch needed
db.update_q::<Product>().by(F.vendor_id, &7).set(F.active, false).run().await?;
db.update_each::<Product>(&rows, &[F.price]).await?;                // per-row values, one statement
db.update::<Product>(42).set_expr(F.stock, "stock - $1", &n).run().await?;  // atomic math in SQL
```

- **R-0336** **Large inserts need no chunking from you.** `insert_many` picks the path by size. A statement has no row limit, because `UNNEST` sends one array per column, not one parameter per value.

| ID | Rows | Path | Notes |
|---|---|---|---|
| R-0337 | Small | `UNNEST` | One statement, one round trip |
| R-0338 | Large | `COPY`, streamed | Fastest, with flat memory on both sides |
| R-0339 | Large, with `.returning()` or an `upsert` | `UNNEST` in chunks, one transaction | `COPY` can't return rows or handle conflicts. Each Postgres value is capped at 1GB, so chunks stay well below it |
| R-0340 | Large, with `None` on some rows of a defaulted column | `UNNEST` in chunks, one per pattern of present columns, one transaction | `COPY` can't choose a default per row |

- **R-0341** `insert_many` is atomic on every path: all rows or none, in one transaction.
- **R-0342** **`.returning()` doesn't promise input order,** because rows are sorted by key and grouped by pattern, so match returned rows by a key.
- **R-0343** **Array columns:** `UNNEST` flattens nested arrays, so two rows holding `text[]` values would become four. A `Vec<T>` column is sent as a `text[]` of array literals and cast back per row (`x::text[]`), which keeps one statement and works with `.returning()`, `upsert` and `update_each`. These columns use the text format, so the escaping must be exact, and property tests cover quoted `"NULL"` strings, real NULLs, quotes, commas and empty arrays. `COPY` handles arrays directly.
- **R-0344** **Explicit ids:** a normal insert always leaves an identity column to Postgres, whatever the field holds. `.explicit_ids()` on `insert` and `insert_many` writes the ids you give, adds `OVERRIDING SYSTEM VALUE` for `GENERATED ALWAYS`, and moves the sequence past them in the same transaction.
- **R-0345** It takes the two-argument `pg_advisory_xact_lock(<morm namespace>, <sequence oid>)`, which can't collide with the hashed keys of `tx.advisory_lock`, so two imports can't overwrite each other's position, then calls `setval(seq, max(id))` only when `max(id) >= last_value`. It's `>=` because a sequence that was never used has `last_value = 1` with nothing taken, and the condition means a stale read can't move the sequence backwards.
- **R-0346** It needs `SELECT` and `UPDATE` on the sequence, unlike normal identity inserts, so give those grants only to the role that imports.
- **R-0347** A normal insert can still collide with an imported id above the sequence until that transaction finishes, and the advisory lock doesn't stop normal inserts. Run imports while the table is quiet.
- **R-0348** An id set without `.explicit_ids()` is ignored, which is almost always a mistake, so debug builds warn.
- **R-0349** `.in_batches(n)` on `insert_many` commits every `n` rows, for bulk loads that shouldn't be one huge transaction with long locks and a large WAL burst. It isn't atomic, and running it again needs `upsert` or a filter to skip rows already loaded.
- **R-0350** `db.insert_stream::<T>(stream)` takes rows from a `Stream`, through `COPY`, so 100 million rows never sit in app memory.
- **R-0357** **Defaults stay in the database.** A field that is `None` on a column with a default is left out of the statement, so a migration changing a default takes effect on running binaries, and identity and `GENERATED ALWAYS` columns work. A model with no such fields has one compile-time statement. Otherwise there is one statement per pattern of present columns, usually one or two, held in `shape_cache`. `insert_many` groups rows by pattern, in one transaction.
- **R-0359** **Base columns:** a bigint `id` is an `i64`, 0 until inserted, and identity columns are never sent. A uuid `id` is a `Uuid`: left nil, `insert` generates a UUIDv7 in the app and fills it in, so the statement doesn't change. `created_at` and `updated_at` are `db_set`.
- **R-0360** **Writing NULL over a default:** `None` on a nullable column with a default means "use the default". When you need a real NULL there, declare the field `morm::OrDefault<Option<T>>`, with the states `Default` and `Set(v)`.

### Performance and memory / Query path

- **R-0701** Bulk insert via `UNNEST` (one stable statement, no 65,535-parameter limit), and `COPY` above a threshold, whose value comes from the benchmark.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0762 | Explicit ids leaving the sequence behind | A normal insert ignores the id; `.explicit_ids()` moves the sequence |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0032 | integration | insert |
| T-0060 | integration | every `insert_many` path at a million rows |
| T-0087 | integration | array columns on every `insert_many` and `update_each` path with property tests on escaping for each element type |
| T-0095 | integration | concurrent `.explicit_ids()` runs and fresh sequences |
| T-0097 | integration | `.explicit_ids()` with both identity kinds and with sequences ahead of the data |
| T-0098 | integration | changed defaults reaching old binaries |
| T-0099 | integration | `OrDefault` |
