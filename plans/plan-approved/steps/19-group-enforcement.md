# Step 19: Groups on queries

- **Phase:** Runtime
- **Depends on:** 12, 13, 17, 07
- **Can run beside:** nothing
- **Crates:** `morm`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

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

### Views and groups / Groups: granular field access

- **R-0259** **Plain `db` is trusted server code,** with no group checks. Checks apply only on a `db.groups(mask)` handle, so request handlers use one. `db.sudo()` passes every group.
- **R-0260** **Writes on a handle:** `.set()` or `.apply()` on a field outside the mask returns `Forbidden`. `insert` leaves such a field out of the statement, so it gets the column's default.
- **R-0261** **Reads on a handle fail closed.** `.filter()`, `fetch_json` and `stream_json` select only allowed columns. `.filter_cols()` naming a column outside the mask returns `Forbidden`, and so does `get`, `fetch` or `fetch_ref` into a full struct that has such a column, because the struct has nowhere to leave it out. Read through a view, `.filter_cols()` or a JSON path.
- **R-0262** `to_json_for(mask)` limits output for a row you already hold.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0726 | A mask handle exposing restricted columns | Reads on a handle fail closed with `Forbidden` |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0076 | integration | group rules on a handle for every read and write path |
