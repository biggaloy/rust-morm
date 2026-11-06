# Step 23: JSON output and schemas

- **Phase:** Runtime
- **Depends on:** 16, 17
- **Can run beside:** 08, 09, 12, 18, 19, 20, 21, 22, 24, 25, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 51, 52, 53, 54
- **Crates:** `morm`, `morm-macros`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Every JSON path gives the same documented output, with no `Value` or map in between.

## Deliverables

- `to_json`, `Serialize`, `ProductRef` serialization, `fetch_json`, `stream_json`, `jsonb` as `RawValue`, `json_schema`, `utoipa`, every fixed format

## End-to-end check

For one row with every type, `fetch`, `fetch_ref` and `fetch_json` produce the same JSON, and no secret appears on any path.

## Out of scope

- `to_json_for` on a handle (step 26)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools

- **R-0051** `json` (default): `jsonb` and JSON output
- **R-0055** `utoipa`: OpenAPI schemas

### Models / Field options (`#[col(...)]`)

| ID | Option | Meaning |
|---|---|---|
| R-0177 | `skip_json_if` | omit a value from JSON output when a function returns true |
| R-0178 | `json_string` | write an integer as a JSON string (section 8) |

### Models / Type inference

- **R-0204** **JSON:** `u64`, `i128` and `u128` are always JSON strings, because JavaScript clients would lose digits. `json_string` does the same for other integers.
- **R-0205** **Floats:** JSON has no `NaN` or `Infinity`, so they're written as `null`, and the README says so.

### Models / Generated per model

- **R-0219** `json_schema()` is generated per model.

### Reading and writing / Decimal

- **R-0429** **JSON:** always a string, such as `"34.50"`, because a JSON number would lose digits in JavaScript clients. `Numeric` follows the same rule.

### Reading and writing / JSON output and schemas

**R-0477** **Rule: data never passes through `serde_json::Value` or a map.** Each path has at most one conversion in each direction.

- **R-0478** `user.to_json()`, or `Serialize` from the `json` feature.
- **R-0480** `ProductRef` also serializes directly, so `fetch_ref` is the path for rows you process and then send.
- **R-0481** `jsonb` columns can decode as `&RawValue`, so stored JSON is embedded without being parsed and rewritten.
- **R-0483** **Schema:** `User::json_schema(Up | Down)` as generated static strings, plus optional `utoipa` for OpenAPI.
- **R-0484** **Large integers:** JavaScript loses digits above 2^53, so `#[col(json_string)]` writes an integer as a JSON string, on every JSON path.
- **R-0485** Formats are fixed and documented: `Decimal` as a string, timestamps as RFC 3339, `bytea` as base64, `None` as `null`, `jsonb` embedded as JSON.
- **R-0486** **`fetch_json()` on a model or view follows the same formats.** Postgres would write `numeric` as a JSON number, `bytea` as hex and a float `NaN` as the string `"NaN"`, so generated SQL converts those columns, including `json_string` integers and timestamps, which are always written in UTC. `skip_json_if` is a Rust function that Postgres can't run, so `fetch_json` and `stream_json` on such a model are compile errors. `db.sql(...).fetch_json()` has no model to go by, so Postgres's own formats apply there, and your SQL can cast.

**R-0487** `fetch_json()` isn't automatically fastest: it moves the CPU work into Postgres, where it occupies a connection. Benchmarks compare it with `fetch_ref()`, and the docs state the result.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0014 | trybuild | `fetch_json` with `skip_json_if` |
| T-0042 | integration | all list and JSON paths |
| T-0073 | integration | every integer type at its limits through insert, read and JSON |
| T-0077 | integration | `json_string` on every path |
| T-0085 | integration | `fetch_json` formats for `NaN`, `numeric`, `bytea` and timestamps |
