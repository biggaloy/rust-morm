# Step 23: Migration execution and recovery

- **Phase:** Migrations
- **Depends on:** 22
- **Can run beside:** nothing
- **Crates:** `morm-migrate`, `morm-admin`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Run migrations safely, with a defined way forward from any failure.

## Deliverables

- `_morm` ledger, `migrate`, `rollback`, `status`, `verify`, `check`, `repair`, `refingerprint`, `sql`, fingerprint gates, advisory lock, preflight, crash recovery

## End-to-end check

Killing the process at every step boundary, `--resume` and `rollback` both reach the right state, and two rival migrators can't both run.

## Out of scope

- Safe forms (step 24)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Runtime guarantees

- **R-0560** **Fingerprint gate.** Fingerprints cover only objects the chain manages, so unmanaged objects can't fail a run. Live schema must match `schema_before`, or the run refuses. A single-transaction migration must match `schema_after` before commit, or it rolls back. A multi-step migration can't, because its steps commit separately, so the check runs after the last step, and a mismatch marks it `failed` for `rollback --failed`. Rollback checks the reverse.
- **R-0561** **Postgres features.** `[migrations] min_postgres` (default 15) sets the oldest server the project targets. `makemigrations` refuses a model feature that server can't run, such as `uuidv7()` before 18, and `morm` refuses to connect to a server below 15. `doctor` reports the server version.
- **R-0562** **Postgres versions.** Catalog text can differ between majors, so `pg_major` is recorded. On a mismatch, the expected state is rebuilt by replaying the chain on the target's version in a temporary database. Without `CREATEDB` it refuses unless `--skip-fingerprint` is given. `refingerprint` handles upgrades.
- **R-0563** **One migrator:** session advisory lock on a dedicated connection. If that connection is lost, the run stops.
- **R-0564** **Lock safety:** `lock_timeout` on every step with bounded retry and backoff, plus per-step `statement_timeout`.
- **R-0565** **Preflight:** privileges, long-running transactions, invalid indexes, checksums, table sizes.
- **R-0566** **Crash recovery:** preconditions make steps safe to re-run. `status` reports the exact step, `migrate --resume` continues, `rollback` undoes only completed steps, failed concurrent indexes are rebuilt, and batches resume from their cursor.
- **R-0567** **Tamper refusal:** edited or missing applied migrations are refused. `repair --reason "..."` re-signs and logs it.
- **R-0568** **LIFO rollback,** plus `rollback --orphans` for another branch's migrations, on development databases.

### Migrations / Commands

**R-0571** All are `morm-admin mgr <command>`:

**R-0572**

```
makemigrations [--plan] [--json] [-n name] [--decide ID=OPT] [--decisions F] [--empty] [--fast]
migrate [--plan] [--json] [--yes] [--to ID] [--resume] [--phase expand|contract]
rollback [--last N | --to ID | --orphans | --failed] [--plan] [--yes] [--accept-data-loss] [--production --reason R]
revert ID [--plan] [--decide ID=OPT]    mode [development --confirm NAME | production]    squash --to ID
status [--json]    verify [--json]    check    lint    check-sql
rehearse --url URL    purge --older-than D | --now ID    baseline    rebase-chain
repair --reason R     refingerprint           sql ID
```

- **R-0573** `check`: offline CI check of chain, checksums, models.
- **R-0574** `verify`: live schema vs models and ledger; read-only and safe in production. Unmanaged objects are listed as information, not as drift.

**R-0580** **Run migrations as a deploy step,** before new code goes live, so a failed migration stops the deploy. The app binary has no migration code, so it can't migrate on start.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0112 | suite | **Fault injection:** kill the process at every step boundary, then prove `--resume` and `rollback` both reach the right state. |
| T-0115 | suite | **Concurrency:** rival migrators, migration under live traffic, lock-timeout retry. |
| T-0118 | suite | **Multi-step:** a fingerprint mismatch after the last step marks the migration `failed`. |
