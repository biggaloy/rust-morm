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

- Every row of the safe forms table, batched backfills, sync triggers for type changes, `--fast`, refusal of key type changes

## End-to-end check

Each safe form runs under live traffic with no long lock, and no write is lost during a type-change copy.

## Out of scope

- Production mode (step 40)

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

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0123 | suite | **Safe forms:** `ADD PRIMARY KEY` on a nullable column takes no scan under lock, writes during a type-change copy aren't lost, long retired names don't collide, and `drop now` removes the data. |
