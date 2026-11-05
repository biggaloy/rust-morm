# Step 22: Proof pipeline, files and decisions

- **Phase:** Migrations
- **Depends on:** 21
- **Can run beside:** nothing
- **Crates:** `morm-migrate`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Write a migration only after it's proven, with decisions as explicit data.

## Deliverables

- `makemigrations`, the five-part proof, decisions, `up.sql`, `down.sql`, `meta.json`, deterministic output, exit codes

## End-to-end check

`makemigrations` on a changed model writes byte-identical files on repeat runs, and refuses to write when any proof part fails.

## Out of scope

- Running migrations (step 23)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Decisions

**R-0547** Every rename, retire, purge, lossy change and missing backfill is a decision with an id, evidence (type match, name similarity, row presence, dependents, size) and options. They're answered interactively, by `--decide ID=OPT`, or by file. `--json` suits CI and agents. `was` is a pre-answered rename, ignored once the old name is gone.

**R-0548** Exit codes: `0` ok, `1` error or drift, `2` refused by a guard, `3` open decisions.

### Migrations / Proof before writing

**R-0549** Files are written only after all of these pass on scratch:

- **R-0550** `up` reaches the models exactly.
- **R-0551** `down` returns to the previous schema exactly.
- **R-0552** `up` again gives the same fingerprint.
- **R-0553** **With data:** synthetic rows per column type (NULLs, boundaries, empties) go in first, and preserved-column checksums must match after up → down. Rows are also inserted and updated after `up`, so a broken trigger fails here and not in production.
- **R-0554** `lint` passes, or each finding is explicitly accepted.

### Migrations / Files and ledger

**R-0555**

```
morm-migrations/
└── 0007_add_products/
    ├── up.sql      # steps split by: -- morm:step tx | no-tx | batch size=5000
    ├── down.sql
    └── meta.json   # format, id, parent, phase, pg_major, schema_before, schema_after,
                    # checksum, decisions, data_effect
```

**R-0556** `format` is a version number, so future tools can read old files. Generated files are byte-identical for the same models: tables, columns and operations are sorted explicitly, and macro registration order is never used. `id` is permanent and `parent` forms a chain, so merged branches are detected (`diverged`) and fixed with `rebase-chain`. The directory number is cosmetic. Ledger, in its own `_morm` schema:

- **R-0557** `_morm.migrations`: what's applied, with exact up and down SQL, so rollback works when files are gone.
- **R-0558** `_morm.steps`: per-step state (`pending/running/done/failed`) and batch cursors.
- **R-0559** `_morm.history`: append-only log of every action, with user, host, duration and tool version.

## Test obligations

None beyond one tagged test per requirement.
