# Step 26: Schema IR, scratch and typed ops

- **Phase:** Migrations
- **Depends on:** 25, 09
- **Can run beside:** 06, 07, 08, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 40
- **Crates:** `morm-migrate`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Read schemas into a typed IR and diff them into typed operations with inverses.

## Deliverables

- `Schema` IR, catalog reader, scratch database handling, diff, `Op` with inverse, lock, transaction and data properties, topological planning

## End-to-end check

For a set of schema pairs, the diff produces ops whose forward and inverse SQL move a real database between both states exactly.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> **How it fits together:** `morm-admin` holds the engine. When a command needs the models, it builds and runs `morm-mgr` to get the manifest. Commands that don't need models (`migrate`, `rollback`, `status`, `purge`) run from `morm-migrations/` alone, so a production host needs `morm-admin` and no Rust toolchain. The app binary carries no migration code or schema metadata, apart from the short per-model column list `check_schema` reads.
>

## Out of scope

- Writing files (step 28)
- Managed objects beyond tables (step 30)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Principles

- **R-0507** Never trust generated SQL until it has run.
- **R-0508** Check the real database before and after every migration.
- **R-0509** Every failure has a defined state and a way forward, including a killed process.
- **R-0510** Down is a first-class product: dropped data is kept where possible.
- **R-0511** Nothing destructive is guessed.

### Migrations / Source of truth

**R-0526** No snapshots. `makemigrations` uses a scratch Postgres (`MORM_SCRATCH_URL`) and never touches staging or production:

- **R-0527** Replay the migration chain into temporary database A.
- **R-0528** Create the models and objects in temporary database B.
- **R-0529** Read both through the same catalog queries (`pg_get_viewdef`, `pg_get_functiondef`, `pg_get_triggerdef` and so on) into a typed `Schema` IR, so Postgres normalizes everything.
- **R-0530** Diff into typed operations.

**R-0531** Column position isn't compared.

### Migrations / Typed operations

**R-0534** The diff yields `Op` values, not SQL strings. Each declares:

| ID | Property | Use |
|---|---|---|
| R-0535 | forward SQL | up step |
| R-0536 | inverse op | down exists **by construction** |
| R-0537 | lock level, rewrite, scan | `lint`, `--plan`, ordering |
| R-0538 | transaction mode | `tx`, `no-tx`, `batch` |
| R-0539 | data effect | `none`, `retires`, `lossy`, `irreversible` |
| R-0540 | precondition | catalog check that makes re-running safe |

**R-0541** The planner topologically sorts ops (foreign key cycles are resolved by adding constraints last), takes table locks in a consistent order to avoid deadlocks, and groups ops into **steps**, each with its own transaction boundary.

### Risks, open items and later work / Risks

- **R-0896** **Scratch databases:** `makemigrations` needs a Postgres where it can `CREATE DATABASE`.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0003 | suite | Unit: diff |
| T-0074 | integration | range CHECKs rejecting direct SQL writes |
| T-0119 | suite | **Op tests:** every op's inverse, with and without data |
