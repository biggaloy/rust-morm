# Step 32: Proof pipeline, files and decisions

- **Phase:** Migrations
- **Depends on:** 31
- **Can run beside:** 08, 09, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 47
- **Crates:** `morm-migrate`, `morm-admin`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Write a migration only after it's proven, with decisions as explicit data.

## Deliverables

- `makemigrations`, the five-part proof, decisions, `up.sql`, `down.sql`, `meta.json`, deterministic output, exit codes, `min_postgres` and extension checks

## End-to-end check

`makemigrations` on a changed model writes byte-identical files on repeat runs, and refuses to write when any proof part fails.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> **How it fits together:** `morm-admin` holds the engine. When a command needs the models, it builds and runs `morm-mgr` to get the manifest. Commands that don't need models (`migrate`, `rollback`, `status`, `purge`) run from `morm-migrations/` alone, so a production host needs `morm-admin` and no Rust toolchain. The app binary carries no migration code or schema metadata, apart from the short per-model column list `check_schema` reads.
>
> ```
> makemigrations [--plan] [--json] [-n name] [--decide ID=OPT] [--decisions F] [--empty] [--fast]
> migrate [--plan] [--json] [--yes] [--to ID] [--resume] [--phase expand|contract]
> rollback [--last N | --to ID | --orphans | --failed] [--plan] [--yes] [--accept-data-loss] [--production --reason R]
> revert ID [--plan] [--decide ID=OPT]    mode [development --confirm NAME | production]    squash --to ID
> status [--json]    verify [--json]    check    lint    check-sql
> rehearse --url URL    purge --older-than D | --now ID    baseline    rebase-chain
> repair --reason R     refingerprint           sql ID
> ```
>

## Out of scope

- Running migrations (step 33)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Models / Generated per model

**R-0225** Postgres checks `filter` and `expr` on scratch, because a macro can't parse SQL.

### Migrations / Source of truth

**R-0549** **Scratch must match production.** `makemigrations` proves nothing if scratch differs, `makemigrations` refuses when a model's `extensions` aren't available on scratch.

### Migrations / Decisions

**R-0587** Every rename, retire, purge, lossy change and missing backfill is a decision with an id, evidence (type match, name similarity, row presence, dependents, size) and options. They're answered interactively, by `--decide ID=OPT`, or by file. `--json` suits CI and agents. `was` is a pre-answered rename, ignored once the old name is gone. An op can declare decisions of its own through this same flow, so later steps add kinds such as enum value changes.

**R-0588** Exit codes: `0` ok, `1` error or drift, `2` refused by a guard, `3` open decisions.

### Migrations / Proof before writing

**R-0589** Files are written only after all of these pass on scratch:

- **R-0590** `up` reaches the models exactly.
- **R-0591** `down` returns to the previous schema exactly.
- **R-0592** `up` again gives the same fingerprint.
- **R-0593** **With data:** synthetic rows per column type (NULLs, boundaries, empties) go in first, and preserved-column checksums must match after up → down. Rows are also inserted and updated after `up`, so a broken trigger fails here and not in production. Rows come from a table keyed by column type, so a later step can add a type, and an op can say which values must or must not appear. A `TextEnum` column's rows use its labels. An op can add checks of its own at this point, on this database, and a failure stops `makemigrations` with exit code `2`, or with code `3` when what it found is an open decision.
- **R-0594** `lint` passes, or each finding is explicitly accepted.

### Migrations / Files and ledger

**R-0595**

```
morm-migrations/
└── 0007_add_products/
    ├── up.sql      # steps split by: -- morm:step tx | no-tx | batch size=5000
    ├── down.sql
    └── meta.json   # format, id, parent, phase, pg_major, schema_before, schema_after,
                    # checksum, decisions, data_effect
```

**R-0596** `format` is a version number, so future tools can read old files. Generated files are byte-identical for the same models: tables, columns and operations are sorted explicitly, and macro registration order is never used. `id` is permanent and `parent` forms a chain, so merged branches are detected (`diverged`) and fixed with `rebase-chain`. The directory number is cosmetic. Ledger, in its own `_morm` schema:

### Migrations / Runtime guarantees

- **R-0602** **Postgres features.** `[migrations] min_postgres` (default 15) sets the oldest server the project targets. `makemigrations` refuses a model feature that server can't run, such as `uuidv7()` before 18.

### Migrations / Commands

- **R-0949** `makemigrations [--plan] [--json] [-n name] [--decide ID=OPT] [--decisions F] [--empty] [--fast]`, with `--empty` writing an empty migration for hand-written steps, and `rebase-chain` fixing a chain marked `diverged`.

### Managed enums

- **R-0989** **Op data in `meta.json`.** An op can store its own data in `meta.json`, under `op_data`, covered by the checksum, so what `makemigrations` learned on scratch reaches `migrate`. Step 53 stores the dependents it expects there.

## Test obligations

None beyond one tagged test per requirement.
