# Step 17: Inserts

- **Phase:** Runtime
- **Depends on:** 14, 07
- **Can run beside:** 08, 09, 15, 16, 29, 30, 31, 32, 33, 34, 35
- **Crates:** `morm`, `morm-macros`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Every way of adding rows, including bulk paths.

## Deliverables

- The internal transaction with confirmed commit that `contracts.md` lists, which step 20 then builds `db.transaction` on
- `insert`, `insert_many` on every path, `insert_stream`, `.in_batches` on inserts, `.explicit_ids()` on `insert`, `insert_many` and `insert_stream` with the sequence move, `OrDefault`, statements per pattern, array columns

## End-to-end check

A million rows go through each `insert_many` path, atomically or in batches, and defaults, arrays and explicit ids behave as specified on real Postgres.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> **A commit is confirmed, never assumed.** After a failed statement, Postgres turns `COMMIT` into `ROLLBACK` with no error, and tokio-postgres's `commit()` returns `Ok` either way, so a swallowed error would look like a saved transaction. Two checks prevent that:
>
> - Any Postgres error marks the transaction as failed. Further use returns that error, and a closure returning `Ok` still gets it.
>
> - `COMMIT` is sent as `SELECT 1; COMMIT` in one message. In a failed transaction the `SELECT` errors and Postgres skips the `COMMIT`, so this also catches errors morm didn't see, with no extra round trip. morm then rolls back and returns `TransactionAborted`.
>

## Out of scope

- Hooks (step 24)
- Upsert (step 19)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Models / Field options (`#[col(...)]`)

| ID | Option | Meaning |
|---|---|---|
| R-0162 | `default_fn` | Rust function that supplies a default on insert |

### Views and groups / Views: restricted models

- **R-0236** **Inserts:** a view that hides a required column with no default can't be inserted.

### Reading and writing / Insert and update

**R-0340**

```rust
db.insert(&mut p).await?;                                  // INSERT ... RETURNING, fills p in place
db.insert_many(&mut rows).await?;                          // UNNEST, COPY above a threshold
db.upsert(&mut p, F.sku).await?;                           // ON CONFLICT

db.update::<Product>(42).set(F.price, dec!(34)).run().await?;       // no fetch needed
db.update_q::<Product>().by(F.vendor_id, &7).set(F.active, false).run().await?;
db.update_each::<Product>(&rows, &[F.price]).await?;                // per-row values, one statement
db.update::<Product>(42).set_expr(F.stock, "stock - $1", &n).run().await?;  // atomic math in SQL
```

- **R-0342** **Large inserts need no chunking from you.** `insert_many` picks the path by size. A statement has no row limit, because `UNNEST` sends one array per column, not one parameter per value.

| ID | Rows | Path | Notes |
|---|---|---|---|
| R-0343 | Small | `UNNEST` | One statement, one round trip |
| R-0344 | Large | `COPY`, streamed | Fastest, with flat memory on both sides |
| R-0345 | Large, with `.returning()` | `UNNEST` in chunks, one transaction | `COPY` can't return rows or handle conflicts. Each Postgres value is capped at 1GB, so chunks stay well below it |
| R-0347 | Large, with `None` on some rows of a defaulted column | `UNNEST` in chunks, one per pattern of present columns, one transaction | `COPY` can't choose a default per row |

- **R-0348** `insert_many` is atomic on every path: all rows or none, in one transaction.
- **R-0349** **`.returning()` doesn't promise input order,** because rows are sorted by key and grouped by pattern, so match returned rows by a key.
- **R-0350** **Array columns:** `UNNEST` flattens nested arrays, so two rows holding `text[]` values would become four. A `Vec<T>` column is sent as a `text[]` of array literals and cast back per row (`x::text[]`), which keeps one statement and works with `.returning()`. These columns use the text format, so the escaping must be exact, and property tests cover quoted `"NULL"` strings, real NULLs, quotes, commas and empty arrays. `COPY` handles arrays directly.
- **R-0352** **Explicit ids:** a normal insert always leaves an identity column to Postgres, whatever the field holds. `.explicit_ids()` on `insert`, `insert_many` and `insert_stream` writes the ids you give, adds `OVERRIDING SYSTEM VALUE` to every `INSERT` that sends an identity column, and moves the sequence past them in the same transaction. Postgres needs it for `GENERATED ALWAYS` and accepts it under `BY DEFAULT`, where it changes nothing: on Postgres 18.6 and 17.11 an insert with it stored the given id in a `BY DEFAULT` column. So it's sent under either kind, whichever the model was built with, and a build made while a column was `BY DEFAULT` keeps writing chosen ids after a migration makes it `GENERATED ALWAYS`. A `COPY`, on `insert_many`'s `COPY` path and in `insert_stream`, sends the key column with no clause, as R-1202 says.
- **R-0353** **Moving the sequence.** It finds the identity's sequence through `pg_depend`, as the one the identity owns, and first takes the two-argument `pg_advisory_xact_lock(<morm namespace>, <sequence oid>)`, which can't collide with the hashed keys of `tx.advisory_lock`, so two imports into one sequence run one after the other and can't overwrite each other's position. Then, inside a savepoint, it takes `LOCK TABLE <table> IN SHARE MODE`, reads the sequence's `last_value`, calls `setval(seq, max(id))` only when `max(id) >= last_value`, where `max(id)` is the highest key the call sends, taken from the rows, with no scan of the table, and rolls back to the savepoint, which releases the table lock and keeps the `setval`, because a sequence isn't transactional. `insert` and `insert_many` do this before sending any row, and with `.in_batches(n)` each batch's transaction does it for that batch's rows. It's `>=` because a sequence that was never used has `last_value = 1` with nothing taken. The table lock is what makes the read safe. A sequence can't be locked: `LOCK TABLE` refuses one with `This operation is not supported for sequences`, `FOR UPDATE` with `cannot lock rows in sequence`, and `nextval` takes only `ROW EXCLUSIVE` on it. So without the table lock a normal insert can take a value between the read and the `setval`, and the `setval` moves the sequence backwards: on Postgres 18.6 and 17.11, with the read and the `setval` as two statements, five normal inserts between them were given 1 to 5, the `setval` put the sequence back to 3, and the next insert was given 4 again. `SHARE` conflicts with the `ROW EXCLUSIVE` lock every insert takes on its table before it draws a value, so the lock waits for inserts already running and holds new ones back until the savepoint is rolled back, and on a partitioned table it holds back inserts straight into a partition too, because `LOCK TABLE` reaches every partition. With it, on both versions, a normal insert during an import was given 101 straight after the move to 100, while the import was still open, and a second import, waiting on the advisory lock, then read 102 and moved the sequence to 150, with no key given twice. Without the advisory lock, two imports that had each written a row deadlocked on the table lock.
- **R-0354** It needs `SELECT` and `UPDATE` on the sequence, and `UPDATE` on the table for the lock, unlike normal identity inserts, so give those grants only to the role that imports. On Postgres 18.6 and 17.11, a role with only `INSERT` and `SELECT` on the table was refused the lock with `permission denied`, and `UPDATE` on the table let it in, as `MAINTAIN` does on 17 and later. On a partitioned table, grants on the parent were enough.
- **R-0355** **What the move costs, and what it doesn't stop.** The lock waits for writes already running on the table and holds new ones back while it waits, for up to `lock_timeout`, past which the call returns `LockTimeout`. For `insert` and `insert_many` that comes before any row is sent, so nothing is written, and once the sequence has moved no normal insert can be given a chosen key above where it stood. A chosen key at or below that point may already be in the table, which fails the call with `Unique`, or, on a sequence with `CACHE` above 1, may sit in another session's cache, which fails that session's later insert: on Postgres 18.6 and 17.11, under `CACHE 20`, a session that had taken 1 was refused with `23505` on its next insert, after an import stored 2 and left the sequence at 20. `insert_stream` moves the sequence only after its `COPY`, as R-1202 says, so a normal insert during the stream can be given a key the stream writes, and one that waits on a streamed row, over that key or any other unique column, deadlocks with the stream's table lock. On Postgres 18.6 and 17.11, Postgres broke it by failing the stream's lock with `deadlock detected`, over the key and over another unique column, so the stream rolls back and returns `Deadlock`, with nothing written. A `nextval` called other than by an insert into the table isn't held back: one ran while the lock was held. So run streams with chosen keys while the table is quiet.
- **R-0356** An id set without `.explicit_ids()` is ignored, which is almost always a mistake, so debug builds warn.
- **R-0357** `.in_batches(n)` on `insert_many` commits every `n` rows, for bulk loads that shouldn't be one huge transaction with long locks and a large WAL burst. It isn't atomic, and running it again needs `upsert` or a filter to skip rows already loaded.
- **R-0358** `db.insert_stream::<T>(stream)` takes rows from a `Stream`, through `COPY`, so 100 million rows never sit in app memory.
- **R-1202** **Chosen keys through `insert_stream`.** `db.insert_stream::<T>(stream).explicit_ids()` sends the key column in its `COPY` and keeps the highest key as it encodes rows. It takes R-0353's advisory lock before the `COPY`, and after the `COPY`, before `COMMIT`, in the same transaction, moves the sequence as R-0353 says. `COPY` takes a sent key under either kind with no error, and has no clause for it: on Postgres 18.6 and 17.11, `COPY` stored keys 5 and 7 in a `GENERATED ALWAYS` column, and stored keys through a partitioned table's parent and straight into a partition, with the sequence still unused at 1, so the next insert was given 1, and `COPY ... WITH (OVERRIDING SYSTEM VALUE)` failed with a syntax error. The move is what keeps the sequence past the streamed keys. Without `.explicit_ids()`, `insert_stream` leaves an identity column out of the `COPY`, as a normal insert does.
- **R-0366** **Defaults stay in the database.** A field that is `None` on a column with a default is left out of the statement, so a migration changing a default takes effect on running binaries, and identity and `GENERATED ALWAYS` columns work. A model with no such fields has one compile-time statement. Otherwise there is one statement per pattern of present columns, usually one or two, held in `shape_cache`. `insert_many` groups rows by pattern, in one transaction.
- **R-0368** **Base columns:** a bigint `id` is an `i64`, 0 until inserted, and identity columns are never sent. A uuid `id` is a `Uuid`: left nil, `insert` generates a UUIDv7 in the app and fills it in, so the statement doesn't change. `created_at` and `updated_at` are `db_set`.
- **R-0369** **Writing NULL over a default:** `None` on a nullable column with a default means "use the default". When you need a real NULL there, declare the field `morm::OrDefault<Option<T>>`, with the states `Default` and `Set(v)`.

### Performance and memory / Query path

- **R-0722** Bulk insert via `UNNEST` (one stable statement, no 65,535-parameter limit), and `COPY` above a threshold, whose value comes from the benchmark.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0783 | Explicit ids leaving the sequence behind | A normal insert ignores the id; `.explicit_ids()` moves the sequence |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0032 | integration | insert |
| T-0060 | integration | every `insert_many` path at a million rows |
| T-0095 | integration | concurrent `.explicit_ids()` runs and fresh sequences |
| T-0097 | integration | `.explicit_ids()` with both identity kinds and with sequences ahead of the data, sending `OVERRIDING SYSTEM VALUE` under both, and from a binary whose model was built with `BY DEFAULT` after `ALTER COLUMN ... SET GENERATED ALWAYS` |
| T-0098 | integration | changed defaults reaching old binaries |
| T-0099 | integration | `OrDefault` |
| T-0275 | integration | **Chosen keys through `insert_stream`, on Postgres 15–18:** under each identity kind, on a plain table and through a partitioned table's parent's model, a stream with `.explicit_ids()` stores its keys as sent, and the sequence ends past the highest, so the next normal insert gets the value after it, on a sequence never used and on one already ahead of the keys, which stays where it was; without `.explicit_ids()` the identity column is left out of the `COPY` |
| T-0276 | integration | **The sequence under concurrent writes, on Postgres 15–18:** with normal inserts running beside `insert`, `insert_many` and `insert_stream` with `.explicit_ids()`, no key is given twice and the sequence never moves backwards; a normal insert started after `insert_many`'s move gets a key past the chosen ones while the call is still writing; two imports into one sequence run one after the other; a write transaction left open makes the call return `LockTimeout` with nothing written; a normal insert waiting on a streamed row ends the stream with `Deadlock` and nothing written; and a role without `UPDATE` on the table is refused the lock |
