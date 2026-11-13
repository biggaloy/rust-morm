# Step 20: Transactions

- **Phase:** Runtime
- **Depends on:** 18
- **Can run beside:** 08, 09, 19, 23, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 51, 52, 53, 54, 57, 58, 59, 60
- **Crates:** `morm`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Transactions whose commit is confirmed, never assumed.

## Deliverables

- `db.transaction`, `tx` with `db`'s query calls, savepoints through `tx.transaction`, `.isolation` and `.retry(n)` on `db` only, per-transaction settings, immediate settings on `tx`, `tx.set_local`, `tx.get_for_update`, confirmed commit, `TransactionAborted`, `CommitUnknown`, pipelining on `tx`

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
db.transaction(async |tx| {
    tx.insert(&mut p).await?;
    Ok(())
}).await?;

db.isolation(Serializable).retry(3).transaction(async |tx| {   // runs again on 40001 or 40P01
    let mut order = Order { user_id, ..Default::default() };   // built inside
    tx.insert(&mut order).await?;
    Ok(())
}).await?;
```

**R-0436** `tx` has every query call `db` has, and `contracts.md` lists its other methods. `tx.transaction(async |tx| ..)` opens a savepoint, released when the closure returns `Ok` and rolled back to when it returns an error. `tx` has no `isolation` or `retry`, so both are compile errors: the isolation level is fixed before a transaction's first query, and a savepoint can't recover from a deadlock or a serialization failure. Each per-transaction setting on `tx` (`statement_timeout`, `lock_timeout`, `transaction_timeout`) applies at once, as in `tx.lock_timeout(d).await?`, one `SET LOCAL` that holds for the rest of the transaction, through savepoints that are released; rolling back to a savepoint opened before it undoes it, as Postgres does. On Postgres 15 and 16, `tx.transaction_timeout(d)` moves the client-side timer instead. `tx.set_local("app.user", &id)` sets a value for RLS through `set_config(..., true)`, so it takes a parameter, which `SET LOCAL` can't. Owner decision DH5.

**R-1229** **`set_local`'s value is cast.** `set_config` takes `text`, so `tx.set_local` casts its value to its own type and then to `text`, as `set_config(($1::text), (($2::int8))::text, true)`. Owner decision DH2.

**R-0437** **A commit is confirmed, never assumed.** After a failed statement, Postgres turns `COMMIT` into `ROLLBACK` with no error, and tokio-postgres's `commit()` returns `Ok` either way, so a swallowed error would look like a saved transaction. Two checks prevent that:

- **R-0438** Any Postgres error marks the transaction as failed. Further use returns that error, and a closure returning `Ok` still gets it.
- **R-0439** `COMMIT` is sent as `SELECT 1; COMMIT` in one message. In a failed transaction the `SELECT` errors and Postgres skips the `COMMIT`, so this also catches errors morm didn't see, with no extra round trip. morm then rolls back and returns `TransactionAborted`.

**R-0441** **Savepoints** clear the failed mark, except after a deadlock or serialization error, which always fails the whole transaction. Rolling back to a savepoint keeps earlier locks, so a deadlock could repeat, and a serialization failure can't succeed on the same snapshot.

**R-0442** **`.retry(n)`** goes before the closure, with `.isolation` and the per-transaction settings, as in `db.retry(n).transaction(async |tx| ..)`, and takes only a closure that can run again: `AsyncFnOnce(&Tx) -> Result<T> + Clone`, run as a fresh copy on each attempt, so a closure that changes a captured row doesn't compile. Rows are built inside the closure, so each attempt starts fresh and `pre_insert` can't run twice on one row, for example hashing a password twice. Between attempts the transaction rolls back and the connection is released during the jittered backoff.

**R-0444** **Read-only pools:** Postgres refuses `SERIALIZABLE` on a hot standby, so `.isolation(Serializable)` on a `read_only` pool returns `ReadOnly` before reaching the server.

### Performance and memory / `morm::Pool`

**R-0630** Queries joined on one `tx` are pipelined on its connection.

### Performance and memory / Statements

- **R-0719** `cached plan must not change result type` (0A000) inside a transaction returns a typed error.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0765 | Swallowing an error, then committing | `TransactionAborted` at runtime |
| R-0767 | `.retry()` on a closure that changes captured rows | Compile error. A `move` closure that changes its own copy compiles, and is safe, because each attempt gets a fresh copy |

### Managed enums

- **R-0984** **Stale statements inside a transaction.** The stale errors that step 14 retries can't be retried inside a transaction, because Postgres has already failed it. morm returns `Error::SchemaChanged`, marks the transaction failed, and clears the statement or replaces the connection as step 14 does; behind a pooler, once the transaction has rolled back, it reads the statement generation and moves its marker as step 14 says, so `.retry(n)`'s next attempt prepares fresh text. Each is known by step 14's signals, never by translated text: `0A000` from `RevalidateCachedQuery`; `XX000` `cache lookup failed for type`, an `elog` that is never translated; `42804` from one of plpgsql's three parameter routines; and `22P02` from `enum_in`, either holding a type morm retired, `_morm_retired_...`, or, from a cached statement that casts a parameter to a non-built-in type, not holding that cast's type name as a whole identifier, with or without its schema, which means a swap left the statement on another enum. Any other error, including any other error step 14 retries once from a cached statement that casts a parameter to a non-built-in type, returns as it is. Owner decisions DH2, choice I, SC and SD.
- **R-0985** `.retry(n)` also retries `Error::SchemaChanged`, because the next attempt runs on fresh statements.
- **R-1247** **A hot standby ends a connection inside a transaction.** Replaying DDL on a hot standby, such as a migration's `ALTER TABLE`, ends a standby session that sits in a transaction that read the table, after `max_standby_streaming_delay`, with `FATAL` `40001` `terminating connection due to conflict with recovery`, and its next statement finds the connection closed. That transaction never committed and, on a standby, wrote nothing, and Postgres's own SQLSTATE says it may run again. So when a statement in a transaction finds its connection closed, the error morm's connection task kept, as step 12 says, was a `FATAL` with `40001`, and the statement wasn't the `COMMIT`, morm returns `Error::Serialization` and discards the connection, so `.retry(n)` runs the transaction again on a fresh one. A connection found closed by the `COMMIT` stays `Error::CommitUnknown`, as for any connection lost then. The routine Postgres names for the conflict differs by version, `ProcessInterrupts` on 15 and 16 and `ProcessRecoveryConflictInterrupt` on 17 and 18, so morm reads only the severity and the SQLSTATE. Owner decision SG.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0049 | integration | swallowed errors never reporting a commit |
| T-0050 | integration | `CommitUnknown` |
| T-0051 | integration | savepoint behaviour after deadlocks |
| T-0287 | trybuild | `tx.isolation(..)` and `tx.retry(n)` don't compile |
| T-0288 | integration | **Settings and savepoints on `tx`, on Postgres 15–18:** `tx.lock_timeout(d).await?`, `tx.statement_timeout(d).await?` and, on 17 and 18, `tx.transaction_timeout(d).await?` show in `SHOW` on the next statement, still after a `tx.transaction(..)` that returned `Ok`, are undone when set inside one that returned an error, and are back to the pool's values on the connection after commit and after rollback; on 15 and 16, `tx.transaction_timeout(d).await?` moves the client-side timer, so a transaction past the new limit returns `TransactionTimeout`; `tx.set_local` with an `i64` and a `String` reads back through `current_setting`; inside a transaction, a cached statement casting to an enum that touches no table of it gets `Error::SchemaChanged` after a swap, and any other error from it returns as it is |
| T-0289 | integration | **Retry pauses are random:** the pause between attempts comes from a real random source, not the clock: over 1,000 pauses, at least 90% are distinct and they spread across the whole jitter range, and two transactions that lose to each other at the same instant don't retry in step on every attempt |
| T-0307 | integration | **A recovery conflict inside a transaction, in the `standby` job step 12 adds, on Postgres 15–18:** on a `read_only` pool on the standby, `.retry(2).transaction(..)` whose closure reads a table, waits past `max_standby_streaming_delay` while an `ALTER TABLE` on that table runs on the primary, then reads again, gets `Error::Serialization` on its first attempt and succeeds on the second, on a fresh connection, with the closure run twice; with no `.retry(n)` the caller gets `Error::Serialization`; a closure that reads, waits the same way and returns, so the `COMMIT` is the first statement to find the connection closed, gets `Error::CommitUnknown` |
