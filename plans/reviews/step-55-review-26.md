# Step 55 independent review, twenty-sixth round

PR #3 (`step-55-enum-proofs`), head `e217a1c`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes.

This round covers `git diff 59286dc origin/step-55-enum-proofs`, three commits: the twenty-fifth review's fix, FC1 to FC4 (`4188648`, `173e6ed`), and its `proofs.md` record (`e217a1c`). I also checked that nothing earlier regressed on the paths these commits touch.

## What I read

- **On the PR branch:**
  - `rules.md`, `contracts.md`'s `Error::SchemaChanged` comment, and `proofs.md`'s new parts for this round.
  - **Every changed requirement in full:** R-0718 (step 14); R-0984 and R-1271 (step 20); R-1253, R-1268 and R-1272 (step 50).
  - **The changed test obligation in full:** T-0312 (step 20).
  - **For coherence:** R-1245's rows, R-1270's procedure (`before_run`, `on_error`, `session`, `request`, `take`, `finish_take`), R-0982, T-0319's I8 and I14, and R-1292.
- **On `origin/main`:** `reviews/step-55-review-25.md`, and INDEX's Decisions from "The EZ fix round's five open questions (2026-11-25)" to "The twenty-fifth review's fix (2026-11-26)".
- **The PR body,** read at the head: the twenty-fifth review's section, the changed-IDs paragraph since `59286dc`, and choices 247 to 252.

## What I checked and ran

- **CI on the head.** `e217a1c` is green: `ci` on push (run 45854696985) and on the pull request (run 45854711326), 18 of 18 jobs each.
- **The coverage checker,** run on a `git archive` of the head. `check_coverage.py --step 55 --code crates tests benches fuzz .github` reports **1294 requirements, 329 test obligations, 0 problems**, and the plain form gives the same.
- **The manifest against INDEX.** From `59286dc` to the head:
  - 6 requirements changed, each in `text` and `block`: R-0718, R-0984, R-1253, R-1268, R-1271 and R-1272.
  - 1 test obligation changed, in `text`: T-0312.
  - No ID is new or gone, no `line` moved, and the keys `master_sha256` and `frame` didn't change.
  - That's exactly the list in INDEX's entry "The twenty-fifth review's fix". `contracts.md` and `coverage.md` didn't change, and they didn't need to: no signature changed, and no changed row's opening changed.
- **FC1 to FC4, each read literally against its recommendation:**
  - **FC1:** R-1271's `search_path` bullet has the review's replacement word for word. R-0984 has "about one transaction per `stale_pending_expiry`, beyond calls that begin before its entry is made". R-1272 has "about once" and an added gap clause in its bound, its "Each failure asks for one repair, once" bullet and "What you give up". The sweep (choice 248) also rewrote R-1271's PgBouncer stale-statement bullet, R-0984's PgBouncer bound and R-1253's two PgBouncer bounds. What the README text now adds is the wrong set of calls in two places (FD1), and one second-row "once" was missed (FD2).
  - **FC2:** R-1271's entries-off clause has the review's words. R-0984 and R-0718 have them with `stale_cooldown` for `W`. R-1253's clause has "until the next repair, which a column change asks for itself", and R-1268's sentence has it too. The salt is also in R-1271's entries-on list (choice 249), and T-0312 has a case for it (choice 250). I read every other entries-off clause behind PgBouncer that names `RECONNECT` or `server_lifetime`. Each now names the salt, or is about a `22P02` that requests nothing without a confirmation, or is a silent-wrong-answer bullet that already ends at the pool-wide action.
  - **FC3:** R-1268 now says "Behind PgBouncer a statement a change made by hand left stale, run inside transactions, ...", the review's second option. "While an old type is kept" now has its antecedent again.
  - **FC4:** the PR body's FB1 bullet now says what the review recommends.
- **Requirement by requirement, for the changed IDs.**
  - **R-1270's `on_error` gives FC1's span as R-1271 states it.** The entry is made after `rollback(call.tx)`. Behind PgBouncer it also comes after D1's `read(conn)`. Straight to Postgres, for a `0A000`, it comes after `request(Clear, unread = true)`'s own read when the gate is closed. R-1272's "when it needs to" is right: straight to Postgres with the window open, nothing is read before the entry.
  - **FC2's healing holds by the procedure,** with entries on and off. Behind PgBouncer a `0A000` that D1's read didn't find moved runs `request(Salt)`. With the window open, `take` sets a fresh salt at once. With it closed, `set_due` defers it, and the first checkout after the window ends takes it. A cached run whose connection is behind the raised statement generation clears its caches first and prepares under the new marker. `entry(..)` does nothing with entries off, and the request comes before it, so turning entries off doesn't remove the salt.
  - **These now agree with each other on the stale-statement bounds:** R-1271's PgBouncer bullet ("calls of that statement that begin before its entry is made", and "beyond calls already running when its entry was made" in its next sentence), R-0984, and T-0319's I8 ("never fails in a run begun while its statement's pending entry exists"). R-1270 itself says a run that misses an entry a step has just made "counts as one already running when that step ran", so both wordings mean the same calls.
  - **T-0312's new case is right by the procedure.** It runs on a new pool, whose window is open, because no action has been taken. So the first `0A000`'s salt is taken at once, no entry is made with entries off, and the other four transactions run cached under the new marker. `pool_stats()` shows the marker and counts unnamed runs, salts and adoptions (step 14's T-0306), so the test can assert both.
  - **No contradiction left** among R-0718, R-0984, R-1245's rows, R-1270's procedure, R-1271's guarantees and costs, I8 and I14, or the README notices R-1253, R-1268, R-1272 and R-1292, apart from FD1 and FD2.
- **Loud notices and "use this when" lines.** R-1253, R-1268 and R-1272 still say what's given up, when it's right and how to proceed. FC3's sentence again says what `"off"` gives up for a stale statement behind PgBouncer. No opt-in call or setting is new in this range, and R-1265's and R-1266's lines still send readers to the notice before they turn pending checks off.
- **The PR body.** The twenty-fifth review's section, the changed-IDs paragraph since `59286dc` and choices 247 to 252 match the text, the manifest and the INDEX entry. There's one loose description of R-1272's wording, which `proofs.md` gets wrong too (FD3).

### Postgres 17.11 (`/opt/pgsql-17`) and 18.6 (`/usr/bin`)

These were throwaway clusters under the worktree's `.claude/scratch/`, with the Unix socket under `~/.claude/scratch/`, `TMPDIR` set to the scratch, and psql 18.6 with `VERBOSITY verbose`. A server statement went through `\parse` and `\bind_named`, and an unnamed run through `\bind`. Both versions gave the same output, apart from the source line (`plancache.c:866` on 18.6).

**FC2's case, T-0312's new test, as the server sees it.** `/* g1 */ SELECT * FROM items WHERE id = $1` was prepared and run in a transaction. Then `ALTER TABLE items ADD COLUMN b int DEFAULT 7` was run by hand.

- **The kept statement,** as a PgBouncer server statement is kept with entries off and no salt yet: `0A000` `cached plan must not change result type` from `RevalidateCachedQuery` in each of five transactions, and outside a transaction too.
- **An unnamed run,** as C1 and `.retry(n)`'s later attempts make: the new row `1 | x | 7`.
- **A salted text,** `/* g1.5f3a */ ...`, prepared fresh: the right row in each of four transactions, while the old text still failed beside it. That's T-0312's "the other four answer right at their first attempt, cached under the fresh text".

**Not run:** PgBouncer (no build on this machine), and Postgres 15 and 16. No finding rests on them. FD1 and FD2 come from morm's own procedure, and the server's side is the signal shown here and in the last round's probes.

### My own model

This is a small event-driven Python model, written from:

- R-0718's rows B1 and D1 and its "Entry";
- R-1270's `before_run` pending check and its `on_error` order: the whole rollback; straight to Postgres the evict, then `request(Clear, unread = true)`, which reads `<n>` only when the window is closed; behind PgBouncer, D1's read first, then `request(Salt)`; then `entry(..)`;
- R-0982's gate: taken at once when the window is open, otherwise due and taken by the first checkout after the window ends, a take removing every pending entry and clearing the copies.

It runs one cached statement under a per-transaction `search_path`, two paths at random, with entries on, on a pool of four, at about 50 transactions a second. A run takes 1 to 10 ms, a rollback 1 to 10 ms, and a read of `<n>` 1 to 200 ms, which is T-0319's load. Straight to Postgres each connection holds its own copy; behind PgBouncer one server statement holds the path it was planned under until a salt.

Each `Error::SchemaChanged` is classed by its run and by its transaction, against a failure whose entry wasn't yet made when the run began:

- the transaction started **before** that failure's error (column "started before");
- the transaction started **after** that error ("started in the gap");
- the run was **already running** at that error ("in flight").

Each setting is 8 seeds of 600 s at `E` = `W` = 60 s, or 16 seeds of 40 s at 1 s. The last column is the time from a transaction's start to the statement's run.

| Setting | Per `E` | The first | Started before | Started in the gap | In flight | Start to run |
|---|---|---|---|---|---|---|
| Straight to Postgres, `E` = `W` = 1 s | 2.23 | 1.00 | 0.06 | 1.09 | 0.08 | 1 to 10 ms |
| Straight to Postgres, the defaults | 1.69 | 1.11 | 0.01 | 0.51 | 0.05 | 1 to 10 ms |
| Straight to Postgres, the defaults | 1.96 | 1.04 | **0.54** | 0.30 | 0.09 | 1 to 100 ms |
| Straight to Postgres, the defaults, reads of 1 to 5 ms | 1.18 | 1.10 | 0.03 | 0 | 0.05 | 1 to 10 ms |
| Behind PgBouncer, `E` = `W` = 1 s | 2.78 | 1.01 | 0.14 | 1.48 | 0.15 | 1 to 10 ms |
| Behind PgBouncer, the defaults | 2.62 | 1.02 | 0.19 | 1.31 | 0.10 | 1 to 10 ms |
| Behind PgBouncer, the defaults | 2.59 | 1.05 | **1.00** | 0.44 | 0.10 | 1 to 100 ms |
| Behind PgBouncer, the defaults, reads of 1 to 5 ms | 1.40 | 1.12 | 0.07 | 0.09 | 0.11 | 1 to 10 ms |

With 1 to 10 ms from start to run, these match the last round's model (2.32 and 2.49 per `E` at 1 s) and the fix's (2.00 and 2.20). FD1 rests on the rows with 1 to 100 ms.

## Findings

### FD1. Minor. R-1253 and R-1272 (step 50): the README's new gap clause counts transactions that *start* while morm rolls back and reads, but the calls that fail are those that *run* the statement before the entry, and most of them can come from transactions that started earlier

- **Where:**
  - **R-1253's second bullet:** "or about once per `stale_pending_expiry`, 60 seconds by default, per process behind PgBouncer, plus the transactions that start while morm rolls back the first and checks the schema's state, before it marks the statement pending".
  - **R-1272's bound:** "about once per `stale_pending_expiry`, 60 seconds by default, per statement and pool, plus the transactions that start while morm rolls back that failure and, when it needs to, checks the schema's state, before it marks the statement pending, a few more at a high rate".
  - **The other three places say "start before morm marks it pending":** R-1253's last bullet, and R-1272's "Each failure asks for one repair, once" and "What you give up".
- **What's wrong:**
  - **R-1271 counts runs.** FC1's fix gives the right set there: "calls of that statement that begin before its entry is made". T-0319's I8 says the same: no failure "in a run begun while its statement's pending entry exists". R-1270's pending check is made when a run begins.
  - **R-1253 and R-1272 count transactions by when they start.** A transaction that began before the first failure's error and reaches the statement during the rollback or the read fails too. So does a run already in flight at that error. Neither "starts while morm rolls back".
  - **That's FC1's error turned the other way.** FC1 found R-1271 excusing only the calls already running at the first failure. These two clauses excuse only the transactions that begin after it. Both are loud notices, and R-1272's is the one a reader checks before deciding whether `.retry(1)` is needed.
  - **The other three places are a superset.** "Start before morm marks it pending" also takes in transactions that began earlier and reach the statement only after the entry, which don't fail. As an upper bound that holds. But it isn't the set R-1271 states, and in the same notice it reads differently from the bound beside it.
- **Evidence:** my model above, at the defaults, with 1 to 100 ms from a transaction's start to the statement's run, as when a transaction runs other statements first.
  - **Straight to Postgres,** 1.96 `Error::SchemaChanged` per `E`. Of the 0.93 beyond the first, 0.54 came from transactions that started before the first failure's error, and 0.09 from runs in flight then. Only 0.30 are the transactions the clause names.
  - **Behind PgBouncer,** 2.59 per `E`. Of the 1.54 beyond the first, 1.00 started before and 0.10 were in flight. Only 0.44 started in the gap.
  - **With 1 to 10 ms from start to run,** the two left-out kinds are 0.06 to 0.29 per `E`. That's small, but it's still outside what the clause names.
- **Fix:** no owner choice is needed: this words the decided procedure's cost as R-1271 already does.
  - **Count runs in all five places.** In R-1253's second bullet write "plus the transactions that run the statement before morm has rolled back the first, checked the schema's state and marked the statement pending". In R-1272's bound write "plus the transactions that run it before morm, after rolling back that failure and, when it needs to, checking the schema's state, marks the statement pending, a few more at a high rate".
  - **The other three places** say "plus the transactions that run it before morm marks it pending": R-1253's last bullet, and R-1272's "Each failure asks for one repair, once" and "What you give up".
  - **Choice 247** in the PR body follows the new wording.

### FD2. Minor. R-1268 (step 50): the second row's cost with a shorter `stale_pending_expiry` is still "once per `stale_pending_expiry` per statement", with no "about" and no exception, which FC1's sweep set out to remove

- **Where:** R-1268's `stale_pending_expiry` bullet, "Shorter:": "a bad enum value a cached statement meets inside a transaction, a label your code sends or text a column holds, gets `Error::SchemaChanged` in place of its `22P02` more often, once per `stale_pending_expiry` per statement, and at every such call with pending checks off, below".
- **What's wrong:**
  - **R-1271 states this cost with an exception.** Its bullet "What the second row costs" says "at most once per `E`, ..., beyond calls of it already running when its entry was made".
  - **The other README places hedge it.** R-1253's bad-label bullet and R-1292 say "about once per `stale_pending_expiry`".
  - **The gap is the same here.** Row B1 makes the entry after the whole rollback, and behind PgBouncer row D1 makes it after a catalog read too. Bad labels come in bursts: a `PgEnum` "sends a bad label only while the database lacks a label the binary has, as during a deploy before its migration runs", and then every call with that label meets the error. So the calls of the statement that run it in that span each return `Error::SchemaChanged`, as FC1's search_path calls do.
  - **The fix's sweep missed it.** `proofs.md` and the PR body say every bound that "said 'once' or 'at most once' with no exception" was found. This one says "once" with neither "about" nor an exception.
- **Evidence:** R-1270's `on_error` (`rollback(call.tx)`, then `if pooled or err is XX000: mv = read(conn)`, then `entry(..)`), and the model above for the same span with a `0A000`. For a `22P02` straight to Postgres the span is only the rollback, so there are fewer such calls, but it isn't empty.
- **Fix:** no owner choice is needed. Write "more often, about once per `stale_pending_expiry` per statement", as R-1253 and R-1292 say. Optionally add R-1272's gap clause in FD1's wording.

### FD3. Minor. `proofs.md` and the PR body: the FC1 record says R-1272's three places all have the "while morm rolls back ... checks the schema's state" wording, but only its bound does

- **Where:**
  - **`proofs.md`, "The twenty-fifth review's fix", FC1:** "R-1272's notice says "about once", plus the transactions that start while morm rolls back that failure and, when it needs to, checks the schema's state, in its bound, in "Each failure asks for one repair, once" and in "What you give up"."
  - **The PR body, FC1's "The review's places":** "plus the transactions that start before morm marks the statement pending, a few more at a high rate, in its bound, its "Each failure asks for one repair, once" bullet and "What you give up"".
- **What's wrong:**
  - **Only the bound has the rollback wording.** The other two places say "plus the transactions that start before morm marks it pending".
  - **"A few more at a high rate" isn't in all three.** It's in the bound and "What you give up", but not in "Each failure asks for one repair, once".
  - Choice 247 describes the three places rightly, so the body contradicts itself in the same section.
- **Fix:** after FD1's fix, write in `proofs.md`'s record and the body's bullet what each of the three places now says, and read the body again against the text.

## Summary

| Severity | Findings |
|---|---|
| Blocks approval | None |
| Should fix | None |
| Minor | FD1, FD2, FD3 |

No finding needs an owner choice.

- **FC1 to FC4 are written in as recommended,** read literally, and the procedure is unchanged.
  - R-1271 and R-0984 now state FC1's bound as R-1270's procedure gives it, and as I8 checks it.
  - FC2's salt is in every entries-off clause behind PgBouncer, and in the entries-on list beside it, and T-0312 tests it.
  - FC3's sentence has its antecedent, and FC4's bullet is right.
- **The new claims hold.**
  - On 17.11 and 18.6 a kept server statement fails with `0A000` in every transaction after a hand-made column change, and a salted text prepared fresh answers. That's T-0312's new case as the server sees it.
  - My model gives the `search_path` case's rate with entries on in the fix's range.
- **Every finding is in the README wording around FC1:**
  - **FD1:** two README clauses count transactions by when they start, where R-1271 counts runs. With transactions that run the statement later than they start, most of the extra failures fall outside what they name.
  - **FD2:** one second-row "once" in R-1268 that the sweep missed.
  - **FD3:** `proofs.md`'s record, and one bullet of the body, misdescribe R-1272's wording.
- **The checks agree.**
  - The changed IDs match INDEX's entry exactly: 6 requirements and 1 test obligation. `contracts.md` and `coverage.md` needed no change.
  - The checker reports 1294 requirements, 329 test obligations and 0 problems, in both forms.
  - CI is green on `e217a1c`, 18 of 18 jobs on push and on the pull request.
  - Choices 247 to 252 match the text.

**PR #3 isn't ready for the owner yet.** The fixes are wording only, then a check of the edits, the checker and CI.
