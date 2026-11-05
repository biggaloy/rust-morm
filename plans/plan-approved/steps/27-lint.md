# Step 27: `lint`

- **Phase:** Migrations
- **Depends on:** 26
- **Can run beside:** 06, 07, 08, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 40
- **Crates:** `morm-migrate`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Report risky statements and weak schemas, gathered here from every rule in the plan.

## Deliverables

- `mgr lint` with `--url` and accepted findings, statement risk from `Op` properties, every index, `numeric`, `secret`-name and `SECURITY DEFINER` rule

## End-to-end check

Each lint rule fires on a fixture that breaks it and stays quiet on one that doesn't, and `--url` reports only tables big enough to matter.

## Out of scope

- Enforcing `lint` in the proof (step 28)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools / Configuration: `morm.toml`

**R-0109** `lint` warns about a `SECURITY DEFINER` function with no `SET search_path` of its own.

### Views and groups / Public input and output

- **R-0257** `lint` warns about columns named like `password`, `token`, `secret` or `api_key` that aren't `secret`, hidden or write-only.

### Indexes

- **R-0312** `morm-admin mgr lint` warns about foreign key columns without an index, and redundant indexes such as a hash and a btree on one column, or an index on `a` when one on `a, b` exists.

### Reading and writing / Decimal

- **R-0417** **Unbounded `numeric`** with `Decimal`: `lint` warns.

### Migrations / Proof before writing

- **R-0578** `lint` passes, or each finding is explicitly accepted.

### Migrations / Commands

- **R-0602** `lint`: statement risk, in the style of `squawk`, plus index advice. With `--url` it reads real table sizes and reports risky operations only on tables big enough to matter.

## Test obligations

None beyond one tagged test per requirement.
