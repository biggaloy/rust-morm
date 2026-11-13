# Step 38: Retire, purge and down with data

- **Phase:** Migrations
- **Depends on:** 37
- **Can run beside:** 08, 09, 19, 20, 21, 22, 23, 24, 25, 26, 27, 47
- **Crates:** `morm-migrate`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Make a drop reversible, with no hidden side effects on anything still using the object.

## Deliverables

- Retire for columns, tables and materialized views, `pg_depend` refusals, function source scan, `purge`, `drop now`, `down_fill`, rollback restoring data with its decisions, retired state in `verify`, refusing to retire a partition or an inheritance child while its parent stays

## End-to-end check

A retired column stops blocking deletes, rollback restores its data and constraints, and every dependency case is refused with the right list.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> ```
> makemigrations [--plan] [--json] [-n name] [--decide ID=OPT] [--decisions F] [--empty] [--fast]
> migrate [--plan] [--json] [--yes] [--to ID] [--resume] [--phase expand|contract] [--decide ID=OPT]
> rollback [--last N | --to ID | --orphans | --failed] [--plan] [--yes] [--accept-data-loss] [--production --reason R] [--phase contract] [--fast] [--decide ID=OPT]
> revert ID [--plan] [--decide ID=OPT]    mode [development --confirm NAME | production]    squash --to ID
> status [--json]    verify [--json]    check    lint    check-sql
> rehearse --url URL [--decide ID=OPT]    purge --older-than D | --now ID    baseline    rebase-chain
> repair --reason R     refingerprint           sql ID
> ```
>
> Steps 32, 33 and 34 own answers to decisions. A decision about a function has an ID built from the migration id, kind and function signature, plus a hash of the function's source. `migrate`, `rollback` and `rehearse` take `--decide ID=OPT` for a decision preflight opens, and an answer stands only for the same migration id, checksum, direction and decision ID. The ledger keeps `meta.json` verbatim, and an op stores its own data in `meta.json` under `op_data`.
>

## Out of scope

- `revert` (step 40)
- The real call that proves a managed function after a retire or a rename (step 59)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / What's managed

**R-0529** Tables and other objects with no definition in code are never changed, and morm never drops something because it's missing from the models. Other services' tables can share the database. If an unmanaged object depends on a change, that's a decision with evidence.

### Migrations / Down that can restore data

- **R-0570** **Retire instead of drop.** A dropped column is renamed `_morm_retired_<migration>_<col>`, with NOT NULL and defaults removed, and with its indexes, CHECK constraints and triggers dropped, along with foreign keys going out from the retired object. Otherwise a column the app no longer knows about would keep blocking deletes, or cascade them silently. The ledger records each definition. A dropped table moves to the `_morm_retired` schema, with its migration in the name. Long names follow the 63-byte rule with the hash suffix, because Postgres would truncate them and two could collide. The ledger records each original name. Rollback restores them with their data, rebuilds indexes concurrently, and recreates foreign keys and CHECKs as `NOT VALID`, then `VALIDATE`. Rows changed in the meantime can fail those steps, such as a parent deleted since, or a duplicate where a unique index was, so it stops with the offending rows and offers a decision: clear those values, delete those rows, or keep the constraint `NOT VALID`.
- **R-0571** **Objects that still depend on it stop the retire.** `makemigrations` and `migrate` read `pg_depend` and refuse, showing the list, while an object that isn't being retired depends on the column or table: a foreign key from a live table, a view, a materialized view, a function, a generated column, an RLS policy, extended statistics or a publication. Functions matter because a `BEGIN ATOMIC` body has an empty `prosrc`, so the text search can't see it, but it does record its dependencies. Renaming doesn't break these, which is the danger: a view would keep serving retired data, and the purge would fail later. Dropping a foreign key would quietly remove integrity from a column the app still uses. Retire them together, change them in the same migration, or remove them first. The planner applies managed changes first, so only what still depends after them counts. A generated column's dependency is recorded through `pg_attrdef`, so the query must include it.
- **R-0572** **Code Postgres doesn't track.** A `plpgsql` function body has no recorded dependency on the columns it names, so a retire or rename can break inserts through a trigger like `NEW.note := 'x'`. Before either, morm searches `pg_proc.prosrc` in every schema for the name, whole words only, in any case, because a body in another schema can name the object too, and an unquoted `SELECT N FROM T` names `n` and `t`. Common names like `status` will give false hits, which is fine, because a hit costs a decision, a `plpgsql_check` run and at most a `proof` (steps 35 and 59). A hit on a managed function is a decision with evidence, `makemigrations` proposes the rewritten function, and step 59 proves it with a real call and `plpgsql_check`. `migrate` preflight repeats the search on the live database, which is the only way to see unmanaged functions, and a hit not yet accepted is an open decision, which stops `migrate` with exit code `3` until `--decide` answers it. `rollback` preflight does the same before an inverse that retires or renames. `makemigrations` stores the names to search for in `op_data`, for each direction, because neither command has the models and `rollback` must work from the ledger's `meta.json` when the files are gone.
- **R-0574** **Data that must really go:** retiring keeps data, which conflicts with a legal duty to delete it. The retire decision also offers `drop now`, which is marked irreversible, and `mgr purge --now ID` drops one retired object before its time.
- **R-0575** **`morm-admin mgr purge --older-than 30d`** does the real drop later, as a recorded operation. It never uses `CASCADE`, and it re-checks dependencies first, because something may have been created during those 30 days. If so, it stops and shows the list. That migration's down then becomes irreversible.
- **R-0576** **Limit:** rows inserted after `up` have no value in a restored column. If it was NOT NULL, the decision must supply a `down_fill` value, or down refuses.
- **R-0577** **Irreversible migrations** must be declared. `rollback` refuses them without `--accept-data-loss`.
- **R-1062** **Retired materialized views.** An online column swap (step 39) retires the old copy of a materialized view, because a view can hold rows its table no longer has, and one may be frozen on purpose. It moves to the `_morm_retired` schema, named with its migration and following the 63-byte rule, and keeps its rows, indexes, owner, comment and grants. The ledger records its original name. A view or `BEGIN ATOMIC` function it still reads is retired with it, the same way. `verify` doesn't report any of these as drift, and a later op's `pg_depend` walk passes over them. The old copy still reads the retired column, and it's an object being retired under R-0571, so it doesn't stop the swap. `purge` drops the copies from the outside in, then what they read, then the retired columns and type, without `CASCADE`, stopping with the list if anything else depends on them. `rollback`, and `revert` of the swap, drop the new one and move the old one back to its schema and name in the inverse's short transaction, exactly as it was: the same rows, unpopulated if it was, and with no refresh. After `purge`, R-0575 applies. A later change Postgres refuses under a retired copy, such as `ALTER COLUMN TYPE` on another column it reads, is refused with exit code `2`, naming it, and `purge --now` clears it.

### Migrations / Production is forward only

- **R-0586** Retired columns are a tracked state, so `verify` doesn't report them as drift.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0779 | A view or generated column still using a retired column | Retire refuses and lists it; `purge` never cascades |
| R-0780 | A retired column blocking deletes | Retiring drops its outgoing foreign keys, CHECKs and triggers |
| R-0781 | A retire or rename breaking a trigger's function body | `makemigrations` and `migrate` search function source and stop on a hit |

### Migrations / Commands

- **R-0953** `purge --older-than D | --now ID`.

### Managed enums

- **R-0995** **Retired types.** A type can be retired along with the columns that still use it. It's renamed `_morm_retired_<migration>_<type>`, following the 63-byte rule, and the ledger records its original name. An enum type's labels are renamed in the same transaction, each to `_morm_retired_<label>`, through the same 63-byte rule, because a label has the same limit, and the ledger already holds the type's labels in order. So code still compiled against the retired type, such as a plpgsql variable or a cached statement, fails with `22P02` naming it, instead of running on the old labels' order with no error. Rows keep every value, because Postgres stores a label by its OID, and read as `_morm_retired_<label>` until `purge`. Down and rollback read a retired column by mapping each placeholder back to its recorded label in their own expression, and never rename the labels back, which would make stale code silent again. Restoring a retired type to live, as a rollback of a retire does, renames the type and its labels back, which is right, because it's live again with the same OID. `verify` doesn't report it as drift, and `purge` drops the retired columns first and the type after them, without `CASCADE`, stopping with the list if anything else depends on it. Owner decision SA.

**R-1172** **Retiring a child or a partition while its parent stays.** Moving a table to the `_morm_retired` schema doesn't take it out of its tree. On Postgres 18.6 and 17.11, a partition moved with `ALTER TABLE ... SET SCHEMA _morm_retired` stayed attached to its partitioned table, which kept routing new rows to it and returning its rows, and an inheritance child moved the same way stayed in `pg_inherits` under its parent, whose reads still returned its rows. So a retire of a table that is a partition or an inheritance child is refused with exit code `2` before the migration's first step, changing nothing, while any parent of it stays live, naming the table and each live parent. That holds wherever a table is retired: in `migrate`, in `rollback` for an inverse or a `down.sql` that retires a table, and for the retire decision's `drop now` and `purge --now`, because dropping a partition would take its rows out of its parent with no error and leave its range with no partition. A table is retired with its parents only when every one of them, up to the top parent, is retired in the same migration, which is step 39's rule for retiring a table with children or partitions, and a table with two parents is refused while either stays live. Scratch is built from the models, so it has no inheritance, and `migrate` and `rollback` preflight read `pg_inherits` on the live database for every table the migration retires. `purge` reads it again before it drops a retired table and refuses the same way if the table has been attached to a live parent since. The message says to detach the table outside morm first, with `ALTER TABLE <parent> DETACH PARTITION <table>` or `ALTER TABLE <table> NO INHERIT <parent>`, and run the command again, or to retire the whole tree in one migration.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0122 | suite | **Retire:** deleting a parent succeeds after its child column is retired, rollback recreates the constraints, orphaned rows produce a decision, every recorded dependency from a live object refuses the retire, including `BEGIN ATOMIC` functions, `purge` never cascades and re-checks dependencies, and retires and renames named in a trigger function are caught, both managed and unmanaged, including a function in another schema that the table's triggers don't call, and a body that writes the names in another case, such as `SELECT N FROM T`. |
| T-0188 | suite | **Retired materialized view:** on Postgres 15–18, after an online swap the old copy sits in `_morm_retired` with its rows, including a row the table has lost, and its indexes, owner, comment and grants, `verify` reports no drift, and a view beneath it is retired with it. `purge` drops the copy and what it reads before the retired column and type, with no `CASCADE`, and stops with the list if a live object depends on one. `rollback` restores it exactly, unpopulated if it was, with the new one gone, and `REFRESH ... CONCURRENTLY` works on it afterwards. A type change on another column a retired copy reads exits with code `2` naming it, and runs after `purge --now`. |
| T-0213 | suite | **Answering a hit at `migrate` and `rollback`:** on Postgres 15–18, an unmanaged function naming a column a retire or a rename takes away stops `migrate` with exit code `3`, printing an ID built from the migration id, kind, function signature and a hash of its source, and `migrate --decide` with that ID lets it through. After a rename, an unmanaged function naming the new name stops `rollback` with exit code `3`, and the answer given to `migrate` doesn't cover it. `rollback --decide` lets it through, with the migration's files deleted too, reading the names from the ledger. Editing the function's body before `migrate` opens a new ID, and an unmanaged function naming the column as `SELECT N FROM T` stops `migrate` the same way. |
| T-0258 | suite | **Retiring a child or a partition, on Postgres 15–18:** a migration that retires a partition while its partitioned table stays, one that retires an inheritance child, one that retires a grandchild and its parent while the top parent stays, and one that retires a child of two parents with only one of them, are each refused by `migrate` with exit code `2` before the first step, naming the table and each live parent, with nothing changed. So are a `rollback` whose inverse retires such a table, `drop now` on one, and `purge --now` of a retired table attached to a live table since. Retiring a whole partitioned table with its partitions, and a whole three-level inheritance tree, in one migration runs, and after `DETACH PARTITION` or `NO INHERIT` by hand the refused migrations run, and the parent no longer returns the retired table's rows. |
| T-0309 | suite | **Retired enum labels, on Postgres 15–18:** retiring an enum type with its column renames each label to `_morm_retired_<label>`, a label near 63 bytes shortened by the name function, and the column's rows read back as those placeholders; a session that compiled a function with `DECLARE v st` before the retire gets `22P02` naming the retired type on every call, never an answer; a rollback of the retire renames the type and its labels back and the rows read their own labels; `purge` drops the column and then the type |
