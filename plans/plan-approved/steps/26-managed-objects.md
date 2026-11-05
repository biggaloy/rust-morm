# Step 26: Managed objects

- **Phase:** Migrations
- **Depends on:** 23
- **Can run beside:** 24, 25
- **Crates:** `morm-migrate`, `morm-macros`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Manage everything a schema depends on, with exact inverses.

## Deliverables

- `pg_function!`, `pg_trigger!`, `pg_policy!`, `pg_procedure!`, `pg_grant!`, `#[pg_view]`, materialized views, enums, domains, dependency recreation, `db.call`, `db.refresh`
- `morm::pg_outbox!` as a managed table, with tests

## End-to-end check

A column type change under a view, a function and a trigger plans, runs and reverses exactly on Postgres 15–18.

## Out of scope

- Unmanaged objects, which are never changed

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / What's managed

**R-0491** **Everything an application's schema depends on, from v1:** tables, columns (type, collation, nullability, default, identity, generated, comment), constraints (including deferrable and validated state), indexes, enums, domains, composite types, sequences, extensions, **database views, materialized views, functions, procedures, triggers, RLS policies, and grants** on managed objects.

**R-0492** Tables and other objects with no definition in code are never changed, and morm never drops something because it's missing from the models. Other services' tables can share the database. If an unmanaged object depends on a change, that's a decision with evidence.

### Migrations / Triggers, functions and other objects

**R-0493** They live next to the model they belong to. Objects are ordinary Rust items, auto-registered with no central list. Short ones go inline. Long ones go in a real `.sql` file beside the module, so editors highlight them and they can be pasted into `psql`.

**R-0494**

```
src/users/
├── mod.rs                  # models, plus the object declarations
└── sql/
    ├── touch_updated_at.sql
    └── active_users.sql
```

**R-0495**

```rust
// src/users/mod.rs
#[model(table = "users", base = "common")]
pub struct User { /* ... */ }

morm::pg_function!(touch_updated_at, file = "sql/touch_updated_at.sql");
morm::pg_trigger!(users_touch, on = User, "BEFORE UPDATE FOR EACH ROW EXECUTE FUNCTION touch_updated_at()");
morm::pg_policy!(own_rows, on = User, "USING (id = current_setting('app.user')::bigint)");
morm::pg_procedure!(archive_users, file = "sql/archive_users.sql");
morm::pg_grant!(on = User, "SELECT, UPDATE TO app_rw");

#[pg_view(file = "sql/active_users.sql")]          // read-only model over a database view
pub struct ActiveUser { pub id: i64, pub email: String }
```

- **R-0496** `file` paths are relative to the Rust file, like `include_str!`, so editing the SQL triggers a rebuild.
- **R-0497** `on = User` is a real type reference, so a typo fails to compile and table renames carry through.
- **R-0498** A `.sql` file holds the complete `CREATE` statement, plain Postgres. morm has no SQL parser: it runs the statement on scratch and reads what was created from the catalog.
- **R-0499** **Order is automatic.** Kinds apply in a fixed order (types, tables, functions, views, triggers, policies, grants). Within a kind, statements retry until all resolve, then `pg_depend` gives the true order.
- **R-0500** **These definitions never reach the app binary.** They're compiled only into `morm-mgr`.
- **R-0501** **Models over views are read-only.** `#[pg_view]` and `#[pg_view(materialized)]` models have no insert, update or delete, so a write is a compile error. `db.refresh::<T>()` refreshes a materialized view, concurrently when it has a unique index.
- **R-0502** Macros also generate a typed caller where the signature allows, for example `db.call(archive_users, &days)`.
- **R-0503** Roles are cluster-wide, so they aren't created. `morm-admin scratch` makes placeholder roles so policies and grants can run on scratch, and `doctor` checks the real ones exist.

**R-0504** **Dependencies are handled by the planner.** A column type change under a view normally fails. Because morm owns the view, the planner reads `pg_depend`, drops the dependent objects, alters the column, and recreates them in order, with an exact inverse. It uses `CREATE OR REPLACE` when safe, and drop and recreate when a signature or result shape changes. Materialized views rebuild as a planned step with their indexes and a refresh.

## Test obligations

None beyond one tagged test per requirement.
