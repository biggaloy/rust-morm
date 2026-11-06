# Step 41: Rolling deploys

- **Phase:** Migrations
- **Depends on:** 40, 15
- **Can run beside:** 19, 20, 21, 22, 23, 24, 25, 27, 43, 44, 46, 47, 48
- **Crates:** `morm-migrate`, `morm-admin`, `morm`, `morm-macros`, `morm-core`
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

### Managed enums

- **R-0997** **Enums in `check_schema`.** The per-model list also holds each enum's labels. Every labelled variant's label must exist, or its `was` label, and a `deprecated` variant may be missing. Extra labels in the database are fine, because a binary only fails on a row that holds one, and `Error::UnknownEnumValue` names it.
- **R-0998** **Enum phases.** Adds, renames and the online swap's steps before its cutover are `expand`. A cutover that removes a label is `contract`, because binaries that still write it would fail. A cutover that only reorders stays in `expand`. In one migration that adds and removes, the add goes in `expand`, so a value and its replacement exist together through the rollout. A `--fast` swap takes the phase its cutover would.
- **R-0999** **Removing a value in a rolling deploy.** `#[pg_enum(deprecated)]` on a variant says that value is on its way out. `contract` refuses a swap that removes a label until a release with that variant marked `deprecated` has shipped, the same gate `#[col(deprecated)]` uses.
- **R-1000** **Renaming a value in a rolling deploy.** A rename runs instantly by default, so old binaries fail on that value during the rollout: reads of rows holding it return `Error::UnknownEnumValue`, and writes of the old label return `22P02`. The rename decision also offers expand/contract: add the new label in `expand`, copy rows across in batches, and remove the old label by swap in `contract`. With `was`, a new binary reads both labels through either path. `was` answers only whether it's a rename. The path stays a decision with the instant rename as its default.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0103 | integration | `check_schema` during expand phases and after a dropped default |
| T-0158 | integration | **Enums in a rolling deploy:** `check_schema` passes during an add and reports a missing label, an old binary reads rows on a plain add and gets `Error::UnknownEnumValue` only on rows holding the new value, `contract` refuses a removal until the `deprecated` variant has shipped, an add and a remove in one migration split across phases, and the expand/contract rename ends with the right labels and rows. |
