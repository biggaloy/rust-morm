# Step 02 open items

PR #2 (`step-02-proofs`), head `f50823e` when the step agent reported. CI green on Postgres 15 to 18; checker 1204 R / 277 T, 0 problems.

## From the step agent, waiting on the owner

- **DA. Retry shape (proof 4).** The plan's `AsyncFn` bound makes a closure that reads a captured row by reference non-`Send` under `tokio::spawn` (a compiler limitation), so it breaks in every Axum handler; a postfix `db.transaction(f).retry(n)` needs a boxed future and `'static` captures. Options: (a) `db.retry(n).transaction(async |tx| ..)` with `AsyncFnOnce + Clone`, each attempt running a fresh copy (recommended, tested against a real serialization failure); (b) keep `AsyncFn`, which works only with closures that move in owned data; (c) boxed postfix form, `'static` captures only.
- **DB. Marking a model with hooks (proof 5).** Options: (a) `#[model(hooks)]` plus a required `OVERRIDES` const, as built (recommended); (b) default to "all hooks", no mark but every model pays the hook path.
- **DC. "Every supported type" for `Param` (proof 9).** The list isn't in step 02's files. Options: (a) step 13 adds the same test for morm's own `Timestamp`, `Range<T>` and `Numeric`, which have no encoding yet (recommended); (b) give step 02 the list now.
- **DD. Loop-detector exclusions (proof 11).** The list isn't in step 02's files. Options: (a) move that half of the proof to step 45, which builds the detector (recommended); (b) copy the list into step 02 and prove it before merge.
- **DE. The spikes in the PR.** `spikes/` and `.github/workflows/spikes.yml` stay for review. Recommended: a last commit before merge deletes both; the PR history keeps them.

## Choices the step agent made

- Attribute spellings in the spikes are its own; what's proven is the mechanism.
- Retry needed a 20 ms pause between attempts in the spike; step 20 checks its own retry rules.
- Refinements written into `contracts.md`: `groups!` masks are newtypes; `NestedConnection` falls back to the thread id when there's no task id; each upsert column has its own marker type; `#[model]` reads `cfg(morm_mgr)` inside `morm`; the generated `morm-mgr` does `use <crate> as _;` per model crate.

- **Answered:** DA to DE as recommended.

## From the review at `0137bb0`

- **DF1 and DF2:** answered as recommended; DF3 to DF10 fixed as recommended. See Decisions.
