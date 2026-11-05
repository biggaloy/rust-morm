# Step 34: Rollback and crash recovery

- **Phase:** Migrations
- **Depends on:** 33
- **Can run beside:** 08, 09, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 47
- **Crates:** `morm-migrate`, `morm-admin`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

A defined way forward from any failure.

## Deliverables

- `rollback` with `--failed` and `--orphans`, `--resume`, batch cursors, `repair`, `verify`, `refingerprint`, Postgres version handling

## End-to-end check

Killing the process at every step boundary, `--resume` and `rollback` both reach the right state, and a tampered migration is refused.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

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

- Production guards (step 40)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Production is forward only

- **R-0583** **A failed migration can always be cleaned up.** `migrate --resume` is the first choice. `rollback --failed` undoes the completed steps of a migration that never finished, because no release depends on it yet.

### Migrations / Runtime guarantees

- **R-0600** **Fingerprint gate:** rollback checks the fingerprints in reverse.
- **R-0604** **Postgres versions.** Catalog text can differ between majors, so `pg_major` is recorded. On a mismatch, the expected state is rebuilt by replaying the chain on the target's version in a temporary database. Without `CREATEDB` it refuses unless `--skip-fingerprint` is given. `refingerprint` handles upgrades.
- **R-0608** **Crash recovery:** preconditions make steps safe to re-run. `status` reports the exact step, `migrate --resume` continues, `rollback` undoes only completed steps, failed concurrent indexes are rebuilt, and batches resume from their cursor.
- **R-0609** **Tamper refusal:** edited or missing applied migrations are refused. `repair --reason "..."` re-signs and logs it.
- **R-0610** **LIFO rollback,** plus `rollback --orphans` for another branch's migrations, on development databases.

### Migrations / Commands

- **R-0616** `verify`: live schema vs models and ledger; read-only and safe in production. Unmanaged objects are listed as information, not as drift.
- **R-0949** `rollback [--last N | --to ID | --orphans | --failed] [--plan] [--yes] [--accept-data-loss] [--production --reason R]`. `verify [--json]`. `repair --reason R`. `refingerprint`.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0117 | suite | **Fault injection:** kill the process at every step boundary, then prove `--resume` and `rollback` both reach the right state. |
