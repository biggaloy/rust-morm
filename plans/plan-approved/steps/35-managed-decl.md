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

- `pg_function!`, `pg_trigger!`, `pg_policy!`, `pg_procedure!`, `pg_grant!`, `pg_sequence!`, `#[pg_view]` with `materialized`, `#[pg_domain]`, `#[pg_composite]`, `#[pg_enum]` with its variant attributes, `file` paths, one or more `proof` lines on functions and procedures, `on = Model` references, each object in the manifest

## End-to-end check

Every macro form appears correctly in the manifest, a bad `on` reference fails to compile, and none of it reaches the app binary.

## Out of scope

- Applying objects to a database (steps 36 and 57)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Triggers, functions and other objects

**R-0530** They live next to the model they belong to. Objects are ordinary Rust items, auto-registered with no central list. Short ones go inline. Long ones go in a real `.sql` file beside the module, so editors highlight them and they can be pasted into `psql`.

**R-0531**

```
src/users/
├── mod.rs                  # models, plus the object declarations
└── sql/
    ├── touch_updated_at.sql
    └── active_users.sql
```

**R-0532**

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

- **R-0533** `file` paths are relative to the Rust file, like `include_str!`, so editing the SQL triggers a rebuild.
- **R-0534** `on = User` is a real type reference, so a typo fails to compile and table renames carry through.
- **R-0537** **These definitions never reach the app binary.** They're compiled only into `morm-mgr`.
- **R-1065** **`proof`.** `pg_function!` and `pg_procedure!` take an optional `proof`, one SQL statement that step 59 runs to prove the object still works, and it can repeat, because a call proves only the path it takes: `morm::pg_function!(totals, file = "sql/totals.sql", proof = "SELECT * FROM totals(1)", proof = "SELECT * FROM totals(-1)")`, or `morm::pg_procedure!(archive_users, file = "sql/archive_users.sql", proof = "CALL archive_users(30)")`. morm has no SQL parser, so the macro only stores each one in the manifest, in the order written. They aren't part of the schema, so changing, adding or removing only a `proof` opens no migration.

**R-0962** Types are declared once, on their Rust type. A sequence has no Rust type, so it uses a macro:

```rust
#[derive(PgDomain)]
#[pg_domain(name = "email", check = "VALUE LIKE '%@%'")]
pub struct Email(pub String);                       // base type comes from String: text

#[derive(PgComposite)]
#[pg_composite(name = "money_amount")]
pub struct MoneyAmount {
    #[col(sql = "numeric(12,2)")] pub amount: Decimal,
    pub currency: String,
}

#[derive(PgEnum)]
#[pg_enum(name = "order_state")]
pub enum OrderState { Pending, Paid }

morm::pg_sequence!(invoice_no, "START 1000 CACHE 20");
morm::pg_sequence!(order_no, file = "sql/order_no.sql");
```

- **R-0963** `#[pg_domain]`, `#[pg_composite]` and `#[pg_enum]` make morm manage the type they sit on, and each registers it in the manifest like any other object. `name` is the Postgres type's name, and `#[pg_domain]` also takes an optional `check`. A bare derive registers nothing, so morm never creates or changes that type: it binds to the snake_case of the Rust name, and `#[pg_type(name = "...")]` overrides that name without turning on management.
- **R-0964** The Rust item gives the type's shape, so nothing is written twice: a domain's base type comes from its field, a composite's attributes from its fields in order, and an enum's values from its variants in order, except an `unknown` variant. Fields use normal type inference, and `#[col(sql = "...")]` overrides it. These three attributes take no `file`, because a `.sql` file would restate the base type, fields or values.
- **R-0965** `morm::pg_sequence!(name, "options")` declares a standalone sequence, and `options` is the text after `CREATE SEQUENCE name`. With `file`, the file holds the complete `CREATE SEQUENCE` statement. `OWNED BY` isn't available: sequences apply before tables, so the table it names doesn't exist yet.

### Managed enums

**R-0990** **Enum changes are declared on the variants.** `was` names a label this variant used to have, and `value` sets its label when snake_case isn't it:

```rust
#[derive(PgEnum)]
#[pg_enum(name = "order_state")]
pub enum OrderState {
    Pending,
    #[pg_enum(was = "paid_up")] Paid,
    #[pg_enum(value = "in-transit")] Shipping,
}
```

The manifest carries the labels in variant order, with each `was` and, from step 41, each `deprecated`.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0143 | trybuild | a `#[pg_composite]` field with no SQL type: its Rust type infers none, and it has no `#[col(sql = "...")]` |
| T-0144 | suite | **Bare derives:** `#[derive(PgDomain)]`, `#[derive(PgComposite)]` or `#[derive(PgEnum)]` with no attribute, or with only `#[pg_type(name = "...")]` or variant-level `#[pg_enum(...)]` attributes, adds nothing to the manifest. |
| T-0154 | trybuild | `#[pg_type]` on a type that also has `#[pg_enum(name = "...")]`, `#[pg_domain]` or `#[pg_composite]` |
| T-0191 | suite | **`proof`:** on `pg_function!` and `pg_procedure!`, with `file` or inline, each `proof` reaches the manifest byte for byte and in the order written, including several on one declaration, a declaration without one stays valid, and changing, adding or removing only a `proof` opens no migration. |
