# Step 25: Hooks on upsert, batches and shutdown

- **Phase:** Runtime
- **Depends on:** 24
- **Can run beside:** 08, 23, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41
- **Crates:** `morm`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Finish hooks on the write paths with special rules.

## Deliverables

- Hooks and outcomes on `upsert`, `update_each`, `.in_batches`, batch upserts, `insert_stream`, and `db.close()` waiting for `after_commit`

## End-to-end check

A recording model proves the order and inputs on each of these paths, including no hook after an unchanged upsert, on Postgres 15–18.

## Out of scope

- Outbox (step 27)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Reading and writing / Insert and update

- **R-0370** **Hooks and outcome.** `upsert` holds a whole row, so `pre_insert` always runs, before Postgres decides.
- **R-0371** An insert runs `post_insert` and fires `Inserted(rows)`.
- **R-0372** An update runs `post_update`, with `Changes` holding the `SET` columns, and fires `Updated`.
- **R-0373** `pre_update` can't run, because the outcome isn't known yet.
- **R-0374** `Unchanged` writes nothing, so no post hook or `after_commit` runs.
- **R-0375** A batch upsert fires one `Upserted { inserted, updated }` event per statement, with counts.

### Reading and writing / Delete

**R-0385** Hooks run once per batch.

### Hooks / Order

| ID | Operation | Order |
|---|---|---|
| R-0494 | `upsert` | `clean` and rules → `pre_insert` → `validate` → **UPSERT** → `post_insert` or `post_update`, or neither if unchanged → *(commit)* → `after_commit` |

### Hooks / Rules

- **R-0499** **`update_each`:** `pre_update` runs once with `Target::Ids` and only the column list. Per-row values go through `clean` and the declarative rules.

### Performance and memory / `db.hold()`

- **R-0699** **Shutdown:** `db.close()` stops new checkouts, then waits up to `shutdown_timeout` for running transactions, `after_commit` handlers and holds to finish, then closes every connection. Apps call it after they stop accepting requests, so a deploy doesn't roll back work or lose events.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0078 | integration | `db.close()` finishing in-flight work |
| T-0093 | integration | `upsert` outcomes, hooks and `after_commit` on each Postgres version |
