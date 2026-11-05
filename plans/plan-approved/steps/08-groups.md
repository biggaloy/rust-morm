# Step 08: Group declarations and masks

- **Phase:** Models
- **Depends on:** 07
- **Can run beside:** 10, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 26, 28, 29, 30, 31, 32, 33, 35, 36, 37, 38, 40, 41, 42, 43, 45
- **Crates:** `morm-macros`, `morm`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Global and model-scoped groups with macro-counted masks.

## Deliverables

- `morm::groups!`, `scoped_groups`, `#[model(groups)]`, `sudo`, `#[view(groups = ...)]`, `G`, `Bits<N>`, `Mask<T>`, `from_names`, `Model::groups()`, `Model::group_fields()`

## End-to-end check

An app with global and scoped groups over 64 entries builds masks, converts names at runtime, and rejects a scoped mask on another model at compile time.

## Out of scope

- Enforcement on queries (step 25)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Models / Struct options (`#[model(...)]`)

| ID | Option | Meaning |
|---|---|---|
| R-0153 | `groups`, `sudo` | global groups for the whole model (section 5) |
| R-0154 | `scoped_groups` | declares groups that exist only on this model (section 5) |

### Models / Field options (`#[col(...)]`)

| ID | Option | Meaning |
|---|---|---|
| R-0172 | `groups`, `sudo` | field access groups (section 5) |

### Models / Generated per model

- **R-0217** `Product::G.<group>` masks, when the model has scoped groups

### Views and groups / Groups: granular field access

**R-0263** There are two kinds of group:

| ID | Kind | Declared | Applies to | Mask |
|---|---|---|---|---|
| R-0264 | **Global** | Once, with `morm::groups!` | Models and fields, everywhere. The same name is the same group on every model | `G.admin` |
| R-0265 | **Model-scoped** | On one model, with `scoped_groups` | That model's fields only | `User::G.billing` |

**R-0266**

```rust
// src/groups.rs: one declaration for the whole app
morm::groups!(admin, staff, moderator);         // sudo is always included

// src/users/mod.rs
use crate::groups::G;

#[model(table = "users", base = "common", scoped_groups = "billing, support")]
pub struct UserAdmin {
    #[col(groups = "admin,moderator")]  pub subscriptions: Vec<i32>,   // global
    #[col(groups = "billing")]          pub card_token: String,        // scoped to this model
    #[col(groups = "staff,support")]    pub notes: String,             // one of each
    #[col(sudo)]                        pub password: String,          // sugar for groups = "sudo"
}

#[model(table = "audit_log", base = "common", groups = "staff")]       // global, whole model
pub struct AuditLog { /* ... */ }
```

**R-0267** `sudo` is an ordinary global group with a reserved name, always present, so it works with no declaration.

**R-0268** **Declaring global groups:** call `morm::groups!` once per app, in any module. It's plain Rust, not `morm.toml`, so names are checked by the compiler. It generates a constant `G`, with one field per group numbered in the order listed, and `sudo` as bit 0. Models find `G` through a normal `use`. If models span several crates, put the call in a crate they all depend on.

- **R-0269** **Resolving names:** a name listed in the model's `scoped_groups` is scoped. Any other name is global and resolves to `G`, so a typo is a compile error. A scoped name hiding a global one is a compile error too.
- **R-0270** **Access:** a field with no groups is open to everyone. A grouped field needs the caller to be in at least one of its groups.
- **R-0271** **Whole model:** `#[model(groups = "staff")]` puts every field without its own `groups` into that group. `#[model(sudo)]` does the same with `sudo`. Only global groups work here, and a field's own `groups` takes priority.
- **R-0272** **Compile time:** `#[view(pub Moderator, groups = "moderator")]` builds a view from ungrouped fields plus that group. Views take both kinds.
- **R-0277** `Model::groups()` lists the model's groups, and `Model::group_fields("moderator")` lists a group's fields.

### Views and groups / Masks

- **R-0279** **Structure:** bits live in an opaque `Bits<N>`, a plain `[u64; N]` array. A group number `n` is bit `n % 64` of word `n / 64`, and a check ANDs the arrays word by word.
- **R-0280** **Size is counted by the macros, with nothing to configure.** `morm::groups!` counts the global groups and `#[model]` counts its scoped groups, then each sets `N` to the fewest words that fit. Up to 64 groups use `[u64; 1]`, 65 to 128 use `[u64; 2]`, and so on. Global and scoped sizes are separate, and a model with no scoped groups stores no scoped bits.
- **R-0282** **Limits:** none fixed, for groups or fields.
- **R-0283** **Cost:** a mask is held by value in the handle with no allocation, 8 bytes per word. Each grouped field has an entry of the same size in a static table, and rows carry no extra bytes. Each extra word adds a fraction of a nanosecond to a check, and the milestone 2 benchmark gives real figures. Group code is monomorphized per mask type, which slightly lengthens compile times.
- **R-0284** **Only grouped fields are checked.** A field without groups generates no check.
- **R-0287** **Bit numbers aren't stable.** They change when the list changes, so store group names, never mask values, and convert with `from_names`. An unknown name is an error.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0025 | trybuild | unknown group names |
| T-0026 | trybuild | scoped groups on `#[model(groups)]` |
