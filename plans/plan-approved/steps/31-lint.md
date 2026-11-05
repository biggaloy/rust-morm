# Step 31: `lint`

- **Phase:** Migrations
- **Depends on:** 30, 07, 13
- **Can run beside:** 08, 09, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 47
- **Crates:** `morm-migrate`, `morm-admin`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Report risky statements and weak schemas, gathered here from every rule in the plan.

## Deliverables

- The `morm-admin mgr` dispatcher, with one module per command
- `mgr lint` with `--url` and accepted findings, statement risk from `Op` properties, every index, `numeric`, `secret`-name and `SECURITY DEFINER` rule

## End-to-end check

Each lint rule fires on a fixture that breaks it and stays quiet on one that doesn't, and `--url` reports only tables big enough to matter.

## Out of scope

- Enforcing `lint` in the proof (step 32)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools / Configuration: `morm.toml`

**R-0110** `lint` warns about a `SECURITY DEFINER` function with no `SET search_path` of its own.

### Views and groups / Public input and output

- **R-0263** `lint` warns about columns named like `password`, `token`, `secret` or `api_key` that aren't `secret`, hidden or write-only.

### Indexes

- **R-0318** `morm-admin mgr lint` warns about foreign key columns without an index, and redundant indexes such as a hash and a btree on one column, or an index on `a` when one on `a, b` exists.

### Reading and writing / Decimal

- **R-0427** **Unbounded `numeric`** with `Decimal`: `lint` warns.

### Migrations / Commands

- **R-0619** `lint`: statement risk, in the style of `squawk`, plus index advice. With `--url` it reads real table sizes and reports risky operations only on tables big enough to matter.
- **R-0955** `morm-admin mgr <command>` runs migration and schema work, and each `mgr` command lives in its own module.

## Test obligations

None beyond one tagged test per requirement.
