# Step 29: Schema IR, catalog and scratch

- **Phase:** Migrations
- **Depends on:** 28, 10
- **Can run beside:** 07, 08, 09, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 47
- **Crates:** `morm-migrate`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Read any schema, from models or a live database, into one typed form.

## Deliverables

- `Schema` IR for everything the plan manages, the catalog reader, scratch database handling, creating models in a temporary database, replaying a chain

## End-to-end check

A model set created in a scratch database reads back into an IR equal to the one built from a replayed chain, on Postgres 15–18.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> **How it fits together:** `morm-admin` holds the engine. When a command needs the models, it builds and runs `morm-mgr` to get the manifest. Commands that don't need models (`migrate`, `rollback`, `status`, `purge`) run from `morm-migrations/` alone, so a production host needs `morm-admin` and no Rust toolchain. The app binary carries no migration code or schema metadata, apart from the short per-model column list `check_schema` reads.
>

## Out of scope

- The diff (step 30)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / What's managed

**R-0528** **Everything an application's schema depends on, from v1:** tables, columns (type, collation, nullability, default, identity, generated, comment), constraints (including deferrable and validated state), indexes, enums, domains, composite types, sequences, extensions, **database views, materialized views, functions, procedures, triggers, RLS policies, and grants** on managed objects.

### Migrations / Source of truth

**R-0543** No snapshots. `makemigrations` uses a scratch Postgres (`MORM_SCRATCH_URL`) and never touches staging or production:

- **R-0544** Replay the migration chain into temporary database A.
- **R-0545** Create the models and objects in temporary database B.
- **R-0546** Read both through the same catalog queries (`pg_get_viewdef`, `pg_get_functiondef`, `pg_get_triggerdef` and so on) into a typed `Schema` IR, so Postgres normalizes everything.

### Risks, open items and later work / Risks

- **R-0918** **Scratch databases:** `makemigrations` needs a Postgres where it can `CREATE DATABASE`.

## Test obligations

None beyond one tagged test per requirement.
