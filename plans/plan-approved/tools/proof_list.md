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
