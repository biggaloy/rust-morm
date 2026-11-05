# Step 20: Transactional outbox

- **Phase:** Runtime
- **Depends on:** 18
- **Can run beside:** nothing
- **Crates:** `morm`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Events saved with their data and delivered at least once.

## Deliverables

- `morm::pg_outbox!`, `tx.emit`, `tx.emit_keyed`, the worker, leases, retries, cleanup, the `outbox` feature

## End-to-end check

Killing the process and worker at every point never loses an event, and repeats carry the same `e.id`.

`pg_outbox!` declares a managed table. Until step 26 exists, tests create it directly; step 26 adds the managed form to its own tests.

## Out of scope

- `morm-admin outbox` (step 27)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools

- **R-0055** `outbox`: the transactional outbox (section 9), compiled out when off

### Hooks / Outbox: events that must not be lost

**R-0474** `after_commit` can lose an event if the process crashes after commit, because the data is saved and the event only exists in memory. The outbox saves the event in the same transaction as the data, so both exist or neither does.

**R-0475**

```rust
morm::pg_outbox!();                                    // declares the table, tracked by migrations

db.transaction(async |tx| {
    tx.insert(&mut order).await?;
    tx.emit("order_placed", &OrderPlaced { id: order.id }).await?;
    Ok(())
}).await?;

morm::outbox::worker(&db)                              // in the app, or its own process
    .on("order_placed", async |e: Event<OrderPlaced>| send_email(e.id, e.payload).await)
    .run().await?;
```

- **R-0476** **`emit` exists only on `tx`,** so an event can't be saved apart from its data. Hooks can reach it through `db.tx()` when they run in a transaction.
- **R-0477** **Delivery is at least once.** A worker can crash after sending and before recording it, so an event can arrive twice. `e.id` is stable across attempts, so handlers and receivers use it to ignore repeats.
- **R-0478** **Claims use a lease, not an open transaction.** A worker marks a batch as claimed with `FOR UPDATE SKIP LOCKED`, commits, then sends. Holding a transaction during network calls would hold locks and hit `transaction_timeout`. If a worker dies, its lease expires and the events are claimed again.
- **R-0479** **Failures retry with backoff.** After `max_attempts`, an event is marked `failed` and kept, never dropped. `pool_stats()` reports pending, failed and oldest-pending age from the worker's last poll, and `morm-admin outbox retry ID` runs it again.
- **R-0480** **Workers claim only event names they handle,** so an event with no handler waits during a deploy instead of burning its attempts. Its age shows in the oldest-pending figure. A payload that won't decode counts as a failed attempt.
- **R-0481** **Order isn't guaranteed by default,** because workers run in parallel. `tx.emit_keyed("order_placed", &order.id, &event)` delivers events with the same key in order, one at a time, which costs some throughput.
- **R-0482** **Workers poll,** using a partial index on pending events, so an idle check is one small query. `NOTIFY` isn't used, because it takes a global lock at commit, which limits write throughput.
- **R-0483** **Cleanup:** delivered events are deleted in batches after `retention`, so the table stays small.
- **R-0484** **Separate worker process:** use `Db::connect_as("jobs")`, as for other jobs.
- **R-0485** The app role needs read and write access on the table, so it's a normal managed table in the app's schema, not in `_morm`.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0123 | suite | **Fault injection:** kill the process and the worker at every point: before commit (no event), after commit (delivered), and after send before recording (delivered twice, same `e.id`). |
| T-0124 | suite | **Concurrency:** rival workers never hold one event at once while a lease is valid, expired leases return, and keyed events stay in order. |
| T-0125 | suite | **Failures:** backoff, `max_attempts`, `failed` events kept, `retry`, and cleanup after `retention`. |
