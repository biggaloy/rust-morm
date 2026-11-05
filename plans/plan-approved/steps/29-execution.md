# Step 29: Migration execution and recovery

- **Phase:** Migrations
- **Depends on:** 28
- **Can run beside:** 06, 07, 08, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 40
- **Crates:** `morm-migrate`, `morm-admin`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Run migrations safely, with a defined way forward from any failure.

## Deliverables

- `_morm` ledger, `migrate`, `rollback` with `--failed`, `status`, `verify`, `check`, `repair`, `refingerprint`, `sql`, fingerprint gates, advisory lock, preflight, crash recovery

## End-to-end check

Killing the process at every step boundary, `--resume` and `rollback` both reach the right state, and two rival migrators can't both run.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> **How it fits together:** `morm-admin` holds the engine. When a command needs the models, it builds and runs `morm-mgr` to get the manifest. Commands that don't need models (`migrate`, `rollback`, `status`, `purge`) run from `morm-migrations/` alone, so a production host needs `morm-admin` and no Rust toolchain. The app binary carries no migration code or schema metadata, apart from the short per-model column list `check_schema` reads.
>

## Out of scope

- Safe forms (step 33)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Production is forward only

- **R-0567** **A failed migration can always be cleaned up.** `migrate --resume` is the first choice. `rollback --failed` undoes the completed steps of a migration that never finished, because no release depends on it yet.

### Migrations / Files and ledger

- **R-0581** `_morm.migrations`: what's applied, with exact up and down SQL, so rollback works when files are gone.
- **R-0582** `_morm.steps`: per-step state (`pending/running/done/failed`) and batch cursors.
- **R-0583** `_morm.history`: append-only log of every action, with user, host, duration and tool version.

### Migrations / Runtime guarantees

- **R-0584** **Fingerprint gate.** Fingerprints cover only objects the chain manages, so unmanaged objects can't fail a run. Live schema must match `schema_before`, or the run refuses. A single-transaction migration must match `schema_after` before commit, or it rolls back. A multi-step migration can't, because its steps commit separately, so the check runs after the last step, and a mismatch marks it `failed` for `rollback --failed`. Rollback checks the reverse.
- **R-0588** **Postgres versions.** Catalog text can differ between majors, so `pg_major` is recorded. On a mismatch, the expected state is rebuilt by replaying the chain on the target's version in a temporary database. Without `CREATEDB` it refuses unless `--skip-fingerprint` is given. `refingerprint` handles upgrades.
- **R-0589** **One migrator:** session advisory lock on a dedicated connection. If that connection is lost, the run stops.
- **R-0590** **Lock safety:** `lock_timeout` on every step with bounded retry and backoff, plus per-step `statement_timeout`.
- **R-0591** **Preflight:** privileges, long-running transactions, invalid indexes, checksums, table sizes.
- **R-0592** **Crash recovery:** preconditions make steps safe to re-run. `status` reports the exact step, `migrate --resume` continues, `rollback` undoes only completed steps, failed concurrent indexes are rebuilt, and batches resume from their cursor.
- **R-0593** **Tamper refusal:** edited or missing applied migrations are refused. `repair --reason "..."` re-signs and logs it.
- **R-0594** **LIFO rollback,** plus `rollback --orphans` for another branch's migrations, on development databases.

### Migrations / Commands

**R-0597** All are `morm-admin mgr <command>`:

**R-0598**

```
makemigrations [--plan] [--json] [-n name] [--decide ID=OPT] [--decisions F] [--empty] [--fast]
migrate [--plan] [--json] [--yes] [--to ID] [--resume] [--phase expand|contract]
rollback [--last N | --to ID | --orphans | --failed] [--plan] [--yes] [--accept-data-loss] [--production --reason R]
revert ID [--plan] [--decide ID=OPT]    mode [development --confirm NAME | production]    squash --to ID
status [--json]    verify [--json]    check    lint    check-sql
rehearse --url URL    purge --older-than D | --now ID    baseline    rebase-chain
repair --reason R     refingerprint           sql ID
```

- **R-0599** `check`: offline CI check of chain, checksums, models.
- **R-0600** `verify`: live schema vs models and ledger; read-only and safe in production. Unmanaged objects are listed as information, not as drift.

**R-0606** **Run migrations as a deploy step,** before new code goes live, so a failed migration stops the deploy. The app binary has no migration code, so it can't migrate on start.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0117 | suite | **Fault injection:** kill the process at every step boundary, then prove `--resume` and `rollback` both reach the right state. |
| T-0121 | suite | **Concurrency:** rival migrators, migration under live traffic, lock-timeout retry. |
| T-0124 | suite | **Multi-step:** a fingerprint mismatch after the last step marks the migration `failed`. |
