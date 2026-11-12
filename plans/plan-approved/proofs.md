# Proofs

What the proof steps found: for each proof, the result, the final shape of the API, and any design change. Every step reads this file. Where a result changes a design, the change stands only once the owner approves it, and the entry says which.

## Step 02: proofs 1 to 12

The spikes were built on stable Rust 1.99.1 with no nightly features, in their own workspace under `spikes/`, and run on Postgres 15.19, 16.15, 17.11 and 18.6 in CI, and on 17.11 and 18.6 locally. Timings below are from release builds, on the CI runners and locally. A pass below means every listed check ran green.

The owner decided DA to DE on the first results, DF1 to DF10 on the independent review, DG1 to DG4 on proof 9's questions, and DH1 to DH9 and the parameter-typing design (A to K) on the second review (`INDEX.md`, Decisions; `reviews/step-02-review-2.md`, `reviews/param-typing-design.md`). Each entry below says where a decision changed the design.

Spelling note: the spikes couldn't see the frozen plan, so attribute spellings such as `#[view(Name, hide(..))]`, `#[unique(..)]`, `#[col(group = ..)]` and `#[model(borrowed)]` are the spikes' own. What's proven is the mechanism and the generated shape. The step that builds each feature uses its own step file's spelling.

How to rerun: `spikes/` and `.github/workflows/spikes.yml` were deleted by PR #2's last commit (DE), and the PR's history keeps them. The last commit that has them, with proof 9 rerun under DH2, is `ca3747b`: `git checkout ca3747b38fe66e28b579d9bae5e5e9632946049b`, then `SPIKE_DSN=... cargo test --locked --release --manifest-path spikes/Cargo.toml --workspace` and `bash spikes/proof10.sh`, on Postgres 15 or later with `citext` available. `spikes.yml` at that commit runs both on Postgres 15 to 18.

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
| 9 | `morm::Param` with `query_typed` | Pass with a carrier and a cast for every parameter (DH2), rerun at `ca3747b` |
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

Also seen: rerunning at once after a serialization failure lost again while the winning transaction was still committing, and a fixed 20 ms pause once let two transactions that had both lost retry in step and collide on every attempt, on Postgres 16 in CI. A pause jittered by the clock's nanoseconds failed the same way once, on 17 in CI. The likely cause: where the clock steps in microseconds, its nanoseconds are a multiple of 1,000, and the pause never varies. The spike now pauses 10 to 50 ms by a fresh random number per pause, from `RandomState`; 80 local runs of its retry tests on 17 and 18 and every CI run since passed. R-0442's jittered backoff covers this, and its jitter must come from a real random source; step 20 tests it (T-0289, DH7). The spike formats `BEGIN` and each `SET LOCAL` into one message per transaction; step 20 chooses how they're sent.

### 5. Hooks: pass, with `#[model(hooks)]` (DB) and two rules (DF3)

- Users write `async fn` in `impl Hooks` against the contract's `fn ..(..) -> impl Future<Output = Result<()>> + Send` methods. A hook that holds a non-`Send` value across an `.await` fails at the hook itself (`future returned by 'pre_insert' is not 'Send'`), not deep inside the web framework.
- **Every contract hook compiles and runs (DF7).** The spike's `Hooks` has the contract's exact signatures: `validate`, `pre_insert(rows: &mut [Self], db)`, `post_insert`, `validate_changes(&Changes<Self>)`, `pre_update(&mut Changes<Self>, &Target<Self>, db)`, `post_update(.., affected: u64, db)`, `pre_delete(&Target<Self>, db)`, `post_delete(.., affected, db)` and `after_commit(Committed<Self>)`. Its `Changes` holds boxed `dyn Param + Send` values and its `Target` holds ids, a list, a filter with parameters, or all rows, so `Send` is checked against realistic contents; the spike's key is `id int8`. A generic update and delete run the hooks the impl overrides, then `after_commit` with `Committed::Updated` or `Committed::Deleted`. Tested: `validate_changes` and `pre_update` refuse before any checkout, `pre_update` adds a change that the statement writes, both update hooks and the statement share one checkout, and the hook log reads `post_update`, `committed update`, `post_delete`, `committed delete`. Axum handlers awaiting a generic insert, and a generic update and delete, compile and serve requests.
- **Marking (owner decision DB).** `#[model]` emits an empty `impl Hooks` for every model, so models without hooks need nothing and run no hook path. A model with hooks says `#[model(hooks)]`, so `#[model]` leaves the impl to `#[morm::hooks]`. `#[morm::hooks]` records which hooks the impl defines in the required `const OVERRIDES`, and the generic paths call only those. An `impl Hooks` written without the attribute doesn't compile (`missing 'OVERRIDES' in implementation`).
- **Rule 1 (DF3): no macro calls in the impl.** `#[morm::hooks]` sees the impl before any macro inside it expands, so a hook written by a macro would go unrecorded and never run. It refuses any macro call among the impl's items: `` `#[morm::hooks]` can't see hooks a macro writes, so they'd never run: write each hook as an `fn` in this impl ``. It also refuses an `OVERRIDES` written inside the impl.
- **Rule 2 (DF3): `Overrides` can't be written by hand.** `Overrides<M>` has a private field, no public constructor and no public constant. `#[morm::hooks]` and `#[model]` build it through a `#[doc(hidden)]` `morm::__private::overrides(&[Hook::PreInsert, ..])`. It's typed by its model, so another model's `OVERRIDES` can't be copied in (`expected 'Overrides<Borrowed>', found 'Overrides<Note>'`). Code asks `M::OVERRIDES.has(Hook::PreInsert)`. Each way around the rules is a pinned compile error, and step 24 builds and tests both rules with a trybuild case each (T-0291, T-0292, DH7).
- The lazy `db` checks out on its first query and keeps that connection: a hook that refuses without a query checked out nothing, and when a hook queries, the statement reuses its connection, one checkout in all.

### 6. Borrowed `ProductRef<'r>`: pass

`ProductRef<'r>` maps `String` to `&'r str`, `Vec<u8>` to `&'r [u8]`, `Option` of either to `Option` of the borrowed form, and keeps other types. A counting allocator measured, over 1,000 rows of text, `bytea` and nullable fields: decoding allocated 0 times, decoding plus `serde_json::to_writer` into the caller's buffer allocated 0 times, and the owned model allocated at least twice per row. Its `Serialize` impl writes the fields directly, with no intermediate value.

### 7. `NestedConnection`: pass, with two refinements

A stream checks out nothing until first polled. On its first poll it registers the polling task, then checks out and runs its query; on each later poll, if the task changed, the registration moves. A checkout reads one relaxed atomic, the count of open streams, and only when it's non-zero looks up the current task. Tested: a query inside a stream loop on a pool of one is refused at once instead of waiting forever; a nested stream is refused; another task may query meanwhile; `join!` of the loop and a query on one task refuses the query; a stream made on one task and polled on another belongs to the second, and moves back when polled on the first; a finished or dropped stream frees the task. The check with no stream open took 0.55 to 1.41 ns.

- **Refinement:** a future driven by `block_on`, including the body of `#[tokio::main]` and `#[tokio::test]`, has no Tokio task id, so the registry keys by task id, or by thread id when there's none. Keyed by task id alone, the loop in `main` deadlocked in the test.
- **Refinement (DF4): the registry is sharded.** While any stream is open anywhere, every checkout on every task looks up its task. Measured with streams open on other tasks, and checks from one thread, then from eight threads at once. With the first spike's one process-wide `Mutex<Vec<..>>`, at commit `1b60b71`, a check took 12 ns and 322 ns with one stream open, 45 ns and 1.07 µs with 100, and 278 ns and 5.4 µs with 1,000 locally (12 threads), and up to 358 ns and 9.5 µs with 1,000 on the CI runners. Sharded 64 ways by a cheap hash of the task, each shard its own mutex and list, it took 8 to 16 ns from one thread and 11 to 30 ns each from eight, from one stream open to 1,000, locally, and 9 to 34 ns and 15 to 116 ns on the CI runners. Step 21 builds the registry, and must not put a single process-wide lock on this path; the sharded form is the measured starting point, and T-0290 measures it with streams open (DH7).

### 8. Upsert conflict targets: pass, with a refinement

Each column's `Column<M, T, C>` has its own zero-sized marker type `C`, generated per field, so each column is a distinct type and can carry its own `UniqueKey<M>` impl. A unique column implements it with `CONFLICT = ("col")`, and `Model::UK.<name>` constants of type `Key<M, marker>` cover expression, partial and composite keys: `(lower("email"))`, `("code") WHERE "deleted" = false`, `("tenant", "code")`. A unique column declared in a base from another crate, reached through two base expansions, is a key too.

- A column with no unique key is a compile error, worded through `#[diagnostic::on_unimplemented]`: ``'Column<Order, String, ..::code>' is not a unique key of 'Order'``, with a note on how to declare one.
- On Postgres 15 to 18, each of the four targets is accepted as the arbiter of `INSERT .. ON CONFLICT .. DO UPDATE`, with a row that conflicts on that key only.
- **Refinement:** the contract's `Cap` parameter becomes the per-column marker type, and capabilities such as refusing `.set()` become traits implemented on the marker, rather than a shared capability type.

### 9. `morm::Param` with `query_typed`: pass with a carrier and a cast for every parameter (DH2)

**The risk the first spike missed.** It took each parameter's declared type from the Rust value, so `String` was always `text`. When the declared type differs from the column's, Postgres casts the column, not the parameter, and the comparison changes meaning with no error. Sent as `text`, `'alice'` found 0 rows in a `citext` column holding `'Alice'`, and `'ab '` found 0 in a `char(5)` holding `'ab'`.

**The design (owner decisions DF1, DG1, DH2 and DH3; `reviews/param-typing-design.md`).** Postgres never infers a parameter's type. Every parameter is declared in Parse as a built-in *carrier* and cast in the SQL as `($n::cast)`:

- **Bound through a field** (builders, `F`, `sql!`'s `${v: M::F.field}`, model inserts and updates), the cast is the column's type, an explicit `sql` included, with no length, precision or scale (`char(5)` casts to `bpchar`, `varchar(5)` to `varchar`, `numeric(10,2)` to `numeric`), and the carrier is that type's built-in carrier. The conversion layer encodes the value as the carrier: a narrower integer as the wider column's, `f32` as `float8`, an instant on `timestamp` as its UTC wall time, a naive time on `timestamptz` as UTC, `IpAddr` on `cidr`, each element alike in a list or an array (DG1). The value must fit the field at compile time (`Fits`, choice K).
- **Free** (`${v: T}`, `.q()`, `set_expr`, `sql_unchecked`), the cast and carrier are the Rust type's own, from `SqlType`: `String` is `($1::text)`.
- **Types with no fixed OID** travel as `text`, or a domain's base type's carrier, and are cast by name: `citext`, an enum, a composite, an extension type, a domain (choice C). No OID is looked up or sent.
- Every carrier is built in, so every parameter goes in binary format. The earlier design's `Type::UNKNOWN`, its text-format rule, DG3's untyped raw strings and lists, and the per-parameter format wrapper are gone. The spike's `Param` has `cast()` and `carrier()` with a blanket impl from `SqlType`, and parameters still go straight from `&[&dyn Param]` into `query_typed_raw` through an iterator of a small `Bound` wrapper, with nothing boxed or collected.
- The parentheses around each cast matter: without them a user's `$1[1]` would read as the type `int8[][1]`. The design session's evidence for that, for index use with casts, and for type swaps under cached statements is in `param-typing-design.md`, section 2; steps 14, 55 and 56 test it on 15 to 18.

**The matrix, rerun under DH2 (choice J).** At `ca3747b`, the last commit of PR #2 with the spikes. Each R-0186 Rust type, against its default column and each column an explicit `sql` can give it, plus an enum, an enum array, a domain over `integer` and a domain over `citext`, inserted and then compared with `=` (or `= ANY(..)` for a list against a scalar column), with the session's `TimeZone` at `Asia/Dhaka` (+06), so a time that moves with the session shows. Each cell two ways: **Field**, bound through the field, and **Free**, cast to its own type. Each runs through `query_typed` and through a prepared statement, which agreed in every cell. Beside each, the findings of a small version of check-sql's typing pass (rules T1 to T3, `param-typing-design.md` section 6), read from the analyzed parse tree Postgres sends back with `debug_print_parse` on. The test fails if a free value's answer differs from the field's and no rule reports it. The table is pinned in the spike, `p09_matrix.expected`, and was identical on Postgres 15.20, 16.16, 17.12 and 18.7 in CI (`spikes.yml` at `ca3747b`, on push and on pull request, with every other spike green) and on 17.11 and 18.6 locally. `morm::Timestamp` (stood in for by `SystemTime`, with the same encoding), `Range<T>` and `Numeric` get their encodings in step 13, which adds them (DC).

| Pairs | Field | Free | Free's findings |
|---|---|---|---|
| Every type on its default column: `i16` `smallint`, `i32` `integer`, `i64` `bigint`, `f32` `real`, `f64` `double precision`, `bool`, `String` and `&str` `text`, `Vec<u8>` `bytea`, `DateTime<Utc>` and `SystemTime` `timestamptz`, `NaiveDateTime` `timestamp`, `NaiveDate` `date`, `NaiveTime` `time`, `Uuid` `uuid`, `Value` `jsonb`, `IpAddr` `inet`, `Decimal` `numeric(10,2)`, `Vec<T>` on `T[]`, `NULL` of each | right | right | none, except `Decimal` into `numeric(10,2)`, T3 (a scale applied) |
| `i16` on `integer`, `bigint`; `i32` on `bigint`; `f32` on `double precision`; `Vec<i16>` on `integer[]`; `Vec<i32>` `= ANY` on `bigint` | right | right | none: a widening |
| `String`, `&str` on `citext`, `= 'alice'` | right | wrong: 0 rows | T1 |
| `String`, `&str` on `char(5)`, `= 'ab '` | right | wrong: 0 rows | T1, and T1 on `= 'ab'` too, which is right |
| `String` on `varchar(5)`; too long gets `22001` | right | right | none |
| `Vec<String>` `= ANY` on `citext`, `char(5)` | right | wrong: 0 rows | T1 |
| `Vec<String>` on `citext[]`, `char(5)[]`, `= ` the array | right | `42883` | a wrong shape is an error |
| `String` on `json`, `jsonb` | right, `json` keeps the text as given | `42804` on insert, `42883` on `=` | an error |
| `Value` on `json` | right | stored text rewritten (`{"a": 2, "b": 1}`) | T3 |
| `DateTime<Utc>`, `SystemTime` on `timestamp`, and `= ANY` of a `Vec` of instants | right | wrong: stored `06:00`, 0 rows | T3 on insert, T1 on `=` |
| `NaiveDateTime` on `timestamptz` | right | wrong: read as +06, 0 rows | T3, T1 |
| `IpAddr` on `cidr` | right | right | T3 on insert (`inet` to `cidr` goes through a function) |
| `String` on an enum, `= ` a label, `= ANY` of labels; `Vec<String>` on an enum array; a label the enum lacks | right; the missing label gets `22P02` | `42804` or `42883` | an error |
| `i32` on a domain over `integer`, and one breaking its `CHECK` | right; `23514` | right; `23514` | none |
| `String` on a domain over `citext`, `= 'alice'` | right | wrong: 0 rows | T1 |
| `Vec<Option<String>>` on `citext[]` holding `a\b`, an empty string, `{}`, the string `NULL`, a quote and a comma, and a real `NULL`, compared with the same elements in another case | right | `42883` on `=` | an error |

The field never needs a finding: no field-bound cell was wrong, and none drew T1 or T2. Under `TimeZone = 'UTC'`, the zone morm's sessions always use (DH1), every free time row is right, and still reported, T3 on insert and T1 on `=`, because its answer would move with any other zone.

Free values where Postgres has nothing to infer from need no cast of their own, because each carries its own: `($1::text)`, `(($1::text) IS NULL)`, `concat(($1::text), 'x')`, `coalesce`, `(($1::text))::int`, `array_length(($1::text[]), 1)` and `unnest(($1::text[]))` are right, where the earlier design's untyped strings and lists failed with `42P18`, `42804` and `42725`. A wrong shape is an error: `1234::int4 = ($1::text)` gets `42883`, a list where one value is expected, `'x'::text = ($1::text[])`, gets `42883`, and `= ANY(($1::text))` with a scalar gets `42809`. With an untyped literal on the other side, `'x' = ($1::text[])`, Postgres reads the literal as an array and fails with `22P02`.

**What came before, superseded.** DF1 first declared a field-bound parameter as the column's OID, or `Type::UNKNOWN` in text format for a type with no fixed OID, and a raw `String` or `&str` as `UNKNOWN`; DG3 then sent raw text lists the same way. The second review found that a raw list where one value is expected was then silently read as one text value (DH2), and the owner chose the carrier and cast instead. DG1's conversions stay, encoding into the carrier. DG2's `timezone` override is gone (DH1). DG4's wording of R-1104 is replaced by DH2's.

- **DH1: sessions always run in UTC.** morm's connections send `TimeZone = 'UTC'` as a startup parameter, with no setting or variable to change it (R-0060, R-1205, R-1206 in step 10), so raw SQL converts times as morm's own encodings do, and the README says why loudly (R-1210). Behind PgBouncer in transaction mode no startup parameter is sent, so the role carries `timezone = 'UTC'` (R-0670). Checked with `psql` on 18.6 and 17.11, on servers whose own `TimeZone` was `Asia/Dhaka`: with `-c TimeZone=UTC` as a startup option, `'2026-01-01 00:00:00+00'::timestamptz::timestamp` is `2026-01-01 00:00:00` and `'2026-01-01 00:00:00'::timestamp::timestamptz` is `2026-01-01 00:00:00+00`, where the server's zone gave `06:00:00` and `+06`. A role with `ALTER ROLE .. SET timezone = 'UTC'` starts in UTC, and a startup option wins over it. Wall-clock time as a field type moved to `later-defo-build.plan`.

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
- **DF1:** a parameter's declared type comes from its column where morm knows it, and raw `String` and `&str` go as `Type::UNKNOWN`; proof 9's matrix (proof 9). The declared type part is superseded by DH2.
- **DF2:** every transaction option goes before the closure on the same prefix builder as retry, the closure always last (proof 4). R-0435, R-0442, R-0453 and R-0767 are rewritten to match.
- **DF3 to DF10:** fixed as the review recommends: the hook rules (proof 5), the stream check measured with streams open (proof 7), the `G` mechanism (proof 3), proof 9's claims, every hook compiled (proof 5), the rerun line above, the open-items summary on `main`, and what `.retry` costs and misses (proof 4, and R-0865's README line).
- **DG1 to DG4:** encode a value as its column's type, times as UTC, and refuse at compile time a `sql` morm can't convert to; connections start with `TimeZone = 'UTC'`, overridable in `morm.toml`; raw text lists go as `UNKNOWN` text and a raw `Value` stays `jsonb`; R-1104 reworded (proof 9). DG2's override and DG3 are superseded by DH1 and DH2.
- **DH1:** sessions always run in UTC, with no `timezone` setting and no `MORM_DB_TIMEZONE`; wall-clock time moves to `later-defo-build.plan` (proof 9).
- **DH2, with the design's choices A to K:** every parameter is declared as a built-in carrier and cast as `($n::cast)`, through the field or free; `sql!` takes values inline (A); `db.sql()` and `db.raw()` take only `sql!` output, and `sql_unchecked` is the escape hatch (B); carriers, not `UNKNOWN` (C); `sql!` refuses a free value beside an `F` column (D); check-sql reads the analyzed parse tree with rules T1 to T4 (E); the scanner writes casts into fragments (F); `get_where` and `delete_where` bind `comp` through its field (G); `#[morm::test]` checks the SQL a test runs (H); R-0718 retries once on any error from a cached statement that casts to a non-built-in type (I); proof 9's matrix rerun in PR #2 (J, proof 9); an `Option` value fits only an `Option` field (K). `sql!`'s core moves to step 16.
- **DH3:** a field's column type is checked through `SqlType`: lossless declarations only, unknown SQL types only on a type with a text form, a compile error naming the field, and `check_schema` checks again at startup (steps 13 and 41).
- **DH4:** step 18 runs the parameter matrix through morm's own query paths, cached statements included (T-0286), and step 45 shows check-sql and `#[morm::test]` report its free mismatches (T-0296); T-0278 runs straight on `tokio_postgres` in step 13.
- **DH5:** `tx.transaction(..)` is a savepoint, `tx` has no `isolation` or `retry`, and a setting on `tx` holds for the rest of the transaction (R-0436, `contracts.md`, R-1216's README notice). Checked on 18.6: a `SET LOCAL` inside a released savepoint holds after it, and rolling back to the savepoint undoes it.
- **DI1:** `check_schema`'s one startup statement also reads `current_setting('TimeZone')` and refuses the pool with `Error::Config` when it isn't `UTC`, which catches a PgBouncer role not set to UTC with no extra round trip (R-1234 in step 41, R-0670, R-1210). Checked on throwaway 18.6 and 17.11 servers whose own zone was `Asia/Dhaka`, the same on both: `UTC` given as a startup option, or as the role's `timezone`, reads back `UTC`, and so does `utc`; a startup option still wins over the role's `UTC` (`Europe/Berlin`). Other names for the same offset read back as written, not as `UTC`: `Etc/UTC`, `UTC0`, `Etc/Universal`, `Universal`, `Zulu`, `GMT` and `UTC+0`, whether set by startup option or by role, and `+00` reads `<+00>-00`. The owner chose a fixed list of aliases (DI3).
- **DI3: the aliases DI1's check passes.** Only names proven to be a permanent zero offset with no daylight saving and no other history. Both throwaway servers, 18.6 and 17.11, are built `--with-system-tzdata=/usr/share/zoneinfo`, tzdata 2026c. In its source (`tzdata.zi`), `Etc/UTC` is one line, `Z Etc/UTC 0 - UTC`, and `Etc/GMT` one line, `Z Etc/GMT 0 - GMT`: offset 0, no rule, no end date. `Etc/UCT`, `UCT`, `Etc/Universal`, `Universal`, `Etc/Zulu`, `Zulu` and `UTC` are links to `Etc/UTC`, and `Etc/GMT0`, `GMT0`, `Etc/GMT+0`, `GMT+0`, `Etc/GMT-0`, `GMT-0`, `Etc/Greenwich`, `Greenwich` and `GMT` links to `Etc/GMT`. In the compiled files, each family's files are byte for byte the same, and `zdump -v -c -1000,3000` shows no transition in any, with `isdst=0 gmtoff=0`. On both servers, each name set as a startup option reads back in that spelling, shows an offset of 0 and one abbreviation (`UTC` or `GMT`) at every 30 days from 1800 to 2200, and `pg_timezone_names` gives `00:00:00` with `is_dst` false. Postgres reports a zone in tzdata's spelling however it was set (`utc` gives `UTC`, `etc/utc` `Etc/UTC`, `ZULU` `Zulu`, by startup option and by role), so R-1234 compares exactly. Near misses, refused: `Europe/London` showed offsets 0, 3600, 7200 and -75 seconds (LMT) over that span, though `pg_timezone_names` gives it `00:00:00`, not daylight saving, in November, which is why a check by today's offset would be wrong; `Africa/Abidjan` and `Atlantic/Reykjavik` 0 and -968 (LMT); `WET` five offsets; `Etc/GMT+1` -3600. **`UTC0`, one of the five the owner named, fails the tzdata proof:** it isn't a tzdata name but a POSIX TZ string, which Postgres parses itself, so it's absent from `pg_timezone_names`; it showed offset 0 and abbreviation `UTC` over the whole span, as do the POSIX strings `UTC+0` and `+00` (which reads back `<+00>-00`). It's left out of R-1234's list, pending the owner (PR #2). The other four named, `Etc/UTC`, `Universal`, `Zulu` and `GMT`, pass.
- **DI2:** a `timezone` key in `morm.toml` is refused with `Error::Config`, naming the key and saying sessions always run in UTC (R-1235 in step 10).
- **DH6 to DH9:** fixed as the review recommends: `Param`'s comment, now the carrier design's; T-0289 to T-0292 for the rules handed to steps 20, 21 and 24; loud README notices R-1210, R-1211, R-1215 and R-1216, and `.statement_timeout(...)`'s line R-1217; the PR body.
