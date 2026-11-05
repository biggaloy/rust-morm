# Step 16: Deadlocks, locks and timeouts

- **Phase:** Runtime
- **Depends on:** 15
- **Can run beside:** nothing
- **Crates:** `morm`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Prevent both kinds of deadlock and bound every wait.

## Deliverables

- `NestedConnection`, `extra_conn`, `tx.lock`, `.for_key_change()`, `.lock_ordered()`, advisory locks, `.skip_locked()`, bulk sorting, single-statement retry, every timeout, cancellation

## End-to-end check

Forced deadlocks of both kinds resolve as specified, and a cancelled request's query is gone from `pg_stat_activity` with its locks released.

## Out of scope

- The lock-order detector (step 32)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Reading and writing / Deadlocks

**R-0413** There are two kinds. In **Postgres**, two transactions lock the same rows in opposite order, and Postgres kills one with `40P01`, by default after `deadlock_timeout` (1s). In the **pool**, tasks holding a connection wait for another one while every connection is taken, and Postgres can't see that.

**R-0414** **Pool deadlocks:**

- **R-0415** **A second connection on the same task is refused.** Using `db` inside a transaction closure, or inside a `stream()` loop, returns `NestedConnection` immediately. This can't be a compile error, because closures can capture `db`. Inside `db.transaction`, morm runs the closure, so a task-local flag works. A `stream()` loop body is ordinary code outside any morm future, so a task-local can't wrap it: an open stream registers its `tokio::task::try_id()` in a small sharded set, and checkout consults that set only while an atomic count of open streams is above zero. A stream registers on its first poll, not at creation, because it's `Send` and can move to another task. Each later poll compares `try_id()` and re-registers if the task changed, which is one integer comparison. An app with no open streams pays one atomic load. The check is per task, so running a stream beside another query in `join!`, `select!` or `FuturesUnordered` is refused too. That really does hold two connections on one task, which is the risk this rule exists for, so such a query uses `db.extra_conn()`. This needs proof early (section 16).
- **R-0416** `db.extra_conn()` allows deliberate cases, such as an audit row that must survive a rollback.
- **R-0417** **Pool waits are always finite.** `wait_timeout` (default 5s) can't be turned off, and it ends every case the flag can't see: `tokio::spawn`, `join!`, and a task holding an app `Mutex` while waiting for a connection when another holds a connection and waits for that `Mutex`.
- **R-0418** **Holds can't use up the pool.** `max_holds` defaults to a quarter of `max`, at least 1. `db.hold()` past it returns `PoolBusy`.
- **R-0419** `after_commit` runs after the connection is released, so it can't cause this.

**R-0420** **Postgres deadlocks:**

**R-0421**

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

- **R-0422** **Bulk writes are sorted.** `insert_many`, `upsert` batches, `update_each` and `delete_ids` sort input by key, skipping sorted input. Inserts lock in that order. For updates and deletes it usually holds, but the planner chooses the order, so a hash join can differ. `.lock_ordered()` is the guarantee.
- **R-0423** **`.lock_ordered()`** on `update_q`, `delete_q`, `update_each` and `delete_ids` locks rows in key order in the same statement, with a CTE and no extra round trip. It uses `FOR NO KEY UPDATE`, so child inserts aren't blocked, and `FOR UPDATE` only for deletes and updates of key columns. It adds query work, so it's opt-in, and the benchmark shows how much.
- **R-0424** **`tx.lock::<T>(&ids)`** runs `SELECT ... WHERE pk = ANY($1) ORDER BY pk FOR NO KEY UPDATE`. `NO KEY` means child inserts aren't blocked by foreign key checks. `tx.lock(&ids).for_key_change()` takes `FOR UPDATE`, for rows you will delete or whose key you will change.
- **R-0425** **One byte order everywhere.** App sorting and `ORDER BY` must agree, so text keys use `COLLATE "C"`, which matches Rust's byte order. Numbers and UUIDs already agree.
- **R-0426** **Foreign keys:** a child insert takes `FOR KEY SHARE` on the parent. A normal update of the parent doesn't conflict with that. Deleting the parent, or changing a key column it's referenced by, does. Plain `tx.lock` doesn't protect those paths: `FOR NO KEY UPDATE` lets a child insert in, and the later delete must wait for it, which can deadlock. They need `tx.lock(&ids).for_key_change()`, which blocks child inserts until commit.
- **R-0427** **`tx.advisory_lock(&keys)`** serializes work across tables when row order isn't practical, and it releases at the end of the transaction. Keys are the first 8 bytes of the SHA-256 of each string, a documented algorithm, because services must compute the same key across builds. Keys are sorted before locking.
- **R-0428** **`.skip_locked()`** on a list adds `FOR UPDATE SKIP LOCKED`, so queue workers skip claimed rows. It exists only on `tx`.

**R-0429** **Automatic retry:** a single statement outside a transaction retries once when Postgres reports `40P01` or `40001`, or when it failed before being sent. That's the complete list: a lost connection after sending has an unknown outcome, and a stream that has delivered rows never retries. Retry covers only the SQL, so hooks don't run again and `RETURNING` fills the row only on success. `hooks(tx)` statements are transactions, so they follow `.retry(n)`.

**R-0430** **Limiting the damage:**

| ID | Setting | Default | Effect |
|---|---|---|---|
| R-0431 | `lock_timeout` | 5s | One lock wait past this returns `LockTimeout` |
| R-0432 | `transaction_timeout` | 30s | A `db.transaction` past this returns `TransactionTimeout` |
| R-0433 | `idle_in_transaction_session_timeout` | 1m | Postgres ends a transaction left open and idle |
| R-0434 | `client_connection_check_interval` | 5s | Postgres stops a running query when the client has gone |
| R-0435 | `tx_warn_after` | 1s | Logs a slow transaction, usually a network call inside it |

- **R-0436** `lock_timeout`, `idle_in_transaction_session_timeout` and `client_connection_check_interval` are connection startup parameters, so they add no round trip. All exist on every supported version, which matters because an unknown startup parameter fails the connection. With `pooler = "transaction"` they aren't sent (section 11).
- **R-0437** **`transaction_timeout` covers only `db.transaction`.** Postgres's setting also applies to single statements and would break long queries, so it's sent as `SET LOCAL`, pipelined with `BEGIN`, on Postgres 17+. It ends the connection, and the pool replaces it. Postgres 15 and 16 use a client-side timer, which cancels the query and discards the connection.
- **R-0438** `.lock_timeout("30s")` and `.transaction_timeout("5m")` raise a limit for one transaction.
- **R-0439** **Cancelled requests:** when a future is dropped mid-query, as when a client disconnects, a queued rollback isn't enough, because the running query keeps its locks. morm spawns a cancel using the same TLS settings, and discards the connection. `client_connection_check_interval` is the backstop if the runtime is shutting down.
- **R-0440** **Visibility:** `pool_stats()` counts deadlocks, retries, lock timeouts and `NestedConnection` errors. Postgres's deadlock detail is logged through `tracing`, with no parameter values.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0708 | Using `db` inside a transaction or stream loop | `NestedConnection` at runtime; `db.extra_conn()` is the visible escape hatch |
| R-0718 | `tx.lock`, `advisory_lock` or `.skip_locked()` outside a transaction | Compile error |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0042 | integration | cancellation |
| T-0043 | integration | forced deadlocks in both kinds |
| T-0047 | integration | single-statement and transaction retry limits |
| T-0048 | integration | every timeout |
| T-0049 | integration | cancelled queries releasing their locks |
| T-0051 | integration | `.lock_ordered()` and bulk writes under concurrent load |
| T-0052 | integration | advisory key stability |
| T-0053 | integration | `skip_locked` with rival workers |
| T-0091 | integration | `NestedConnection` with `join!` and with a stream moved between tasks |
| T-0095 | integration | `tx.lock` with and without `.for_key_change()` against child inserts |
