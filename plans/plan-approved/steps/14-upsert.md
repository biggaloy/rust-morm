# Step 14: Upsert

- **Phase:** Runtime
- **Depends on:** 13, 05
- **Can run beside:** nothing
- **Crates:** `morm`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

`upsert` with exact rules for what it overwrites and what it reports.

## Deliverables

- Conflict targets by field, tuple and `UK`, partial index filters, `SET` list rules, `.update_only`, the empty-`SET` path, outcome reporting on every Postgres version

## End-to-end check

Upserts on every target form, with and without supplied columns, leave unsupplied values alone and report the right outcome on Postgres 15–18.

## Out of scope

- Hook calls on upsert (step 18)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Reading and writing / Insert and update

- **R-0349** **Hooks and outcome.** `upsert` holds a whole row, so `pre_insert` always runs, before Postgres decides. Afterwards morm knows what happened, through `RETURNING old.ctid IS NULL` on Postgres 18 and later, and `RETURNING (xmax = 0)` before it. `ctid` works for every model, because a user column can't be trusted here: a model may have no primary key, and under `NULLS NOT DISTINCT` a NULL key column can conflict, which would report an update as an insert. That second form is undocumented but long established, and CI proves it on 15–17. `upsert` returns `Inserted`, `Updated` or `Unchanged`. An insert runs `post_insert` and fires `Inserted(rows)`. An update runs `post_update`, with `Changes` holding the `SET` columns, and fires `Updated`. `pre_update` can't run, because the outcome isn't known yet. `Unchanged` writes nothing, so no post hook or `after_commit` runs. A batch upsert fires one `Upserted { inserted, updated }` event per statement, with counts.
- **R-0350** Conflict targets can be one column, a tuple as in `(F.vendor_id, F.sku)`, or a generated unique key such as `Product::UK.lower_email`, which is the only way to target an expression unique index.
- **R-0351** **An upsert only overwrites what the row supplies.** A field left out of the `INSERT` isn't in the `DO UPDATE SET` list, because `EXCLUDED.col` would hold the column's default and reset an existing value. The list also leaves out the conflict target, the primary key and `insert_only` columns, decided by attributes and never by name, and `updated_at` and `auto_now` columns get `now()`. `None` on a nullable column with no default is a value, so it writes NULL. `.update_only(&[F.price, F.stock])` narrows the list further, for upserts that must leave everything else alone.
- **R-0352** **Conflict targets are checked at compile time.** The target must match a unique constraint or unique index, or it's a compile error, and so is a deferrable constraint, which Postgres refuses as an arbiter. The model macro emits a marker per unique key, and `upsert` requires it. Two partial unique indexes on the same columns with different filters make the columns ambiguous, which is a compile error asking for the `UK` name. For a partial unique index, the macro adds its `filter` as the `WHERE` of the `ON CONFLICT` clause, because Postgres doesn't match that index otherwise.
- **R-0353** **An upsert with nothing to update still returns the row.** When the `SET` list is empty, it runs `DO NOTHING`, which writes nothing but returns no row on a conflict, then `SELECT`s by the conflict target, including any `filter`. That's a second round trip only on a conflict. If the row was deleted between the two, the upsert retries once. This path takes no row lock, unlike an upsert with a `SET` list, so inside a transaction use `tx.get_for_update` if you need one. A no-op `SET` was rejected, because it locks the row and writes a new row version on every call.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0727 | An upsert resetting columns the row didn't supply | Only supplied columns are in `DO UPDATE SET` |
| R-0731 | An `upsert` target with no matching unique key | Compile error |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0012 | trybuild | an `upsert` target with no unique key, a deferrable one or an ambiguous one |
| T-0083 | integration | `upsert` leaving unsupplied and `insert_only` columns alone |
| T-0084 | integration | `.update_only` |
| T-0085 | integration | `upsert` on partial unique indexes |
| T-0086 | integration | `upsert` on expression unique indexes through `UK` |
| T-0087 | integration | `upsert` outcomes on models with no primary key and with a NULL key under `NULLS NOT DISTINCT` |
| T-0089 | integration | `upsert` with nothing to update, including the row deleted between its two statements |
