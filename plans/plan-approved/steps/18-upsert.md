# Step 18: Upsert

- **Phase:** Runtime
- **Depends on:** 17
- **Can run beside:** 08, 09, 12, 19, 20, 22, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39
- **Crates:** `morm`, `morm-macros`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

`upsert` with exact rules for what it overwrites and what it reports.

## Deliverables

- Conflict targets by field, tuple and `UK`, partial index filters, `SET` list rules, `.update_only`, the empty-`SET` path, outcome reporting on every Postgres version

## End-to-end check

Upserts on every target form, with and without supplied columns, leave unsupplied values alone and report the right outcome on Postgres 15–18.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> ```rust
> db.insert(&mut p).await?;                                  // INSERT ... RETURNING, fills p in place
> db.insert_many(&mut rows).await?;                          // UNNEST, COPY above a threshold
> db.upsert(&mut p, F.sku).await?;                           // ON CONFLICT
>
> db.update::<Product>(42).set(F.price, dec!(34)).run().await?;       // no fetch needed
> db.update_q::<Product>().by(F.vendor_id, &7).set(F.active, false).run().await?;
> db.update_each::<Product>(&rows, &[F.price]).await?;                // per-row values, one statement
> db.update::<Product>(42).set_expr(F.stock, "stock - $1", &n).run().await?;  // atomic math in SQL
> ```
>

## Out of scope

- Hook calls on upsert (step 23)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Models / Field options (`#[col(...)]`)

| ID | Option | Meaning |
|---|---|---|
| R-0180 | `insert_only` | written by `insert`, then never changed: left out of `upsert` updates, and `.set()`, `set_expr`, `update_each`, `.update_only` or a `patch` naming it is a compile error. Bases set it on `created_at`, and custom bases use it the same way |

### Reading and writing / Insert and update

| ID | Rows | Path | Notes |
|---|---|---|---|
| R-0342 | Large, with `.returning()` or an `upsert` | `UNNEST` in chunks, one transaction | `COPY` can't return rows or handle conflicts. Each Postgres value is capped at 1GB, so chunks stay well below it |

- **R-0359** `upsert` also increments a `version` column.
- **R-0366** Afterwards morm knows what happened, through `RETURNING old.ctid IS NULL` on Postgres 18 and later, and `RETURNING (xmax = 0)` before it. `ctid` works for every model, because a user column can't be trusted here: a model may have no primary key, and under `NULLS NOT DISTINCT` a NULL key column can conflict, which would report an update as an insert. That second form is undocumented but long established, and CI proves it on 15–17. `upsert` returns `Inserted`, `Updated` or `Unchanged`.
- **R-0373** Conflict targets can be one column, a tuple as in `(F.vendor_id, F.sku)`, or a generated unique key such as `Product::UK.lower_email`, which is the only way to target an expression unique index.
- **R-0374** **An upsert only overwrites what the row supplies.** A field left out of the `INSERT` isn't in the `DO UPDATE SET` list, because `EXCLUDED.col` would hold the column's default and reset an existing value. The list also leaves out the conflict target, the primary key and `insert_only` columns, decided by attributes and never by name, and `updated_at` and `auto_now` columns get `now()`. `None` on a nullable column with no default is a value, so it writes NULL. `.update_only(&[F.price, F.stock])` narrows the list further, for upserts that must leave everything else alone.
- **R-0375** **Conflict targets are checked at compile time.** The target must match a unique constraint or unique index, or it's a compile error, and so is a deferrable constraint, which Postgres refuses as an arbiter. The model macro emits a marker per unique key, and `upsert` requires it. Two partial unique indexes on the same columns with different filters make the columns ambiguous, which is a compile error asking for the `UK` name. For a partial unique index, the macro adds its `filter` as the `WHERE` of the `ON CONFLICT` clause, because Postgres doesn't match that index otherwise.
- **R-0376** **An upsert with nothing to update still returns the row.** When the `SET` list is empty, it runs `DO NOTHING`, which writes nothing but returns no row on a conflict, then `SELECT`s by the conflict target, including any `filter`. That's a second round trip only on a conflict. If the row was deleted between the two, the upsert retries once. This path takes no row lock, unlike an upsert with a `SET` list, so inside a transaction use `tx.get_for_update` if you need one. A no-op `SET` was rejected, because it locks the row and writes a new row version on every call.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0773 | An upsert resetting columns the row didn't supply | Only supplied columns are in `DO UPDATE SET` |
| R-0777 | An `upsert` target with no matching unique key | Compile error |

### README.md

| ID | Call | Use this when |
|---|---|---|
| R-0877 | `insert_only` | A column is set once on insert and must never change afterwards |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0015 | trybuild | an `upsert` target with no unique key, a deferrable one or an ambiguous one |
| T-0016 | trybuild | every write path on an `insert_only` column |
| T-0088 | integration | `upsert` leaving unsupplied and `insert_only` columns alone |
| T-0089 | integration | `.update_only` |
| T-0090 | integration | `upsert` on partial unique indexes |
| T-0091 | integration | `upsert` on expression unique indexes through `UK` |
| T-0092 | integration | `upsert` outcomes on models with no primary key and with a NULL key under `NULLS NOT DISTINCT` |
| T-0094 | integration | `upsert` with nothing to update, including the row deleted between its two statements |
