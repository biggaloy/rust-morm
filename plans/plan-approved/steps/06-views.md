# Step 06: Views, input, patch and secret

- **Phase:** Models
- **Depends on:** 05
- **Can run beside:** 08, 09, 10, 11, 12, 13, 26, 27, 28, 29
- **Crates:** `morm-macros`, `morm`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Generate restricted views and safe public input and output types.

## Deliverables

- `#[view]` with `read_hide`, `write_hide`, `fields`, `input`, `patch`
- `WriteOnly<T>`, `Patch<T>`, `#[col(secret)]`, generated `Serialize`, `Debug` and `Deserialize`, `#[model(deserialize)]`

## End-to-end check

A request body parses into an `input` and a `patch` view with every field state, unknown fields are rejected, and every forbidden form fails to compile.

## Out of scope

- `.apply()` running an update (step 16)
- `groups =` on views (step 07)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Models / Struct options (`#[model(...)]`)

| ID | Option | Meaning |
|---|---|---|
| R-0156 | `deserialize` | generate `Deserialize` for the full model, for trusted data only (section 5) |

### Models / Field options (`#[col(...)]`)

| ID | Option | Meaning |
|---|---|---|
| R-0173 | `secret` | never in JSON or schemas, redacted in `Debug` (section 5) |

### Views and groups / Views: restricted models

**R-0221** A view is a restricted model over the same table. Views are declared on the full definition, because an attribute macro can't read another struct's fields.

**R-0222**

```rust
#[model(table = "users", base = "common")]
#[view(pub User,
       read_hide  = "password, expires, is_super_user",
       write_hide = "is_super_user, expires")]
#[view(pub UserProfile, groups = "staff")]
pub(in crate::auth) struct UserAdmin { /* every column */ }
```

- **R-0223** Each view is a real generated plain struct with its own columns, `F`, `SELECT` constant and decoder.
- **R-0224** **Hidden means absent.** `user.password` and `User::F.password` are compile errors, and JSON output can't include the field.
- **R-0225** **Write-only works:** `read_hide` without `write_hide` generates a `WriteOnly<T>` field, which can be set but has no reader, `Debug` or `Serialize`.
- **R-0226** **Read-only works:** `write_hide` fields can't be used in `.set()` (compile error).
- **R-0227** **Privilege follows Rust visibility.** With `UserAdmin` private to `crate::auth`, the compiler proves no other module touches passwords.
- **R-0228** **Narrowing is one-way.** `From<UserAdmin> for User` exists; nothing goes the other way.
- **R-0229** **Raw SQL stays safe.** Named decoding ignores columns a view lacks.
- **R-0231** **Inserts:** a view that hides a required column with no default can't be inserted.

### Views and groups / Public input and output

**R-0233** Hide lists fail open: a column added later is exposed until someone remembers to hide it. So public-facing views list what they allow.

**R-0234**

```rust
#[model(table = "users", base = "common")]
#[view(pub UserOut, fields = "id, name, email")]                 // response body
#[view(pub UserIn, input = "name, email, password")]             // request body
#[view(pub UserPatch, patch = "name, email, bio")]               // partial update body
pub(in crate::auth) struct UserAdmin {
    pub name: String,
    pub email: String,
    #[col(secret)] pub password: String,
    pub bio: Option<String>,
    #[col(default = "false")] pub is_admin: bool,
}
```

- **R-0235** **`fields`** is an allow-list for reads, the opposite of `read_hide`. A new column stays out until it's added.
- **R-0236** **`input`** is an allow-list for request bodies. **Only views with `input` or `patch` get `Deserialize`**, and only those fields are accepted, so a client can't set `id`, `is_admin` or `created_at`. This closes mass assignment by default: parsing a request into a model or any other view is a compile error.
- **R-0237** **Unknown JSON fields are rejected,** so a client sending `is_admin` gets a clear error.
- **R-0238** `input` and `patch` can't list a primary key, identity, generated, `expr`, `version`, `auto_now` or `deprecated` column, or `created_at` and `updated_at`. That's a compile error.
- **R-0239** A `secret` field can be listed in `input` or `patch`, because a password has to arrive somewhere. It's write-only in that view, so it can't be read back or printed.
- **R-0240** `#[model(deserialize)]` opts a full model in, for trusted data such as a cache or a file. It's marked as unsafe for request bodies in the docs.

**R-0241** **Views write too.** `db.insert(&mut new_user)` works on `UserIn`, and `db.update::<UserPatch>(id)` on `UserPatch`, so a handler never names the private full model. A column the view lacks is left out of the statement.

**R-0242** **Partial updates.** A PATCH field has three states: missing, null or a value. A `patch` view holds them with one small enum, not `Option<Option<T>>`:

**R-0243**

```rust
pub enum Patch<T> { Missing, Set(T) }

// generated from the view above
pub struct UserPatch {
    pub name: Patch<String>,          // NOT NULL column
    pub email: Patch<String>,
    pub bio: Patch<Option<String>>,   // nullable column
}

db.update::<UserPatch>(id).apply(body).run().await?;
```

| ID | JSON field | NOT NULL column | Nullable column |
|---|---|---|---|
| R-0244 | Missing | `Missing`, not touched | `Missing`, not touched |
| R-0245 | A value | `Set(v)` | `Set(Some(v))` |
| R-0246 | `null` | Rejected while parsing | `Set(None)`, stored as NULL |

**R-0252** **`#[col(secret)]` protects a field on every model, including one with no views:**

- **R-0253** It never appears in `Serialize`, `to_json`, `to_json_for`, `fetch_json`, `stream_json` or `json_schema`, not even with `sudo`.
- **R-0254** `Debug` prints it as `***`.
- **R-0255** Code can still read it, because a login has to check a hash. Keep such models private with Rust visibility.
- **R-0256** `Serialize` and `Debug` are generated by the macro, so deriving your own on a model is a compile error, and a secret can't be exposed by accident.

**R-0259** **Return a view from public endpoints, never a full model.** The README says so plainly.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0732 | Reading a hidden field | Compile error |
| R-0733 | Parsing a request body into a model | Compile error: only views with `input` or `patch` have `Deserialize` |
| R-0737 | Returning a password in JSON | `secret` is never serialized, even with `sudo` |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0005 | trybuild | hidden view fields |
| T-0018 | trybuild | `Deserialize` on models and on views without `input` or `patch` |
| T-0019 | trybuild | protected columns in `input` and `patch` |
| T-0020 | trybuild | a derived `Serialize` or `Debug` on a model |
| T-0068 | integration | unknown input fields rejected |
