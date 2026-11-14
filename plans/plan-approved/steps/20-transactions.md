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

- **R-0984** **Stale statements inside a transaction.** The stale errors that step 14 retries can't be retried inside a transaction, because Postgres has already failed it. morm returns `Error::SchemaChanged`, marks the transaction failed, and clears the statement or replaces the connection as step 14 does; behind a pooler, once the transaction has rolled back, it reads the statement generation and moves its marker as step 14 says, so `.retry(n)`'s next attempt prepares fresh text. Each is known by step 14's signals, never by translated text: `0A000` from `RevalidateCachedQuery`; `XX000` `cache lookup failed for type`, an `elog` that is never translated; `42804` from one of plpgsql's three parameter routines; and `22P02` from `enum_in`, either holding a type morm retired, `_morm_retired_...`, or, from a cached statement that casts a parameter to a non-built-in type, not holding that cast's type name as a whole identifier, with or without its schema, which means a swap left the statement on another enum. A `22P02` from `enum_in` holding `_morm_retired_` follows step 14's parameter test and its memory of statements. When a bound parameter's text holds `_morm_retired_`, or step 14's pool remembers the statement, it returns as it is, not as `Error::SchemaChanged`, with nothing discarded or raised. Otherwise morm returns `Error::SchemaChanged`. Straight to Postgres, once the transaction has rolled back, morm reads the database's statement generation on that connection, from the `_morm` schema's comment, which every migration `tx` step raises, one catalog read on a rare path, and records in step 14's pool the statement's hash, that generation and the time of that read, so a migration that commits before the read is in the record; then it discards that one connection and raises nothing, because nothing has been confirmed. The records are at most 64 per pool, the oldest dropped first, and all are cleared when the connection generation moves; a dropped record only turns a later failure into a first failure, which costs one discard. When the same statement later fails the same way in a transaction, morm reads the generation again after the rollback. If it has moved since the record, a migration has run since, and that connection may be stale from it, so the failure is a new first failure: the record is replaced with the new generation and time, the connection is discarded, and `.retry(n)` goes on as for a first failure. With the generation unmoved, a failure on a connection opened after the recorded time counts as a failed confirmation, because that connection compiled the function after the generation was read and still failed: the statement enters step 14's memory, the error returns as it is, and nothing is discarded or raised. A failure on a connection opened before the recorded time, with the generation unmoved, keeps the record as it is and discards that connection, as for a first failure. So straight to Postgres a real stale session after any cutover, a second one included, is never taken for stored or client text, and that text costs at most the pool's connections that were open when it first failed, and again after each migration, because each migration moves the generation. Behind a pooler morm keeps no record and discards nothing, because a client connection's age says nothing about the server session it reaches: such a `22P02` is `Error::SchemaChanged` unless the parameter test or step 14's memory says otherwise, so `.retry(n)`'s next attempt can reach another server session. Straight to Postgres, morm also has `.retry(n)`'s next attempt run on a newly opened connection, opened through the pool's ordinary on-demand open, with its backoff on too many clients, as part of that attempt, and taking the slot the discarded connection left. If it can't be opened within the pool's timeout, `.retry(n)` returns that error, and the statement enters step 14's memory as for a failed open. The attempt confirms only if it runs the statement that failed with the same text and the same parameter bytes, which morm compares by a hash of both, and ends as step 14's retry confirms, with rows or a server error outside step 14's list of errors that can end a statement before it reaches the conversion: then morm raises the connection generation, the pool's other connections are replaced at checkout, and the caller gets the attempt's own result, success or error. If it runs that statement with other parameters, or doesn't run it, nothing is confirmed or raised, and the new connection stays as an ordinary pooled one; if it fails the same way, that's a failure on a fresh connection, as above; if it ends in one of step 14's listed errors, it confirms nothing, raises nothing, returns that error and puts the statement in step 14's memory. With no `.retry(n)` nothing is raised: each pooled connection that has the function compiled fails one transaction and is replaced, and a statement fed such text enters the memory as above. Any other error, including any other error step 14 retries once from a cached statement that casts a parameter to a non-built-in type, returns as it is. Owner decisions DH2, choice I, SC, SD, EF1, EG1, EH1, EH2, EI1 and EJ2, with this fix's choices the owner approves through PR #3: the records cleared when the connection generation moves, and no record kept behind a pooler.
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
| T-0312 | integration | **The stale `22P02` inside a transaction, on Postgres 15–18:** after a hand-run swap that keeps the old type with its labels renamed, with a pool of four connections that each called a `DECLARE v st` function before it: `.retry(2).transaction(..)` calling the function with no parameter holding `_morm_retired_` gets `Error::SchemaChanged` on its first attempt, which discards that one connection and raises nothing, and succeeds on its second, on a newly opened connection in the discarded one's slot, after which the generation is raised and the pool's other connections are replaced at checkout; a closure that binds a different value on its second attempt, or doesn't call the function then, raises no generation, and the new connection stays in the pool; with no `.retry(n)` the caller gets `Error::SchemaChanged`, one connection is discarded and no generation is raised; the same transaction passing a label holding `_morm_retired_x` gets its `22P02` as it is, not `SchemaChanged`, with the closure run once and no connection discarded; a function that reads a stored `_morm_retired_x` and converts it, in `.retry(2).transaction(..)`, gets `SchemaChanged` on its first attempt and its `22P02` as it is on the second, on the new connection, which puts the statement in the memory; the same function in transactions with no `.retry(n)`, called five times, discards at most the four connections open at its first failure and then none, its call on a connection opened after that failure putting it in the memory, and called one at a time more than eight times, discards stop at four, a failure on an older connection keeping the record; a remembered statement in a transaction returns its `22P02` as it is with nothing discarded; behind PgBouncer, in the `pooler` job, none of these discards a connection, no record is kept, and a stale server session reached through a newly opened client connection still returns `SchemaChanged`, so `.retry(n)` goes on; where `.retry(n)`'s attempt takes the discarded connection's slot, the pool's size in `pool_stats()` is unchanged, and where connections are discarded with no `.retry(n)`, the pool's slots never pass `max` and it's back to four connections after four concurrent checkouts; with the role's `CONNECTION LIMIT` reached, `.retry(2)`'s attempt whose new connection can't be opened returns the connection error, counts as that attempt, and puts the statement in the memory; across two swaps of the type the function reads, each raising the `_morm` schema's statement generation as a migration's `tx` step does, the second after the first was confirmed through `.retry(2)`, each connection the second swap left stale fails one transaction, is discarded and isn't taken for stored text, and the second swap is confirmed through `.retry(2)` and raises the generation again; in transactions with no `.retry(n)`, so a first-failure record survives, a first swap healed by discards and then a second such swap: each connection the second swap left stale, those opened after the first swap's first failure included, returns `SchemaChanged` and is discarded, and none puts the statement in the memory; a `.retry(2)` attempt cancelled by `statement_timeout` before the conversion raises no generation and puts the statement in the memory; a 65th recorded first failure drops the oldest record |
