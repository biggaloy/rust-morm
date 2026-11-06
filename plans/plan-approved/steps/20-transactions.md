# Step 20: Transactions

- **Phase:** Runtime
- **Depends on:** 18
- **Can run beside:** 08, 09, 19, 23, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 51, 52, 53, 54, 57, 58, 59
- **Crates:** `morm`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Transactions whose commit is confirmed, never assumed.

## Deliverables

- `db.transaction`, `tx` with the same API as `db`, savepoints, `.isolation`, `.retry(n)`, per-transaction settings, `tx.set_local`, `tx.get_for_update`, confirmed commit, `TransactionAborted`, `CommitUnknown`, pipelining on `tx`

## End-to-end check

A swallowed error can never report a commit, a lost connection during `COMMIT` returns `CommitUnknown`, and a deadlock retried with `.retry(3)` succeeds with fresh rows.

## Out of scope

- Lock helpers (step 22)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Reading and writing / Get

- **R-0336** `tx.get_for_update::<T>(id)` locks the row while reading it, for read-then-write logic. It exists only on `tx`.

### Reading and writing / Transactions

**R-0435**

```rust
db.transaction(|tx| async move {
    tx.insert(&mut p).await?;
    Ok(())
}).await?;

db.transaction(async |tx| {                       // runs again on 40001 or 40P01
    let mut order = Order { user_id, ..Default::default() };   // built inside
    tx.insert(&mut order).await?;
    Ok(())
}).isolation(Serializable).retry(3).await?;
```

**R-0436** Same API as `db`, with savepoints and per-transaction settings (`statement_timeout`, `lock_timeout`, `transaction_timeout`). `tx.set_local("app.user", &id)` sets a value for RLS through `set_config(..., true)`, so it takes a parameter, which `SET LOCAL` can't.

**R-0437** **A commit is confirmed, never assumed.** After a failed statement, Postgres turns `COMMIT` into `ROLLBACK` with no error, and tokio-postgres's `commit()` returns `Ok` either way, so a swallowed error would look like a saved transaction. Two checks prevent that:

- **R-0438** Any Postgres error marks the transaction as failed. Further use returns that error, and a closure returning `Ok` still gets it.
- **R-0439** `COMMIT` is sent as `SELECT 1; COMMIT` in one message. In a failed transaction the `SELECT` errors and Postgres skips the `COMMIT`, so this also catches errors morm didn't see, with no extra round trip. morm then rolls back and returns `TransactionAborted`.

**R-0441** **Savepoints** clear the failed mark, except after a deadlock or serialization error, which always fails the whole transaction. Rolling back to a savepoint keeps earlier locks, so a deadlock could repeat, and a serialization failure can't succeed on the same snapshot.

**R-0442** **`.retry(n)`** is available only for a closure that can run again (`AsyncFn`), so a closure that changes a captured row doesn't compile. Rows are built inside the closure, so each attempt starts fresh and `pre_insert` can't run twice on one row, for example hashing a password twice. Between attempts the transaction rolls back and the connection is released during the jittered backoff.

**R-0444** **Read-only pools:** Postgres refuses `SERIALIZABLE` on a hot standby, so `.isolation(Serializable)` on a `read_only` pool returns `ReadOnly` before reaching the server.

### Performance and memory / `morm::Pool`

**R-0630** Queries joined on one `tx` are pipelined on its connection.

### Performance and memory / Statements

- **R-0719** `cached plan must not change result type` (0A000) inside a transaction returns a typed error.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0765 | Swallowing an error, then committing | `TransactionAborted` at runtime |
| R-0767 | `.retry()` on a closure that changes captured rows | Compile error |

### Managed enums

- **R-0984** **Stale statements inside a transaction.** The stale errors that step 14 retries can't be retried inside a transaction, because Postgres has already failed it. morm returns `Error::SchemaChanged`, marks the transaction failed, and clears the statement or replaces the connection as step 14 does.
- **R-0985** `.retry(n)` also retries `Error::SchemaChanged`, because the next attempt runs on fresh statements.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0049 | integration | swallowed errors never reporting a commit |
| T-0050 | integration | `CommitUnknown` |
| T-0051 | integration | savepoint behaviour after deadlocks |
