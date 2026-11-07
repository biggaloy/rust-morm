# Step 44: `morm-admin import`

- **Phase:** Tools
- **Depends on:** 42
- **Can run beside:** 08, 23, 26, 39, 40, 41, 43, 45, 46, 47, 48, 49, 51, 52, 53, 54, 58, 59
- **Crates:** `morm-admin`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Generate models from any existing database, gathered here from every import rule in the plan.

## Deliverables

- `import` with exact types, names, `keep_case`, `secret`, `Decimal` choices, and a list of what models can't express

## End-to-end check

`import` on the fixture `tests/fixtures/import.sql` round-trips to no diff. Write that fixture to hold mixed-case names, composite keys, every index kind, deferrable constraints, views, triggers and an exclusion constraint.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> - `lint` warns about columns named like `password`, `token`, `secret` or `api_key` that aren't `secret`, hidden or write-only.
>

## Out of scope

- Changing the imported database

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools / Configuration: `morm.toml`

**R-0118** **`morm-admin import`** is a simple starting point for any existing database. It reads the catalog only, never rows, and writes model files that match the schema exactly: types, nullability, defaults, indexes, constraints and their names. Nothing is "improved". Anything models can't express, such as exclusion constraints or partitioned tables, is listed in its output and stays unmanaged. `morm-admin mgr baseline` then records that state as migration zero. Views, hooks and validation are added by hand afterwards.

### Models / Table and column names

- **R-0126** `import` adds it automatically.

### Views and groups / Public input and output

- **R-0264** `import` adds `secret` to them.

### Indexes / Names

- **R-0331** `morm-admin import` keeps existing names, as explicit `name` values where they differ.

### Reading and writing / Decimal

- **R-0428** `import` writes `Decimal` for bounded columns that fit, and `Decimal` with an explicit `sql = "numeric"` for unbounded ones.

### Managed enums

- **R-1001** `import` writes bare `#[derive(PgEnum)]`, `#[derive(PgDomain)]` and `#[derive(PgComposite)]`, and never `#[pg_enum(name = "...")]`, `#[pg_domain]` or `#[pg_composite]`, so nothing becomes managed by accident. It adds `#[pg_type(name = "...")]` and `#[pg_enum(value = "...")]` where snake_case wouldn't give the existing name. A `text` column with a CHECK stays `String` with `checks(...)`, and `import` never writes `TextEnum`.

## Test obligations

None beyond one tagged test per requirement.
