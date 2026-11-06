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

- Every row of the safe forms table, batched backfills, sync triggers for type changes, carrying a managed view through a column swap's cutover, building a materialized view beside the old one and swapping it at cutover, the shadow column's name replacement, `--fast`, refusal of key type changes

## End-to-end check

Each safe form runs under live traffic with no long lock, and no write is lost during a type-change copy. A managed view above a swapped column serves reads through the cutover and ends on the new column, and a materialized view above it is built beside the old one and swapped in the same cutover of milliseconds.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> Step 37 reads `pg_depend` for what depends on a column, drops managed dependents from the outside in, and recreates them from their declarations in the fixed kind order, keeping their owner, comment and grants. Anything no model declares is refused.
>
> Step 38 retires a column. It refuses, with the list, while an object that isn't being retired depends on the column, and the planner applies managed changes first, so only what still depends after them counts. That's why a view R-1061 carries no longer stops the retire, and why anything else still does. `purge` never uses `CASCADE`. Step 38 also holds the retired copy of a materialized view and restores it on `rollback`.
>
> Step 14's scanner for `$n`, in `morm-core`, skips quoted strings and comments.
>
> Step 59 comes after this step and adds its source search, function refresh and unmanaged function decision to every op that changes a column's type, including this cutover. Nothing here needs to call them.
>
> Measured on Postgres 18.6 and 17.11 with 2 million rows: the cutover took 5 to 21 ms with no view, 10 to 14 ms with one view, and 8 to 16 ms with five stacked views and their functions. With 8 readers on the view at about 88,000 reads a second and 2 writers, it took 13 to 20 ms, the longest read 10 ms and the longest write 78 ms, with no failed read and no lost write.
>
> Locking the table first deadlocked a reader or the cutover, and cost a second. Recreating a materialized view took 1.4 to 1.5 s for a copy of those rows and about 70 ms for an aggregate of them, all under the table's lock. A view left in place followed the rename and read the retired column, and `purge` was refused. A `SELECT *` view recreated with the retired column in it, so `purge` was refused there too. A prepared read of the column failed once with `cached plan must not change result type`, with a view or without one, and step 14 re-prepares it.
>
> Measured on Postgres 18.6 and 17.11 with 2 million rows, under 8 readers on a view, a materialized view and a view over it, and 2 writers, with no failed read and no lost write: building a copy beside the old one took about 1 s, its indexes and `ANALYZE` about 1 s, an aggregate about 0.4 s, and a `WITH NO DATA` view 1 ms. A cutover swapping three of them, with three views recreated, took 3 to 4 ms, and swapping back took 35 to 38 ms.
>
> `LOCK TABLE` refuses a materialized view with `42809`, and `ALTER MATERIALIZED VIEW ... RENAME` or `SET SCHEMA` locks only that view. A view or function above one follows it by OID, so one left in place ends up reading the retired copy, and a plpgsql variable of its row type then failed with `type of parameter ... does not match that when preparing the plan` until a refresh. Replacing the name alone gave an output column called `n_new`. A column list kept `n`, and after the rename `pg_get_viewdef` matched the declared query. A retired copy stopped `DROP COLUMN` on the retired column, and Postgres refused `ALTER COLUMN TYPE` on any other column it read.
>
> `LOCK TABLE` on a view also locks every table under it, straight away, in the view's `FROM` order, and passes over a materialized view. With two views side by side, or a view with a second table first in its `FROM`, that deadlocked a reader, a writer or the cutover.
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

**R-1061** **A view above an online column swap.** R-0566's swap carries a managed view above the column instead of stopping at the retire. Its cutover is one short transaction with `lock_timeout`, retried with backoff. It locks each carried view from the outside in, then the table, because a reader locks the view first and the other order deadlocks. It walks `pg_depend` again, drops the sync trigger and the carried objects from the outside in, renames the old column to its retired name and the new column to the real name, and recreates the objects from their declarations in the fixed kind order, keeping their owner, comment and grants, so the view reads the new column. Its inverse carries them back the same way. A view no model declares, materialized or not, is refused with the full list before the swap's first step, and `migrate` preflight repeats that. An object the walk can't carry that appears during the copy stops the cutover, which changes nothing. A view that would still read the retired column once recreated, materialized or not, such as `SELECT *`, is refused the same way, which the scratch proof finds in `pg_depend`. A materialized view anywhere above the column follows R-1063. `--fast` still handles all of these in one transaction.

**R-1063** **A materialized view above an online column swap.** Recreating one would run its query under the table's lock, so it's built beside the old one. After the copy, `VALIDATE` and `ANALYZE`, as late as possible before the cutover, a step of its own creates it under a temporary name on the shadow column, using R-1064's query, with its indexes, owner, comment and grants, and runs `ANALYZE`. Reads and writes carry on during the build. A view, materialized view or `BEGIN ATOMIC` function between it and the column is built beside it in the same way, innermost first. The cutover swaps each one by rename: the old one is retired, as step 38 says, and the new one and its indexes take the real names. `LOCK TABLE` refuses a materialized view, so the rename takes its lock, from the outside in, before the table's. A view, function or other object above it would stay on the old copy, so R-1061 carries it in the same cutover, recreating it on the new one, and a view no model declares is refused. A rebuild opens a decision naming the view: its contents will come from today's table, and the old copy stays until `purge`, because a view can hold rows the table no longer has, and one may be frozen on purpose. One created `WITH NO DATA` holds nothing, so it's built `WITH NO DATA`, stays unpopulated and opens no decision. `--resume` drops an unfinished build and builds again, so the new copy stays as late as possible, and `rollback --failed` drops it and leaves the old one in place.

**R-1064** **Text on the shadow column.** morm has no SQL parser, so R-1063's query is rewritten by replacing the column's name outside quoted strings and comments, with step 14's scanner. Identifiers in double quotes are compared, not skipped. Replacing the name would also rename an output column, so the `CREATE` lists every output column by the name it has on the old view, which keeps `SELECT id, n` as `id, n`. The scratch proof confirms the result: after cutover, each definition must match the one built from the models, output names and types included. A wrong replacement fails the proof and writes no file.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0123 | suite | **Safe forms:** `ADD PRIMARY KEY` on a nullable column takes no scan under lock, writes during a type-change copy aren't lost, long retired names don't collide, and `drop now` removes the data. |
| T-0187 | suite | **View above an online column swap:** on Postgres 15–18, with readers on the view and writers on the table through the swap, no read fails, no write is lost, and a prepared read recovers. Afterwards the view reads the new column and its new type and keeps its owner, comment and grants, `pg_depend` shows nothing on the retired column, and `purge` drops it with no `CASCADE`. A view over that view and a function over the view come through the same way, up, down and up, ending on the exact catalog. A view over a second table, which `LOCK TABLE` on the view also locks, comes through with either table first in its `FROM`, and so do two views side by side, with readers on each view and on the second table and writers on both tables: no deadlock, no failed read and no lost write. A reader arriving while the cutover holds its first lock gets its row, with no deadlock. A lock timeout leaves everything unchanged, and the cutover retries. A view no model declares, materialized or not, and a `SELECT *` view, materialized or not, are each refused with the full list before the swap's first step, changing nothing, and `--fast` handles the `SELECT *` views. A view created during the copy stops the cutover with nothing changed. If a result contradicts this design, stop and ask the owner with the evidence and options. |
| T-0189 | suite | **Materialized view above an online column swap:** on Postgres 15–18, with readers on it, on a view over it and on the table, and writers on the table, a copy, an aggregate and a materialized view over another are built beside the old ones on the shadow column and swapped in a cutover of milliseconds. No read fails, no write is lost, and a prepared read recovers. Afterwards each reads the new column and its new type, matches the models and keeps its indexes, owner, comment and grants, with statistics. One over a managed view, and one over a `BEGIN ATOMIC` function, come through with what they read built beside them. A view and a function over it end on the new one, and only retired copies still read the retired column. Up, down and up ends on the exact catalog. The rebuild opens a decision, and no file is written until it's answered. A `WITH NO DATA` view stays unpopulated and opens none. One over a view no model declares is refused before the swap's first step. Killed during a build, `--resume` builds it again, and `rollback --failed` leaves the old one in place. If a result contradicts this design, stop and ask the owner with the evidence and options. |
| T-0190 | suite | **Proof, rewritten query:** on Postgres 15–18, an output column keeps its name, as a bare `n`, a qualified `t.n`, `n AS total` and an expression, and the column's name inside a string literal, a comment, a quoted or mixed-case identifier, and as part of a longer name is handled right. Final definitions match the models, and a wrong replacement fails the proof and writes no file. If a result contradicts this design, stop and ask the owner with the evidence and options. |
