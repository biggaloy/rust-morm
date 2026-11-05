# Step 15: Transactions

- **Phase:** Runtime
- **Depends on:** 13
- **Can run beside:** 16
- **Crates:** `morm`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Transactions whose commit is confirmed, never assumed.

## Deliverables

- `db.transaction`, savepoints, `.isolation`, `.retry(n)`, per-transaction settings, `tx.set_local`, confirmed commit, `TransactionAborted`, `CommitUnknown`, pipelined `BEGIN` and `COMMIT`

## End-to-end check

A swallowed error can never report a commit, a lost connection during `COMMIT` returns `CommitUnknown`, and a deadlock retried with `.retry(3)` succeeds with fresh rows.

## Out of scope

- Lock helpers (step 16)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Reading and writing / Transactions

**R-0404**

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

**R-0405** Same API as `db`, with savepoints and per-transaction settings (`statement_timeout`, `lock_timeout`, `transaction_timeout`). `tx.set_local("app.user", &id)` sets a value for RLS through `set_config(..., true)`, so it takes a parameter, which `SET LOCAL` can't.

**R-0406** **A commit is confirmed, never assumed.** After a failed statement, Postgres turns `COMMIT` into `ROLLBACK` with no error, and tokio-postgres's `commit()` returns `Ok` either way, so a swallowed error would look like a saved transaction. Two checks prevent that:

- **R-0407** Any Postgres error marks the transaction as failed. Further use returns that error, and a closure returning `Ok` still gets it.
- **R-0408** `COMMIT` is sent as `SELECT 1; COMMIT` in one message. In a failed transaction the `SELECT` errors and Postgres skips the `COMMIT`, so this also catches errors morm didn't see, with no extra round trip. morm then rolls back and returns `TransactionAborted`.
- **R-0409** `after_commit` fires only after a confirmed commit. If the connection is lost during `COMMIT`, the outcome is unknown, so the result is `CommitUnknown`, with no retry and no `after_commit`.

**R-0410** **Savepoints** clear the failed mark, except after a deadlock or serialization error, which always fails the whole transaction. Rolling back to a savepoint keeps earlier locks, so a deadlock could repeat, and a serialization failure can't succeed on the same snapshot.

**R-0411** **`.retry(n)`** is available only for a closure that can run again (`AsyncFn`), so a closure that changes a captured row doesn't compile. Rows are built inside the closure, so each attempt starts fresh and `pre_insert` can't run twice on one row, for example hashing a password twice. Each attempt starts with no queued `after_commit` events. Between attempts the transaction rolls back and the connection is released during the jittered backoff.

**R-0412** **Read-only pools:** Postgres refuses `SERIALIZABLE` on a hot standby, so `.isolation(Serializable)` on a `read_only` pool returns `ReadOnly` before reaching the server.

### Performance and memory / Query path

- **R-0673** `BEGIN`, statement and `COMMIT` pipelined to cut auto-transaction round trips. This applies only to models without post hooks, because `post_insert` and `post_update` must run before the `COMMIT`.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0714 | Swallowing an error, then committing | `TransactionAborted` at runtime |
| R-0716 | `.retry()` on a closure that changes captured rows | Compile error |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0044 | integration | swallowed errors never reporting a commit |
| T-0045 | integration | `CommitUnknown` |
| T-0046 | integration | savepoint behaviour after deadlocks |
