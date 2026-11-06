# Step 30: Diff and typed ops

- **Phase:** Migrations
- **Depends on:** 29
- **Can run beside:** 07, 08, 09, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 47
- **Crates:** `morm-migrate`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Diff two schemas into typed operations with inverses.

## Deliverables

- Diff, `Op` with inverse, lock, transaction and data properties, topological planning, ops for tables, columns, constraints, indexes, schemas and extensions

## End-to-end check

For a set of schema pairs, the diff produces ops whose forward and inverse SQL move a real database between both states exactly.

## Out of scope

- Writing files (step 32)
- Managed objects beyond tables (step 35)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Principles

- **R-0523** Never trust generated SQL until it has run.
- **R-0524** Check the real database before and after every migration.
- **R-0525** Every failure has a defined state and a way forward, including a killed process.
- **R-0526** Down is a first-class product: dropped data is kept where possible.
- **R-0527** Nothing destructive is guessed.

### Migrations / Source of truth

- **R-0547** Diff into typed operations.

**R-0548** Column position isn't compared.

### Migrations / Typed operations

**R-0551** The diff yields `Op` values, not SQL strings. Each declares:

| ID | Property | Use |
|---|---|---|
| R-0552 | forward SQL | up step |
| R-0553 | inverse op | down exists **by construction** |
| R-0554 | lock level, rewrite, scan | `lint`, `--plan`, ordering |
| R-0555 | transaction mode | `tx`, `no-tx`, `batch` |
| R-0556 | data effect | `none`, `retires`, `lossy`, `irreversible` |
| R-0557 | precondition | catalog check that makes re-running safe |

**R-0558** The planner topologically sorts ops (foreign key cycles are resolved by adding constraints last), takes table locks in a consistent order to avoid deadlocks, and groups ops into **steps**, each with its own transaction boundary.

### Managed enums

- **R-0987** **Step boundaries.** An op can declare that its step ends after it, so the planner puts whatever follows in a new transaction. Step 51 needs this for `ALTER TYPE ... ADD VALUE`, because nothing can use a new value until the transaction that added it commits.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0003 | suite | Unit: diff |
| T-0074 | integration | range CHECKs rejecting direct SQL writes |
| T-0119 | suite | **Op tests:** every op's inverse, with and without data |
| T-0135 | suite | **Migration suite runs in CI:** the `Migration suite` step of `ci.yml`'s `integration` job runs the T-0119 tests, and the PR links that step's log showing them pass. A run of zero tests doesn't count. At step 01 it ran `cargo test --locked -p morm-migrate --all-features`, which doesn't reach `tests/` (package `morm-tests`). If its command doesn't reach these tests, change it and update the line in `tests/tests/repo_policy.rs` that pins it. |
