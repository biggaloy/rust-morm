# Step 36: Managed objects: catalog and ops

- **Phase:** Migrations
- **Depends on:** 35, 18
- **Can run beside:** 12, 19, 20, 21, 22, 23, 24, 25, 27, 47
- **Crates:** `morm-migrate`, `morm-macros`, `morm`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Read each object kind from the catalog, diff it, and apply it with an exact inverse.

## Deliverables

- IR, catalog reading, diff and `Op` with inverse for each kind: functions, procedures, triggers, policies, grants, views, materialized views, enums, domains
- Running `.sql` definitions on scratch
- Placeholder roles on scratch
- Read-only view models, `db.call`, `db.refresh`

## End-to-end check

Each object kind goes from a declaration to a real database and back through the diff with no drift, on Postgres 15–18.

## Out of scope

- Dependency recreation (step 37)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Triggers, functions and other objects

- **R-0534** A `.sql` file holds the complete `CREATE` statement, plain Postgres. morm has no SQL parser: it runs the statement on scratch and reads what was created from the catalog.
- **R-0537** **Models over views are read-only.** `#[pg_view]` and `#[pg_view(materialized)]` models have no insert, update or delete, so a write is a compile error. `db.refresh::<T>()` refreshes a materialized view, concurrently when it has a unique index.
- **R-0538** Macros also generate a typed caller where the signature allows, for example `db.call(archive_users, &days)`.
- **R-0539** Roles are cluster-wide, so they aren't created. `morm-admin scratch` makes placeholder roles so policies and grants can run on scratch.

## Test obligations

None beyond one tagged test per requirement.
