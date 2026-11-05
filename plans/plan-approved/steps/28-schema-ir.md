# Step 28: Schema IR, catalog and scratch

- **Phase:** Migrations
- **Depends on:** 27, 10
- **Can run beside:** 05, 07, 08, 09, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 45
- **Crates:** `morm-migrate`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

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

- The diff (step 29)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / What's managed

**R-0524** **Everything an application's schema depends on, from v1:** tables, columns (type, collation, nullability, default, identity, generated, comment), constraints (including deferrable and validated state), indexes, enums, domains, composite types, sequences, extensions, **database views, materialized views, functions, procedures, triggers, RLS policies, and grants** on managed objects.

**R-0525** Tables and other objects with no definition in code are never changed, and morm never drops something because it's missing from the models. Other services' tables can share the database. If an unmanaged object depends on a change, that's a decision with evidence.

### Migrations / Source of truth

**R-0539** No snapshots. `makemigrations` uses a scratch Postgres (`MORM_SCRATCH_URL`) and never touches staging or production:

- **R-0540** Replay the migration chain into temporary database A.
- **R-0541** Create the models and objects in temporary database B.
- **R-0542** Read both through the same catalog queries (`pg_get_viewdef`, `pg_get_functiondef`, `pg_get_triggerdef` and so on) into a typed `Schema` IR, so Postgres normalizes everything.

### Risks, open items and later work / Risks

- **R-0912** **Scratch databases:** `makemigrations` needs a Postgres where it can `CREATE DATABASE`.

## Test obligations

None beyond one tagged test per requirement.
