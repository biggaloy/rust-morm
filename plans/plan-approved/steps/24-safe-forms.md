# Step 24: Safe forms, production mode and deploys

- **Phase:** Migrations
- **Depends on:** 23
- **Can run beside:** 25, 26
- **Crates:** `morm-migrate`, `morm-admin`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Default to migrations that are safe on live tables, and to forward-only production.

## Deliverables

- Every safe form, batched backfills, `mode`, `revert`, `rollback --failed`, emergency override, `squash`, `lint`, `rehearse`, expand and contract, sync triggers, `check_schema`, data hooks

## End-to-end check

Each safe form runs under live traffic with no long lock, a production database refuses `rollback` by default, and an old binary boots during an expand phase.

## Out of scope

- Retire and purge (step 25)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Safe forms are the default on existing tables

| ID | Change | Generated steps |
|---|---|---|
| R-0520 | Add FK or CHECK | `ADD ... NOT VALID`, then `VALIDATE` in a **separate** transaction (one transaction would hold the lock through the scan) |
| R-0521 | Add unique | `CREATE UNIQUE INDEX CONCURRENTLY`, then `ADD CONSTRAINT ... USING INDEX` |
| R-0522 | Add primary key | As above, after the `Set NOT NULL` steps for any nullable key column. `ADD PRIMARY KEY USING INDEX` otherwise runs `SET NOT NULL` itself, which scans the table under the strongest lock |
| R-0523 | Add index | `CREATE INDEX CONCURRENTLY` (no-tx) |
| R-0524 | Set NOT NULL | Batched backfill, `CHECK NOT VALID`, `VALIDATE`, `SET NOT NULL`, drop the check |
| R-0525 | Add NOT NULL column | Add nullable, batched backfill, then as above |
| R-0526 | Add column with a volatile default, such as `gen_random_uuid()` | Add with no default, `SET DEFAULT` so new rows get it, then batched backfill. A constant default is instant and stays one step |
| R-0527 | Type change that is lossy or rewrites the table, such as `int` to `bigint` | New column, sync trigger so writes during the copy aren't lost, batched copy, swap (old column retired) |
| R-0528 | Type change on a key referenced by foreign keys | Refused with guidance on a live table, because every referencing column and the identity sequence must change together. `--fast` does it in one rewrite for small tables or maintenance windows. An online form is in `later-defo-build.plan` |
| R-0529 | Widening type change | In place when no rewrite is needed |

**R-0530** New tables use plain DDL in one transaction. `--fast` allows the simple forms for small tables or maintenance windows.

### Migrations / Production is forward only

**R-0538** Rolling back after new data has been written can fail or lose that data, so production moves forward only. `down.sql` is still proven for every migration, because `revert` and development depend on it.

- **R-0539** **`mode development` must name the database,** as in `mgr mode development --confirm app_staging`, so a command aimed at the wrong URL can't remove the protection.
- **R-0540** **Every database is production until marked otherwise.** The mode is stored in `_morm`, not in an environment variable, so a wrong setting on a host can't turn it off, and a backup restores with it. Forgetting to mark a database leaves it safe.
- **R-0541** `morm-admin mgr mode development` marks a local or staging database, and `mode` shows the current value. Every change goes in `_morm.history`.
- **R-0542** **On a production database,** `rollback` and `rehearse` exit with code `2`. To rehearse on a clone of production, mark the clone first.
- **R-0543** **`morm-admin mgr revert ID`** is the way to undo in production. It creates a new forward migration from that migration's down, runs the full proof on scratch, and needs the same decisions, such as `down_fill`. Retired columns and tables come back with their data, and the chain stays linear, so every database has the same history.
- **R-0544** **A failed migration can always be cleaned up.** `migrate --resume` is the first choice. `rollback --failed` undoes the completed steps of a migration that never finished, because no release depends on it yet.
- **R-0545** **Emergency override:** `rollback --production --reason "..."` works but is logged to `_morm.history`, and it still needs `--accept-data-loss` where that applies.
- **R-0546** Retired columns are a tracked state, so `verify` doesn't report them as drift.

### Migrations / Rolling deploys

**R-0569** Static column lists mean a dropped column breaks the old binary mid-deploy. Migrations carry `phase: expand | contract`, and `--phase` splits them. `contract` refuses while a required `#[col(deprecated)]` hasn't shipped. Marking a `NOT NULL` column with no default as `deprecated` generates an expand step dropping that `NOT NULL`, because the new binary stops writing it. A rename decision offers an expand/contract form for live tables: add the new column, keep both in sync with a generated trigger, backfill in batches, and drop the old column and trigger in `contract`. `db.check_schema()` checks compatibility at startup, not equality, so an old binary restarting during an expand phase still boots. Every column the binary reads or writes must exist with the same type, or one the conversion layer accepts. Extra columns and objects are fine. Two cases that would fail that binary's inserts are reported: an extra `NOT NULL` column with no default, and a column the binary may leave out that is no longer nullable and has lost its default.

### Migrations / Data migrations

**R-0570** SQL steps first, including `batch` steps, covered by checksums. Custom Rust data hooks are registered by migration id and use raw SQL only, since model types change. A hook needs a matching down hook, or the migration is declared irreversible.

### Migrations / Commands

- **R-0575** `rehearse`: pending up → down → up on a clone or restored backup, with **real data**. The recommended last gate.
- **R-0576** `lint`: statement risk, in the style of `squawk`, plus index advice. With `--url` it reads real table sizes and reports risky operations only on tables big enough to matter.
- **R-0579** `squash --to ID`: replaces the chain up to `ID` with one baseline generated by replaying it. A database that has applied all of them just continues, and one partway through is refused until it catches up. Run it only when every database has passed `ID`. It refuses while retired objects from those migrations haven't been purged, because their rollback path would be gone.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0711 | Rolling back a production database | Refused, exit code `2`, unless marked `development`; `revert` is the forward path |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0098 | integration | `check_schema` during expand phases and after a dropped default |
| T-0117 | suite | **Safe forms:** `ADD PRIMARY KEY` on a nullable column takes no scan under lock, writes during a type-change copy aren't lost, long retired names don't collide, and `drop now` removes the data. |
| T-0119 | suite | **Forward only:** `rollback` and `rehearse` refused on production mode, the default on a new database, and `revert` restoring retired data. |
| T-0120 | suite | **Squash:** a squashed chain gives the same schema as the full chain, and a database partway through is refused. |
