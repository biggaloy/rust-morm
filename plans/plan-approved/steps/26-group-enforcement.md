# Step 26: Groups on queries

- **Phase:** Runtime
- **Depends on:** 23, 25, 08
- **Can run beside:** 27, 29, 30, 31, 32, 33, 34, 37, 38, 39, 40, 42, 43, 44, 46, 47
- **Crates:** `morm`, `morm-macros`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Enforce groups on a `db.groups(mask)` handle for every read and write.

## Deliverables

- `db.groups`, `db.sudo`, `Forbidden`, failing closed on reads, `to_json_for`

## End-to-end check

Each read and write path, on plain `db` and on a handle, either returns only allowed data or returns `Forbidden`.

## Out of scope

- New group declarations

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Views and groups / Public input and output

- **R-0258** It never appears in `Serialize`, `to_json`, `to_json_for`, `fetch_json`, `stream_json` or `json_schema`, not even with `sudo`.

### Views and groups / Groups: granular field access

- **R-0275** **Plain `db` is trusted server code,** with no group checks. Checks apply only on a `db.groups(mask)` handle, so request handlers use one. `db.sudo()` passes every group.
- **R-0276** **Writes on a handle:** `.set()` or `.apply()` on a field outside the mask returns `Forbidden`. `insert` leaves such a field out of the statement, so it gets the column's default.
- **R-0277** **Reads on a handle fail closed.** `.filter()`, `fetch_json` and `stream_json` select only allowed columns. `.filter_cols()` naming a column outside the mask returns `Forbidden`, and so does `get`, `fetch` or `fetch_ref` into a full struct that has such a column, because the struct has nowhere to leave it out. Read through a view, `.filter_cols()` or a JSON path.
- **R-0278** `to_json_for(mask)` limits output for a row you already hold.

### Views and groups / Masks

**R-0280**

```rust
let db = db.groups(G.admin | G.staff);                  // works on every model
let db = db.sudo();                                     // same as db.groups(G.sudo)
let db = db.groups(G.staff | User::G.billing);          // works on User only
let mask = G::from_names(&claims.roles)?;               // roles from a token or a table
```

- **R-0283** **Generic code:** stable Rust can't write `[u64; Self::N]` in generic code, so each model names its mask as an associated type (`type Scoped = Bits<2>`), and `db.groups()` is generic over the mask.
- **R-0287** `.set()` checks once per call, and `to_json_for` works out its field list once per mask, so output cost doesn't grow with the number of rows.
- **R-0288** **Scoped bits are per model,** so a mask containing them is typed `Mask<User>`, and using it on another model is a compile error. A caller with scoped groups on two models uses two handles. A mask of only global groups works everywhere.

### Reading and writing / JSON output and schemas

- **R-0478** `user.to_json_for(mask)` limits output to groups.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0776 | A mask handle exposing restricted columns | Reads on a handle fail closed with `Forbidden` |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0027 | trybuild | a scoped mask on another model |
| T-0037 | integration | groups |
| T-0067 | integration | `secret` absent from every JSON path and `Debug` |
| T-0081 | integration | group rules on a handle for every read and write path |
