# Step 52: Managed enums: online swap

- **Phase:** Migrations
- **Depends on:** 39, 53
- **Can run beside:** 08, 09, 19, 20, 21, 22, 23, 24, 25, 26, 27, 42, 43, 44, 45, 46, 47, 48
- **Crates:** `morm-migrate`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Remove or reorder enum values on a live table with no long lock.

## Deliverables

- The online swap as the default on existing tables, `--fast` for the one-transaction form, keyset copy, literal name replacement, cutover, retired columns and types, refusals, recovery

## End-to-end check

On Postgres 15–18, a swap with a map runs on a table under live writers with a cutover of milliseconds, no table rewrite and no lost write, and the process killed at every step still reaches the right state.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> Step 39 builds the sync trigger, batched copy and concurrent index forms for column type changes, and `--fast`. Step 38 retires columns and types. Step 53 finds dependents and refreshes functions, and step 54 handles domains and composite types in the one-transaction form.
>
> Step 14's scanner for `$n` skips quoted strings and comments.
>
> `down.sql` is always the one-transaction form.
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
5. `tx`, then a separate `tx`: add `CHECK (shadow IS NOT NULL) NOT VALID`, then `VALIDATE`. A CHECK with a literal goes the same way.
6. `ANALYZE`, because statistics follow the column through a rename.
7. `tx`: the cutover, below.

- **R-1048** **The copy walks by key until nothing is left,** and reads each batch after the sync trigger has committed. A range worked out up front misses rows inserted just before the trigger attached. `VALIDATE` is the safety net: it fails if any row was missed.

- **R-1049** **The cutover** is one short transaction across all affected tables, locked in sorted order with `lock_timeout`, retried with backoff, because a waiting lock holds up new readers behind it. It drops the sync trigger, the old indexes and step 53's dependents, renames the old column to a retired column with `NOT NULL` and its default removed, and renames the shadow column to the real name. It sets `NOT NULL` from the validated check, renames the indexes, retires the old type, gives the new type the real name and restores defaults. Then it recreates the dependents, copies the owner, comment and grants, and refreshes functions.

- **R-1050** **Old values stay.** The retired column holds every old value and keeps the retired type alive until `purge`, which drops the column and then the type. So a map's data effect here is `retires`, and down restores a value only where the row still holds what it was mapped to.

- **R-1051** **Text on the shadow column.** morm has no SQL parser, so a partial index filter or CHECK that names the column is rewritten by replacing the column's name outside quoted strings and comments, with step 14's scanner. The scratch proof confirms the result: after cutover, each definition must match the one built from the models.

- **R-1052** **Refused in this form, allowed with `--fast`,** naming the cause: partitioned tables, because concurrent indexes don't work on the parent; enum columns in a primary key or foreign key, as for key type changes; stored generated columns that read the column, because recreating them rewrites under lock; and columns of a domain or composite type.

- **R-1053** **Recovery.** Each step has a precondition, so `--resume` continues, the copy resumes from its cursor, and failed concurrent indexes are rebuilt. `rollback --failed` drops the shadow objects and leaves the old column and type in place.

- **R-1054** **Column position.** The real column moves to the end of the table. Position isn't compared, and morm's own queries name columns, but a raw `SELECT *` read by position changes, so `--plan` says so.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0175 | suite | **Live traffic:** with writers on an old binary, including writes of the removed value until cutover and inserts during the copy, no write is lost and every row holds its mapped value. The table isn't rewritten, the stale statements recover, indexes are used after cutover, and `purge` drops the retired column and type. |
| T-0176 | suite | **Proof, killed mid-run:** kill the process at every step boundary, and during the copy and a concurrent index build, then prove `--resume` and `rollback --failed` both reach the right state with no write lost. The design's tests never killed this form. If a result contradicts this design, stop and ask the owner with the evidence and options. These tests run in CI's `Migration suite` step, and the PR links that step's log. |
| T-0177 | suite | **Proof, foreign keys and domains:** each refused layout is refused before anything changes, with `--fast` handling the same change. A foreign key on a different column of the table, and a domain on a different column, run online with no lost write. The design's online test had neither. If a result contradicts this design, stop and ask the owner with the evidence and options. |
| T-0178 | suite | **Proof, name replacement:** on Postgres 15–18, the column's name inside a string literal, a comment, a label, a quoted or mixed-case identifier, and as part of a longer name is handled right, and final definitions match the models. A wrong replacement fails the proof and writes no file. If a result contradicts this design, stop and ask the owner with the evidence and options. |
| T-0179 | suite | **Copy edges:** rows inserted just before the trigger attached are copied, a missed row fails `VALIDATE`, and a run on a table with a composite key resumes from its cursor. |
