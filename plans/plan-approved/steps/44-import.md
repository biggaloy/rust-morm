# Step 44: `morm-admin import`

- **Phase:** Tools
- **Depends on:** 42
- **Can run beside:** 08, 23, 26, 39, 40, 41, 43, 45, 46, 47, 48, 49, 51, 52, 53, 54, 58, 59, 60
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

**R-1181** **`import` on children and partitions.** `import` writes a child's or a partition's model with every column and only its own constraints and indexes, and what the table holds from its parent only in the parent's model, under step 29's rule. A partition that holds its parent's key under `<partition>_pkey` gets `pk` on those columns, in their order, under step 29's rule for a partition's key. One that holds it under another name, such as `p3_other` or `p5_pkey1`, gets a model with no `pk`, which matches it too, and `import`'s output says why, naming the partition, the key under its name and the parent's key, and giving `ALTER TABLE <partition> RENAME CONSTRAINT <name> TO <partition>_pkey`, after renaming the relation that already has that name, which it names, as what would let its model declare `pk`. A partition's model never gets `identity`, which its column holds from the parent's, under step 29's rule for a partition's identity. That column's field is written `db_set`, so inserts through the partition's model leave it out and take the parent's next value, which step 41's check at pool start requires.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0267 | integration | **`import` on a partition's key and identity, on Postgres 15–18:** on a partitioned table with a key on an identity column, a partition made by `PARTITION OF`, one whose own key `ATTACH PARTITION` took over as `p3_other`, one holding it as `p5_pkey1` beside a table named `p5_pkey`, and a partition of a partition, `import` writes `pk` on the parent's key columns, in their order, in the models of the first and of the partition of a partition, and no `pk` in the other two, with a line in its output for each of those naming the partition, the key under its name and the parent's key, and giving the rename, with the table `p5_pkey` to rename first. No partition's model has `identity`, and each marks the identity column `db_set`. The imported models round-trip to no diff, a binary built from them passes `check_schema` when its pool starts and inserts through each partition's model with the parent's next key, and after the renames a second `import` writes `pk` on every partition. |
