# Step 33: Migration execution and recovery

- **Phase:** Migrations
- **Depends on:** 32
- **Can run beside:** 08, 09, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 47
- **Crates:** `morm-migrate`, `morm-admin`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Run migrations safely, with a defined way forward from any failure.

## Deliverables

- `_morm` ledger, `migrate`, `status`, `check`, `sql`, fingerprint gates, advisory lock, lock safety, preflight

## End-to-end check

A chain of migrations runs on a real database, each gate refuses when its condition is forced, and two rival migrators can't both run.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> **How it fits together:** `morm-admin` holds the engine. When a command needs the models, it builds and runs `morm-mgr` to get the manifest. Commands that don't need models (`migrate`, `rollback`, `status`, `purge`) run from `morm-migrations/` alone, so a production host needs `morm-admin` and no Rust toolchain. The app binary carries no migration code or schema metadata, apart from the short per-model column list `check_schema` reads.
>

## Out of scope

- Recovery (step 34)
- Safe forms (step 39)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Files and ledger

- **R-0596** `_morm.migrations`: what's applied, with exact up and down SQL, so rollback works when files are gone.
- **R-0597** `_morm.steps`: per-step state (`pending/running/done/failed`) and batch cursors.
- **R-0598** `_morm.history`: append-only log of every action, with user, host, duration and tool version.

### Migrations / Runtime guarantees

- **R-0599** **Fingerprint gate.** Fingerprints cover only objects the chain manages, so unmanaged objects can't fail a run. Live schema must match `schema_before`, or the run refuses. A single-transaction migration must match `schema_after` before commit, or it rolls back. A multi-step migration can't, because its steps commit separately, so the check runs after the last step, and a mismatch marks it `failed` for `rollback --failed`.
- **R-0605** **One migrator:** session advisory lock on a dedicated connection. If that connection is lost, the run stops.
- **R-0606** **Lock safety:** `lock_timeout` on every step with bounded retry and backoff, plus per-step `statement_timeout`.
- **R-0607** **Preflight:** privileges, long-running transactions, invalid indexes, checksums, table sizes.

### Migrations / Commands

**R-0613** All are `morm-admin mgr <command>`:

**R-0614**

```
makemigrations [--plan] [--json] [-n name] [--decide ID=OPT] [--decisions F] [--empty] [--fast]
migrate [--plan] [--json] [--yes] [--to ID] [--resume] [--phase expand|contract]
rollback [--last N | --to ID | --orphans | --failed] [--plan] [--yes] [--accept-data-loss] [--production --reason R]
revert ID [--plan] [--decide ID=OPT]    mode [development --confirm NAME | production]    squash --to ID
status [--json]    verify [--json]    check    lint    check-sql
rehearse --url URL    purge --older-than D | --now ID    baseline    rebase-chain
repair --reason R     refingerprint           sql ID
```

- **R-0615** `check`: offline CI check of chain, checksums, models.

**R-0622** **Run migrations as a deploy step,** before new code goes live, so a failed migration stops the deploy. The app binary has no migration code, so it can't migrate on start.

- **R-0948** `migrate [--plan] [--json] [--yes] [--to ID] [--resume] [--phase expand|contract]`, where `--yes` skips the prompt and `--to ID` stops after that migration. `status [--json]`. `check`. `sql ID` prints a migration's SQL.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0121 | suite | **Concurrency:** rival migrators, migration under live traffic, lock-timeout retry. |
| T-0124 | suite | **Multi-step:** a fingerprint mismatch after the last step marks the migration `failed`. |
