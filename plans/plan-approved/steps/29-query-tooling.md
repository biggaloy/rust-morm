# Step 29: `sql!`, `check-sql`, test support and the loop detector

- **Phase:** Tools
- **Depends on:** 12, 27
- **Can run beside:** 28, 30
- **Crates:** `morm`, `morm-macros`, `morm-admin`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Checked raw SQL with no database at build time, and test tooling for users.

## Deliverables

- `sql!`, `check-sql` with `PREPARE` and `EXPLAIN`, `#[morm::test]`, the loop detector, `morm::allow_repeats`

## End-to-end check

A wrong column in `sql!` fails `check-sql`, a query in a loop fails a `#[morm::test]`, and each exclusion passes.

## Out of scope

- The lock-order detector (step 32)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Reading and writing / Query safety

- **R-0400** `sql!` builds a `&'static str` from `F` constants and registers it. `morm-admin mgr check-sql` runs `PREPARE` on each against scratch and compares types. That gives checked raw SQL with no database at build time.

### Migrations / Commands

- **R-0577** `check-sql`: `PREPARE` and `EXPLAIN` on every registered query.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0734 | Queries in a loop | Detector in debug builds and `#[morm::test]`: the same statement repeating many times in a short window warns, and fails the test. `fetch_page`, `.in_batches`, streams, the outbox worker and retries don't count, and `morm::allow_repeats(fut)` marks an intended loop |
| R-0736 | Filtering on an unindexed column | `morm-admin mgr check-sql` runs `EXPLAIN` on every registered query |

**R-0737** The compiler can't see a query inside a loop, so that one is caught in tests and debug builds, not at compile time.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0035 | integration | the loop detector |
| T-0111 | suite | **`#[morm::test]`:** gives each test a database cloned from a migrated template, for this project and its users. |
