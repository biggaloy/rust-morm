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
- Refinements written into `contracts.md`: `groups!` masks are newtypes; each upsert column has its own marker type; `#[model]` reads `cfg(morm_mgr)` inside `morm`; the generated `morm-mgr` does `use <crate> as _;` per model crate. Only in `proofs.md`: `NestedConnection` falls back to the thread id when there's no task id.

- **Answered:** DA to DE as recommended.

## From the review at `0137bb0`

- **DF1 and DF2:** answered as recommended; DF3 to DF10 fixed as recommended. See Decisions.

## From the fix at `8301619`, waiting on the owner

- **DG1. A field whose `sql` gives a column its type's encoder refuses** (e.g. `bigint` on an `i32`, `timestamp` on a `DateTime<Utc>`). Options: (a) the conversion layer encodes the value as the column's type, times as UTC, and `#[model]` refuses at compile time a `sql` it can't convert (recommended); (b) fall back to the value's type, still wrong for time pairs unless the session is UTC; (c) refuse every such pair at compile time.
- **DG2. Instants against `timestamp`, and naive times against `timestamptz`, in raw SQL** depend on the session time zone. Options: (a) morm's connections start with `TimeZone = 'UTC'`, overridable in `morm.toml`, and the README says so (recommended); (b) keep the server's time zone and document it.
- **DG3. A raw `Vec<String>` against `citext` or `char(n)`** finds 0 rows, and **a raw `Value` into a `json` column** stores jsonb's normalised text. Options: (a) raw text lists go as `UNKNOWN` text like raw strings (`array_length($1, 1)` and `unnest($1)` then need `$1::text[]`), `Value` stays `jsonb`, and the README says both (recommended); (b) keep lists as `text[]` and tell users to cast.
- **DG4. R-1104 says "each parameter's type from `Param`",** which no longer holds for known columns. Options: (a) reword to "the column's type where morm knows the column, otherwise `Param`'s" (recommended); (b) leave it and let `contracts.md` carry the rule.
- **Answered:** DG1 to DG4 as recommended.

## From the second review at `5d5a199`, waiting on the owner

- **DH1:** answered: UTC session with no override (option C), and first-class wall-clock time in 0.1, designed first. See Decisions.
- **DH2, DH3, DH5:** see `reviews/step-02-review-2.md`; with the owner.
- **Answered:** DH2 (automatic casts, detect every mismatch possible, design first), DH3 (trait-based, lossless only), DH5 (savepoints, no `isolation` or `retry` on `tx`, settings for the rest of the transaction). See Decisions.
- **Parameter typing design A to K:** answered, all as recommended.

## From the fix at `e78b970`, waiting on the owner

- **DI1. A PgBouncer role not set to UTC** isn't caught at startup. Options: (a) add `current_setting('TimeZone')` to the startup check's existing statement and refuse the pool (recommended); (b) leave it to `doctor`, and the README says so.
- **DI2. A stray `timezone` key in `morm.toml`.** Options: (a) refuse it with `Error::Config` (recommended); (b) ignore it. The plan doesn't say how unknown keys are treated in general.
- **Answered:** DI1 (a) and DI2 (a).
- **DI3. Which zone names pass DI1's check.** On 17.11 and 18.6, `UTC` and `utc` read back as `UTC`, but `Etc/UTC`, `UTC0`, `Etc/Universal`, `Universal`, `Zulu`, `GMT` and `UTC+0` read back as written, and `+00` as `<+00>-00`. Options: (a) only exact `UTC`, the value morm sends, the error naming what it saw (recommended); (b) also a fixed list of aliases; (c) any zone `pg_timezone_names` shows at offset zero with no DST, which goes by today's date, so `Europe/London` passes in winter.
- **Answered:** DI3 (b), UTC plus a fixed list of proven aliases; and the `check_schema = "off"` notice states the zone isn't checked.
- **DI4. `UTC0`.** It's a POSIX TZ string Postgres parses itself, absent from tzdata and `pg_timezone_names`, though it showed offset 0 and abbreviation `UTC` over 1800 to 2200. Options: (a) keep it out, since allowing it opens the rest of the POSIX syntax (`UTC+0`, `+00`) and the fix is one `ALTER ROLE` (recommended); (b) add `UTC0` alone, with T-0297 pinning it on 15 to 18.
