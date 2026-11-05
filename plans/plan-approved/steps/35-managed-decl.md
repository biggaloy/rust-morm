# Step 35: Managed objects: declarations

- **Phase:** Migrations
- **Depends on:** 34
- **Can run beside:** 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 27, 47
- **Crates:** `morm-macros`, `morm-core`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Declare every managed object in Rust, next to its model.

## Deliverables

- `pg_function!`, `pg_trigger!`, `pg_policy!`, `pg_procedure!`, `pg_grant!`, `#[pg_view]` with `materialized`, `file` paths, `on = Model` references, each object in the manifest

## End-to-end check

Every macro form appears correctly in the manifest, a bad `on` reference fails to compile, and none of it reaches the app binary.

## Out of scope

- Applying objects to a database (step 36)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Triggers, functions and other objects

**R-0529** They live next to the model they belong to. Objects are ordinary Rust items, auto-registered with no central list. Short ones go inline. Long ones go in a real `.sql` file beside the module, so editors highlight them and they can be pasted into `psql`.

**R-0530**

```
src/users/
├── mod.rs                  # models, plus the object declarations
└── sql/
    ├── touch_updated_at.sql
    └── active_users.sql
```

**R-0531**

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

- **R-0532** `file` paths are relative to the Rust file, like `include_str!`, so editing the SQL triggers a rebuild.
- **R-0533** `on = User` is a real type reference, so a typo fails to compile and table renames carry through.
- **R-0536** **These definitions never reach the app binary.** They're compiled only into `morm-mgr`.

## Test obligations

None beyond one tagged test per requirement.
