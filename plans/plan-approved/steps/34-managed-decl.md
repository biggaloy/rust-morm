# Step 34: Managed objects: declarations

- **Phase:** Migrations
- **Depends on:** 33, 17
- **Can run beside:** 12, 18, 19, 20, 21, 22, 23, 24, 26, 45
- **Crates:** `morm-migrate`, `morm-macros`, `morm`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Declare everything a schema depends on, and read it back through the catalog.

## Deliverables

- `pg_function!`, `pg_trigger!`, `pg_policy!`, `pg_procedure!`, `pg_grant!`, `#[pg_view]`, materialized views, enums, domains, read-only view models, `db.call`, `db.refresh`, placeholder roles on scratch

## End-to-end check

Each object kind goes from a declaration to a real database and back through the diff with no drift, on Postgres 15–18.

## Out of scope

- Dependency recreation (step 35)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Triggers, functions and other objects

**R-0526** They live next to the model they belong to. Objects are ordinary Rust items, auto-registered with no central list. Short ones go inline. Long ones go in a real `.sql` file beside the module, so editors highlight them and they can be pasted into `psql`.

**R-0527**

```
src/users/
├── mod.rs                  # models, plus the object declarations
└── sql/
    ├── touch_updated_at.sql
    └── active_users.sql
```

**R-0528**

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

- **R-0529** `file` paths are relative to the Rust file, like `include_str!`, so editing the SQL triggers a rebuild.
- **R-0530** `on = User` is a real type reference, so a typo fails to compile and table renames carry through.
- **R-0531** A `.sql` file holds the complete `CREATE` statement, plain Postgres. morm has no SQL parser: it runs the statement on scratch and reads what was created from the catalog.
- **R-0533** **These definitions never reach the app binary.** They're compiled only into `morm-mgr`.
- **R-0534** **Models over views are read-only.** `#[pg_view]` and `#[pg_view(materialized)]` models have no insert, update or delete, so a write is a compile error. `db.refresh::<T>()` refreshes a materialized view, concurrently when it has a unique index.
- **R-0535** Macros also generate a typed caller where the signature allows, for example `db.call(archive_users, &days)`.
- **R-0536** Roles are cluster-wide, so they aren't created. `morm-admin scratch` makes placeholder roles so policies and grants can run on scratch.

## Test obligations

None beyond one tagged test per requirement.
