# Step 25: Retire, purge and down with data

- **Phase:** Migrations
- **Depends on:** 23
- **Can run beside:** 24, 26
- **Crates:** `morm-migrate`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Make a drop reversible, with no hidden side effects on anything still using the object.

## Deliverables

- Retire for columns and tables, `pg_depend` refusals, function source scan, `purge`, `drop now`, `down_fill`, rollback restoring data with its decisions

## End-to-end check

A retired column stops blocking deletes, rollback restores its data and constraints, and every dependency case is refused with the right list.

## Out of scope

- Managed object recreation (step 26)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Down that can restore data

- **R-0531** **Retire instead of drop.** A dropped column is renamed `_morm_retired_<migration>_<col>`, with NOT NULL and defaults removed, and with its indexes, CHECK constraints and triggers dropped, along with foreign keys going out from the retired object. Otherwise a column the app no longer knows about would keep blocking deletes, or cascade them silently. The ledger records each definition. A dropped table moves to the `_morm_retired` schema, with its migration in the name. Long names follow the 63-byte rule with the hash suffix, because Postgres would truncate them and two could collide. The ledger records each original name. Rollback restores them with their data, rebuilds indexes concurrently, and recreates foreign keys and CHECKs as `NOT VALID`, then `VALIDATE`. Rows changed in the meantime can fail those steps, such as a parent deleted since, or a duplicate where a unique index was, so it stops with the offending rows and offers a decision: clear those values, delete those rows, or keep the constraint `NOT VALID`.
- **R-0532** **Objects that still depend on it stop the retire.** `makemigrations` and `migrate` read `pg_depend` and refuse, showing the list, while an object that isn't being retired depends on the column or table: a foreign key from a live table, a view, a materialized view, a function, a generated column, an RLS policy, extended statistics or a publication. Functions matter because a `BEGIN ATOMIC` body has an empty `prosrc`, so the text search can't see it, but it does record its dependencies. Renaming doesn't break these, which is the danger: a view would keep serving retired data, and the purge would fail later. Dropping a foreign key would quietly remove integrity from a column the app still uses. Retire them together, change them in the same migration, or remove them first. The planner applies managed changes first, so only what still depends after them counts. A generated column's dependency is recorded through `pg_attrdef`, so the query must include it.
- **R-0533** **Code Postgres doesn't track.** A `plpgsql` function body has no recorded dependency on the columns it names, so a retire or rename can break inserts through a trigger like `NEW.note := 'x'`. Before either, morm searches `pg_proc.prosrc` in the affected schemas, and in every function the table's triggers call, in any schema, for the name, whole words only. Common names like `status` will give false hits, which is fine, because a hit is only a decision. A hit on a managed function is a decision with evidence, and `makemigrations` proposes the rewritten function. `migrate` preflight repeats the search on the live database, which is the only way to see unmanaged functions, and refuses on a hit unless accepted. `doctor` runs the same scan.
- **R-0534** **Data that must really go:** retiring keeps data, which conflicts with a legal duty to delete it. The retire decision also offers `drop now`, which is marked irreversible, and `mgr purge --now ID` drops one retired object before its time.
- **R-0535** **`morm-admin mgr purge --older-than 30d`** does the real drop later, as a recorded operation. It never uses `CASCADE`, and it re-checks dependencies first, because something may have been created during those 30 days. If so, it stops and shows the list. That migration's down then becomes irreversible.
- **R-0536** **Limit:** rows inserted after `up` have no value in a restored column. If it was NOT NULL, the decision must supply a `down_fill` value, or down refuses.
- **R-0537** **Irreversible migrations** must be declared. `rollback` refuses them without `--accept-data-loss`.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0728 | A view or generated column still using a retired column | Retire refuses and lists it; `purge` never cascades |
| R-0729 | A retired column blocking deletes | Retiring drops its outgoing foreign keys, CHECKs and triggers |
| R-0730 | A retire or rename breaking a trigger's function body | `makemigrations` and `migrate` search function source and stop on a hit |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0116 | suite | **Retire:** deleting a parent succeeds after its child column is retired, rollback recreates the constraints, orphaned rows produce a decision, every recorded dependency from a live object refuses the retire, including `BEGIN ATOMIC` functions, `purge` never cascades and re-checks dependencies, and retires and renames named in a trigger function are caught, both managed and unmanaged. |
