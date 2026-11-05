# Step 18: Updates and deletes

- **Phase:** Runtime
- **Depends on:** 16, 17
- **Can run beside:** 08, 09, 12, 23, 28, 29, 30, 31, 32, 33, 34, 35
- **Crates:** `morm`, `morm-macros`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Every way of changing and removing rows.

## Deliverables

- `update`, `update_q`, `update_each`, `.apply()`, `set_expr`, `.version()`, `delete` forms, `.in_batches`, `.pause_ms`, `.returning()`, `ReadOnly` on replicas

## End-to-end check

`.apply()` states, version conflicts, batches stopping and resuming, and `NotFound` all behave as specified on real Postgres.

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

- Hooks (step 24)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools / Configuration: `morm.toml`

**R-0116** `read_only = true` makes a write on that pool fail with `ReadOnly` at once, sets `default_transaction_read_only`, and exempts it from the `25006` discard rule.

### Models / Field options (`#[col(...)]`)

- **R-0170** `.set()` on a generated column is a compile error.

| ID | Option | Meaning |
|---|---|---|
| R-0171 | `version` | optimistic locking; a stale update fails with `Conflict` (section 8) |

### Views and groups / Views: restricted models

- **R-0230** **Read-only works:** `write_hide` fields can't be used in `.set()` (compile error).

### Views and groups / Public input and output

**R-0245** **Views write too.** `db.insert(&mut new_user)` works on `UserIn`, and `db.update::<UserPatch>(id)` on `UserPatch`, so a handler never names the private full model. A column the view lacks is left out of the statement.

- **R-0251** **`.apply()` acts as one `.set()` per field present,** It's also available on `update_q`, and it satisfies the compile-time "no `.set()`" check.
- **R-0253** **Server values win.** In `.set(F.updated_by, uid).apply(body)`, a column in both uses the `.set()` value, so a client can't override your code.
- **R-0254** **An empty patch returns `EmptyPatch`** and sends no `UPDATE`. A no-op can't tell whether the row exists, so success could hide a `NotFound`, and `updated_at` must not change when nothing did.
- **R-0255** `.allow_empty()` accepts an empty patch. By primary key it checks that the row exists, so `NotFound` still works, and it writes nothing. On `update_q` it returns a count of 0.
- **R-0256** Only the columns sent are written. One fixed `CASE WHEN` statement would break that rule and fire every `UPDATE OF column` trigger. Section 11 explains how the varying SQL stays cheap.

### Reading and writing / Insert and update

- **R-0340** An update never loads a row, and it writes exactly the columns named. Only `.returning()` brings rows back.
- **R-0350** Array columns work through `update_each` the same way.
- **R-0358** **`update` and `delete` by primary key return `NotFound` when no row matched,** so a mistyped id can't pass as success. `.allow_missing()` returns a count, for deletes that may have already happened. `update_q`, `update_each` and the other filtered and bulk forms return a count, because zero rows is a normal result for a filter.
- **R-0359** `updated_at` and `auto_now` columns are set to `now()` in SQL.
- **R-0360** **Request-driven updates** use a `patch` view and `.apply()` (section 5), which needs no conditional `.set()` calls.
- **R-0361** **`#[col(version)]`:** an `update` or `delete` by primary key must call `.version(v)`, which adds `AND version = $n` and returns `Conflict` when no row matches, or `.ignore_version()`. Leaving out both is a compile error. Every update increments the column, including `.apply()`. A PATCH handler passes the version the client read, for example from `If-Match`, because `patch` can't list it.
- **R-0363** **Lost updates:** under Read Committed, two requests can read `stock = 10` and both write `9`. Use `set_expr` for arithmetic, `#[col(version)]` for optimistic locking (`Conflict` on a stale write), or `tx.get_for_update` when logic must see the current row.
- **R-0364** **`now()` is the transaction's start time,** so every row in a long transaction gets the same `updated_at`. Use `clock_timestamp()` in `set_expr` when the actual time matters.

### Reading and writing / Delete

**R-0380**

```rust
db.delete::<Product>(42).await?;                           // one row
db.delete_ids::<Product>(&[1, 2, 3]).await?;               // many ids, = ANY($1)
db.delete_by::<Product>(F.vendor_id, &7).await?;           // any column
db.delete_where::<Log>(F.created_at, "< $1", &[&cutoff]).await?;
db.delete_q::<Hist>().by(F.owner_id, &1).by(F.app, &"chat").any(F.key, &keys).run().await?;
```

**R-0381** Filtered and bulk deletes return a count, and `.returning()` gives the rows back. A delete with no filter doesn't compile; `db.delete_all::<T>()` is the explicit form. The same rule applies to `db.update_all::<T>()`.

**R-0382** **Batches:** one statement over millions of rows holds locks for a long time, delays replicas and causes bloat. `.in_batches(n)` on `update_q`, `delete_q`, `delete_where`, `update_all` and `delete_all` works through the rows by primary key, and it works on `insert_many` too (above):

**R-0383**

```rust
db.delete_where::<Log>(F.created_at, "< $1", &[&cutoff]).in_batches(5000).pause_ms(50).await?;
```

**R-0384** Each batch is its own statement and commits on its own, so the whole operation isn't atomic, it can stop partway, and running it again continues. It needs a primary key, and it exists only on `db`, not `tx`. It returns the total count.

### Performance and memory / Query path

- **R-0719** `= ANY($1)` instead of `IN` lists, so list length doesn't create statement variants.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0746 | `update` or `delete` with no filter | Compile error (`update_all` and `delete_all` are explicit) |
| R-0747 | `update` with no `.set()` | Compile error |
| R-0748 | Forgetting `.run()` | `#[must_use]` warning, `-D warnings` in CI |
| R-0754 | A client sending an empty update | `EmptyPatch` at runtime |
| R-0764 | Updating or deleting a missing id | `NotFound` at runtime |
| R-0774 | Forgetting to pass a `version` | Compile error: `.version(v)` or `.ignore_version()` is required |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0007 | trybuild | unfiltered update and delete |
| T-0008 | trybuild | update with no `.set()` |
| T-0012 | trybuild | a missing `.version()` |
| T-0034 | integration | every update and delete form |
| T-0036 | integration | views through every write |
| T-0059 | integration | `.in_batches` stopping and resuming |
| T-0069 | integration | `patch` missing, null and value states on nullable and NOT NULL columns |
| T-0070 | integration | `EmptyPatch` and `.allow_empty()` |
| T-0071 | integration | `.set()` winning over `.apply()` |
| T-0082 | integration | `.version()` conflicts |
| T-0087 | integration | array columns on every `insert_many` and `update_each` path with property tests on escaping for each element type |
