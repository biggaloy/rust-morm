# Step 53: Managed enums: dependents

- **Phase:** Migrations
- **Depends on:** 58
- **Can run beside:** 08, 09, 19, 20, 21, 22, 23, 24, 25, 26, 27, 42, 43, 44, 45, 46, 47, 48
- **Crates:** `morm-migrate`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Find everything that depends on an enum on the live database, carry managed objects through a swap, and refuse the rest.

## Deliverables

- The dependency walk, the expected list in `meta.json`, preflight comparison, drop and recreate for every kind below, foreign keys, generated columns, every refusal

## End-to-end check

On Postgres 15–18, a swap under one object of every kind below plans, runs and reverses exactly, and a dependent no model declares stops `migrate` before anything changes.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> Step 51 swaps a type on columns with what Postgres rebuilds by itself. Step 37 drops and recreates managed objects in order.
>
> `op_data` in `meta.json` and op preflight checks come from steps 32 and 33.
>
> Update the earlier tests this changes.
>

## Out of scope

- Stale functions, source search and recovery on a pooler or standby (step 59)
- Swapping domains and composite types (step 54)
- The online form (step 52)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Managed enums / Dependents

- **R-1029** **The walk.** Start at the enum and follow `pg_depend` through every type built on it: its array, domains, composite types, range types and table row types. From each of those, collect what depends on the type and on columns of it, and repeat until nothing new turns up. It covers every schema, and partitions, and it includes the array type.

- **R-1030** **Compared on the live database.** `makemigrations` stores the walk's result from scratch in `op_data`. `migrate` preflight repeats the walk and refuses with exit code `2`, listing every object that isn't on the stored list, because scratch can't see what no model declares.

- **R-1031** **Rebuilt by Postgres, so left alone:** plain, unique and multi-column indexes, unique and exclusion constraints, extended statistics, inheritance and partition children when the enum isn't the key, CHECK constraints with no enum literal, and triggers that don't name the column.

- **R-1032** **Dropped and recreated, if managed:** partial and expression indexes and CHECK constraints that hold a literal, whose rebuilt text would still name the old type; views and materialized views, with a refresh; `BEGIN ATOMIC` functions; triggers with `WHEN` or `OF`; and policies, including those on other tables that read the column. Each is dropped before the type is renamed and recreated after the rewrite, in the fixed kind order, with an exact inverse.

- **R-1033** **Foreign keys between enum columns** are dropped, both columns change, and the key is added back in the same transaction. Postgres refuses to change either side alone.

- **R-1034** **Stored generated columns** that read the column, or are of the enum type, are dropped and added back after the rewrite, because Postgres refuses `ALTER COLUMN TYPE` on them.

- **R-1035** **A function or procedure with the enum in its signature** is dropped and created, because `CREATE OR REPLACE` can't change a signature, and its managed grants are applied again.

- **R-1036** **Always refused for a remove or reorder,** listing the object: the enum as a partition key, because Postgres can't change one, though adds and renames still work; a range type over the enum, and a rule, which morm doesn't manage; a table's row type used as a column elsewhere; and a foreign table column, because only the local definition would change.

- **R-1037** **Unmanaged dependents are refused,** whatever their kind, including a column in another schema. Scratch can't see them, so the SQL for them would be unproven, and morm never changes unmanaged objects. The message says to declare the object, or to change it in a hand-written `--empty` migration.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0166 | suite | **Every dependent kind:** one test per kind in the rebuilt, recreated, foreign key, generated, signature and refused lists above, with data, up, down and up, on Postgres 15–18. Refused cases change nothing and name the object. |
| T-0167 | integration | **Proof, the dependency list on Postgres 15–18:** the walk on one schema holding every kind returns the expected list on each version. Postgres 18 records NOT NULL in `pg_constraint` and has virtual generated columns, which can't use an enum, so version differences are named in the test. Add publications and rules to the schema, which the test on 18 didn't run. If a result contradicts this design, stop and ask the owner with the evidence and options. |
| T-0169 | suite | **Live refusals:** an unmanaged column in another schema, an unmanaged view and a new dependent created after `makemigrations` each stop `migrate` with exit code `2` and the list, changing nothing. |
| T-0170 | suite | **Fault injection:** kill the process during a swap with recreated dependents, and every dropped object is still there. These tests run in CI's `Migration suite` step, and the PR links that step's log. |
