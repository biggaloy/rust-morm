# Step 21: Schema IR, scratch and typed ops

- **Phase:** Migrations
- **Depends on:** 05
- **Can run beside:** 12, 13, 15
- **Crates:** `morm-migrate`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Read schemas into a typed IR and diff them into typed operations with inverses.

## Deliverables

- Model registration behind `cfg(morm_mgr)` and a minimal `morm-mgr` that prints the manifest, which later migration steps need before step 27 packages it
- `Schema` IR, catalog reader, scratch database handling, diff, `Op` with inverse, lock, transaction and data properties, topological planning

## End-to-end check

For a set of schema pairs, the diff produces ops whose forward and inverse SQL move a real database between both states exactly.

## Out of scope

- Writing files (step 22)
- Managed objects beyond tables (step 26)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools

**R-0045** **Registration:** models, objects and `sql!` queries register themselves through a distributed static list, with no central list to maintain. Their metadata is behind `cfg(morm_mgr)`, not a cargo feature, because `cargo build --workspace` unifies features and would pull it into the app. `morm-admin` builds `morm-mgr` in its own target directory with that `cfg` set. This needs proof early, including models in several crates, where a linker can drop a crate nothing references (section 16).

### Migrations / Principles

- **R-0486** Never trust generated SQL until it has run.
- **R-0487** Check the real database before and after every migration.
- **R-0488** Every failure has a defined state and a way forward, including a killed process.
- **R-0489** Down is a first-class product: dropped data is kept where possible.
- **R-0490** Nothing destructive is guessed.

### Migrations / Source of truth

**R-0505** No snapshots. `makemigrations` uses a scratch Postgres (`MORM_SCRATCH_URL`) and never touches staging or production:

- **R-0506** Replay the migration chain into temporary database A.
- **R-0507** Create the models and objects in temporary database B.
- **R-0508** Read both through the same catalog queries (`pg_get_viewdef`, `pg_get_functiondef`, `pg_get_triggerdef` and so on) into a typed `Schema` IR, so Postgres normalizes everything.
- **R-0509** Diff into typed operations.

**R-0510** Column position isn't compared.

**R-0511** **Scratch must match production.** `makemigrations` proves nothing if scratch differs, so `doctor` compares the Postgres major version and installed extensions of scratch and the target, and `makemigrations` refuses when a model's `extensions` aren't available on scratch.

### Migrations / Typed operations

**R-0512** The diff yields `Op` values, not SQL strings. Each declares:

| ID | Property | Use |
|---|---|---|
| R-0513 | forward SQL | up step |
| R-0514 | inverse op | down exists **by construction** |
| R-0515 | lock level, rewrite, scan | `lint`, `--plan`, ordering |
| R-0516 | transaction mode | `tx`, `no-tx`, `batch` |
| R-0517 | data effect | `none`, `retires`, `lossy`, `irreversible` |
| R-0518 | precondition | catalog check that makes re-running safe |

**R-0519** The planner topologically sorts ops (foreign key cycles are resolved by adding constraints last), takes table locks in a consistent order to avoid deadlocks, and groups ops into **steps**, each with its own transaction boundary.

### Risks, open items and later work / Risks

- **R-0866** **Scratch databases:** `makemigrations` needs a Postgres where it can `CREATE DATABASE`.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0114 | suite | **Op tests:** every op's inverse, with and without data, and every dependency recreate path. |
