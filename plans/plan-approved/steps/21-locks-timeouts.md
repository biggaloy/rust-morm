# Step 21: Locks, timeouts and cancellation

- **Phase:** Runtime
- **Depends on:** 20, 18
- **Can run beside:** 08, 09, 22, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39
- **Crates:** `morm`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Prevent Postgres deadlocks and bound every lock wait.

## Deliverables

- `tx.lock`, `.for_key_change()`, `.lock_ordered()`, advisory locks, `.skip_locked()`, bulk sorting, single-statement retry, every timeout, cancellation

## End-to-end check

Forced deadlocks of both kinds resolve as specified, and a cancelled request's query is gone from `pg_stat_activity` with its locks released.

## Out of scope

- The lock-order detector (step 47)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Reading and writing / Deadlocks

**R-0448** **Postgres deadlocks:**

**R-0449**

```rust
db.transaction(async |tx| {
    tx.lock::<Account>(&[from, to]).await?;             // same order every time
    tx.update::<Account>(from).set_expr(F.balance, "balance - $1", &amount).run().await?;
    tx.update::<Account>(to).set_expr(F.balance, "balance + $1", &amount).run().await?;
    Ok(())
}).retry(3).await?;

db.update_q::<Job>().any(F.id, &ids).set(F.done, true).lock_ordered().run().await?;
tx.select::<Job>().filter().qc(F.state, "=", &"queued").limit(10).skip_locked().fetch().await?;
tx.advisory_lock(&["billing", "reports"]).await?;
```

- **R-0450** **Bulk writes are sorted.** `insert_many`, `upsert` batches, `update_each` and `delete_ids` sort input by key, skipping sorted input. Inserts lock in that order. For updates and deletes it usually holds, but the planner chooses the order, so a hash join can differ. `.lock_ordered()` is the guarantee.
- **R-0451** **`.lock_ordered()`** on `update_q`, `delete_q`, `update_each` and `delete_ids` locks rows in key order in the same statement, with a CTE and no extra round trip. It uses `FOR NO KEY UPDATE`, so child inserts aren't blocked, and `FOR UPDATE` only for deletes and updates of key columns. It adds query work, so it's opt-in, and the benchmark shows how much.
- **R-0452** **`tx.lock::<T>(&ids)`** runs `SELECT ... WHERE pk = ANY($1) ORDER BY pk FOR NO KEY UPDATE`. `NO KEY` means child inserts aren't blocked by foreign key checks. `tx.lock(&ids).for_key_change()` takes `FOR UPDATE`, for rows you will delete or whose key you will change.
- **R-0453** **One byte order everywhere.** App sorting and `ORDER BY` must agree, so text keys use `COLLATE "C"`, which matches Rust's byte order. Numbers and UUIDs already agree.
- **R-0454** **Foreign keys:** a child insert takes `FOR KEY SHARE` on the parent. A normal update of the parent doesn't conflict with that. Deleting the parent, or changing a key column it's referenced by, does. Plain `tx.lock` doesn't protect those paths: `FOR NO KEY UPDATE` lets a child insert in, and the later delete must wait for it, which can deadlock. They need `tx.lock(&ids).for_key_change()`, which blocks child inserts until commit.
- **R-0455** **`tx.advisory_lock(&keys)`** serializes work across tables when row order isn't practical, and it releases at the end of the transaction. Keys are the first 8 bytes of the SHA-256 of each string, a documented algorithm, because services must compute the same key across builds. Keys are sorted before locking.
- **R-0456** **`.skip_locked()`** on a list adds `FOR UPDATE SKIP LOCKED`, so queue workers skip claimed rows. It exists only on `tx`.

**R-0457** **Automatic retry:** a single statement outside a transaction retries once when Postgres reports `40P01` or `40001`, or when it failed before being sent. That's the complete list: a lost connection after sending has an unknown outcome, and a stream that has delivered rows never retries.

**R-0460** **Limiting the damage:**

| ID | Setting | Default | Effect |
|---|---|---|---|
| R-0461 | `lock_timeout` | 5s | One lock wait past this returns `LockTimeout` |
| R-0462 | `transaction_timeout` | 30s | A `db.transaction` past this returns `TransactionTimeout` |
| R-0463 | `idle_in_transaction_session_timeout` | 1m | Postgres ends a transaction left open and idle |
| R-0464 | `client_connection_check_interval` | 5s | Postgres stops a running query when the client has gone |
| R-0465 | `tx_warn_after` | 1s | Logs a slow transaction, usually a network call inside it |

- **R-0466** `lock_timeout`, `idle_in_transaction_session_timeout` and `client_connection_check_interval` are connection startup parameters, so they add no round trip. All exist on every supported version, which matters because an unknown startup parameter fails the connection. With `pooler = "transaction"` they aren't sent (section 11).
- **R-0467** **`transaction_timeout` covers only `db.transaction`.** Postgres's setting also applies to single statements and would break long queries, so it's sent as `SET LOCAL`, pipelined with `BEGIN`, on Postgres 17+. It ends the connection, and the pool replaces it. Postgres 15 and 16 use a client-side timer, which cancels the query and discards the connection.
- **R-0468** `.lock_timeout("30s")` and `.transaction_timeout("5m")` raise a limit for one transaction.
- **R-0469** **Cancelled requests:** when a future is dropped mid-query, as when a client disconnects, a queued rollback isn't enough, because the running query keeps its locks. morm spawns a cancel using the same TLS settings, and discards the connection. `client_connection_check_interval` is the backstop if the runtime is shutting down.
- **R-0470** **Visibility:** `pool_stats()` counts deadlocks, retries, lock timeouts and `NestedConnection` errors.

### Performance and memory / Behind a pooler

- **R-0667** **`SET LOCAL` still works,** because it ends with the transaction.
- **R-0669** `tx.advisory_lock` is unaffected, because it ends with the transaction.
- **R-0672** **Prepared statements** need PgBouncer 1.21+ with `max_prepared_statements` set. On older versions, set `statement_cache = 0` and `shape_cache = 0`, so everything runs through `query_typed`.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0764 | `tx.lock`, `advisory_lock` or `.skip_locked()` outside a transaction | Compile error |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0047 | integration | cancellation |
| T-0048 | integration | forced deadlocks in both kinds |
| T-0052 | integration | single-statement and transaction retry limits |
| T-0053 | integration | every timeout |
| T-0054 | integration | cancelled queries releasing their locks |
| T-0056 | integration | `.lock_ordered()` and bulk writes under concurrent load |
| T-0057 | integration | advisory key stability |
| T-0058 | integration | `skip_locked` with rival workers |
| T-0100 | integration | `tx.lock` with and without `.for_key_change()` against child inserts |
| T-0104 | integration | every `pooler = "transaction"` rule and the skipped limit check behind a real PgBouncer |
