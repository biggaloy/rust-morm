# Step 08: Declarative rules and `clean`

- **Phase:** Models
- **Depends on:** 04
- **Can run beside:** 05, 06, 07, 09, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35
- **Crates:** `morm-macros`, `morm`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Generate inline validation with no per-field runtime cost.

## Deliverables

- `min`, `max`, `len`, `choices`, `clean`, `db_check`, as generated functions with no function pointers

## End-to-end check

Every rule accepts and rejects at its boundary in unit tests, and a field with no rules generates no code.

## Out of scope

- When rules run in a write (step 22)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Models / Field options (`#[col(...)]`)

| ID | Option | Meaning |
|---|---|---|
| R-0174 | `min`, `max`, `len`, `choices`, `clean`, `blocking`, `db_check` | validation (section 6) |

### Validation

**R-0285** One mechanism, instead of python-morm's overlapping `validator`, `modifier` and `_clean_*`. The compiler replaces type checks, native decoding replaces JSON modifiers, and the rest is declarative rules or `clean`.

**R-0286**

```rust
#[col(min = 1, max = 100)]            pub qty: i32,      // declarative, generated inline
#[col(len = "3..=65")]                pub username: String,
#[col(choices = "admin,user,system")] pub kind: String,
#[col(clean = "hash_password", blocking)] pub password: String,  // fn(T) -> Result<T>
```

- **R-0289** **No function pointers per field.** Everything is static and inlined, and a field without rules generates no code.
- **R-0296** `db_check` also emits a declarative rule as a real `CHECK` constraint.

## Test obligations

None beyond one tagged test per requirement.
