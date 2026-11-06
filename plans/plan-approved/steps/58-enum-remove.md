# Step 58: Managed enums: removing values and inverses

- **Phase:** Migrations
- **Depends on:** 51
- **Can run beside:** 08, 09, 19, 20, 21, 22, 23, 24, 25, 26, 27, 42, 43, 44, 45, 46, 47, 48
- **Crates:** `morm-migrate`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Remove enum values safely, with maps for the rows that hold them, and finish the inverse of every enum op.

## Deliverables

- Removed-value decisions, maps in the swap, `down_map`, retired copies of old values, the remaining inverses, and with-data proof rows

## End-to-end check

On Postgres 15–18, a managed enum on a populated table goes through an add, a remove with a map, a reorder and all the way back with `rollback`, with the exact labels and rows at each point, and with the process killed at every step boundary.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> Step 51 reads the value diff, adds values, and swaps a type in one transaction, in these steps: lock the affected tables; drop defaults; for a map that keeps old values, add the retired column and fill it; rename the old type and create the new one; change each column with `ALTER COLUMN ... TYPE ... USING`, applying any map; copy the owner, comment and grants, then drop the old type; restore defaults and check the fingerprint. Array columns change in the same way, and a map applies to each element with `array_replace` on a `text[]` cast. This step builds the two map parts, and until now a swap has only reordered.
>
> Step 51 also refuses to roll back an add, and step 57 refuses to remove a value, each with exit code `2`. This step replaces both refusals.
>
> Steps 31, 32 and 33 let an op add `lint` findings, decisions, proof rows, `op_data` and preflight checks.
>
> Step 38 retires columns and types, and `purge` drops them. Retired columns are named `_morm_retired_<migration>_<col>`.
>
> Update the earlier tests this changes.
>

## Out of scope

- Dependents that Postgres won't rebuild by itself, and refusals (step 53)
- Domains and composite types built on the enum (step 54)
- The online form (step 52)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Managed enums

- **R-1015** **Rows holding a removed value.** `makemigrations` can't see live data, so removing a value is a decision, and `migrate` enforces its answer. The options are `refuse`, the default answer, which stops with exit code `2` and row counts per table while any row holds the value; `map` to another label, keeping the old values; and `map` with `drop now`, which keeps nothing. An add takes an optional `down_map`, given to `makemigrations` as a decision answer and stored in `op_data`, and it never leaves a decision open. On down, rows holding the added value move to that label. With none, down refuses with row counts.

- **R-1016** **Keeping old values.** A `map` that keeps old values adds a `text` column named as a retired column, filled only for rows it changes, and step 38's retired state tracks it until `purge`.

### Migrations / Managed enums / Inverses

| ID | Op | Data effect | Inverse |
|---|---|---|---|
| R-1018 | Add | `none` | Remove by swap. It refuses while rows hold the value, unless a `down_map` is decided |
| R-1020 | Remove, no rows | `none` | `ADD VALUE` at the old position |
| R-1021 | Remove, map, keep old values | `retires` | `ADD VALUE`, then in a later step restore the rows below |
| R-1022 | Remove, map, `drop now` | `lossy` | `ADD VALUE`. The label returns and the rows don't |

### Migrations / Managed enums / After the swap

- **R-1023** **Restoring rows.** Down restores an old value only where the row still holds the value it was mapped to. A row the app changed since `up`, or a row inserted since, keeps what it has. `purge` makes that down irreversible.

- **R-1027** **With-data proof.** Enum columns get a row per label. A map also puts rows on the removed label, and `refuse` keeps rows off it. A column kept by a retired copy must match its checksum after up and down, and with `drop now` it's left out.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0159 | suite | **Enum ops:** every op in the inverse table, and a reorder, with and without data, up, down and up, on Postgres 15–18, ending on the exact labels, order and rows, with no drift. |
| T-0160 | suite | **Removed values:** `refuse` stops with row counts and changes nothing, `map` keeps old values and restores them on down, a row changed or inserted after `up` keeps its value, `drop now` reports `lossy`, a column array maps per element, and a row written between preflight and the swap fails the cast and rolls back. An add with a `down_map` moves rows holding the added value to that label on down, and with none, down refuses with row counts. A remove runs as a role that owns the type and its tables but isn't superuser, and the owner, comment and every grant survive. |
