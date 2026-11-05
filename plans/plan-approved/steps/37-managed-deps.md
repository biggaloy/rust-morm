# Step 37: Managed objects: dependencies

- **Phase:** Migrations
- **Depends on:** 36
- **Can run beside:** 08, 09, 12, 19, 20, 21, 22, 23, 24, 25, 26, 27, 47
- **Crates:** `morm-migrate`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Change columns under objects that depend on them, with exact inverses.

## Deliverables

- Fixed kind order, resolution through `pg_depend`, `CREATE OR REPLACE` or drop and recreate, materialized view rebuilds

## End-to-end check

A column type change under a view, a function and a trigger plans, runs and reverses exactly on Postgres 15–18.

## Out of scope

- Unmanaged objects, which are never changed

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Triggers, functions and other objects

- **R-0535** **Order is automatic.** Kinds apply in a fixed order (types, tables, functions, views, triggers, policies, grants). Within a kind, statements retry until all resolve, then `pg_depend` gives the true order.

**R-0541** **Dependencies are handled by the planner.** A column type change under a view normally fails. Because morm owns the view, the planner reads `pg_depend`, drops the dependent objects, alters the column, and recreates them in order, with an exact inverse. It uses `CREATE OR REPLACE` when safe, and drop and recreate when a signature or result shape changes. Materialized views rebuild as a planned step with their indexes and a refresh.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0120 | suite | every dependency recreate path |
