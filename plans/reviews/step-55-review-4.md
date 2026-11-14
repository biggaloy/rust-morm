# Step 55 independent review, fourth round

PR #3 (`step-55-enum-proofs`), head `baf82f0`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes. This round covers the fix for the third review's EG1 to EG8, `git diff 9caf8a2 origin/step-55-enum-proofs`, and checks that nothing earlier regressed.

## What I read

- **On the PR branch:** `rules.md`, `contracts.md` (`SqlType`, `Param`), `proofs.md` (step 55's section, including the new "third review" bullets), and every changed requirement and test in context: steps 12, 13, 14, 20, 38, 45, 50, 52, 56 and 59. For the fix's claims I also read R-0575, R-0586, R-0595, R-0596, R-0650, R-0653, R-0982, R-0995, R-1032, R-1063 and step 34's text on `down.sql` and its one-transaction form.
- **On `origin/main`:** INDEX's "Stale-statement design, SA to SH", "Step 55 second review, EF1 to EF9" and "Step 55 third review, EG1 to EG8" entries, and `reviews/step-55-review-3.md`.
- **The PR body.**

## What I checked and ran

- **CI on the head.** `baf82f0` is green: `ci` on push (run 43483096792) and on the pull request (run 43483111784), every job, including `coverage`, `pooler`, `msrv`, `features` and integration on 15 to 18 over TCP and Unix sockets. The diff since `9caf8a2` touches only `plans/plan-approved/` (14 files). No code, test or workflow changed, and `spikes/` and `spikes.yml` stay deleted.
- **Coverage checker,** on an export of the head:
  - `check_coverage.py --step 55 --code crates tests benches fuzz .github`: **1255 requirements, 314 test obligations, 0 problems**;
  - the plain form: the same.
- **Manifest and INDEX.** Comparing `9caf8a2`'s manifest with the head's, ID by ID, gives 21 changed IDs, 1 new and none removed:
  - changed: R-0981, R-0984, R-1040, R-1050, R-1062, R-1080, R-1083, R-1084, R-1093, R-1213, R-1238, R-1241, R-1243, R-1245, R-1249, R-1250, R-1254, T-0175, T-0310, T-0311 and T-0312;
  - new: T-0314.

  That is exactly the INDEX entry's "written in at `baf82f0`" list, and the PR body's list since `9caf8a2` matches both.
- **Postgres spot checks.** These ran on throwaway 17.11 and 18.6 servers, with data under `.claude/scratch/` in my worktree, `TMPDIR` set there, and the sockets in `~/.claude/scratch/` because the worktree path is longer than a Unix socket allows. Both versions printed the same.

  | Claim | Result |
  |---|---|
  | EG6: after a simulated cutover (kept type renamed `_morm_retired_m2_st` with labels `_morm_retired_a`..`c`, retired column, live column on the new type, old copies of a populated and an unpopulated materialized view moved to `_morm_retired`), changing the live column under the live copy | `0A000 cannot alter type of a column used by a view or rule`, `rule _RETURN on materialized view mv depends on column "s"` |
  | EG6: one transaction that drops both live copies, creates the restored type, changes the live column to it with a `USING` that maps placeholders back from the retired column, drops the swap's type, renames the restored one and creates both copies again | Commits. The rebuilt copy reads `a` and `b` as `st`, the `WITH NO DATA` copy has `relispopulated = f`, and the retired copy keeps its three frozen rows, the deleted row 3 included, as `_morm_retired_<label>` |
  | EG6: a second run renaming objects to the same retired names | `42710 type "_morm_retired_m2_st" already exists`, `42701 column "_morm_retired_m2_s" ... already exists`, `42P07 relation "_morm_retired_m2_mv" already exists`. Suffixed names work |
  | EG6: `purge` with no `CASCADE` | In the wrong order, `DROP TYPE` and `DROP COLUMN` fail with `2BP01`, listing the column and both copies. In `purge`'s order (the copies, the retired column, the type) every drop succeeds |
  | EG5: a function with `SET standard_conforming_strings = off` returning `'PRAGMA:DISABLE:\x43HECK'`, with `prosrc ~* 'disable:check'` | Returns `PRAGMA:DISABLE:CHECK`, and the search is false, in a session with the setting on or off |
  | EG5: the same body with no `SET` clause, under a role default of `off`, in a fresh session that sets `standard_conforming_strings = on` first | Returns `PRAGMA:DISABLE:\x43HECK`, not decoded (EH7) |
  | EG5: `e'...'` with a lowercase prefix | Decoded like `E'...'` |
  | EG5: `U&'...'` in a function with `SET standard_conforming_strings = off` | `CREATE FUNCTION` itself is refused, with `unsafe use of string constant with Unicode escapes`, so no `U&` row is needed under `off` |

  So `proofs.md`'s EG6 paragraph holds: the single-transaction order, the collision and `purge` without `CASCADE`.

## EG1 to EG8: are they fixed?

| Finding | Verdict and evidence |
|---|---|
| EG1, inside a transaction | **Fixed for `.retry(n)`, as the owner decided.** R-0984 discards only that one connection after the rollback and raises nothing. `.retry(n)`'s next attempt runs on a newly opened connection, and the generation rises only if that attempt runs the failed statement with no `22P02`. A second failure enters step 14's memory. T-0312, R-1040, R-1249 and R-1250 match. **Two gaps remain.** With no `.retry(n)`, and behind a pooler, each call still discards a connection, and nothing ever enters the memory (EH1). The confirming attempt may run the statement with other parameters (EH2) |
| EG2, the cool-down | **Fixed outside a transaction, and inside one with `.retry(n)`.** R-0981 remembers the statement by a 64-bit hash for 60 seconds, or until the connection generation moves, in at most 64 entries per pool. R-1245's row and T-0314 match. **Not reached** inside a transaction with no `.retry(n)` (EH1) |
| EG3, the new connection's slot | **Mostly fixed.** The connection is opened outside the slots and counted against the budget. On success it takes the slot; on failure it is closed; with a full budget the first connection is closed first; T-0311 asserts the pool's size. **Two sentences contradict the branches,** and a failed open costs a connection attempt on every call (EH5) |
| EG4, `READ_AS_LIST` | **Fixed:** `contracts.md`, R-1241 and R-1213 add `READ_AS_LIST` beside `CAST_LIST`, and `Vec<T>` and `&[T]` take `T::READ_AS_LIST` as their `READ_AS` |
| EG5, plain literals | **Fixed for the function's own `SET` clause, as the owner decided.** `e''` and `u&''` are read in either case, and T-0310 and R-1084 match. The database or role default branch is moot, because morm's check session sets the value `on` itself (EH7) |
| EG6 (a), mechanics | **Fixed.** R-1050 rebuilds inside `down.sql`'s one transaction under R-1032's drop-then-create, `WITH NO DATA` for an unpopulated copy. Confirmed on 17.11 and 18.6 |
| EG6 (b), contents decision | **Fixed.** `rollback` preflight opens R-1063's decision with a default that refuses, and none for an unpopulated copy. T-0175 asserts both. The README doesn't say the rollback stops until it is answered (EH8) |
| EG6 (c), ledger and re-run | **Partly fixed.** The rollback's `_morm.history` row lets `purge` find the retired objects, and a re-run suffixes its names with its own row id. But the migration's files are static and checksummed, so nothing says how a re-run's `up.sql` and `down.sql` reach the suffixed names (EH3). Only `purge` reads the rollback's row, not `verify`, the walk or R-1062's later-change refusal (EH4) |
| EG7 | **Fixed:** R-1062's exception is a sentence of its own, and the EF8 reason in `proofs.md` and the PR body is now the retired column. The attribution needs one more touch (EH8) |
| EG8 | **Fixed:** R-1238's "apart from the third 1.26.1 instance", R-1083 "where `makemigrations` runs", R-1243's `::text[]` fix, R-1254's domain-array sentence, R-1084's escaped spellings, `proofs.md`'s merged parenthetical, and R-1255 in the PR body's README line |

Nothing earlier regressed. Only `plan-approved/` changed. The checker, CI and the ID list agree, and SA to SH, EF1, the `22P02` narrowing and SF still read consistently with the new text, apart from the points below.

## Findings

### EH1. Blocks approval. R-0984, T-0312, R-1249, R-1250 and R-1040: inside a transaction with no `.retry(n)`, and behind a pooler, a `_morm_retired_` value still discards one connection per call, and the cool-down is never reached

- **Where:** R-0984: "Otherwise morm returns `Error::SchemaChanged`, and once the transaction has rolled back it discards that one connection and raises nothing ... With no `.retry(n)` nothing is raised, and the next call outside a transaction confirms through step 14's retry."
- **What's wrong:**
  - R-0981 only adds a statement to the memory after "a retry on a new connection fails". Inside a transaction with no `.retry(n)`, there is no such retry. So a function that builds the enum value from stored or client text discards one pooled connection on every call. The pool then opens a replacement, and the memory is never filled.
  - A transaction with no `.retry(n)` is the default way to write one. An app that calls such a function only inside transactions never reaches "the next call outside a transaction".
  - That is the "one new connection per call" that the owner's EG2 decision replaced with "returns that statement's error at once for a cool-down period instead of opening a new connection each time, whatever the client sent".
  - Behind a pooler, R-0984's discard has no pooler exception, though R-0981 says a client connection "can't reach a server session" and discards nothing there. So each such call in a transaction drops and reopens a client connection for nothing, again with no memory, whether or not `.retry(n)` is set.
  - R-1249 and R-1250 tell users the cost is "one new connection, and then that statement's `22P02` returns at once for 60 seconds". That isn't true in these two cases.
  - T-0312 asserts only one call with no `.retry(n)`. No test calls a remembered statement inside a transaction, so R-0984's "or step 14's pool remembers the statement, it returns as it is ... with nothing discarded" branch is untested.
- **Evidence:** the text of R-0981, R-0984 and R-1249. The third review showed on 18.6 that a function reading a stored `_morm_retired_x` into `v st` fails with `22P02` from `enum_in` from a call whose parameters hold no such text. That failure comes from the data, so it repeats on every call and on any connection, a newly opened one included.
- **Options:**
  - **(a) Recommended.** Use the connection's age as the confirmation that a transaction can't run.
    - On an unconfirmed `22P02` in a transaction, morm records the statement's hash and the time.
    - If the same statement later fails the same way on a connection opened after that time, a fresh connection compiled the function and still failed. That counts as a failed confirmation, and the statement enters R-0981's memory.
    - A real stale session never fails on a connection opened after the cutover, so recovery is unchanged. Stored or client text stops discarding after at most the pool's pre-existing connections, usually one or two calls.
  - **(b)** Inside a transaction with no `.retry(n)`, discard a connection for a given statement at most once per cool-down period. A later unconfirmed `22P02` from that statement returns `Error::SchemaChanged` with nothing discarded. This is simpler, but a pool whose connections are all stale and used only in transactions then recovers one connection per period.
- **Either way:**
  - Behind a pooler, R-0984 discards nothing for a `22P02`, as R-0981 says.
  - T-0312 adds three cases: stored text in a transaction with no `.retry(n)`, called five times, discarding at most the pool's original connections and then none; a remembered statement in a transaction returning its `22P02` with nothing discarded; and the pooler case discarding nothing.
  - R-1249 and R-1250 state the cost in a transaction.

### EH2. Should fix. R-0984 and T-0312: a transaction's confirming attempt counts as a success when it runs the same statement with other parameters

- **Where:** R-0984: "If that attempt runs the statement that failed with no `22P02`, morm raises the connection generation then, and the pool's other connections are replaced at checkout."
- **What's wrong:**
  - Outside a transaction, R-0981's retry sends the same bytes again. Inside one, `.retry(n)` runs the user's closure again, which can bind other parameters to the same statement text.
  - Take a queue worker that picks the next row `FOR UPDATE SKIP LOCKED` and calls `process($1)`. Its first attempt hits a row holding `_morm_retired_x` and rolls back. Its second attempt picks another row, because a second worker has meanwhile locked the first, and succeeds.
  - Under R-0984 that success raises the generation and replaces the whole pool. Raising the generation also clears R-0981's memory, so the next poisoned row can repeat the cycle. That is SF's "raised only for a confirmed stale statement" broken again, by stored data.
  - The text also doesn't say what happens when the attempt never runs the failed statement, as when the closure takes another branch.
- **Fix:**
  - The confirmation needs the same statement text with the same parameter bytes. morm encodes every parameter itself, so it can compare a hash of both.
  - The same statement with other parameters, or no run of it at all, confirms nothing: nothing is raised, and the newly opened connection stays in the pool as an ordinary connection.
  - T-0312 adds a closure that binds a different value on its second attempt and asserts that no generation is raised.

### EH3. Should fix. R-1050, T-0175 and R-0595/R-0596: a re-run's suffixed names can't appear in the migration's static, checksummed files, so its `down.sql` can read the first run's retired column

- **Where:** R-1050: a second run "names its own retired objects with a suffix holding its `_morm.history` row's id, as `_morm_retired_<migration>_<name>_<run>`".
- **What's wrong:**
  - `up.sql` and `down.sql` are generated once, byte-identical for the same models (R-0596). `down.sql` is kept verbatim in `_morm.migrations` and covered by the checksum (step 34).
  - `<run>` is a `_morm.history` row id that exists only when `migrate` runs. So the static files can only name `_morm_retired_<migration>_<name>`. Nothing says that `migrate` rewrites those names for a re-run, or how the second run's `rollback` finds its own retired column.
  - After the first rollback, the first run's retired column still exists under the unsuffixed name, because the retired copy reads it. A `down.sql` that reads that name runs without error and reads the wrong column.
  - That gives a silent wrong restore. Down restores "only where the row still holds what it was mapped to", reading old values from the retired column. A row changed between the runs gets the first run's old value or none, and a row inserted between them has `NULL` there, so it keeps its mapped value.
  - T-0175 runs up, rollback, up and `purge`, but never rolls back the second run. So it can't catch this.
- **Evidence:** the text of R-0595, R-0596, step 34's `down.sql` rule and R-1050. On 17.11 and 18.6, after the first rollback the unsuffixed retired column and type are still there, so a statement naming them succeeds.
- **Fix, with a small design choice:**
  - **(a) Recommended.** `up.sql` and `down.sql` name retired objects by their unsuffixed names, as now. The ledger records, for each applied run, the actual name each retired object got. `migrate` and `rollback` map the unsuffixed names to that run's actual names through `morm-core`'s scanner before executing, and the checksum covers the files as written. Say so in R-1050 and R-0995.
  - **(b)** Refuse a re-run, with exit code `2`, while the earlier run's retired objects remain, naming them and `purge --now` as the fix. The third review offered this, but it reverses the owner's suffix choice, so it is only the fallback.
  - **Either way,** T-0175 changes and inserts rows between the runs, rolls back the second run, and asserts that each row gets the value from the second run's retired column.

### EH4. Should fix. R-1050, R-0586, R-1062 and T-0175: only `purge` reads the rollback's `_morm.history` row, so `verify`, a later op's walk and R-1062's refusal don't see the objects a rollback leaves

- **Where:** R-1050: "`purge` finds retired objects there as well as through applied migrations".
- **What's wrong:**
  - After the rollback, the migration is no longer applied, but its retired copy, retired column and retired type stay.
  - R-0586 ("Retired columns are a tracked state, so `verify` doesn't report them as drift") and R-1062 ("`verify` doesn't report any of these as drift, and a later op's `pg_depend` walk passes over them") rely on the ledger knowing them. So does R-1062's "A later change Postgres refuses under a retired copy ... is refused with exit code `2`, naming it, and `purge --now` clears it".
  - R-1050 extends only `purge` to the rollback's row. As written, `verify` reports the three objects as drift after a rollback. A later op's walk counts the retired copy as a live dependent. A later `ALTER COLUMN TYPE` under it fails with Postgres's raw `0A000` instead of the named refusal.
- **Fix:**
  - R-1050 says that the objects the rollback's row records are retired objects for every reader of the ledger: `verify`, `check_schema`, the walk, R-1062's refusal and `purge`.
  - T-0175 asserts that `verify` reports no drift after the rollback, and that a later migration changing another column the retired copy reads is refused with exit code `2`, naming it.

### EH5. Should fix. R-0981, R-0984 and T-0311: EG3's text contradicts two of its branches, and a failed open costs a connection attempt on every call

- **What's wrong:**
  - **The pool's size.** "When the budget has no room for one more, morm closes the first connection and opens the new one in its slot ... If the new connection can't be opened ... the original `22P02` is returned". In that branch the first connection is already closed, so the pool is one short until it refills, yet the next sentence says "In every branch the pool's size is what it was".
  - **The pool's `max`.** The new connection is "opened outside the pool's slots", so a pool at `max` with room in the budget holds `max + 1` connections during the retry. Yet the text says "neither the pool's `max` nor the budget is ever exceeded".
  - **A failed open.** When the open fails, "the original `22P02` is returned at once, with no backoff", and the memory isn't filled, because no retry ran. While Postgres refuses connections, every such call makes one more connection attempt, and in the full-budget branch also closes a good connection. That contradicts R-0653's backoff on "too many clients", and the cool-down's aim.
  - **Inside a transaction.** R-0984 opens the new connection "as step 14 opens one", but by then the first connection is already discarded. So step 14's "takes the first connection's slot" can't apply as written.
- **Fix:**
  - Say that in the full-budget branch a failed open leaves the slot to the pool's own refill under R-0653.
  - Say that `max` bounds the pool's slots, and that the confirming connection is a momentary extra within the budget. Or, if `max` is meant to bound connections, close the first connection first when the pool is at `max` too.
  - A failed open adds the statement to R-0981's memory, as a failed retry does.
  - In R-0984, the attempt's connection takes the slot the discarded connection left.
  - T-0311 adds an open refused by Postgres, called twice, with the second call making no connection attempt.

### EH6. Minor. T-0314: a `cfg(test)` constant can't be shortened from the `morm-tests` crate

- **Where:** T-0314: "once the cool-down ends, which the test shortens through a `cfg(test)` constant".
- **What's wrong:** the integration tests live in the separate `morm-tests` crate (`tests/Cargo.toml`, `tests/tests/*.rs`), which depends on `morm`. `cfg(test)` is set only when `morm`'s own unit tests are compiled, not when `morm` is built as a dependency. So a test in `tests/` can't shorten that constant, and the obligation can't be built as written there.
- **Fix:** either place T-0314 in `morm`'s own `#[cfg(test)]` module, which can reach Postgres, and say so, or shorten it through a `#[doc(hidden)]` feature that only `morm-tests` enables, with a `repo_policy.rs` check that no published crate turns it on. This is a choice of the fix, so it changes with the owner's approval through PR #3.

### EH7. Minor. R-1093, T-0310 and R-1084: decoding plain literals under a database or role default is moot, because morm's check session sets `standard_conforming_strings = on`

- **Where:** R-1093 decodes plain literals when the setting is off "by its own `SET` clause ... or by the database's or the checking role's default, in `pg_db_role_setting`", and then says "morm's check session also sets `standard_conforming_strings = on` itself". T-0310 expects a refusal for a function "checked while the checking role's default sets it `off`".
- **What's wrong:**
  - A session `SET` overrides both defaults, so in morm's check session only a function's own `SET` clause can turn the setting off.
  - On 17.11 and 18.6, a function with no `SET` clause, under a role default of `off`, in a fresh session that set the value `on`, returned `PRAGMA:DISABLE:\x43HECK` undecoded. The same body with `SET standard_conforming_strings = off` returned `PRAGMA:DISABLE:CHECK`.
  - So T-0310's role-default row asserts a refusal for a function `plpgsql_check` checks in full, a false exit code `2`.
  - "The database's default" also has no clear referent: the check database is a fresh `CREATE DATABASE ... TEMPLATE`, which copies no `pg_db_role_setting` rows.
- **Fix:**
  - Decode plain literals only when the function's `proconfig` sets the value off, and keep the session's `SET`.
  - T-0310's role-default row asserts that the narrowing is reported, as R-1080 says, rather than refused.
  - R-1084's notice drops "or in a plain one while `standard_conforming_strings` is off" in favour of "in a function declared with `SET standard_conforming_strings = off`".

### EH8. Minor. README notices, attribution and the PR body

- **R-1249, the rollback decision.** It says `rollback` "opens a decision saying so", but not that its default refuses. The surprise is that an emergency rollback over a populated materialized view stops with exit code `3` until `--decide` answers it. Say so, with the `--decide` form.
- **R-1249, a delayed recovery.** R-0981 says a remembered statement "only delays recovery ... by at most 60 seconds" after a real cutover. That is a surprise an operator would see as 60 seconds of `22P02` after a migration, and the notice doesn't say it. Add one sentence.
- **Attribution.** R-0981 ends "Owner decisions ... EF1 and EG2", and R-1050 "SA, EF8 and EG6". But INDEX lists the cool-down's numbers and key, and the `<run>` suffix, as "the fix's own choices the owner approves through PR #3". Add, as R-1062's earlier fix did, "with this fix's choices the owner approves through PR #3", or list those choices by name.
- **The PR body.** The "What's in it" bullet for `contracts.md` names `SqlType::READ_AS` but not `READ_AS_LIST`, which EG4 added.

## What I couldn't run

- **`plpgsql_check`.** It isn't installed here, so EG5's hidden narrowing still rests on the extension's source and on CI's earlier spike runs. My EG5 checks show only how Postgres decodes the literals.
- **Postgres 15 and 16, PgBouncer and a standby.** I had 17.11 and 18.6 only. The pooler parts of EH1 and EH5 follow from the text of R-0981 and R-0984, not from a run.
- **morm itself.** There is no runtime yet, so EH1, EH2 and EH5 are shown from the rules and from Postgres's behaviour. EH3 and EH4 are shown from the text of the file and ledger rules, with the leftover names confirmed on 17.11 and 18.6.

## Verdict

**Not approved yet.** The fix is faithful in most places:

- every changed ID is in INDEX's EG entry, and the PR body matches;
- the checker reports 0 problems, and CI is green on the head;
- EG4, EG7 and EG8 are fixed, and EG6's Postgres claims hold on 17.11 and 18.6;
- EG1 and EG2 are fixed where `.retry(n)` runs or the call is outside a transaction.

What remains:

- **Blocks approval:** EH1. Inside a transaction with no `.retry(n)`, and behind a pooler, stored or client text still costs one connection per call and never reaches the cool-down the owner decided on in EG2. This needs a small design choice.
- **Should fix:**
  - EH2, a transaction's confirming attempt must bind the same parameters;
  - EH3, a re-run's suffixed names versus static `up.sql` and `down.sql`, with a silent wrong restore at risk (a small design choice);
  - EH4, `verify`, the walk and R-1062's refusal must read the rollback's row too;
  - EH5, EG3's size and `max` sentences, and a failed open's cost.
- **Minor:** EH6, EH7 and EH8.
