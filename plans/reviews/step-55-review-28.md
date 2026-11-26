# Step 55 independent review, twenty-eighth round

PR #3 (`step-55-enum-proofs`), head `c458987`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes.

This round covers `git diff b98bada origin/step-55-enum-proofs`, three commits: the twenty-seventh review's fix, FE1's sweep (`db7e5cf`) and FE2 (`76a41f3`), and its `proofs.md` record with FE3 in the PR body (`c458987`). I also checked that nothing earlier regressed on the paths these commits touch.

## What I read

- **On the PR branch:**
  - `rules.md`; `contracts.md`'s `Error::SchemaChanged` comment; and `proofs.md`'s new parts, "The twenty-seventh review's fix" and "The last round's model, carried on", and the FD2 record it corrects.
  - **Every changed requirement and test obligation in full:** R-1246, R-1262 and T-0311 (step 14); R-0984, R-1271 and T-0319 (step 20); R-1249, R-1250, R-1258, R-1268, R-1272, R-1288 and R-1294 (step 50); R-1040 (step 59).
  - **For coherence:** R-0718, R-0981 (the epoch, rows A2 to B4 and its closing bounds), R-0982 (the gate, events E1 to E8), R-1245's rows, R-1270's state table and procedure (`session`, `request`, `take`, `finish_take`, `checkout`, `on_moved`, `old`), rows C2 and D2, R-1271's other bullets ("One attempt per stale thing", "Churn"), T-0314, T-0319's I3, I4 and I8, and the README notices R-1253 and R-1292.
- **On `origin/main`:** `reviews/step-55-review-27.md`, and INDEX's Decisions from "The EZ fix round's five open questions (2026-11-25)" to "The twenty-seventh review's fix (2026-11-26)".
- **The PR body,** read at the head: the twenty-seventh review's section, choices 254, 255 and 257 to 262, the changed-IDs paragraph since `b98bada`, "Open questions" and "End-to-end result".

## What I checked and ran

- **CI on the head.** `c458987` is green: `ci` on push (run 45868792628) and on the pull request (run 45868805102), 18 of 18 jobs each, both on head SHA `c4589873821c…`.
- **The coverage checker,** run on a `git archive` of the head. `check_coverage.py --step 55 --code crates tests benches fuzz .github` reports **1294 requirements, 329 test obligations, 0 problems**, and the plain form gives the same.
- **The manifest against INDEX.** From `b98bada` to the head:
  - 12 requirements changed, each in `text` and `block`: R-0984, R-1040, R-1246, R-1249, R-1250, R-1258, R-1262, R-1268, R-1271, R-1272, R-1288 and R-1294. 2 test obligations changed, in `text`: T-0311 and T-0319.
  - No ID is new or gone (1,623 entries either side), no `line` moved, and `master_sha256` and `frame` didn't change.
  - That's exactly the list in INDEX's entry "The twenty-seventh review's fix", with `contracts.md`'s `Error::SchemaChanged` comment. Only `contracts.md`, `proofs.md`, the four step files and `manifest.json` differ between the two trees; `coverage.md` didn't change and didn't need to, because no row's opening changed, as the checker confirms.
- **FE1 to FE3, each read literally against its recommendation:**
  - **FE1:** R-1249's last bullet has the review's words, with the cap added: "inside one it returns `Error::SchemaChanged`, and then that statement's `22P02` returns at once for up to `stale_cooldown`, 60 seconds by default, so about once per `stale_cooldown` per statement and pool, and once more after each pool-wide repair morm makes or migration it sees meanwhile, or when other statements push it out of morm's cool-down, … until PgBouncer replaces the server connection". R-1258 has the review's "for up to `stale_cooldown`, 60 seconds by default, ending sooner at morm's next pool-wide repair or a migration it sees", the cap, and "after which the next such call inside a transaction returns `Error::SchemaChanged` again". The cap is right to add: R-1270's state table drops the oldest cool-down entry at `stale_cooldown_max`, and R-1268 already states its cost. `proofs.md`'s FD2 record and choice 255 now say the bound has no gap but isn't once. The alternative the review didn't recommend wasn't taken.
  - **FE1's sweep, behind PgBouncer,** is right in each place it changed, against R-1270: `session()` checks `in_cooldown` before anything, calls `cooldown(call.stmt)` in D2's own step before `rollback`, and the entry goes at `take`, `finish_take` and `on_moved` (E5, E1), at the cap, and `stale_cooldown` after it was made (E7). So R-1246's "for up to `stale_cooldown` … until a pool-wide action, a moved `<n>` or the cap removes its entry sooner, after which its next such call is handled as the first was", R-1262's "the longest", R-0984's and R-1271's count ("at most once per `W` per statement and pool, plus once after each pool-wide action, adoption or drop by the cap"), R-1040's decision text, the contracts comment's "failure outside the cool-down", and T-0319's new I8 clause all say what the procedure does. R-1271's kept-type bullet rightly names only the cap as ending the cool-down sooner, because a salt or an adoption gives the statement fresh text, which is where that case ends.
  - **FE1's sweep, straight to Postgres:** R-1249's, R-1250's and R-1040's bounds on replacements from `_morm_retired_` text now match R-0981's closing paragraph: at most `max` per epoch, `2 × max` in any span of `stale_cooldown`, plus `max` per migration, and a suspended epoch pausing replacements until it ends, or a migration starts a new one. For that text this is right: its `22P02` requests *replace* only if the new connection's run confirms, and stored text fails there too, so only the epoch replaces. T-0311's two bounds now separate the epoch's replacements from the *replace* action's. But R-1268's "Shorter:" took the same wording for every client-driven replacement, which a *replace* action adds to (FF1), and R-1294's sentence states this mode's cost with no scope and none behind PgBouncer (FF2).
  - **FE2:** R-1288's "What you give up" has the review's clause after "once per connection", word for word, with ", which `.retry(n)` hides" folded into "; `.retry(n)` hides both" (choice 261). It matches R-1271's bullet on stale statements behind PgBouncer and R-1253.
  - **FE3:** "End-to-end result" names `c458987` and runs 45868792628 and 45868805102, which are the head's.
- **Requirement by requirement, for the changed IDs.** Apart from FF1 and FF2 I found no contradiction among R-0718, R-0984, R-1245's rows, R-1270's procedure (unchanged this round), R-1271's guarantees and costs, T-0319's I-clauses and the README notices R-1249, R-1250, R-1253, R-1258, R-1268, R-1272, R-1288, R-1292 and R-1294 on the bounds this round touched. T-0314's PgBouncer case still says "for `stale_cooldown`", which is right for a test in which nothing else removes the entry, as choice 260 says.
- **Loud notices and "use this when" lines.** Every changed notice still says what it does, when it's right, what's given up and how to proceed; FF1 is a cost stated short. No opt-in call or setting is new in this range.
- **The PR body.** The twenty-seventh review's section, the changed-IDs paragraph since `b98bada`, choices 254, 255, 257, 259, 261 and 262, and "Open questions" match the text, the manifest and INDEX's entry. Choice 258, and `proofs.md`'s sweep record, call R-1268's "Shorter:" worded as R-0981 gives it, which is FF1, and choice 260 and `proofs.md` speak of "R-1294's straight-to-Postgres half", which the README sentence doesn't scope (FF2).

### Postgres 17.11 (`/opt/pgsql-17`) and 18.6 (`/usr/bin`)

Throwaway clusters under the worktree's `.claude/scratch/`, Unix sockets under `~/.claude/scratch/`, `TMPDIR` set to the scratch, psql with `VERBOSITY verbose`. Both versions gave the same results, apart from source lines.

- **A `42804` a client drives, with no schema change.** `CREATE FUNCTION g5(tbl text) … FOR q IN EXECUTE 'SELECT c FROM ' || tbl LOOP x := x || (q.c + 1); …` over a table `ti (c int)` and a table `tb (c bigint)`. In one session `SELECT g5('ti')` answers `22`, then `SELECT g5('tb')` fails `42804` `type of parameter 5 (bigint) does not match that when preparing the plan (integer)` from `plpgsql_param_eval_recfield` (`pl_exec.c:6638` on 17.11, `:6798` on 18.6), one of R-1245's three routines. A function that does `EXECUTE … INTO q` for each table in turn fails the same way. So a client's choice of argument alone gives the session-class `42804` that T-0311's record-shape case tests, and by R-1270's `session()` it requests *replace* at the failure (`sure`).
- **The stale session FE1 rests on, again.** After a session called `f('a')` (`DECLARE v st; BEGIN v := t; …`), a second session renamed `st` to `_morm_retired_st` and its labels to `_morm_retired_7c1e9a04b52fd386_<label>`, created a new `st` and ran the reset, in one transaction. Back in the first session, `f('a')` failed `22P02` `invalid input value for enum _morm_retired_st: "a"` from `enum_in` in each of two transactions, and also from a statement with new text (`SELECT /* morm g0.deadbeef */ f('a')`, as a salt gives), while a new session answered `a`. So a salt doesn't cure it, as R-1258 says, and the next call after a removed cool-down entry meets it again.

**Not run:** PgBouncer (no build on this machine), and Postgres 15 and 16. No finding rests on them: FF1 and FF2 come from morm's own procedure, and the server's side is shown above.

### My own model

A small Python model of R-0981's epoch and R-0982's gate straight to Postgres, outside a transaction with no migration, built from R-1270's `session()`, `request()`, `take()` and `checkout()`: a pool of `max` = 4, `W` = 60 s, random sequences of 400 events over 600 s, each a checkout or a session-class failure on a random connection. A `22P02` from stored text replaces an old connection and requests nothing, because its new connection's run fails the same way; a client's `42804` replaces an old connection and requests *replace*, through the gate. Each sequence counts every stale-driven replacement, the epoch's and the action's at checkout. Over 3,000 sequences each:

| Failures | Most replacements in one span of `W` | Most in one epoch's life |
|---|---|---|
| `22P02` from stored text only | 7 | 4 |
| `42804` only | 11 | 8 |
| Both | 11 | 10 |

The first row keeps R-0981's bound, `max` per epoch and `2 × max` = 8 per span. The other two pass it, and stay within R-1271's `4 × max + 4` opens.

## Findings

### FF1. Minor. R-1268 (step 50), `proofs.md` and choice 258: the shorter `stale_cooldown`'s cost counts only the epoch's replacements, and leaves out the *replace* action's

- **Where:** R-1268's `stale_cooldown` bullet: "Longer: fewer clears and replacements when clients cause stale errors on purpose; … Shorter: up to the pool's `max` replacements per `stale_cooldown` period, twice that in any span of its length, because a period's replacements come at each connection's next failure, and one cache clear per window, each plus a round per migration". `proofs.md`'s sweep record says "R-1268's "Shorter:" says the same", as R-0981 gives, and choice 258 lists R-1268 among the bounds "now worded as R-0981 gives them".
- **What's wrong:**
  - **R-0981's bound is the epoch's alone.** It's written for "a client's stored or transformed text", whose `22P02` requests *replace* only when the new connection's run confirms, which stored text doesn't. For that text R-1249, R-1250 and R-1040 are right.
  - **R-1268's sentence is about every stale error clients cause.** That includes a `42804` a client drives with no schema change, which T-0311 tests ("a record-shape function fed rows of a table with an `int` column and of one with a `bigint` column in turn, with no schema change, gets `42804` and costs at most `max` replacements per epoch and per *replace* action"). R-1270's `session()` makes a `42804` on an old connection request *replace* at the failure, so it adds a *replace* action of up to `max` replacements per window to the epoch's `max`.
  - **R-1271's "Churn" states the larger bound:** "in any span of `W`, at most two epochs' and two *replace* actions' worth, `4 × max + 4` stale-driven opens, plus `2 × max` per migration", and T-0319's I4 checks the same. R-1268 says `2 × max` per span and `max` per period, so the two contradict.
  - **"One cache clear per window"** also drops the third kind of repair. R-1268's own first sentence lists "or replaces every connection, at most once per window".
  - **The text changed this round.** Before it, "up to the pool's `max` replacements and one cache clear per window" could be read as counting either kind. Now "per `stale_cooldown` period … because a period's replacements come at each connection's next failure" names the epoch's alone.
- **Evidence:**
  - R-0981's closing paragraph, R-0982's gate and E5, R-1270's `session()` (`sure = err in {42804, XX000}`; `if sure: request(Replace)`), R-1271's "Churn", T-0311 and T-0319's I4, quoted above.
  - The probe above: a session-class `42804` from a client's argument alone, with no schema change.
  - My model above: with client `42804`s, 11 replacements in one span of `W` at `max` = 4, against R-1268's 8, and 10 in one epoch's life, against its 4. With stored text only it stays within 8 and 4.
- **Fix:** no owner choice is needed. This words the decided procedure's cost.
  - **R-1268:** "Shorter: more pool-wide repairs, one per window, each a cache clear or, straight to Postgres, a replacement of every connection, and up to the pool's `max` more replacements in each `stale_cooldown` period for stale sessions, because a period's replacements come at each connection's next failure, so up to four times `max` in any span of its length, plus a round of each per migration, and behind PgBouncer that `Error::SchemaChanged` about once per window per statement."
  - **`proofs.md`'s sweep record and choice 258:** say that R-1268 counts both kinds, as R-1271's "Churn" does, and that R-0981's wording fits only R-1249, R-1250, R-1040 and R-1294, whose text requests no *replace*.
  - **The sweep:** check every other client-driven replacement bound for the same omission. I found none: R-1249, R-1250, R-1040 and R-1294 are about `_morm_retired_` text, and R-1271 and T-0319 have the full bound.

### FF2. Minor. R-1294 (step 50), `proofs.md` and choice 260: "If something is still on the kept type" states the replacements straight to Postgres with no scope, and no cost behind PgBouncer

- **Where:** R-1294, "If something is still on the kept type, don't rename it": "a query or a write using an old label gets `22P02` naming `_morm_retired_byhand_st`, from new sessions too, which morm takes for a stale session, at up to `max` replaced connections in each of morm's `stale_cooldown` periods for stale sessions, so up to twice that in any span of `stale_cooldown`, and `max` more per migration". `proofs.md` and choice 260 speak of "R-1294's straight-to-Postgres half".
- **What's wrong:**
  - **The sentence doesn't say "straight to Postgres".** It's the same class as FE2: a loud notice states a cost for one mode, and a reader behind PgBouncer finds none for theirs. Here it's worse, because the stated cost reads as applying behind PgBouncer too.
  - **Behind PgBouncer morm replaces no client connection** (R-1246: "opens or discards no client connection"). Such an error holds `_morm_retired_`, so it's R-1245's seventh row, session class, and rows C2 and D2 apply. Outside a transaction it costs one more run on a connection checked out again, and then the error returns at once from the cool-down for up to `stale_cooldown`. Inside one it returns `Error::SchemaChanged` in place of the `22P02`, about once per `stale_cooldown` per statement and pool, and once more after each pool-wide repair, migration or push-out. The error comes from new server sessions too, so this lasts as long as the query does, with no `RECONNECT` or `server_lifetime` to end it.
  - **The sweep changed this sentence** ("per `stale_cooldown`" became the period wording) and left its scope as it was.
- **Evidence:** the text above; R-1246's bullet "A stale session is never acted on pool-wide" and rows C2 and D2; R-1270's `session()` behind PgBouncer; R-1258 and R-1249, which state the PgBouncer cost for the same kind of error.
- **Fix:** no owner choice is needed.
  - **R-1294:** "which morm takes for a stale session: straight to Postgres at up to `max` replaced connections in each of morm's `stale_cooldown` periods for stale sessions, so up to twice that in any span of `stale_cooldown`, and `max` more per migration; behind PgBouncer one more run outside a transaction, on a connection checked out again, and inside one `Error::SchemaChanged` in place of the `22P02` about once per `stale_cooldown` per statement and pool, and once more after each pool-wide repair, migration or push-out, as the notice on stale functions behind PgBouncer says, for as long as such queries run".
  - **`proofs.md` and choice 260:** say what the sentence now reads.

## Summary

| Severity | Findings |
|---|---|
| Blocks approval | None |
| Should fix | None |
| Minor | FF1, FF2 |

No finding needs an owner choice.

- **FE1 to FE3 are written in as recommended,** read literally. R-1270's procedure is unchanged.
  - Behind PgBouncer, every changed bound on the cool-down and on a stale session's `Error::SchemaChanged` now says "up to" and what ends it sooner, and R-0984, R-1271 and T-0319's I8 give the count.
  - R-1288 states the stale statement's cost behind PgBouncer.
  - The body names the head's CI.
- **FE1's sweep straight to Postgres is right for `_morm_retired_` text** (R-1249, R-1250, R-1040, T-0311), but:
  - **FF1:** R-1268 took the epoch-only bound for every client-driven replacement. A client's `42804` adds a *replace* action, so R-1268 says `2 × max` per span where R-1271 and T-0319's I4 say `4 × max + 4` opens.
  - **FF2:** R-1294's sentence states the replacements with no "straight to Postgres", and no cost behind PgBouncer.
- **The checks agree.**
  - The changed IDs match INDEX's entry exactly: 12 requirements, 2 test obligations and the `contracts.md` comment. `coverage.md` needed no change.
  - The checker reports 1294 requirements, 329 test obligations and 0 problems, in both forms.
  - CI is green on `c458987`, 18 of 18 jobs on push and on the pull request.
  - The PR body matches the text, apart from choice 258's and choice 260's records of FF1's and FF2's places.
- **The probes on 17.11 and 18.6** show a client-driven `42804` with no schema change, and a stale function's `22P02` coming back in every transaction of its session, under new statement text too.

**PR #3 isn't ready for the owner yet.** Both fixes are wording only, then a check of the edits, the checker and CI.
