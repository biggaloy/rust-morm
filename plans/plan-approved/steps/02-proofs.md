# Step 02: Proofs of the risky parts

- **Phase:** Foundation
- **Depends on:** 01
- **Can run beside:** nothing
- **Crates:** throwaway `spikes/` directory, deleted when the step ends
- **PR approval:** the owner approves the PR

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Prove on stable Rust 1.99.1 that the designs everything else depends on actually compile and work, before anything depends on them. A failed proof stops the project and goes to the owner with options.

## Deliverables

- One small spike per item below, each compiled and run
- `plans/plan-approved/proofs.md`: result, the final shape of each API, and any design change the owner approved
- `contracts.md` updated with what was learned

## End-to-end check

Every spike builds and passes on stable 1.99.1 with no nightly features, and `proofs.md` records a pass or an owner-approved change for each item.

## Proof list

| # | Proof | Pass means |
|---|---|---|
| 1 | `#[view]` generating sibling structs from one attribute list | Hidden fields are absent, `F` differs per view, and the error message for a hidden field is readable |
| 2 | `#[morm::base]` through a generated `macro_rules!`, across crates, base extending a base | Fields appear as plain fields, with no nesting |
| 3 | Macro-counted `Bits<N>` with an associated type | `db.groups()` is generic over the mask on stable, and a mask for another model is a compile error |
| 4 | `async |tx|` closures with `.retry(n)` | A closure mutating a captured row doesn't compile, and the plain `FnOnce` form still works without `.retry` |
| 5 | Hook signatures, lazy `db: &impl Conn`, `#[morm::hooks]` | Users write `async fn`, futures are `Send` under Axum, the macro records overrides, and a hook with no query checks out nothing |
| 6 | Borrowed `ProductRef<'r>` | Rows decode with no per-field allocation and serialize directly |
| 7 | `NestedConnection` for stream loops | Registry on first poll, re-registration on task change, `join!`, a stream moved between tasks, and one atomic load when no stream is open |
| 8 | Upsert conflict-target markers across macro expansions | A column with no unique key is a compile error, and `UK` constants work for expression and partial indexes |
| 9 | `morm::Param` with `query_typed` | Works for every supported type, `Option<T>`, arrays and `NULL`, through `&dyn morm::Param`, with no boxing |
| 10 | Registration behind `cfg(morm_mgr)` | Models in several crates are all found, including a crate nothing references, and no metadata reaches the app binary under `cargo build --workspace` |
| 11 | The loop detector | The same statement repeating in a short window is detected at low cost, and the listed exclusions work |
| 12 | `deadpool` with our own manager | Dynamic size, idle shrinking and checkout with no extra round trip all work |

## Out of scope

- Production code: spikes are thrown away

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Risks, open items and later work / Risks

- **R-0915** **Needs proof early:** borrowed `ProductRef` decoding, the loop detector, macro feasibility for `#[view]` and `#[morm::base]`, macro-counted group mask sizes on stable Rust, `AsyncFn` closures for `.retry()`, hook signatures including the lazy `db`, `NestedConnection` detection for stream loops, including `join!` and streams moved between tasks, upsert conflict-target markers across macro expansions, `morm::Param` with `query_typed` on every supported type, and registration with `cfg(morm_mgr)` across crates.

## Test obligations

None beyond one tagged test per requirement.
