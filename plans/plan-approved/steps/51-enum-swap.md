# Step 51: Managed enums: add, remove and reorder

- **Phase:** Migrations
- **Depends on:** 38
- **Can run beside:** 08, 09, 12, 19, 20, 21, 22, 23, 24, 25, 26, 27, 42, 43, 44, 45, 46, 47, 48
- **Crates:** `morm-migrate`, `morm-core`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Change a managed enum's values with exact inverses, including the changes Postgres has no command for.

## Deliverables

- The enum value diff, `ADD VALUE` ops, the type swap in one transaction, removed-value decisions, `down_map`, inverses, owner, grant and comment copies, `ANALYZE`, `max_rewrite_mb`, `lint` findings and with-data proof rows

## End-to-end check

On Postgres 15–18, a managed enum on a populated table goes through an add, a remove with a map, a reorder and all the way back with `rollback`, with the exact labels and rows at each point, and with the process killed at every step boundary.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> Step 36 creates and drops enums, renames values, and refuses every other value change with exit code `2`. This step replaces that refusal.
>
> Step 30 lets an op end its step. Steps 31, 32 and 33 let an op add `lint` findings, decisions, proof rows, `op_data` and preflight checks.
>
> Step 37 refreshes functions. Step 38 retires columns and types, and `purge` drops them.
>
> Retired columns are named `_morm_retired_<migration>_<col>`.
>

## Out of scope

- Dependents that Postgres won't rebuild by itself, and refusals (step 53)
- Domains and composite types built on the enum (step 54)
- The online form (step 52). Until it exists, every swap uses the one-transaction form this step builds

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Managed enums

- **R-1009** **Reading the change.** The diff compares the chain's labels with the declared labels, in order, after renames. If nothing is removed and the labels that stay keep their relative order, the change is one or more adds. Anything else needs a swap. The rename decision gains the answer `remove and add`.

- **R-1010** **Adding a value.** Each new label becomes `ALTER TYPE ... ADD VALUE` with `BEFORE` or `AFTER` its neighbour in the declared order, in a transaction, after any renames, with data effect `none`. The op ends its step, because Postgres refuses any use of the value until that transaction commits: an `UPDATE`, `SET DEFAULT`, a partial index, a view, even `enum_range`. A plpgsql body naming it isn't checked, so the boundary is unconditional. Several adds can share a step. `ADD VALUE` needs no table lock and rewrites nothing.

- **R-1011** **Adds and renames run before a swap,** so a swap never introduces a label, and a label rows are mapped to can be added earlier in the same migration.

- **R-1012** **The swap.** Postgres stores an enum value as an internal ID, so moving to a new type always rewrites the data. The one-transaction form is one `tx` step:

1. Set `lock_timeout` and lock every affected table in sorted order, so a busy table fails before anything has changed.
2. Drop column defaults on the affected columns, and anything step 53 drops.
3. For a map that keeps old values, add the retired column and fill it.
4. Rename the old type and create the new one under the real name, with the declared labels.
5. Change each column with `ALTER COLUMN ... TYPE ... USING`, applying any map, with one `ALTER TABLE` per table so each is rewritten once. The cast fails if a row holds a removed label with no map, which rolls everything back.
6. Copy the owner, comment and grants from the old type, then drop it.
7. Restore defaults and anything step 53 dropped, and check the fingerprint before commit.

`ANALYZE` of each affected table then runs as its own step, because the rewrite removes the column's statistics.

- **R-1013** **Array columns** change in the same way, and a map applies to each element with `array_replace` on a `text[]` cast.

- **R-1014** **What the old type carried.** `COMMENT`, the owner and grants belong to the type's OID, so a plain swap loses them. A `DO` block in the swap reads them from the catalog and applies them to the new type, including `WITH GRANT OPTION`, so the step's SQL stays fixed and live values are kept.

- **R-1015** **Rows holding a removed value.** `makemigrations` can't see live data, so removing a value is a decision, and `migrate` enforces its answer. The options are `refuse`, the default answer, which stops with exit code `2` and row counts per table while any row holds the value; `map` to another label, keeping the old values; and `map` with `drop now`, which keeps nothing.

- **R-1016** **Keeping old values.** A `map` that keeps old values adds a `text` column named as a retired column, filled only for rows it changes, and step 38's retired state tracks it until `purge`.

- **R-1017** **Reordering is a decision.** Sorting variants alphabetically must not quietly rewrite production tables. The evidence names the tables that would be rewritten, and with no answer `makemigrations` exits with code `3`.

### Migrations / Managed enums / Inverses

| ID | Op | Data effect | Inverse |
|---|---|---|---|
| R-1018 | Add | `none` | Remove by swap. It refuses while rows hold the value, unless a `down_map` is decided |
| R-1019 | Reorder | `none` | Reorder back by swap |
| R-1020 | Remove, no rows | `none` | `ADD VALUE` at the old position |
| R-1021 | Remove, map, keep old values | `retires` | `ADD VALUE`, then in a later step restore the rows below |
| R-1022 | Remove, map, `drop now` | `lossy` | `ADD VALUE`. The label returns and the rows don't |

### Migrations / Managed enums / After the swap

- **R-1023** **Restoring rows.** Down restores an old value only where the row still holds the value it was mapped to. A row the app changed since `up`, or a row inserted since, keeps what it has. `purge` makes that down irreversible.

- **R-1024** **Crash recovery.** The swap is all or nothing, so a killed run leaves the old type, and `--resume` reruns it. `ADD VALUE` has a precondition that the label is absent. `ANALYZE` can always rerun.

- **R-1025** **`max_rewrite_mb`.** `[migrations] max_rewrite_mb`, default 1024, limits a table rewrite under `ACCESS EXCLUSIVE`. Preflight refuses a one-transaction swap with exit code `2` when an affected table is bigger, for `migrate` and `rollback`, naming the table and its size. Raise the setting to go past it.

- **R-1026** **`lint`** reports every swap, with the tables it rewrites, an add on an enum that's a list partition key with no default partition, because a row with the new value then has nowhere to go, and any table in a publication, because the subscriber must migrate first.

- **R-1027** **With-data proof.** Enum columns get a row per label. A map also puts rows on the removed label, and `refuse` keeps rows off it. A column kept by a retired copy must match its checksum after up and down, and with `drop now` it's left out.

- **R-1028** **morm never edits `pg_enum`.** It needs superuser, and it would leave indexes referring to values that no longer exist, or in the wrong order.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0159 | suite | **Enum ops:** every op in the inverse table, with and without data, up, down and up, on Postgres 15–18, ending on the exact labels, order and rows, with no drift. |
| T-0160 | suite | **Removed values:** `refuse` stops with row counts and changes nothing, `map` keeps old values and restores them on down, a row changed or inserted after `up` keeps its value, `drop now` reports `lossy`, a column array maps per element, and a row written between preflight and the swap fails the cast and rolls back. |
| T-0161 | suite | **Decisions:** a reorder with no answer exits with code `3`, a remove and add with no `was` exits with code `3`, and an add in one migration with a use of the new value puts that use in a later step. |
| T-0162 | suite | **Fault injection:** kill the process at every step boundary and during a table rewrite. `--resume` and `rollback` both reach the right state, and a kill during the rewrite leaves the old type, its name, the dropped default and the table untouched. These tests run in CI's `Migration suite` step, and the PR links that step's log. |
| T-0163 | integration | **Proof, a non-superuser swap on Postgres 15–18:** add, remove and reorder as a role that owns the type and its tables but isn't superuser, and again with the type owned by another role it's a member of. The owner, comment and every grant, including `WITH GRANT OPTION`, survive, and a role lacking a privilege is refused by preflight. This was tested only as a superuser. If a result contradicts this design, stop and ask the owner with the evidence and options. |
| T-0164 | integration | **Proof, logical replication on Postgres 15–18:** with a subscription on the same server, run an add and a swap in both orders of primary and subscriber. Record what breaks, and confirm that migrating the subscriber first works, as the `lint` finding says. If a result contradicts this design, stop and ask the owner with the evidence and options. |
| T-0165 | integration | **`max_rewrite_mb`:** a swap over the limit is refused with exit code `2` naming the table, and raising the setting lets it run. |
