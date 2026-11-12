# Proofs

What the proof steps found: for each proof, the result, the final shape of the API, and any design change. Every step reads this file. Where a result changes a design, the change stands only once the owner approves it, and the entry says which.

## Step 02: proofs 1 to 12

The spikes were built on stable Rust 1.99.1 with no nightly features, in their own workspace under `spikes/`, and run on Postgres 15.19, 16.15, 17.11 and 18.6 in CI, and on 17.11 and 18.6 locally. Timings below are from release builds, on the CI runners and locally. They're thrown away when the step ends; the PR that held them keeps them in its history. A pass below means every listed check ran green.

Spelling note: the spikes couldn't see the frozen plan, so attribute spellings such as `#[view(Name, hide(..))]`, `#[unique(..)]`, `#[col(group = ..)]`, `#[model(borrowed)]` and `#[model(global = ..)]` are the spikes' own. What's proven is the mechanism and the generated shape. The step that builds each feature uses its own step file's spelling.

How to rerun, from the PR's branch: `SPIKE_DSN=... cargo test --release --manifest-path spikes/Cargo.toml --workspace` and `bash spikes/proof10.sh`. Workflow `spikes.yml` runs both on Postgres 15 to 18.

| # | Proof | Result |
|---|---|---|
| 1 | `#[view]` sibling structs | Pass |
| 2 | `#[morm::base]` across crates | Pass |
| 3 | Macro-counted `Bits<N>` | Pass, with a refinement |
| 4 | `async \|tx\|` closures with `.retry(n)` | **Fails as written. Needs an owner decision** |
| 5 | Hooks, lazy `db`, `#[morm::hooks]` | Pass. One choice needs the owner |
| 6 | Borrowed `ProductRef<'r>` | Pass |
| 7 | `NestedConnection` for stream loops | Pass, with a refinement |
| 8 | Upsert conflict-target markers | Pass, with a refinement |
| 9 | `morm::Param` with `query_typed` | Pass for the types tried. One question for the owner |
| 10 | Registration behind `cfg(morm_mgr)` | Pass, with two refinements |
| 11 | The loop detector | Detection and cost pass. **The exclusions aren't proven: question for the owner** |
| 12 | `deadpool` with our own manager | Pass |

### 1. `#[view]`: pass

`#[model]` reads each `#[view(..)]` on the struct and emits a sibling struct per view, from the model's one field list, with only that view's fields. Each view gets its own `Model` impl (same table, its own `COLUMNS` and `SELECT`) and its own `F`.

- `Product::F` is a `ProductColumns`, `ProductPublic::F` a `ProductPublicColumns`, and a view's columns are typed by the view: `Column<ProductPublic, String, _>`. Passing one where the model's column is expected is a type error naming both.
- A hidden field is absent from the view and its `F`. The error is rustc's own and reads well: `no field 'cost' on type 'ProductPublicColumns'`, with `available fields are: 'id', 'name', 'photo', 'tags'`. A view naming a field the model doesn't have is an error at that name.

### 2. `#[morm::base]`: pass

`#[morm::base]` emits a `#[macro_export]` `macro_rules!` that holds the base's fields as tokens, plus `pub use <that macro> as <Base>;`, so the macro sits at the base's own path. `#[model(base = path)]` calls `path! { @model [args] <struct> }`, and the macro calls `#[::morm::model(__base_fields { .. }, args)]` back with the fields in front. They land as plain fields: a struct literal of the model names them directly.

- A base extending a base asks its parent for the fields once, where it's defined, so its own macro already holds every field. A model using `bases_b::Audit` (which extends `bases_a::Timestamps`) compiles in a crate that doesn't depend on `bases-a`.
- Works across crates, inside the base's own crate, and from a module path.
- `#[macro_export]` puts every such macro at the crate root, so the macro's name carries a hash of the file and line where the base is written (`Span::file` and `Span::line`, stable). Two bases with the same name in one crate both work.
- A unique column declared in a base reaches the model's unique keys (proof 8).

### 3. `Bits<N>`: pass, with a refinement

`groups!` counts its groups (plus the built-in `sudo` at bit 0) and picks `N = ceil(count / 64)`: 4 groups give 8 bytes, 65 give `Bits<2>`, 16 bytes. `#[model(groups(..))]` counts the model's scoped groups the same way, and a model with none has `()` (0 bytes). Generic code works through a `MaskBits` trait implemented for every `Bits<N>` and `()`, so `db.groups(&mask)` is an ordinary generic method on stable, with no `generic_const_exprs`.

- A `Mask<Order>` where `Mask<Product>` is expected, or another model's `ScopedGroup`, is a type error naming both models.
- **Refinement:** `groups!` emits its mask as its own newtype, `pub struct Mask(pub Bits<N>)`, not an alias for `Bits<N>`. With an alias, a group from another `groups!` of the same size would be accepted silently; with the newtype it's a type error (`expected 'app::groups::Mask', found 'app::big::Mask'`).
- `Model` gains `type Global: MaskBits; type Scoped: MaskBits;`.
- How a model in another crate finds the app's mask is step 08's to settle; the spike names it with `#[model(global = crate::groups)]`.

### 4. `async |tx|` closures with `.retry(n)`: fails as written, needs an owner decision

The plain form works: `db.transaction(async |tx| ..)` takes `AsyncFnOnce(&Tx) -> Result<T>`, so a closure may move or mutate what it captures, and it runs under `tokio::spawn`.

The retry form doesn't work as the plan names it, `AsyncFn` closures for `.retry()`:

- With an `AsyncFn(&Tx)` bound, a closure that only *reads* a captured row by reference isn't `Send` under `tokio::spawn`: rustc reports `implementation of 'Send' is not general enough` (a known compiler limitation with higher-ranked lifetimes). So it would fail in every Axum handler that does the most common thing. Only closures that `move` in owned data work.
- A postfix shape, `db.transaction(f).retry(n).await`, can't be built on stable for borrowing closures either. Awaiting a builder needs `IntoFuture`, whose future must be a named type; on stable that's a boxed `dyn Future + Send`, and proving the closure's future `Send` for the box takes a bound over every lifetime, which forces captures to be `'static` (`'row.id' does not live long enough ... borrowed for 'static`). It would also box every transaction.

What works, tested on Postgres 15 to 18: `.retry(n)` takes `AsyncFnOnce(&Tx) -> Result<T> + Clone`, and each attempt runs a fresh clone of the closure. A closure that captures by shared reference is `Clone` at no cost, works under `tokio::spawn`, and is rerun after a real `40001`. A closure that mutates a capture holds a `&mut`, which isn't `Clone`, so it doesn't compile: `the trait bound '&mut String: Clone' is not satisfied ... required for '{async closure}' to implement 'RetryBody<()>'`. A `move` closure gets a fresh copy of its values on each attempt, so nothing leaks from a failed attempt. Because the closure is passed last, the retry count goes first: `db.retry(3).transaction(async |tx| ..).await`.

Options for the owner:

- **A (recommended):** `db.retry(n).transaction(async |tx| ..)`, bound `AsyncFnOnce(&Tx) -> Result<T> + Clone`. No allocation, works with borrowing closures under Axum, refuses mutation at compile time. The cost: the count is written before the closure, and the mutation error names `Clone` rather than saying "retry"; the README would explain it in one line.
- **B:** keep `AsyncFn`. Works only with `move` closures of owned data in spawned code, and gives the obscure `Send` error otherwise.
- **C:** postfix `.retry(n)` with a boxed future. Only `'static` captures, and a box per transaction. Not viable.

Also seen: rerunning at once after a serialization failure lost again while the winning transaction was still committing; a 20 ms pause fixed it. Step 20 should check its own retry rules for a pause between attempts.

### 5. Hooks: pass, one choice needs the owner

- Users write `async fn` in `impl Hooks` against the contract's `fn ..(..) -> impl Future<Output = Result<()>> + Send` methods, including `rows: &mut [Self]` and `db: &impl Conn`. A hook that holds a non-`Send` value across an `.await` fails at the hook itself (`future returned by 'pre_insert' is not 'Send'`), not deep inside the web framework.
- Under Axum: a handler that awaits a generic insert, which awaits the user's hooks, compiles as an Axum handler and serves a request.
- `#[morm::hooks]` records which hooks the impl defines, as `const OVERRIDES: Overrides`, a bit set, and the generic insert calls only those. The const is required, with no default, so an `impl Hooks` written without the attribute doesn't compile (`missing 'OVERRIDES' in implementation`), and a hook can never be skipped because its override went unrecorded.
- The lazy `db` checks out on its first query and keeps that connection: a hook that refuses without a query checked out nothing, and when a hook queries, the insert reuses its connection, one checkout in all.

**Choice for the owner:** `#[model]` emits an empty `impl Hooks` for every model, so models without hooks need nothing. A model with hooks must say so on the model, `#[model(hooks)]`, so `#[model]` leaves the impl to `#[morm::hooks]`; otherwise the two impls conflict, a compile error. The macro can't see the impl block, so some mark on the model is needed. Recommended: `#[model(hooks)]`, with a README line. The alternative is a default `OVERRIDES` that means "all", which compiles without the mark but runs every hook path for every model.

### 6. Borrowed `ProductRef<'r>`: pass

`ProductRef<'r>` maps `String` to `&'r str`, `Vec<u8>` to `&'r [u8]`, `Option` of either to `Option` of the borrowed form, and keeps other types. A counting allocator measured, over 1,000 rows of text, `bytea` and nullable fields: decoding allocated 0 times, decoding plus `serde_json::to_writer` into the caller's buffer allocated 0 times, and the owned model allocated at least twice per row. Its `Serialize` impl writes the fields directly, with no intermediate value.

### 7. `NestedConnection`: pass, with a refinement

A stream checks out nothing until first polled. On its first poll it registers the polling task, then checks out and runs its query; on each later poll, if the task changed, the registration moves. A checkout reads one relaxed atomic, the count of open streams, and only when it's non-zero looks up the current task. Tested: a query inside a stream loop on a pool of one is refused at once instead of waiting forever; a nested stream is refused; another task may query meanwhile; `join!` of the loop and a query on one task refuses the query; a stream made on one task and polled on another belongs to the second, and moves back when polled on the first; a finished or dropped stream frees the task. The check with no stream open took 0.55 to 1.41 ns.

- **Refinement:** a future driven by `block_on`, including the body of `#[tokio::main]` and `#[tokio::test]`, has no Tokio task id, so the registry keys by task id, or by thread id when there's none. Keyed by task id alone, the loop in `main` deadlocked in the test.

### 8. Upsert conflict targets: pass, with a refinement

Each column's `Column<M, T, C>` has its own zero-sized marker type `C`, generated per field, so each column is a distinct type and can carry its own `UniqueKey<M>` impl. A unique column implements it with `CONFLICT = ("col")`, and `Model::UK.<name>` constants of type `Key<M, marker>` cover expression, partial and composite keys: `(lower("email"))`, `("code") WHERE "deleted" = false`, `("tenant", "code")`. A unique column declared in a base from another crate, reached through two base expansions, is a key too.

- A column with no unique key is a compile error, worded through `#[diagnostic::on_unimplemented]`: ``'Column<Order, String, ..::code>' is not a unique key of 'Order'``, with a note on how to declare one.
- On Postgres 15 to 18, each of the four targets is accepted as the arbiter of `INSERT .. ON CONFLICT .. DO UPDATE`, with a row that conflicts on that key only.
- **Refinement:** the contract's `Cap` parameter becomes the per-column marker type, and capabilities such as refusing `.set()` become traits implemented on the marker, rather than a shared capability type.

### 9. `morm::Param` with `query_typed`: pass for the types tried, one question

`Param` is as in `contracts.md`. A blanket impl covers every type with a fixed Postgres type through a small `PgType` trait of two constants, the type and its array type. Parameters go from `&[&dyn Param]` straight into `query_typed_raw` as an iterator, so nothing is boxed or collected, and `&dyn Param` upcasts to `&(dyn ToSql + Sync)` (stable trait upcasting). On Postgres 15 to 18 each value round-tripped as the same Rust type, with the declared types the ones Postgres used, and bound to real columns: `bool`, `i16`, `i32`, `i64`, `f32`, `f64`, `String`, `&str`, `Vec<u8>`, `&[u8]`, `Uuid`, `serde_json::Value` (`jsonb`), `rust_decimal::Decimal`, `SystemTime` (`timestamptz`), `None` of each, `Some`, and arrays: `Vec<T>`, `&[T]`, `Vec<Option<T>>` and empty.

- An enum label sent as `Type::UNKNOWN`, as the contract says for `PgEnum`, binds to an enum column and an enum comparison with no OID sent, and a label the type doesn't have gets the database's own `22P02`.

**Question for the owner:** the step asks for "every supported type", but the list of supported types isn't in this step's files. The spike covered every type above. morm's own `Timestamp`, `Range<T>` and `Numeric` have no encoding until step 13, so they couldn't be tried. Recommended: step 13 adds the same round-trip test for its own types through `&dyn Param`. The alternative is to give step 02 the list and extend the spike before merging.

### 10. Registration: pass, with two refinements

Models in three crates were all found by a stand-in `morm-mgr` built with `--cfg morm_mgr` in its own target folder, including a crate no code references. `cargo build --workspace` without the cfg put no metadata in the app or `mgr`. As a control, the app built with the cfg did carry it, so the search would have found it. Each model registers through `inventory`.

- **Refinement:** a `#[cfg(morm_mgr)]` in `#[model]`'s output warns `unexpected_cfgs` in every crate with a model, which breaks `-D warnings`. So `#[model]` calls `morm::__morm_register!`, a `macro_rules!` that `morm` defines twice, under `cfg(morm_mgr)` and `cfg(not(morm_mgr))`. The cfg is read when `morm` is built, and `morm` declares it in its own `[lints.rust] unexpected_cfgs` `check-cfg`. User crates never see it, and the workspace builds with warnings denied.
- **Refinement:** a crate that `morm-mgr` only lists in `Cargo.toml` is dropped by the linker with its models; tested, its models went missing. The generated `morm-mgr` must name each crate with models, `use <crate> as _;`.

### 11. The loop detector: detection and cost pass, exclusions not proven

Each worker thread keeps eight slots keyed by (task, statement), where a statement is its `&'static str`'s address, so there's no hashing or string compare. The same statement reaching the threshold inside the window on one task is reported once. Nine runs under a threshold of ten, slow repeats outside the window, and one run on each of many tasks are not reported. A task that moves thread starts counting again, which can miss a loop but never invents one. Every query through `Db` passes the detector, and an N+1 loop of twelve queries was reported. The cost was 27 to 47 ns per statement.

**Question for the owner:** the pass condition says "the listed exclusions work", but the list isn't in this step's files, `rules.md` or `contracts.md`, and the rules forbid reading other step files or the frozen plan to fill a gap. So the exclusions aren't built or proven here. Options:

- **A (recommended):** move "the listed exclusions work" to step 45, which builds the detector with its exclusions. Most exclusions likely depend on features that don't exist yet, so a spike now would have to fake them.
- **B:** copy the exclusion list into step 02's proof row, and prove it in this PR before it merges.

### 12. `deadpool` with our own manager: pass

`deadpool`'s `managed` core with our `Manager` for `tokio_postgres::Client`. `recycle` runs no query; it only checks `is_closed()`. Tested on Postgres 15 to 18:

- Nothing opens until needed, and three concurrent checkouts open three.
- A hundred checkouts opened nothing new and ran nothing on the server: each backend's `state_change` and last query in `pg_stat_activity` were unchanged. A checkout took 0.33 to 0.52 µs, against 40 to 166 µs for an empty round trip.
- `resize` while running: down while idle closes the extra connections at once, down while busy closes them as they come back, and up lets more open.
- Idle shrinking: `retain` on `last_used` closed every connection idle past the limit, on demand and from a timer task, and the pool opened again afterwards.
- Noted for step 11: `retain` allocates a `Vec` per call, so it belongs on the reaper's timer, never on a checkout.
