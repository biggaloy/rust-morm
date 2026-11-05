# Step 41: Rolling deploys

- **Phase:** Migrations
- **Depends on:** 40, 15
- **Can run beside:** 12, 19, 20, 21, 22, 23, 24, 25, 27, 43, 44, 46, 47, 48
- **Crates:** `morm-migrate`, `morm-admin`, `morm`, `morm-macros`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Keep old and new binaries working through a deploy.

## Deliverables

- `phase`, expand and contract, rename sync triggers, `deprecated` steps, `db.check_schema()` with its per-model column list, data migration hooks run through `morm-mgr` with `--hooks`

## End-to-end check

An old binary boots and works during an expand phase, a migration with a Rust data hook runs through `--hooks ./morm-mgr`, and `check_schema` reports a dropped default.

## Out of scope

- New features

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools

- **R-0044** `morm-mgr` also runs a migration's Rust data hooks, and nothing else. Build it in CI and ship the binary with the migrations. `migrate --hooks ./morm-mgr` and `rollback --hooks ./morm-mgr` call it as `morm-mgr hook ID up` or `down`, and a migration with hooks refuses to run without it, or if that binary doesn't contain the hook, because `morm-admin` holds no project code. A hook connects with the migration role and runs as a `no-tx` step, so it must be safe to run again on `--resume`.

### Migrations / Rolling deploys

**R-0612** Static column lists mean a dropped column breaks the old binary mid-deploy. Migrations carry `phase: expand | contract`, and `--phase` splits them. `contract` refuses while a required `#[col(deprecated)]` hasn't shipped. Marking a `NOT NULL` column with no default as `deprecated` generates an expand step dropping that `NOT NULL`, because the new binary stops writing it. A rename decision offers an expand/contract form for live tables: add the new column, keep both in sync with a generated trigger, backfill in batches, and drop the old column and trigger in `contract`. `db.check_schema()` checks compatibility at startup, not equality, so an old binary restarting during an expand phase still boots. Every column the binary reads or writes must exist with the same type, or one the conversion layer accepts. Extra columns and objects are fine. Two cases that would fail that binary's inserts are reported: an extra `NOT NULL` column with no default, and a column the binary may leave out that is no longer nullable and has lost its default.

### Migrations / Data migrations

**R-0613** SQL steps first, including `batch` steps, covered by checksums. Custom Rust data hooks are registered by migration id and use raw SQL only, since model types change. A hook needs a matching down hook, or the migration is declared irreversible.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0103 | integration | `check_schema` during expand phases and after a dropped default |
