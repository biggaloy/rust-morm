# Step 02 independent review, round 2

PR #2 (`step-02-proofs`), head `632ab63`. This is the repeat review after the fixes, done as `rules.md` "Independent review" says, by an agent that didn't write the step. I read `rules.md`, `contracts.md`, `proofs.md` and `steps/02-proofs.md` on the PR branch; "Decisions since the split" in `INDEX.md` on `origin/main`, including "Step 02's proofs, DA to DE", "Step 02 review, DF1 to DF10" and "Step 02, DG1 to DG4"; `reviews/step-02-review.md` and `reviews/step-02-open-items.md` on `main`; and the whole diff, `git diff origin/main...origin/step-02-proofs`. To check the new text against the steps it lands in, I also read the parts of steps 05, 13, 14, 15, 16, 17, 20, 21, 22, 24, 43, 50, 55 and 56 that it touches or relies on.

## What I checked and ran

- **CI on `632ab63`:** every `ci.yml` job passed, on both push and pull request runs: fmt, clippy, unit, trybuild, doc, integration on Postgres 15 to 18 over TCP and Unix sockets, pooler, msrv, features, coverage. There's no `spikes` job, because DE deleted it.
- **Checker:** `check_coverage.py --step 02 --code crates tests benches fuzz .github` and the plain form both report 1211 requirements, 278 test obligations, 0 problems.
- **Manifest:** `git diff origin/main...origin/step-02-proofs -- plans/plan-approved/tools/manifest.json` changes R-0060, R-0435, R-0442, R-0453, R-0670, R-0767, R-0865 and R-1104, and adds R-1205 to R-1211 and T-0278. INDEX's Decisions covers every one: the DF entry names R-0435, R-0442, R-0453, R-0767 and R-0865, and the DG entry names R-0060, R-0670, R-1104, R-1205 to R-1211 and T-0278. The quoted copies of R-0060 in steps 11, 12, 27 and 47, and of R-0670 in step 43, match their owners.
- **DE:** `origin/step-02-proofs` holds no file under `spikes/` and no `spikes.yml`. Commit `8301619` deletes 72 files and changes nothing else except the rerun line in `proofs.md`, which names `65dec8560dc25829c987c85f670d491c7538beac`. That commit's `spikes` runs passed on Postgres 15, 16, 17 and 18, on both push and pull request, and so did its `ci` runs. The PR body names `65dec85` too.
- **Rerun locally:** I extracted `spikes/` from `65dec85` into scratch and ran it on a throwaway Postgres 18.6 with rustc 1.99.1. `cargo test --locked --release --manifest-path spikes/Cargo.toml --workspace` passed: every proof, the pinned proof 9 matrix (`p09_matrix.expected`) and all trybuild cases. So did `bash spikes/proof10.sh`.
- **Stale text:** a search of `plan-approved/` finds no postfix `}).retry`, `}).isolation` or timeout modifiers, no `|tx| async move`, no `AsyncFn` bound, and no "type from `Param`". The only `AsyncFn` left is R-0917's `info` text, copied word for word from the frozen plan.
- **My own checks** on that Postgres 18.6, with `psql`'s `\parse`, `\bind_named` and `\bind`, which send untyped text-format parameters. The results are given under each finding.

## The first review's findings

| Finding | Fixed as recommended? | Evidence |
|---|---|---|
| DF1 | Yes, at the proof level | `Column` carries a `ParamType` (`contracts.md`). Raw `String` and `&str` go as `UNKNOWN`. The matrix covers every R-0186 type against its default column and against `citext`, `varchar(5)`, `char(5)`, `json`, `jsonb`, `timestamp` and `timestamptz`, wider integers, `double precision`, `cidr`, arrays and `= ANY` lists, inserted and compared. It's pinned in `p09_matrix.expected`, green on 15 to 18 in CI and on 18.6 for me. The steps that build it raise DH4. |
| DF2 | Yes, on `db` | `contracts.md` gives `TxOptions<'d, Once/Retry>`. R-0435, R-0442 and R-0767 in step 20 and R-0453 in step 22 are rewritten. `p04_tx.rs` runs the options under `tokio::spawn` with a borrowing closure, and `p04_options_retry_mutates` pins the refusal. The `tx` side raises DH5. |
| DF3 | Yes | `#[morm::hooks]` refuses a macro call (`p05_hooks_macro_in_impl`) and a written `OVERRIDES`. `Overrides<M>` has no `NONE` and is typed by its model (`p05_overrides_by_hand`, three pinned errors). `proofs.md` gives step 24 both rules. DH7 notes that no ID carries them. |
| DF4 | Yes | The registry is split into 64 shards (`nested.rs`). `proofs.md` gives the single-mutex and sharded timings, and tells step 21 not to use one process-wide lock. |
| DF5 | Yes | `G` is both a type and a constant, through `GlobalGroups`, in `contracts.md` and proof 3. `models-a` does only `use bases_b::groups::G;`. A model with no global group gets `type Global = ()`. |
| DF6 | Yes | Every R-0186 type is bound to a column of its own type in the matrix. The unused enum-array column is gone. |
| DF7 | Yes | The spike's `Hooks` matches the contract's nine signatures exactly. The update and delete hooks run through generic calls and Axum, and `after_commit` is called. |
| DF8 | Yes | The rerun line names `65dec85…`, which holds the spikes. |
| DF9 | Yes | `step-02-open-items.md` on `main` now says the thread-id fallback is only in `proofs.md`. |
| DF10 | Yes | The last attempt runs the original closure (`contracts.md`, proof 4). `p04_tx.rs` counts clones for `.retry(0)`, `.retry(2)` and `.retry(3)`. R-0865 states the cost and the interior-mutability gap. |

DA to DE are written in faithfully. DA and DF2 are in `contracts.md`, proof 4 and steps 20 and 22. DB is in `contracts.md` and proof 5, and agrees with R-0153's existing `hooks` flag. DC is in proof 9 and T-0278. DD is in `contracts.md`'s query entry point row and proof 11. DE is the deletion commit. DG4 is written in exactly, in R-1104. DG1 to DG3 are written in, but DH1 to DH4 below are problems in how they're written in.

## Findings

### DH1. A non-UTC `timezone` makes morm's own time encodings disagree with the session, and R-1205 says the opposite

- **Severity:** blocks approval
- **IDs:** R-1205, R-1207, R-1210; DG1, DG2
- **Files:** `steps/10-config.md` (R-1205), `steps/13-types.md` (R-1207), `steps/50-readme-release.md` (R-1210), `contracts.md` (the conversion-layer comment), `proofs.md` (proof 9, DG1 and DG2)

**What's wrong.**

- R-1207, DG1 and `contracts.md` say the conversion layer encodes an instant on a `timestamp` column as its **UTC** wall time, and a `NaiveDateTime` on `timestamptz` as UTC, and that "no answer depends on the session's `TimeZone`".
- R-1205 lets `morm.toml` or `MORM_DB_TIMEZONE` set another zone. It then says raw SQL "works in that zone, which is the one morm's own encodings use". `contracts.md` says the same: "so raw SQL converts times the same way".
- Both can't be true. With `timezone = "Asia/Dhaka"`, morm's known-column paths write and compare UTC wall times, but raw SQL on the same `timestamp` column converts in +06. The data is silently mixed.
- R-1210, the README notice, doesn't mention it.

**Evidence** (Postgres 18.6, session `TimeZone = 'Asia/Dhaka'`, as R-1205 sets it for that config):

- A `timestamp` column holds `2026-01-01 00:00:00`. That's what R-1207 writes for the instant `2026-01-01 00:00Z`.
- Raw SQL with the same instant as `timestamptz` (`WHERE at = $1::timestamptz`, `$1 = '2026-01-01 00:00:00+00'`) finds 0 rows.
- The session reads the row back as `2026-01-01 00:00:00+06`, six hours from the instant morm wrote.
- Under UTC, which is the only zone the matrix's "Under `TimeZone = 'UTC'`" table checks, they agree.

**This needs an owner decision.** It touches the conversion layer, whose shape step 03 freezes.

- **A (recommended).** Keep DG1's UTC conversions, and make the text say so.
  - R-1205 drops "which is the one morm's own encodings use".
  - The `contracts.md` comment drops "so raw SQL converts times the same way".
  - R-1210 says, beside `timezone`: morm's own reads and writes of an instant on a `timestamp` column, or a naive time on `timestamptz`, always use UTC. So with another zone set, raw SQL that converts those columns disagrees with morm, unless it says `AT TIME ZONE 'UTC'`.
  - No new dependency, and DG1's "no answer depends on the session" stays true for morm's own paths.
- **B.** The conversion layer converts in the configured `timezone`. That needs time zone rules on the client (such as `chrono-tz`), or declaring the value as `timestamptz` and letting a session in that zone convert it. The second breaks behind PgBouncer unless the role's zone matches, so `doctor` (R-0671) would have to compare the role's `timezone` with the configured one, not only check it's there.
- **C.** Drop the override and always use UTC. This reverses part of DG2.

### DH2. Under DG3, a raw list where Postgres expects one value is silently read as one text value

- **Severity:** should fix
- **IDs:** R-1209, R-1211; DG3
- **Files:** `steps/13-types.md` (R-1209), `steps/50-readme-release.md` (R-1211), `proofs.md` (proof 9, DG3)

**What's wrong.**

- A raw `Vec<String>` now goes as an `UNKNOWN` text literal, `{"a","b"}`.
- Where the SQL expects a single value, as in `WHERE name = $1`, a mistaken `=` for `= ANY`, Postgres infers `text` and compares with the literal string `{"a","b"}`. Before DG3, the same mistake failed with `42883`.
- The spike pinned this but didn't report it. `p09_matrix.expected` has `SELECT $1::text` with `["a", "b"]`: `wrong: {"a","b"}`.
- It's in none of proof 9's text, the DG3 question the owner answered, R-1209 or R-1211. R-1211 even says `$1::text` "fixes it", which is true for a scalar but turns a list into this literal.

**Evidence** (Postgres 18.6):

- `name = $1` with `'{"a","b"}'` sent as untyped text matched the one row whose `name` is the text `{"a","b"}`.
- `nick = $1` on `citext` matched 0 rows, silently.
- `PREPARE p1(text[]) AS ... WHERE name = $1` fails with `operator does not exist: text = text[]`.
- `anyarray` is no way out: `PREPARE p2(anyarray) ... = ANY($1)` fails with `op ANY/ALL (array) requires array on right side`.

**Also:** proof 9's main table still shows the R column for list rows from before DG3: `Vec<String>` on `citext[]` gives `42883`, and `= ANY` gives `wrong: 0 rows`. Under DG3 both are right. My check: `ca = $1` with `'{alice}'` on `citext[]` found 1 row, and an untyped `'{bob,Carol}'` inserted into `citext[]` matched `'BOB' = ANY(ca)`. Mark those cells "before DG3", or give the DG3 result beside them.

**This needs an owner decision:**

- **A (recommended).** Keep DG3. Say in R-1209, and loudly in R-1211, that a list where Postgres expects one value is read as one text value, so a list belongs only in `= ANY($1)`, an array column, or `$1::text[]`. Add the case to T-0278.
- **B.** Also refuse it on the cached-statement path: after a prepare, a list parameter whose inferred type isn't an array returns an error naming the parameter. The first run, through `query_typed`, gets no parameter description and can't be checked. So the same SQL would behave differently on its first and second runs, which is why I don't recommend B on its own.
- **C.** Reverse DG3: send lists as `text[]`, and `citext` and `char(n)` lists need a cast. The silent wrong answers with `= ANY` come back.

### DH3. R-1208's compile-time refusal has no rule for types morm doesn't map, or for `sql` names it doesn't know

- **Severity:** should fix
- **IDs:** R-1208; DG1; affects R-0186 ("anything else requires an explicit `sql`")
- **Files:** `steps/13-types.md` (R-1208), `contracts.md` (the conversion-layer comment)

**What's wrong.** R-1208 refuses "a field whose `sql` gives a column type that neither the field's type's encoder takes nor R-1207 converts to". Two cases are left open:

1. **A field type morm doesn't map.** R-0186 says anything outside its list needs an explicit `sql`, so user types with their own `ToSql` are expected. `ToSql::accepts` is a runtime function, so `#[model]` can't know at compile time what a user type's encoder takes. R-1208 doesn't say whether such a field is refused, trusted, or checked through a trait.
2. **A `sql` naming a type morm doesn't know,** such as `ltree`, `hstore`, pgvector's `vector(3)` or a domain made by hand. The two cases split:
   - On a `String`, it works today as `UNKNOWN` text, as `citext` does.
   - On `Vec<f32>` or `i32`, the value's binary encoding goes as `UNKNOWN` and is read by that type's receive function. That works for a domain over `int4`, and is wrong or an error for `vector`.

**Also:** the check can't be done by matching the field's type tokens, because a type alias (`type Id = i64;`) hides the real type from the macro. It has to go through a trait, such as the contracts row "The trait that gives a column its SQL type", with a `const` assertion that the macro words with the field name, the type as written and the `sql`.

**This needs an owner decision:**

- **A (recommended).**
  - The refusal goes through morm's column-type trait, so aliases resolve.
  - A type morm maps, or a user type that implements the trait, declares the SQL types it takes.
  - A `sql` morm doesn't know is allowed only on a field whose type has a text form (`String`, `&str`, morm's own enum, domain and composite types), and goes as `UNKNOWN` text. Any other pairing is refused, naming the field.
- **B.** Refuse only the pairs between types morm maps and `sql` names it knows. Accept everything else, and leave it to `accepts` at the first bind. That's simpler, but `Vec<f32>` on `vector` fails only at runtime.

### DH4. T-0278 can't run "through morm's own paths" in step 13, and no later step re-proves them

- **Severity:** should fix
- **IDs:** T-0278, R-1207, R-1209; R-1104, R-0206; DF1
- **Files:** `steps/13-types.md`, `proofs.md` (proof 9, DG1), with steps 14, 16, 17, 18 and 45 affected

**What's wrong.**

- Step 13 depends only on 05, and can run beside 10, 11 and 12.
- But R-1207 names the paths "builders, `F`, `sql!`, model inserts and updates", R-1209 names `db.sql()`, and T-0278 says the matrix "gives the right answer through the known-column path". `proofs.md` says step 13 "runs this matrix through morm's own paths".
- Those paths come later: the query entry point and caches in 14, reads in 15, `db.sql()` in 16, inserts in 17, updates in 18, `sql!` in 45.
- Once they're built, no step carries a test that they bind with the column's `ParamType`. A builder that passes the value's own `pg_type` would bring back DF1's case-sensitive `citext` with every test green. R-0206 ("`citext` works") doesn't say it compares.

**Also:**

- The spike sent parameters only through `query_typed_raw`. R-1104 prepares cached statements with the same declared types, and the text-format `UNKNOWN` rule must hold there too. Nothing tests it, and no step's requirements own the wrapper that sets each parameter's format. That's in `contracts.md` only.
- My check: through a prepared statement (`\parse` then `\bind_named`), untyped text-format `'alice'` against `citext`, `'1234'` against `int4` and `'ab '` against `char(5)` each found 1 row. So the protocol supports it. The risk is in morm's code, whose `accepts` must not refuse the type the prepare inferred.

**Fix:**

- **T-0278** runs in step 13 through the conversion layer, `Param`, and the wrapper that sets each parameter's format, straight on `tokio_postgres`, and says so. Step 13's text names the wrapper as its own deliverable, or step 14 owns it with a requirement.
- **Add a test obligation in step 18,** the first step after reads, raw SQL, inserts and updates exist. It reruns the matrix's known-column cases through `.by`, `.any`, `insert`, a batch insert (`UNNEST`) and `update().set()`, and R-1209's raw cases through `db.sql()`. Each runs twice, so the second run goes through a cached statement.
- **Add the same obligation in step 45** for `sql!`.
- Add array-literal cases with `\`, an empty string, `{}` and the string `NULL`. The spike quotes every element, so these should pass, but nothing pins them.

### DH5. DF2 settles the options on `db`, but not what `tx` offers

- **Severity:** should fix
- **IDs:** R-0436, R-0441; DF2
- **Files:** `contracts.md` (transactions), `steps/20-transactions.md`

**What's wrong.**

- R-0436 says `tx` has "the same API as `db`, with savepoints and per-transaction settings (`statement_timeout`, `lock_timeout`, `transaction_timeout`)".
- `contracts.md` now defines `isolation`, `retry` and the setting starters only on `impl Db`, as starters of a `TxOptions` that ends in `.transaction(..)`.
- Read literally, "same API" gives `tx.isolation(..)`, `tx.retry(n)` and `tx.lock_timeout(d)` starting a savepoint builder. None of them can mean that.

**Evidence** (Postgres 18.6, inside one transaction):

- `SAVEPOINT s1; SET LOCAL lock_timeout = '1234ms'; RELEASE SAVEPOINT s1; SHOW lock_timeout;` shows `1234ms`. A setting scoped to a savepoint outlives it.
- `SET TRANSACTION ISOLATION LEVEL SERIALIZABLE` after the first query fails: `must be called before any query`.
- R-0441 already says a savepoint can't recover from `40001`, so `tx.retry(n)` can't work.

**This needs an owner decision, before step 03 freezes `Tx`:**

- **A (recommended).**
  - `tx.transaction(async |tx| ..)` is a savepoint.
  - `tx` has no `isolation` or `retry`, so they're compile errors.
  - Each setting on `tx` is immediate, `tx.lock_timeout(d).await?`, a `SET LOCAL` for the rest of the transaction.
  - R-0436 says so, and `contracts.md` lists the `Tx` methods.
- **B.** Settings scope a savepoint, `tx.lock_timeout(d).transaction(..)`, and morm restores the old value on release. That costs a round trip per savepoint and needs the old value read first.

### DH6. `Param`'s comment contradicts R-0961 and R-0978 for types with no fixed OID

- **Severity:** minor
- **IDs:** R-0961, R-0978; DF1
- **Files:** `contracts.md` (`Param`)

**What's wrong.**

- The comment says "Every other type returns its own type", and lists `has_text` as "String, &str, lists of them, PgEnum".
- But R-0961 gives `PgDomain` and `PgComposite` `Type::UNKNOWN`, with composites in text format, and R-0978 sends enum arrays in text format.
- Step 03 freezes this text, and a step-56 session would find the contract saying otherwise.

**Fix.** Say that every type with no fixed OID (`PgEnum`, `PgDomain`, `PgComposite`, and arrays of them) returns `UNKNOWN` and has a text form, and that every other type returns its own type.

### DH7. Rules the fix gave later steps carry no ID, so the checker can't hold those steps to them

- **Severity:** minor
- **IDs:** DF3, DF4, DF2's random jitter
- **Files:** `proofs.md`; `steps/24-hooks.md`, `steps/21-pool-deadlocks.md`, `steps/20-transactions.md`

**What's wrong.** These live only in `proofs.md` and `contracts.md`, which every step reads:

- step 24 must refuse a macro call and a written `OVERRIDES`, with a trybuild case each (DF3);
- step 21 must not put one process-wide lock on the stream check (DF4);
- the retry jitter must come from a real random source (an owner-approved fix choice).

But a rule with no ID needs no tagged test, so `check_coverage.py` passes a step that skips one.

**Fix.** Give each a test obligation in its step: two trybuild cases in 24, a check-cost measurement with streams open in 21 or 48, and a jitter-spread test in 20. Each needs the owner's approval, and goes into `coverage.md` and the manifest.

### DH8. R-1210 and R-1211 aren't loud notices, and `.statement_timeout(..)` has no "use this when" line

- **Severity:** minor
- **IDs:** R-1210, R-1211, R-0866 to R-0868
- **Files:** `steps/50-readme-release.md`

**What's wrong.**

- R-1210 (UTC sessions) and R-1211 (untyped raw text, and the `42P18` that needs a cast) say "The README says this plainly". These are exactly the behaviour changes that stop or surprise a developer: the session zone overrides the server's and the role's, and queries that worked fail. R-1008 sets the form for such a notice ("shows this notice loudly", as a quoted callout).
- DF2 makes `db.statement_timeout(d)` a public starter. The README table has `.isolation(...)`, `.lock_timeout(...)` and `.transaction_timeout(...)` (R-0866 to R-0868) but no line for `.statement_timeout(...)`.

**Fix.**

- Make R-1210 and R-1211 loud notices in R-1008's form, with DH1's and DH2's content.
- Add a "use this when" line for `.statement_timeout(...)`, for example "A statement in this transaction must be stopped if it runs too long, such as one serving a user's request". The pool's own `statement_timeout` defaults to `off` (step 10).

### DH9. The PR body has two stale lines

- **Severity:** minor
- **IDs:** none
- **Files:** PR #2's description

**What's wrong.**

1. Under "Choices to review", it says INDEX's Decisions names only R-0442, R-0767 and step 20's example, "so the coordinator may want to add these two IDs", meaning R-0453 and R-0865. INDEX's DF entry now names both.
2. "Retry pauses are jittered (10 to 50 ms)" leaves out the choice INDEX records as owner-approved: the pause comes from a real random source (`RandomState`), after jitter from the clock failed once on 17 in CI.

**Fix.** Update both lines. The rest of the body matches the branch: CI, the checker totals, the changed IDs, `65dec85` and the proof table.

## Summary

- **Blocks approval:** DH1.
- **Should fix:** DH2, DH3, DH4, DH5. Each of DH1, DH2, DH3 and DH5 needs an owner decision, with options above.
- **Minor:** DH6, DH7, DH8, DH9.

DF1 to DF10 are fixed as recommended. DA to DE and DG4 are written in faithfully. Every changed manifest ID has a Decisions entry, the spikes are deleted and rerun green from `65dec85`, and CI on `632ab63` is green.
