# Step 02 independent review

PR #2 (`step-02-proofs`), head `f50823e`. Reviewed as `rules.md` "Independent review" says, by an agent that didn't write the step. I read `rules.md`, `contracts.md`, `proofs.md` and `steps/02-proofs.md` on the PR branch, "Decisions since the split" in `INDEX.md` on `origin/main` (including `1734252`, which records the owner's answers to DA to DE), `reviews/step-02-open-items.md`, and the whole diff, `git diff origin/main...origin/step-02-proofs`. To check whether the open questions were framed right, I also read the later step files they touch: 05, 08, 14, 15, 20, 45, 55 and 56.

## What I checked and ran

- **CI on `f50823e`:** every check passed: `spikes` on Postgres 15 to 18, `ci` (fmt, clippy, unit, trybuild, doc, integration on 15 to 18 over TCP and Unix sockets, pooler, msrv, features) and `coverage`.
- **Checker:** `check_coverage.py --step 02 --code crates tests benches fuzz .github`: 1204 requirements, 277 test obligations, 0 problems. R-0917 is `info`, so it needs no tag.
- **Manifest:** `git diff origin/main...origin/step-02-proofs -- plans/plan-approved/tools/manifest.json` is empty. No ID changed, so no Decisions entry is needed for one.
- **Re-run locally,** on a throwaway Postgres 18.6 with rustc 1.99.1: `cargo test --locked --release --manifest-path spikes/Cargo.toml --workspace` passed (every proof plus all 10 trybuild cases), and so did `bash spikes/proof10.sh`. My timings: `check()` with no stream open 0.93 ns, `observe()` 28.2 ns, checkout 398 ns against a 49 µs round trip. All are inside the ranges `proofs.md` gives.
- **No nightly:** there's no `#![feature]` anywhere in `spikes/`, and the workflow pins 1.99.1.
- **`contracts.md` refinements:** each one is backed by a proof and contradicts no owner decision. `Global` and `Scoped` gain a `MaskBits` bound, and `groups!` gets a newtype mask (proof 3). `Cap` becomes a per-column marker and `Key<M, K>` is added (proof 8). The registration paragraph comes from proof 10. `Decoders` and `from_row_as`, which come from an earlier decision, are kept.
- **My own spikes,** in a scratch crate, for claims that looked doubtful (results under each finding):
  - `AsyncFn` with a direct call, without the spike's wrapper closure.
  - A postfix `.isolation()` with no retry.
  - `Param` types against `citext`, `char(n)`, `timestamp` and `json` columns.
  - Enum arrays sent as `UNKNOWN`.
  - Naming the global mask through a `use`.

Proofs 1, 2, 6, 8, 10 and 12 are built as written, and their tests prove what they claim. I found nothing to fix in them.

## Findings

### DF1. A `Param` type taken from the value gives wrong results on `citext`, `char(n)`, `timestamp` and `json` columns

- **Severity:** blocks approval
- **ID:** proof 9, R-0917; also affects R-0716, R-1104 and R-0206
- **Files:** `plans/plan-approved/proofs.md` (proof 9), `plans/plan-approved/contracts.md` (`Param`), `spikes/morm/src/param.rs`

**What's wrong.** Proof 9 passes "for every supported type", and DC was answered by giving `Timestamp`, `Range<T>` and `Numeric` to step 13. But both proof 9 and the DC question treat "supported" as a list of Rust types. They miss the real risk:

- `Param::pg_type(&self)` takes the declared type from the Rust value. `String` is always `text`, and `SystemTime` and `Timestamp` are always `timestamptz`.
- R-1104 prepares every statement with those same types, so the cached path and the `query_typed` path behave alike.
- But R-0186 lets an explicit `sql` put a `String` on `citext`, `varchar(n)` or `char(n)`, and R-0206 promises that `citext` works.
- When the declared type differs from the column's type, Postgres casts the column, not the parameter. That changes the comparison's meaning, with no error.

**Evidence.** Postgres 18.6, with `PREPARE p(text)`, which is what `query_typed` sends, against an inferred `PREPARE`:

| Column | Parameter | Declared type | Inferred type |
|---|---|---|---|
| `citext` `'Alice'`, indexed | `'alice'` | 0 rows. The plan is `Seq Scan`, `Filter: ((c)::text = 'alice'::text)`, so the index is lost too | 1 row |
| `char(5)` `'ab'` | `'ab '` | 0 rows | 1 row |
| `timestamp` | `2026-01-01 00:00Z` as `timestamptz`, `TimeZone=Asia/Dhaka` | Matches nothing. An insert stores `06:00:00` | – |
| `json` | `jsonb` | An insert rewrites the stored text. `=` fails with `operator does not exist: json = jsonb` | – |

Through `query_typed_raw` itself, `String` as `TEXT` on `citext` returned 0 rows, and the same `String` as `Type::UNKNOWN` returned 1.

**Why it blocks.** Step 03 freezes `Param` next. Its shape, a type per value, is what needs the decision. `.by(F.email, ..)` on a `citext` email column would silently become case-sensitive, which breaks R-0206 and goal R-0009.

**Options:**

- **A (recommended).**
  - Where morm knows the column (builders, `F`, `sql!`, inserts and updates from a model), the declared type comes from the column: `Column` carries the column's SQL type, including an explicit `sql`.
  - Raw parameters of the text family (`String`, `&str`) go as `Type::UNKNOWN`, as `PgEnum` already does, so Postgres infers the column's type. That was tested: it binds to `citext` and matches case-insensitively.
  - Extend proof 9 before merge: each R-0186 Rust type, bound to each column type an explicit `sql` can give it (`citext`, `varchar(n)`, `char(n)`, `json`, and `timestamp` from a timestamptz value), compared and inserted, on 15 to 18.
- **B.** Keep a type per value, and make a field whose `sql` leaves its type's family use a wrapper type (`morm::Citext`, `morm::Json`, ...) with its own `PgType`. This is simpler, but every `citext` user has to know the rule.
- **C.** Keep it as built and document it. Not recommended: the result is silently wrong.

Whichever is chosen, it changes `Param` in `contracts.md`, so it needs the owner before step 03.

### DF2. Postfix `.isolation(..)` and per-transaction settings hit the same wall as `.retry(n)`, so the DA answer covers only half of the transaction API

- **Severity:** blocks approval
- **ID:** proof 4, R-0917; affects R-0442, R-0767 and step 20's example at line 49
- **Files:** `plans/plan-approved/proofs.md` (proof 4), `plans/plan-approved/steps/20-transactions.md`

**What's wrong.** DA framed the problem as `.retry(n)` alone, and the owner answered with `db.retry(n).transaction(..)`. But step 20 writes its modifiers after the closure too: `}).isolation(Serializable).retry(3).await?;` (line 49), plus "per-transaction settings". Any postfix modifier on `db.transaction(closure)` needs `IntoFuture`, and so a named future.

On stable, a boxed `dyn Future + Send` can't be built around a generic `AsyncFnOnce` closure's future. The bound it needs, `<F as AsyncFnOnce<(&Tx,)>>::CallOnceFuture: Send`, names `CallOnceFuture`, which is `#[unstable(feature = "async_fn_traits")]` in 1.99.1's `core/src/ops/async_function.rs`.

Proof 4 says "the plain form works", but it was tested only as `db.transaction(f).await`, with no modifier.

**Evidence:**

- **Postfix `.isolation_serializable()` with no retry:** a builder whose `IntoFuture` boxes `(self.f)(&tx).await`, with `F: AsyncFnOnce(&Tx) -> Result<T> + Send`. The impl itself fails: ``the trait `Send` is not implemented for `<F as AsyncFnOnce<(&Tx,)>>::CallOnceFuture` `` ... ``required for the cast ... to `Pin<Box<dyn Future<Output = Result<T, ()>> + Send>>` ``. Without `Send`, it couldn't be used in Axum.
- **The DA evidence holds:** I also confirmed it isn't caused by the spike's wrapper closure. A direct `f(&tx).await` loop with `F: AsyncFn(&Tx)`, given a closure borrowing `row.id` under `tokio::spawn`, fails with ``implementation of `AsyncFnMut` is not general enough``.

**This needs an owner decision, to be written into step 20 with DA.**

- **A (recommended):** every option goes before the closure, on the same prefix builder DA chose: `db.isolation(Serializable).retry(3).transaction(async |tx| ..)`. Per-transaction settings work the same way. The closure is always last, and `transaction` is an `async fn`, so its future's `Send` comes from the concrete closure.
- **B:** only `retry` is prefix, and the other options become arguments: `db.transaction_with(opts, async |tx| ..)`.

Either way, rewrite R-0442's "(`AsyncFn`)" and the line 49 example to the chosen form, and say in R-0767 that a `move` closure that mutates its own copy compiles, and is safe because each attempt gets a fresh copy. Update `coverage.md` and the manifest in the same PR, as "Changing the plan" requires.

### DF3. `#[morm::hooks]` can miss a hook, so "a hook can never be skipped" is overstated

- **Severity:** should fix
- **ID:** proof 5, R-0917
- **Files:** `plans/plan-approved/proofs.md` (proof 5), `spikes/morm-macros/src/lib.rs` (`hooks`), `spikes/morm/src/hooks.rs`

**What's wrong.** `proofs.md` says that because the `OVERRIDES` const is required, "a hook can never be skipped because its override went unrecorded". Two ways around that remain:

1. `Overrides::NONE` is public, so a hand-written `impl Hooks` with `const OVERRIDES: Overrides = Overrides::NONE;` and a real `pre_insert` compiles, and its hook never runs.
2. `#[morm::hooks]` looks only at `ImplItem::Fn`. A hook written by a macro inside the impl, such as `impl Hooks for M { my_hooks!(); }`, reaches the attribute unexpanded, so it isn't recorded and is silently skipped.

**Fix.**

- `#[morm::hooks]` refuses any macro invocation among the impl's items, with a compile error.
- `Overrides` gets no public constructor. The attribute calls a `#[doc(hidden)]` `__private` function, so writing it by hand is clearly unsupported.
- `proofs.md` states both rules, so step 24 builds and tests them, with a trybuild case for each.

DB's answer (`#[model(hooks)]` plus a required const) stays safe with this fix.

### DF4. The `NestedConnection` check is measured only with no stream open

- **Severity:** should fix
- **ID:** proof 7, R-0917
- **Files:** `plans/plan-approved/proofs.md` (proof 7), `spikes/morm/src/nested.rs`

**What's wrong.**

- The pass condition is "one atomic load when no stream is open", and that is met.
- But while any stream is open anywhere in the process, every checkout on every task takes one global `Mutex<Vec<(Key, usize)>>` and scans it.
- On a server with long-lived streams (exports, SSE), that's the normal state, not the exception.
- `proofs.md` gives no cost for this path. As written, step 21 would inherit a global lock on the hot path without knowing it.

**Fix.** Say in `proofs.md` that the global mutex is spike-only and the cost with streams open is unmeasured. Then have step 21, or a quick follow-up here, measure checkouts with N streams open on other tasks, against a sharded or per-task registry, before the design is fixed.

### DF5. R-0271's "models find `G` through a normal `use`" isn't what proof 3 proves

- **Severity:** should fix
- **ID:** proof 3, R-0917; affects R-0271 and `Model::Global` in `contracts.md`
- **Files:** `plans/plan-approved/proofs.md` (proof 3)

**What's wrong.** The spike names the app's groups with a path argument, `#[model(global = crate::groups)]`, and `proofs.md` leaves "how a model in another crate finds the app's mask" to step 08. But step 03 freezes `type Global: MaskBits` first.

R-0271 already settles the user-facing side: `G` is found through a plain `use`, and the `groups!` call goes in a crate every model crate depends on. What's open is the mechanism: `#[model]` must write the mask's *type* from an identifier in scope, and on stable an associated type can't be taken from a constant.

**Evidence that it works (scratch spike, 1.99.1):** `groups!` emits a struct `G` and a `const G: G`, which live in separate namespaces, plus `impl GlobalGroups for G { type Mask = Mask; }`. Then a model in a module that only does `use crate::groups::G;` compiles with `type Global = <G as GlobalGroups>::Mask; const REQ: u16 = G.admin;`.

**Fix.** Record this mechanism in proof 3 and `contracts.md`, as the way `type Global` gets filled, so step 03 freezes something proven. Also state what a model that names no global group gets (`()`).

### DF6. Proof 9's text says more types were bound to real columns than the test binds

- **Severity:** minor
- **ID:** proof 9
- **Files:** `plans/plan-approved/proofs.md`, `spikes/app/tests/p09_param.rs`

**What's wrong.** `proofs.md` says each type "round-tripped ... and bound to real columns: `bool`, `i16`, ... `SystemTime`". But `params_bind_to_columns_and_enum_labels_need_no_oid` binds only `int8`, `text`, `jsonb`, `numeric`, `int4[]` and an enum. The other types only go through `SELECT $n`, which doesn't bind to a column. The test also creates `mm spike_mood[]` and never uses it.

**Fix.** Either bind every listed type to a column of its type, which fits naturally with DF1's matrix, or reword the claim. Enum arrays are fine to leave out: step 55's proof 13 and R-0978 cover them. I confirmed `Vec<PgEnum>` as `UNKNOWN` fails ("error serializing parameter 0"), while a text-format literal as `UNKNOWN` binds, both to `= ANY($1)` and to an `enum[]` insert, which matches R-0978.

### DF7. Proof 5 compiled only the insert hooks

- **Severity:** minor
- **ID:** proof 5
- **Files:** `spikes/morm/src/hooks.rs`

**What's wrong.** The spike's `Hooks` has `validate`, `pre_insert`, `post_insert`, `pre_delete(ids: &[i64], ..)` and `after_commit`. The contract's update and delete hooks take `&mut Changes<Self>`, `&Target<Self>` and `affected: u64`, and none of them was compiled. `after_commit` is defined but never called.

The mechanism is the same (return-position `impl Future + Send`), so I don't expect a failure. Still, `Send` for those futures depends on `Target<Self>: Sync` and `Changes<Self>: Send`, which step 03 now defines.

**Fix.** Add the contract's exact update and delete signatures to the spike's `Hooks`, and one Axum-handler compile check that awaits a generic update. Or say in proof 5 that only the insert hooks were compiled, so step 24 checks the rest.

### DF8. The `proofs.md` rerun line points at a branch that DE deletes

- **Severity:** minor
- **ID:** DE
- **Files:** `plans/plan-approved/proofs.md`

**What's wrong.** "How to rerun, from the PR's branch" stops working once the deletion commit lands and the branch is removed after merge.

**Fix.** Name the commit that holds the spikes (currently `f50823e`, or the last commit before the deletion), with `git checkout <hash>`.

### DF9. The open-items summary misstates what reached `contracts.md`

- **Severity:** minor
- **ID:** none
- **Files:** `plans/reviews/step-02-open-items.md` (on `main`)

**What's wrong.** Under "Choices the step agent made", it says the `NestedConnection` thread-id fallback was written into `contracts.md`. It wasn't: the PR body says so correctly, and `contracts.md` has no `NestedConnection` text beyond the error variant.

**Fix.** Correct the line, or add the fallback to `contracts.md` if step 21 should see it there.

### DF10. DA's "no allocation" holds only for borrowing closures

- **Severity:** minor
- **ID:** proof 4, DA
- **Files:** `plans/plan-approved/proofs.md` (proof 4)

**What's wrong.** `Retrying::transaction` clones the closure before every attempt, including the first. A borrowing closure costs nothing to copy. But a `move` closure that owns its rows clones them, and allocates, on every transaction, even one that never retries. That's inherent, since a retry needs a pristine copy, but the last attempt could use the original.

Interior mutability also slips past the compile-time refusal. The spike's own retry test carries state across attempts through a captured `&AtomicU32`.

**Fix.** In proof 4, say that a `move` closure is cloned once per attempt, minus the last, and that `Cell`, atomics and mutexes aren't caught. Give step 20's README line both points.

## On DA to DE (answered by the owner at `1734252`)

- **DA:** the evidence holds. I reproduced the `AsyncFn` failure without the spike's wrapper. But the answer must also cover `.isolation(..)` and per-transaction settings: DF2.
- **DB:** safe, given DF3's two rules.
- **DC:** framed too narrowly. Proving step 13's own types later is fine, but the risk proof 9 should have caught is a value-chosen type against a column of a different type in its family. That's DF1, and it has to be settled before step 03 freezes `Param`.
- **DD:** safe. The list is R-0785: `fetch_page`, `.in_batches`, streams, the outbox worker, retries and `morm::allow_repeats(fut)`. Each is either a flag at morm's own call sites or a scope marked while a future is polled, the same technique proof 7 proves. Neither changes a step-03 signature, as long as step 03's internal query entry point can carry a "don't count this" flag. Step 03 should leave room for one.
- **DE:** safe, given DF8.
