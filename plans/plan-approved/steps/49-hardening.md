# Step 49: Lock-order detector and hardening

- **Phase:** Finish
- **Depends on:** 26, 27, 41, 45, 47
- **Can run beside:** 43, 44, 46, 48
- **Crates:** all
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

The long-running proofs across the whole system.

## Deliverables

- Lock-order detector
- Long property tests for models and migrations
- Fuzzing
- Full chain on Postgres 15–18
- An audit that every compile-time guarantee has a `trybuild` case
- `cargo audit` and `cargo deny` clean

## End-to-end check

`nightly.yml` passes seven runs in a row on this step's branch, started through `workflow_dispatch`.

## Out of scope

- New features

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Reading and writing / Deadlocks

**R-0476** **Lock-order detector:** debug builds and `#[morm::test]` record the order each transaction touches tables, including foreign key locks that can conflict, such as cascades. If one path goes `accounts → orders` and another `orders → accounts`, it reports the cycle, even if no deadlock happened. Unregistered raw SQL is invisible to it, and table-level tracking can give false alarms, so it warns by default.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0768 | Locks taken in opposite order across transactions | Lock-order detector in debug builds and tests |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0114 | suite | **Property tests:** random models and data round-trip through insert, read and JSON. |
| T-0118 | suite | **Property tests:** random schema A → B with data, including views, functions and triggers: up reaches B, down returns to A, preserved rows are intact. |
| T-0127 | suite | **Full chain:** all up → all down → all up on Postgres 15–18. |
| T-0128 | suite | **Fuzzing:** step parser and `meta.json`. |
| T-0133 | trybuild | every compile-time guarantee has a `trybuild` case |
| T-0137 | suite | **Property tests run in CI:** `nightly.yml`'s `property` job runs the T-0114 and T-0118 tests, and the PR links that job's log from a `workflow_dispatch` run showing them pass. The job runs `cargo test --locked --workspace`, which also passes on other tests alone, so a run without these tests doesn't count. |
| T-0138 | suite | **Full chain runs in CI:** `nightly.yml`'s `full-chain` job runs the T-0127 test on Postgres 15–18, and the PR links each version's log from a `workflow_dispatch` run showing it pass. A run of zero tests doesn't count. At step 01 it ran `cargo test --locked -p morm-migrate --all-features`, which doesn't reach `tests/` (package `morm-tests`). If its command doesn't reach this test, change it and update the line in `tests/tests/repo_policy.rs` that pins it. |
| T-0139 | suite | **Fuzzing runs in CI:** `nightly.yml`'s `fuzz` job runs the T-0128 targets, and the PR links that job's log from a `workflow_dispatch` run showing `cargo fuzz run` on the step parser and on `meta.json`. A run with no targets doesn't count. The job exits 0 when `fuzz/` is missing, and its loop passes when `cargo fuzz list` is empty, so remove that exit and make an empty list fail. |
| T-0140 | suite | **Full chain covers the whole chain:** the migrations in `nightly.yml`'s `full-chain` job grow to cover tables and every managed object kind: functions, procedures, triggers, policies, grants, views, materialized views, enums, domains, composite types and sequences. Step 34 set this job up before managed objects existed, so a T-0127 run that covers only what step 34 could run doesn't count. For enums the chain includes an add, a rename, a remove with a map and a reorder, through both the online and one-transaction forms, and an enum under a domain and a composite type. The PR lists each kind the run covered. |
