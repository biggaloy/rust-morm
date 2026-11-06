# Step 39: Safe forms

- **Phase:** Migrations
- **Depends on:** 38
- **Can run beside:** 08, 09, 19, 20, 21, 22, 23, 24, 25, 26, 27, 42, 43, 44, 45, 46, 47, 48
- **Crates:** `morm-migrate`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Default to migrations that are safe on live tables.

## Deliverables

- Every row of the safe forms table, batched backfills, sync triggers for type changes, carrying a managed view through a column swap's cutover, `--fast`, refusal of key type changes

## End-to-end check

Each safe form runs under live traffic with no long lock, and no write is lost during a type-change copy. A managed view above a swapped column serves reads through the cutover and ends on the new column.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> Step 37 reads `pg_depend` for what depends on a column, drops managed dependents from the outside in, and recreates them from their declarations in the fixed kind order, keeping their owner, comment and grants. Anything no model declares is refused.
>
> Step 38 retires a column. It refuses, with the list, while an object that isn't being retired depends on the column, and the planner applies managed changes first, so only what still depends after them counts. That's why a view R-1061 carries no longer stops the retire, and why anything else still does. `purge` never uses `CASCADE`.
>
> Step 59 comes after this step and adds its source search, function refresh and unmanaged function decision to every op that changes a column's type, including this cutover. Nothing here needs to call them.
>
> Measured on Postgres 18.6 and 17.11 with 2 million rows: the cutover took 5 to 21 ms with no view, 10 to 14 ms with one view, and 8 to 16 ms with five stacked views and their functions. With 8 readers on the view at about 88,000 reads a second and 2 writers, it took 13 to 20 ms, the longest read 10 ms and the longest write 78 ms, with no failed read and no lost write.
>
> Locking the table first deadlocked a reader or the cutover, and cost a second. Recreating a materialized view took 1.4 to 1.5 s for a copy of those rows and about 70 ms for an aggregate of them, all under the table's lock. A view left in place followed the rename and read the retired column, and `purge` was refused. A `SELECT *` view recreated with the retired column in it, so `purge` was refused there too. A prepared read of the column failed once with `cached plan must not change result type`, with a view or without one, and step 14 re-prepares it.
>
> Update the earlier tests this changes.
>

## Out of scope

- Production mode (step 40)
- The source search, function refresh and unmanaged function decision at cutover (step 59)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Indexes

- **R-0319** On existing tables, migrations build indexes concurrently by default (section 10).

### Migrations / Safe forms are the default on existing tables

| ID | Change | Generated steps |
|---|---|---|
| R-0559 | Add FK or CHECK | `ADD ... NOT VALID`, then `VALIDATE` in a **separate** transaction (one transaction would hold the lock through the scan) |
| R-0560 | Add unique | `CREATE UNIQUE INDEX CONCURRENTLY`, then `ADD CONSTRAINT ... USING INDEX` |
| R-0561 | Add primary key | As above, after the `Set NOT NULL` steps for any nullable key column. `ADD PRIMARY KEY USING INDEX` otherwise runs `SET NOT NULL` itself, which scans the table under the strongest lock |
| R-0562 | Add index | `CREATE INDEX CONCURRENTLY` (no-tx) |
| R-0563 | Set NOT NULL | Batched backfill, `CHECK NOT VALID`, `VALIDATE`, `SET NOT NULL`, drop the check |
| R-0564 | Add NOT NULL column | Add nullable, batched backfill, then as above |
| R-0565 | Add column with a volatile default, such as `gen_random_uuid()` | Add with no default, `SET DEFAULT` so new rows get it, then batched backfill. A constant default is instant and stays one step |
| R-0566 | Type change that is lossy or rewrites the table, such as `int` to `bigint` | New column, sync trigger so writes during the copy aren't lost, batched copy, swap (old column retired) |
| R-0567 | Type change on a key referenced by foreign keys | Refused with guidance on a live table, because every referencing column and the identity sequence must change together. `--fast` does it in one rewrite for small tables or maintenance windows. An online form is in `later-defo-build.plan` |
| R-0568 | Widening type change | In place when no rewrite is needed |

**R-0569** New tables use plain DDL in one transaction. `--fast` allows the simple forms for small tables or maintenance windows.

**R-1061** **A view above an online column swap.** R-0566's swap carries a managed view above the column instead of stopping at the retire. Its cutover is one short transaction with `lock_timeout`, retried with backoff. It locks each carried view from the outside in, then the table, because a reader locks the view first and the other order deadlocks. It walks `pg_depend` again, drops the sync trigger and the carried objects from the outside in, renames the old column to its retired name and the new column to the real name, and recreates the objects from their declarations in the fixed kind order, keeping their owner, comment and grants, so the view reads the new column. Its inverse carries them back the same way. A view no model declares is refused with the full list before the swap's first step, and `migrate` preflight repeats that. An object the walk can't carry that appears during the copy stops the cutover, which changes nothing. A materialized view anywhere above the column is refused the same way, because recreating it runs its query under the table's lock, and so is a view that would still read the retired column once recreated, such as `SELECT *`, which the scratch proof finds in `pg_depend`. `--fast` handles both.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0123 | suite | **Safe forms:** `ADD PRIMARY KEY` on a nullable column takes no scan under lock, writes during a type-change copy aren't lost, long retired names don't collide, and `drop now` removes the data. |
| T-0187 | suite | **View above an online column swap:** on Postgres 15–18, with readers on the view and writers on the table through the swap, no read fails, no write is lost, and a prepared read recovers. Afterwards the view reads the new column and its new type and keeps its owner, comment and grants, `pg_depend` shows nothing on the retired column, and `purge` drops it with no `CASCADE`. A view over that view and a function over the view come through the same way, up, down and up, ending on the exact catalog. A reader arriving while the cutover holds its first lock gets its row, with no deadlock. A lock timeout leaves everything unchanged, and the cutover retries. A view no model declares, a materialized view and a `SELECT *` view are each refused with the full list before the swap's first step, changing nothing, and `--fast` handles the last two. A view created during the copy stops the cutover with nothing changed. If a result contradicts this design, stop and ask the owner with the evidence and options. |
