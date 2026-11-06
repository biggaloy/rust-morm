# Step 21: Pool deadlocks

- **Phase:** Runtime
- **Depends on:** 20, 12
- **Can run beside:** 08, 09, 19, 23, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 51, 52, 53, 54
- **Crates:** `morm`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

A task can't deadlock the pool against itself, and pool waits are always finite.

## Deliverables

- `NestedConnection` in transactions and stream loops, `db.extra_conn()`, `wait_timeout`, `max_holds` on waits

## End-to-end check

Using `db` inside a transaction or a stream loop is refused, including `join!` and a stream moved between tasks, and an app with no open streams pays one atomic load.

## Out of scope

- Postgres deadlocks (step 22)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Reading and writing / Deadlocks

**R-0445** There are two kinds. In **Postgres**, two transactions lock the same rows in opposite order, and Postgres kills one with `40P01`, by default after `deadlock_timeout` (1s). In the **pool**, tasks holding a connection wait for another one while every connection is taken, and Postgres can't see that.

**R-0446** **Pool deadlocks:**

- **R-0447** **A second connection on the same task is refused.** Using `db` inside a transaction closure, or inside a `stream()` loop, returns `NestedConnection` immediately. This can't be a compile error, because closures can capture `db`. Inside `db.transaction`, morm runs the closure, so a task-local flag works. A `stream()` loop body is ordinary code outside any morm future, so a task-local can't wrap it: an open stream registers its `tokio::task::try_id()` in a small sharded set, and checkout consults that set only while an atomic count of open streams is above zero. A stream registers on its first poll, not at creation, because it's `Send` and can move to another task. Each later poll compares `try_id()` and re-registers if the task changed, which is one integer comparison. An app with no open streams pays one atomic load. The check is per task, so running a stream beside another query in `join!`, `select!` or `FuturesUnordered` is refused too. That really does hold two connections on one task, which is the risk this rule exists for, so such a query uses `db.extra_conn()`. This needs proof early (section 16).
- **R-0448** `db.extra_conn()` allows deliberate cases, such as an audit row that must survive a rollback.
- **R-0449** **Pool waits are always finite.** `wait_timeout` (default 5s) can't be turned off, and it ends every case the flag can't see: `tokio::spawn`, `join!`, and a task holding an app `Mutex` while waiting for a connection when another holds a connection and waits for that `Mutex`.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0759 | Using `db` inside a transaction or stream loop | `NestedConnection` at runtime; `db.extra_conn()` is the visible escape hatch |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0096 | integration | `NestedConnection` with `join!` and with a stream moved between tasks |
