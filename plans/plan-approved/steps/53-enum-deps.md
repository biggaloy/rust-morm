# Step 53: Managed enums: dependents

- **Phase:** Migrations
- **Depends on:** 51, 12
- **Can run beside:** 08, 09, 19, 20, 21, 22, 23, 24, 25, 26, 27, 42, 43, 44, 45, 46, 47, 48
- **Crates:** `morm-migrate`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Find everything that depends on an enum on the live database, carry managed objects through a swap, and refuse the rest.

## Deliverables

- The dependency walk, the expected list in `meta.json`, preflight comparison, drop and recreate for every kind below, foreign keys, generated columns, function body search, function refresh, every refusal

## End-to-end check

On Postgres 15–18, a swap under one object of every kind below plans, runs and reverses exactly, and a dependent no model declares stops `migrate` before anything changes.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> Step 51 swaps a type on columns with what Postgres rebuilds by itself. Step 37 drops and recreates managed objects in order, and refreshes functions.
>
> Step 38 searches function source for a name, in whole words, in the affected schemas and in every function an affected table's triggers call.
>
> `op_data` in `meta.json` and op preflight checks come from steps 32 and 33.
>

## Out of scope

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

- **R-1038** **Code Postgres doesn't track.** Before a swap, a rename or a removal, step 38's source search also runs for the type's name, for each label that changes, and for each affected table and column. A hit on a managed function is a decision, and `makemigrations` proposes the rewritten function. A `plpgsql` body can name a removed label, and a `s::text = 'old'` comparison never fails, it just stops matching.

- **R-1039** **Refreshing managed functions.** Every managed plpgsql function that search finds, and every one an affected table's triggers call, is refreshed in the swap's transaction. A trigger function needs this even when it never names the type: on Postgres 18 a trigger comparing `NEW.s` failed on every call in every open session until it was refreshed.

- **R-1040** **Unmanaged functions** that would go stale are a decision, repeated in preflight on the live database. The default refuses. Accepting says other services must reconnect, because morm doesn't change unmanaged functions. morm apps recover by themselves.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0166 | suite | **Every dependent kind:** one test per kind in the rebuilt, recreated, foreign key, generated, signature and refused lists above, with data, up, down and up, on Postgres 15–18. Refused cases change nothing and name the object. |
| T-0167 | integration | **Proof, the dependency list on Postgres 15–18:** the walk on one schema holding every kind returns the expected list on each version. Postgres 18 records NOT NULL in `pg_constraint` and has virtual generated columns, which can't use an enum, so version differences are named in the test. Add publications and rules to the schema, which the test on 18 didn't run. If a result contradicts this design, stop and ask the owner with the evidence and options. |
| T-0168 | integration | **Stale plpgsql on Postgres 15–18:** a session that compiled `DECLARE v the_enum`, `SELECT ... INTO`, `%ROWTYPE` and a trigger function before a swap gets what proof 15 recorded with no refresh, and all four work with one. If a result contradicts this design, stop and ask the owner with the evidence and options. |
| T-0169 | suite | **Live refusals:** an unmanaged column in another schema, an unmanaged view, a function that only names the type in its body, and a new dependent created after `makemigrations` each stop `migrate` with exit code `2` and the list, changing nothing. Accepting the unmanaged function decision lets it run. |
| T-0170 | suite | **Fault injection:** kill the process during a swap with recreated dependents, and every dropped object is still there. These tests run in CI's `Migration suite` step, and the PR links that step's log. |
| T-0171 | integration | **Proof, PgBouncer:** in a `pooler` test, in transaction mode, an app recovers from a swap the way proof 16 recorded, with managed functions refreshed. If replacing the client connection can't clear stale server state and no refresh covers it, stop and ask the owner with the evidence and options. |
| T-0172 | integration | **Proof, hot standby:** a `read_only` pool on a standby recovers from a swap on the primary the way proof 17 recorded. If a result contradicts this design, stop and ask the owner with the evidence and options. |
