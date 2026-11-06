# Step 52: Managed enums: online swap

- **Phase:** Migrations
- **Depends on:** 39, 54, 59
- **Can run beside:** 08, 09, 19, 20, 21, 22, 23, 24, 25, 26, 27, 42, 43, 44, 45, 46, 47, 48
- **Crates:** `morm-migrate`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Remove or reorder enum values on a live table with no long lock.

## Deliverables

- The online swap as the default on existing tables, `--fast` for the one-transaction form, keyset copy, definitions read from Postgres on scratch, materialized views built beside the old ones, cutover, retired columns and types, refusals, recovery

## End-to-end check

On Postgres 15–18, a swap with a map runs on a table under live writers with a cutover of milliseconds, no table rewrite and no lost write, and the process killed at every step still reaches the right state.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> Step 39 builds the sync trigger, batched copy and concurrent index forms for column type changes, and `--fast`. It also builds the rules this form follows for what sits above the column: a managed view is recreated in the cutover, and a materialized view is built beside the old one with its indexes, owner, comment and grants, opens a decision about its contents, stays unpopulated if it was created `WITH NO DATA`, and is swapped by rename, leaving the old copy retired until `purge`. It reads each object's text from Postgres on a throwaway scratch, after renaming the column there, so names in strings stay as they are and output columns keep their names, it checks a view whose text differs for the same meaning, and it refuses a `USING` view's table if the live columns don't sit where scratch's do. Step 38 retires columns and types. Step 53 finds dependents, step 59 refreshes functions, and step 54 handles domains and composite types in the one-transaction form.
>
> Measured on Postgres 18.6 and 17.11 with 2 million rows, under readers on a view and a materialized view and 2 writers: a cutover that rebuilt two materialized views took 1.1 to 1.2 s, with every read and write waiting on it. Built beside the old ones in about 1.2 s, the same cutover took about 2 ms, the longest read and write took 2 ms, and nothing failed.
>
> Step 39's route also fits this step's indexes and CHECKs. Tested on Postgres 18.6 and 17.11 on a column with the same name as its enum type: a partial index, a unique partial index, an expression index and CHECK constraints, with casts to the type and `'status'` in a string, were read through `pg_get_indexdef` and `pg_get_constraintdef` after renaming the column and the type on scratch, built on live, and each matched the models exactly after cutover. A materialized view with `USING` between two tables swapped together read the shadow columns of both.
>
> `down.sql` is always the one-transaction form.
>
> Update the earlier tests this changes, and pass `--fast` in the tests of 51, 53, 54, 58 and 59.
>

## Out of scope

- Phases and `deprecated` variants (step 41)
- `revert` (step 40)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Managed enums / Online swap

- **R-1046** **Online by default.** On an existing table a swap uses this form, and `--fast` uses the one-transaction form. `max_rewrite_mb` doesn't apply here, because nothing is rewritten under a full lock.

- **R-1047** **The steps,** each with its own boundary:

1. `tx`: create the new type under a temporary name.
2. `tx` per table: add the shadow column and a sync trigger on every insert and update, applying the map.
3. `batch` per table: copy by key, with a resumable cursor.
4. `no-tx`: build each index on the shadow column with `CREATE INDEX CONCURRENTLY`.
5. `tx`, then a separate `tx`: for a `NOT NULL` column add `CHECK (shadow IS NOT NULL) NOT VALID`. For a nullable one add `CHECK (<col> IS NULL OR shadow IS NOT NULL) NOT VALID`, dropped at cutover. Then `VALIDATE`. Any CHECK naming the column goes the same way.
6. `ANALYZE`, because statistics follow the column through a rename.
7. `tx` per materialized view above the column: build it beside the old one on the shadow column, as late as possible, with R-1051's text.
8. `tx`: the cutover, below.

- **R-1048** **The copy walks by key until nothing is left,** and reads each batch after the sync trigger has committed. A range worked out up front misses rows inserted just before the trigger attached. `VALIDATE` is the safety net: it fails if any row was missed.

- **R-1049** **The cutover** is one short transaction across all affected tables, with `lock_timeout`, retried with backoff, because a waiting lock holds up new readers behind it. It takes each carried view's lock with its own `DROP VIEW`, from the outside in, dropping anything carried above a view before that view, then locks the tables in sorted order and walks `pg_depend` again, because a reader locks the view first, locking the table first deadlocked every time, and `LOCK TABLE` on a view takes every table under it at once. It drops the sync trigger, the old indexes and the rest of step 53's dependents, apart from materialized views, renames the old column to a retired column with `NOT NULL` and its default removed, and renames the shadow column to the real name. It sets `NOT NULL` from the validated check, for a column that had it, renames the indexes, re-attaches unique constraints with `ADD CONSTRAINT ... USING INDEX`, and copies the column's comment and grants. It retires the old type, gives the new type the real name and restores defaults. It swaps each materialized view by rename, under step 39's rules, retiring the old copy, because rebuilding one here would run its query under the lock and break a cutover of milliseconds. Then it recreates the dependents, copies the owner, comment and grants, and refreshes functions.

- **R-1050** **Old values stay.** The retired column holds every old value and keeps the retired type alive until `purge`, which drops any retired materialized view, then the column, then the type. So a map's data effect here is `retires`, and down restores a value only where the row still holds what it was mapped to.

- **R-1051** **Text on the shadow column.** A partial index, an expression index, a CHECK or a materialized view that names the column takes its text from Postgres by step 39's route. On its throwaway scratch, the new column, the new type and each new object are renamed to their temporary names, and morm reads `pg_get_indexdef`, `pg_get_constraintdef` or `pg_get_viewdef`. So a cast reads the temporary type's name with no text replaced, even when the column and its type share a name. The scratch proof confirms the result: after cutover, each definition must match the one built from the models, and a view whose text differs is checked for the same meaning, as step 39 says.

- **R-1052** **Refused in this form, allowed with `--fast`,** naming the cause: partitioned tables, because concurrent indexes don't work on the parent; enum columns in a primary key or foreign key, as for key type changes; stored generated columns that read the column, because recreating them rewrites under lock; columns of a domain or composite type; exclusion constraints and extended statistics on the column; and a view or materialized view that would still read the retired column once recreated, such as `SELECT *`.

- **R-1053** **Recovery.** Each step has a precondition, so `--resume` continues, the copy resumes from its cursor, and failed concurrent indexes are rebuilt. `rollback --failed` drops the shadow objects and leaves the old column and type in place.

- **R-1054** **Column position.** The real column moves to the end of the table. Position isn't compared, and morm's own queries name columns, but a raw `SELECT *` read by position changes, so `--plan` says so.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0175 | suite | **Live traffic:** with writers on an old binary, including writes of the removed value until cutover and inserts during the copy, no write is lost and every row holds its mapped value. The table isn't rewritten, the stale statements recover, indexes are used after cutover, and `purge` drops the retired column and type. The cutover runs as a role that owns the type and its tables but isn't superuser. With a view and a materialized view above the column, readers on both see no failed read, the materialized view is built beside the old one and swapped in a cutover of milliseconds, ends on the new type with the mapped labels, and the old copy stays retired until `purge`. |
| T-0176 | suite | **Proof, killed mid-run:** kill the process at every step boundary, and during the copy and a concurrent index build, then prove `--resume` and `rollback --failed` both reach the right state with no write lost. The design's tests never killed this form. If a result contradicts this design, stop and ask the owner with the evidence and options. These tests run in CI's `Migration suite` step, and the PR links that step's log. |
| T-0177 | suite | **Proof, foreign keys and domains:** each refused layout is refused before anything changes, with `--fast` handling the same change. A foreign key on a different column of the table, and a domain on a different column, run online with no lost write. A unique constraint, a CHECK with no literal and a cast to the old type's name in a definition come through cutover, and so do the column's comment and grants. The design's online test had neither. If a result contradicts this design, stop and ask the owner with the evidence and options. |
| T-0178 | suite | **Proof, definitions from scratch:** on Postgres 15–18, a partial index, a unique partial index, an expression index, a CHECK and a materialized view on a column with the same name as its type build on the shadow column and the temporary type, with a cast to the type, and with the column's name inside a string literal, a comment, a label, a quoted or mixed-case identifier, and as part of a longer name, and final definitions match the models. A materialized view with `USING` between two tables swapped together comes through under step 39's check for the same meaning. A wrong definition fails the proof and writes no file. If a result contradicts this design, stop and ask the owner with the evidence and options. |
| T-0179 | suite | **Copy edges:** rows inserted just before the trigger attached are copied, a missed row fails `VALIDATE`, and a run on a table with a composite key resumes from its cursor. |
