# Proofs

What the proof steps found: for each proof, the result, the final shape of the API, and any design change. Every step reads this file. Where a result changes a design, the change stands only once the owner approves it, and the entry says which.

## Step 02: proofs 1 to 12

The spikes were built on stable Rust 1.99.1 with no nightly features, in their own workspace under `spikes/`, and run on Postgres 15.19, 16.15, 17.11 and 18.6 in CI, and on 17.11 and 18.6 locally. Timings below are from release builds, on the CI runners and locally. A pass below means every listed check ran green.

The owner decided DA to DE on the first results and DF1 to DF10 on the independent review (`INDEX.md`, Decisions). Each entry below says where a decision changed the design.

Spelling note: the spikes couldn't see the frozen plan, so attribute spellings such as `#[view(Name, hide(..))]`, `#[unique(..)]`, `#[col(group = ..)]` and `#[model(borrowed)]` are the spikes' own. What's proven is the mechanism and the generated shape. The step that builds each feature uses its own step file's spelling.

How to rerun: `spikes/` and `.github/workflows/spikes.yml` were deleted by PR #2's last commit (DE), and the PR's history keeps them. `git checkout 65dec8560dc25829c987c85f670d491c7538beac`, then `SPIKE_DSN=... cargo test --locked --release --manifest-path spikes/Cargo.toml --workspace` and `bash spikes/proof10.sh`, on Postgres 15 or later with `citext` available. `spikes.yml` at that commit runs both on Postgres 15 to 18.

| # | Proof | Result |
|---|---|---|
| 1 | `#[view]` sibling structs | Pass |
| 2 | `#[morm::base]` across crates | Pass |
| 3 | Macro-counted `Bits<N>` | Pass, with two refinements |
| 4 | `async \|tx\|` closures with `.retry(n)` | Pass in the shape the owner chose (DA, DF2): every option before the closure |
| 5 | Hooks, lazy `db`, `#[morm::hooks]` | Pass, with `#[model(hooks)]` (DB) and two rules (DF3) |
| 6 | Borrowed `ProductRef<'r>` | Pass |
| 7 | `NestedConnection` for stream loops | Pass, with two refinements |
| 8 | Upsert conflict-target markers | Pass, with a refinement |
| 9 | `morm::Param` with `query_typed` | Pass with the declared type from the column (DF1). Four questions for the owner |
| 10 | Registration behind `cfg(morm_mgr)` | Pass, with two refinements |
| 11 | The loop detector | Detection and cost pass. The exclusions are proven in step 45 (DD) |
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

### 3. `Bits<N>`: pass, with two refinements

`groups!` counts its groups (plus the built-in `sudo` at bit 0) and picks `N = ceil(count / 64)`: 4 groups give 8 bytes, 65 give `Bits<2>`, 16 bytes. `#[model(groups(..))]` counts the model's scoped groups the same way, and a model with none has `()` (0 bytes). Generic code works through a `MaskBits` trait implemented for every `Bits<N>` and `()`, so `db.groups(&mask)` is an ordinary generic method on stable, with no `generic_const_exprs`.

- A `Mask<Order>` where `Mask<Product>` is expected, or another model's `ScopedGroup`, is a type error naming both models.
- **Refinement:** `groups!` emits its mask as its own newtype, `pub struct Mask(pub Bits<N>)`, not an alias for `Bits<N>`. With an alias, a group from another `groups!` of the same size would be accepted silently; with the newtype it's a type error (`expected 'app::groups::Mask', found 'app::big::Mask'`).
- **Refinement (DF5): how `type Global` is filled.** `groups!` emits `G` twice: a struct `G`, with one `GlobalGroup<Mask>` field per group, and `pub const G: G`. Types and values live in separate namespaces, so one `use crate::groups::G;` brings in both. It also emits `impl GlobalGroups for G { type Mask = Mask; }`. A model that names a global group gets `type Global = <G as GlobalGroups>::Mask;` from the type, and each group's bit from the constant, `G.admin`. Tested: a model in another crate, `models-a`, that only does `use bases_b::groups::G;`, including one whose fields come back through two base expansions, has `bases_b::groups::Mask` as its `Global`, and `Mask::<Order>::new().with(bases_b::groups::G.auditor)` shows the column that group guards. This is the mechanism under R-0271's plain `use`.
- A model that names no global group gets `type Global = ()`, 0 bytes, and needs no `G` in scope.
- A model that names a global group with no `G` in scope fails with rustc's `cannot find type 'G' in this scope`, pointing at `#[model]`. Step 08 may word it better.

### 4. `async |tx|` closures and `.retry(n)`: pass in the owner's shape (DA, DF2)

**What failed as the plan wrote it.** `.retry(n)` on `AsyncFn` closures: with an `AsyncFn(&Tx)` bound, a closure that only *reads* a captured row by reference isn't `Send` under `tokio::spawn` (`implementation of 'Send' is not general enough`, a compiler limitation with higher-ranked lifetimes), so it would fail in every Axum handler. The review reproduced it without the spike's wrapper closure. And any postfix modifier, `db.transaction(f).retry(n)` or `db.transaction(f).isolation(..)`, needs `IntoFuture` and so a named future: on stable that's a boxed `dyn Future + Send`, and proving the closure's future `Send` for the box takes a bound over every lifetime, which forces captures to be `'static` (`'row.id' does not live long enough ... borrowed for 'static`). Naming the future directly would need `AsyncFnOnce::CallOnceFuture`, which is unstable. The plan's first example, `db.transaction(|tx| async move { .. })`, a plain closure returning an async block, doesn't infer against an `AsyncFnOnce(&Tx)` bound either (`type annotations needed`).

**The shape (owner decisions DA and DF2).** Every transaction option goes before the closure, on one prefix builder, and the closure is always last:

```rust
db.transaction(async |tx| { .. }).await?;                         // AsyncFnOnce(&Tx) -> Result<T>
db.isolation(Serializable).retry(3).transaction(async |tx| { .. }).await?;
db.lock_timeout(d).statement_timeout(d).transaction(async |tx| { .. }).await?;
```

- `db.isolation(..)`, `db.retry(n)` and each per-transaction setting start the builder, and they chain in any order. `transaction` is an `async fn` on it, so its future is `Send` whenever the concrete closure's is, with no box.
- Without `.retry`, the closure is `AsyncFnOnce(&Tx) -> Result<T>`, so it may move or mutate what it captures.
- With `.retry(n)`, it's `AsyncFnOnce(&Tx) -> Result<T> + Clone`, and each attempt runs a copy. A closure that mutates a capture holds a `&mut`, which isn't `Clone`, so it doesn't compile, whatever the order of the options: `the trait bound '&mut String: Clone' is not satisfied ... required for '{async closure}' to implement 'RetryBody<()>'`. A `move` closure gets a fresh copy of its values on each attempt, so nothing leaks from a failed attempt.
- Tested on Postgres 15 to 18, each under `tokio::spawn` with a closure reading a captured row by reference: `.isolation(Serializable)` with `.statement_timeout` and `.lock_timeout` and no retry (the closure also mutates a capture), `.retry(3)` before the other options, and a setting before `.retry(1)`. Inside, `SHOW` reports `serializable`, `1234ms`, `250ms`, `4321ms` and `99ms`, and on the pool's one connection afterwards every setting is back to its default. A retried closure is rerun after a real `40001`, and without `.retry` the `40001` surfaces.

**What `.retry` costs and doesn't catch (DF10).**

- An attempt that could still be retried runs a copy of the closure, and the last attempt allowed runs the original. Measured with a counting `Clone`: `.retry(0)` copies nothing, `.retry(3)` that succeeds at once copies once, and two failures (real `40001`s) copy twice under `.retry(2)` and three times under `.retry(3)`. A closure that borrows its captures copies only references, so it allocates nothing. A `move` closure that owns rows clones them on each copy, so it allocates even on a transaction that never retries.
- Only mutation through `&mut` is refused. State behind a shared reference, in a `Cell`, an atomic or a mutex, carries over from one attempt to the next: the spike's own retry test counts attempts that way.

Also seen: rerunning at once after a serialization failure lost again while the winning transaction was still committing, and a fixed 20 ms pause once let two transactions that had both lost retry in step and collide on every attempt, on Postgres 16 in CI. A pause jittered by the clock's nanoseconds failed the same way once, on 17 in CI. The likely cause: where the clock steps in microseconds, its nanoseconds are a multiple of 1,000, and the pause never varies. The spike now pauses 10 to 50 ms by a fresh random number per pause, from `RandomState`; 80 local runs of its retry tests on 17 and 18 and every CI run since passed. R-0442's jittered backoff covers this, and its jitter must come from a real random source; step 20 tests it. The spike formats `BEGIN` and each `SET LOCAL` into one message per transaction; step 20 chooses how they're sent.

### 5. Hooks: pass, with `#[model(hooks)]` (DB) and two rules (DF3)

- Users write `async fn` in `impl Hooks` against the contract's `fn ..(..) -> impl Future<Output = Result<()>> + Send` methods. A hook that holds a non-`Send` value across an `.await` fails at the hook itself (`future returned by 'pre_insert' is not 'Send'`), not deep inside the web framework.
- **Every contract hook compiles and runs (DF7).** The spike's `Hooks` has the contract's exact signatures: `validate`, `pre_insert(rows: &mut [Self], db)`, `post_insert`, `validate_changes(&Changes<Self>)`, `pre_update(&mut Changes<Self>, &Target<Self>, db)`, `post_update(.., affected: u64, db)`, `pre_delete(&Target<Self>, db)`, `post_delete(.., affected, db)` and `after_commit(Committed<Self>)`. Its `Changes` holds boxed `dyn Param + Send` values and its `Target` holds ids, a list, a filter with parameters, or all rows, so `Send` is checked against realistic contents; the spike's key is `id int8`. A generic update and delete run the hooks the impl overrides, then `after_commit` with `Committed::Updated` or `Committed::Deleted`. Tested: `validate_changes` and `pre_update` refuse before any checkout, `pre_update` adds a change that the statement writes, both update hooks and the statement share one checkout, and the hook log reads `post_update`, `committed update`, `post_delete`, `committed delete`. Axum handlers awaiting a generic insert, and a generic update and delete, compile and serve requests.
- **Marking (owner decision DB).** `#[model]` emits an empty `impl Hooks` for every model, so models without hooks need nothing and run no hook path. A model with hooks says `#[model(hooks)]`, so `#[model]` leaves the impl to `#[morm::hooks]`. `#[morm::hooks]` records which hooks the impl defines in the required `const OVERRIDES`, and the generic paths call only those. An `impl Hooks` written without the attribute doesn't compile (`missing 'OVERRIDES' in implementation`).
- **Rule 1 (DF3): no macro calls in the impl.** `#[morm::hooks]` sees the impl before any macro inside it expands, so a hook written by a macro would go unrecorded and never run. It refuses any macro call among the impl's items: `` `#[morm::hooks]` can't see hooks a macro writes, so they'd never run: write each hook as an `fn` in this impl ``. It also refuses an `OVERRIDES` written inside the impl.
- **Rule 2 (DF3): `Overrides` can't be written by hand.** `Overrides<M>` has a private field, no public constructor and no public constant. `#[morm::hooks]` and `#[model]` build it through a `#[doc(hidden)]` `morm::__private::overrides(&[Hook::PreInsert, ..])`. It's typed by its model, so another model's `OVERRIDES` can't be copied in (`expected 'Overrides<Borrowed>', found 'Overrides<Note>'`). Code asks `M::OVERRIDES.has(Hook::PreInsert)`. Each way around the rules is a pinned compile error, and step 24 builds and tests both rules with a trybuild case each.
- The lazy `db` checks out on its first query and keeps that connection: a hook that refuses without a query checked out nothing, and when a hook queries, the statement reuses its connection, one checkout in all.

### 6. Borrowed `ProductRef<'r>`: pass

`ProductRef<'r>` maps `String` to `&'r str`, `Vec<u8>` to `&'r [u8]`, `Option` of either to `Option` of the borrowed form, and keeps other types. A counting allocator measured, over 1,000 rows of text, `bytea` and nullable fields: decoding allocated 0 times, decoding plus `serde_json::to_writer` into the caller's buffer allocated 0 times, and the owned model allocated at least twice per row. Its `Serialize` impl writes the fields directly, with no intermediate value.

### 7. `NestedConnection`: pass, with two refinements

A stream checks out nothing until first polled. On its first poll it registers the polling task, then checks out and runs its query; on each later poll, if the task changed, the registration moves. A checkout reads one relaxed atomic, the count of open streams, and only when it's non-zero looks up the current task. Tested: a query inside a stream loop on a pool of one is refused at once instead of waiting forever; a nested stream is refused; another task may query meanwhile; `join!` of the loop and a query on one task refuses the query; a stream made on one task and polled on another belongs to the second, and moves back when polled on the first; a finished or dropped stream frees the task. The check with no stream open took 0.55 to 1.41 ns.

- **Refinement:** a future driven by `block_on`, including the body of `#[tokio::main]` and `#[tokio::test]`, has no Tokio task id, so the registry keys by task id, or by thread id when there's none. Keyed by task id alone, the loop in `main` deadlocked in the test.
- **Refinement (DF4): the registry is sharded.** While any stream is open anywhere, every checkout on every task looks up its task. Measured with streams open on other tasks, and checks from one thread, then from eight threads at once. With the first spike's one process-wide `Mutex<Vec<..>>`, at commit `1b60b71`, a check took 12 ns and 322 ns with one stream open, 45 ns and 1.07 µs with 100, and 278 ns and 5.4 µs with 1,000 locally (12 threads), and up to 358 ns and 9.5 µs with 1,000 on the CI runners. Sharded 64 ways by a cheap hash of the task, each shard its own mutex and list, it took 8 to 16 ns from one thread and 11 to 30 ns each from eight, from one stream open to 1,000, locally, and 9 to 34 ns and 15 to 116 ns on the CI runners. Step 21 builds the registry, and must not put a single process-wide lock on this path; the sharded form is the measured starting point.

### 8. Upsert conflict targets: pass, with a refinement

Each column's `Column<M, T, C>` has its own zero-sized marker type `C`, generated per field, so each column is a distinct type and can carry its own `UniqueKey<M>` impl. A unique column implements it with `CONFLICT = ("col")`, and `Model::UK.<name>` constants of type `Key<M, marker>` cover expression, partial and composite keys: `(lower("email"))`, `("code") WHERE "deleted" = false`, `("tenant", "code")`. A unique column declared in a base from another crate, reached through two base expansions, is a key too.

- A column with no unique key is a compile error, worded through `#[diagnostic::on_unimplemented]`: ``'Column<Order, String, ..::code>' is not a unique key of 'Order'``, with a note on how to declare one.
- On Postgres 15 to 18, each of the four targets is accepted as the arbiter of `INSERT .. ON CONFLICT .. DO UPDATE`, with a row that conflicts on that key only.
- **Refinement:** the contract's `Cap` parameter becomes the per-column marker type, and capabilities such as refusing `.set()` become traits implemented on the marker, rather than a shared capability type.

### 9. `morm::Param` with `query_typed`: pass with the declared type from the column (DF1), four questions

**The risk the first spike missed.** It took each parameter's declared type from the Rust value, so `String` was always `text`. When the declared type differs from the column's, Postgres casts the column, not the parameter, and the comparison changes meaning with no error. Sent as `text`, `'alice'` found 0 rows in a `citext` column holding `'Alice'`, and `'ab '` found 0 in a `char(5)` holding `'ab'`.

**The design (owner decision DF1).**

- Where morm knows the column (builders, `F`, `sql!`, model inserts and updates), the declared type is the column's: `Column` carries its SQL type, including an explicit `sql`. A built-in type goes as its fixed OID. A type with no fixed OID, such as `citext`, goes as `Type::UNKNOWN`, so no OID is sent.
- A raw parameter, where no column is known, goes as its own type, except `String` and `&str`, which go as `Type::UNKNOWN`, as `PgEnum` labels already do, so Postgres infers the type from the SQL.
- **Choice made here: an `UNKNOWN` text value is sent in text format.** Postgres reads an `UNKNOWN` parameter with the receive function of the type it infers, so in binary format the bytes of `"1234"` compared with an `int4` read as the number 825373492: `1234 = $1` was silently false, `"12x4"` too, and `int8`, `jsonb` and `timestamptz` failed with `08P01`, `XX000` and `22P03`. In text format Postgres reads it as it reads a quoted literal: `1234 = $1` is true, `"12x4"` gets `22P02`, and `jsonb`, `timestamptz`, `citext` and `char(5)` comparisons are right. A text value's bytes are the same in both formats, so this costs nothing. The spike sends parameters through a `Bound` wrapper that sets the format per parameter, straight from `&[&dyn Param]` into `query_typed_raw`, with nothing boxed or collected.
- `PgEnum` labels stay `UNKNOWN` as the contract says: a label binds to an enum column and an enum comparison with no OID sent, and a label the type doesn't have gets the database's own `22P02`.

**The matrix (DF1, DF6).** Each R-0186 Rust type, against its default column and each column an explicit `sql` can give it, inserted and then compared with `=` (or `= ANY($1)` for a list against a scalar column), with the session's `TimeZone` at `Asia/Dhaka` (+06) so a time that moves with the session shows. Each cell under three ways of declaring: **V**, the value's own type, as the first spike did; **C**, the column's type (DF1, column known); **R**, raw (DF1). The table is pinned in the spike, identical on Postgres 15, 16, 17 and 18. Every R-0186 type is bound to a column of its own type here, which the first spike claimed but only did for six types. `morm::Timestamp` (stood in for by `SystemTime`, with the same encoding), `Range<T>` and `Numeric` get their encodings in step 13, which adds them to this test (DC).

| Pairs | V | C | R |
|---|---|---|---|
| Every type on its default column: `i16` `smallint`, `i32` `integer`, `i64` `bigint`, `f32` `real`, `f64` `double precision`, `bool`, `String` and `&str` `text`, `Vec<u8>` `bytea`, `DateTime<Utc>` and `SystemTime` `timestamptz`, `NaiveDateTime` `timestamp`, `NaiveDate` `date`, `NaiveTime` `time`, `Uuid` `uuid`, `Value` `jsonb`, `IpAddr` `inet`, `Decimal` `numeric(10,2)`, `Vec<T>` on `T[]`, `NULL` of each | right | right | right |
| `String`, `&str` on `citext`, `char(5)`; `Vec<String>` on `citext[]`, `char(5)[]` | inserts right, `=` wrong (0 rows) or `42883` on the arrays | right | scalars right; arrays `42883` |
| `String` on `varchar(5)`; too long gets `22001` | right | right | right |
| `String` on `json`, `jsonb` | `42804`, `42883` | refused | right |
| `Value` on `json` | stored text rewritten (`{"a": 2, "b": 1}`) | right | rewritten |
| `DateTime<Utc>` on `timestamp`, and `= ANY` of a `Vec` of them | wrong: stored `06:00`, 0 rows | refused | wrong |
| `SystemTime` on `timestamp` | wrong: stored `06:00`, 0 rows | right | wrong |
| `NaiveDateTime` on `timestamptz` | wrong: read as +06, 0 rows | refused | wrong |
| `i16` on `integer`, `bigint`; `i32` on `bigint`; `f32` on `double precision`; `IpAddr` on `cidr`; `Vec<i16>` on `integer[]`; `Vec<i32>` `= ANY` on `bigint` | right | refused | right |
| `Vec<String>` `= ANY` on `text`; `Vec<i64>`, `Vec<Uuid>` `= ANY` on theirs | right | right | right |
| `Vec<String>` `= ANY` on `citext`, `char(5)` | wrong: 0 rows | right | wrong: 0 rows |

"Refused" means the value's encoder doesn't take the column's type, so the bind fails before anything is sent (`error serializing parameter`). It's an error, never a wrong answer. Under `TimeZone = 'UTC'`, every time row that was wrong under V and R is right.

Raw SQL with a `String`, as R sends it: `SELECT $1`, `lower($1)`, `length($1)`, `$1 || 'x'`, `coalesce($1, 'd')`, `$1::int`, `LIKE $1`, and comparisons with `citext`, `char(5)`, `jsonb`, `timestamptz`, `int4` and `int8` are right. `$1 IS NULL` and `concat($1, 'x')` fail with `42P18` (`could not determine data type of parameter $1`), where `text` worked: Postgres can't infer a type there, and `$1::text` fixes it. Sent as `text`, the `citext` and `char(5)` comparisons are wrong, and those with `jsonb`, `timestamptz`, `int4` and `int8` fail with `42883`.

With DF1, no known-column pair gives a wrong answer. These need the owner:

- **Question 1: a value whose encoder refuses its column's type.** An explicit `sql` can give a field a column its Rust type's encoder doesn't take: the "refused" pairs above. Options:
  - **A (recommended):** the conversion layer (step 03, contracts) encodes the value as its column's type where the two have a conversion: wider integers, `f32` to `double precision`, an instant to `timestamp` as its UTC wall time and a `NaiveDateTime` to `timestamptz` as UTC (the same bytes, as `SystemTime` on `timestamp` already shows), `IpAddr` to `cidr`, and `String` to `json` or `jsonb` as `UNKNOWN` text. `#[model]` refuses at compile time an explicit `sql` it has no conversion for. Right answers everywhere, and none depends on the session's time zone.
  - **B:** fall back to the value's own type when its encoder refuses, and let Postgres convert. Right for integers, floats and `cidr`; wrong for the three time pairs whenever the session's time zone isn't UTC; `42804` for `String` on `json`.
  - **C:** refuse every such pair at compile time. Safe, but `sql = "bigint"` on an `i32` field stops compiling.
- **Question 2: instants against `timestamp`, and naive times against `timestamptz`, in raw SQL.** With no column known, Postgres converts in the session's `TimeZone`, so the answer moves with it: wrong under `Asia/Dhaka`, right under UTC. Options:
  - **A (recommended):** morm's connections start with `TimeZone = 'UTC'`, overridable in `morm.toml`, so raw SQL converts the way the known-column paths do. The README says so, because raw SQL that formats or truncates times then works in UTC unless it says `AT TIME ZONE`.
  - **B:** keep the server's `TimeZone`, and the README says raw SQL comparing an instant with a `timestamp` column, or a naive time with `timestamptz`, converts in the session's time zone.
- **Question 3: a raw `Vec<String>` against `citext` or `char(n)`, and a raw `Value` into `json`.** A list goes as `text[]`, so `citext_col = ANY($1)` compares case-sensitively and finds 0 rows, and `char(5)` with `'ab '` the same. A raw `Value` goes as `jsonb`, so a `json` column stores jsonb's normalized text. Sent as an `UNKNOWN` text array literal, the list is right against `citext` and `char(5)`, but `array_length($1, 1)` fails with `42804` and `unnest($1)` with `42725` until written `$1::text[]`. Options:
  - **A (recommended):** a raw text list goes as an `UNKNOWN` text literal, like a raw `String`: the silent wrong answers become right, and the places that can't infer a type fail loudly with a fix the message points to. A raw `Value` stays `jsonb`: a `json` column keeps the same JSON value, normalized, and morm's known-column paths keep the text as given. The README says both.
  - **B:** lists stay `text[]`, and the README says raw SQL against `citext` or `char(n)` casts the list, `= ANY($1::citext[])`.
- **Question 4: R-1104's wording.** R-1104 (step 14) says every statement is prepared "with each parameter's type from `Param`". With DF1, a parameter bound to a known column is declared as the column's type, which `Param::pg_type` doesn't give. Options:
  - **A (recommended):** reword it to "with each parameter's declared type, as `query_typed` sends it: the column's where morm knows the column, otherwise `Param`'s", and update the manifest and `coverage.md`.
  - **B:** leave it, and let `contracts.md` carry the rule.

### 10. Registration: pass, with two refinements

Models in three crates were all found by a stand-in `morm-mgr` built with `--cfg morm_mgr` in its own target folder, including a crate no code references. `cargo build --workspace` without the cfg put no metadata in the app or `mgr`. As a control, the app built with the cfg did carry it, so the search would have found it. Each model registers through `inventory`.

- **Refinement:** a `#[cfg(morm_mgr)]` in `#[model]`'s output warns `unexpected_cfgs` in every crate with a model, which breaks `-D warnings`. So `#[model]` calls `morm::__morm_register!`, a `macro_rules!` that `morm` defines twice, under `cfg(morm_mgr)` and `cfg(not(morm_mgr))`. The cfg is read when `morm` is built, and `morm` declares it in its own `[lints.rust] unexpected_cfgs` `check-cfg`. User crates never see it, and the workspace builds with warnings denied.
- **Refinement:** a crate that `morm-mgr` only lists in `Cargo.toml` is dropped by the linker with its models; tested, its models went missing. The generated `morm-mgr` must name each crate with models, `use <crate> as _;`.

### 11. The loop detector: detection and cost pass, exclusions in step 45 (DD)

Each worker thread keeps eight slots keyed by (task, statement), where a statement is its `&'static str`'s address, so there's no hashing or string compare. The same statement reaching the threshold inside the window on one task is reported once. Nine runs under a threshold of ten, slow repeats outside the window, and one run on each of many tasks are not reported. A task that moves thread starts counting again, which can miss a loop but never invents one. Every query through `Db` passes the detector, and an N+1 loop of twelve queries was reported. The cost was 27 to 47 ns per statement.

**The exclusions (owner decision DD)** are proven in step 45, which builds the detector. The list is R-0785's: `fetch_page`, `.in_batches`, streams, the outbox worker, retries and `morm::allow_repeats(fut)`. Each is either a flag at morm's own call sites or a scope marked while a future is polled, the technique proof 7 proves. Neither changes a step-03 signature, provided step 03's internal query entry point can carry a "don't count this" flag; `contracts.md` says so.

### 12. `deadpool` with our own manager: pass

`deadpool`'s `managed` core with our `Manager` for `tokio_postgres::Client`. `recycle` runs no query; it only checks `is_closed()`. Tested on Postgres 15 to 18:

- Nothing opens until needed, and three concurrent checkouts open three.
- A hundred checkouts opened nothing new and ran nothing on the server: each backend's `state_change` and last query in `pg_stat_activity` were unchanged. A checkout took 0.33 to 0.52 µs, against 40 to 166 µs for an empty round trip.
- `resize` while running: down while idle closes the extra connections at once, down while busy closes them as they come back, and up lets more open.
- Idle shrinking: `retain` on `last_used` closed every connection idle past the limit, on demand and from a timer task, and the pool opened again afterwards.
- Noted for step 11: `retain` allocates a `Vec` per call, so it belongs on the reaper's timer, never on a checkout.

### Owner decisions on step 02

- **DA:** retry is `db.retry(n).transaction(async |tx| ..)` with an `AsyncFnOnce + Clone` closure, each attempt running a fresh copy (proof 4).
- **DB:** a model with hooks is marked `#[model(hooks)]`, with a required `OVERRIDES` const; models without it pay nothing (proof 5).
- **DC:** `Param` binding for morm's own `Timestamp`, `Range<T>` and `Numeric` is proven in step 13, when they get encodings (proof 9).
- **DD:** the loop detector's exclusions, R-0785's list, are proven in step 45, which builds it, provided step 03's internal query entry point can carry a "don't count this" flag (proof 11).
- **DE:** PR #2's last commit deletes `spikes/` and `spikes.yml`; the PR history keeps them.
- **DF1:** a parameter's declared type comes from its column where morm knows it, and raw `String` and `&str` go as `Type::UNKNOWN`; proof 9's matrix (proof 9).
- **DF2:** every transaction option goes before the closure on the same prefix builder as retry, the closure always last (proof 4). R-0435, R-0442, R-0453 and R-0767 are rewritten to match.
- **DF3 to DF10:** fixed as the review recommends: the hook rules (proof 5), the stream check measured with streams open (proof 7), the `G` mechanism (proof 3), proof 9's claims, every hook compiled (proof 5), the rerun line above, the open-items summary on `main`, and what `.retry` costs and misses (proof 4, and R-0865's README line).
