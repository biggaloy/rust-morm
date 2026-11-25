# Step 55 independent review, twenty-second round

PR #3 (`step-55-enum-proofs`), head `93854c2`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes.

This round covers `git diff f25aefe origin/step-55-enum-proofs`, two commits: `3565679`, the fix for the twenty-first review's EY1 to EY3, and `93854c2`, which records it in `proofs.md`. I also checked that nothing earlier regressed on the paths the fix touches.

## What I read

- **On the PR branch:**
  - `rules.md` in full, the new round of `proofs.md` in full, and `contracts.md`'s parts on `Conn`, transactions, `Error` and `PoolStats`. I didn't re-read the earlier rounds of `proofs.md`.
  - **Every changed ID in full:** T-0311 (step 14), R-1271 and T-0319 (step 20), R-1293 and the new R-1294 (step 50).
  - **For coherence:** R-1245, R-0718, R-0981, R-0982, R-1270 and its procedure, T-0151 to T-0153 (step 14), R-0984 (step 20), R-0698 (step 12), R-0995 (step 38), R-0997 (step 41), R-1001 (step 44), R-1050 (step 52), R-1253 and R-1258 (step 50).
- **On `origin/main`:** `reviews/step-55-review-21.md`, and INDEX's Decisions entry "A hand-made enum swap without relabel is stated, not cured (EY2)" with what `da29e0c` appended, and the entries before it.
- **The PR body:** this round's section, choices 165 to 172, the open questions and the end-to-end result.

## What I checked and ran

- **CI on the head.** `93854c2` is green: `ci` on push (run 45595874235) and on the pull request (run 45595889230), 18 of 18 jobs each.
- **The coverage checker,** run on a `git archive` of the head. `check_coverage.py --step 55 --code crates tests benches fuzz .github` reports **1294 requirements, 329 test obligations, 0 problems**, and the plain form gives the same.
- **The manifest against INDEX.** From `f25aefe` to the head, the `text` and `block` of R-1271 and R-1293 changed, the `text` of T-0311 and T-0319, and R-1294 is new. That's exactly the EY2 entry's list, and nothing else moved.
- **Each fix against its finding:**
  - **EY1:** I8 checks held connections, and the 32nd switch shows that check can fail. Its wording has a scope error: EZ1.
  - **EY2, option (a):** R-1271's bullet, its exception in the stale-session bullet, R-1293's release branch, R-1294, T-0311's assertion, I8's exception and I9's loud half are all written in as decided. EZ2 and EZ3 are about two of them.
  - **EY3:** the PR body's EX1 bullet counts five settings and one widened name, as choice 159 does.
- **The loud notice.** R-1294 has all four parts: what it does, when it's right, what you give up and how to proceed. EZ3 is about what the last one says. R-1294 adds no opt-in call or setting, so no "use this when" line is due.
- **The fault count.** 7 + 1 + 9 + 7 + 3 + 3 + 1 + 1 + 1 is 33, as T-0319 says.

### The 240 rule and the 32nd switch

**The rule doesn't apply to the 32nd switch's 236 of 256, and nothing goes to the owner because of it.**

- **The rule's 240 is out of 2,048, not out of 256.** T-0319 says "fewer than 240 of 2,048, 30 per 256, stops the step". INDEX's shorthand for it leaves the 2,048 out.
- **It's about another measurement:** step 20's PR measuring its own simulation, not `proofs.md`'s model.
- **The 32nd switch would clear it anyway.** 235 per 256 over 2,048 is about 1,880 of 2,048. The 33rd, 217 per 256, is about 1,736.
- **The floor that does apply to the model's counts is 50 of 256.** 236 and 222 clear it, and so do my own counts below.

### My own model

It's written from R-1270's procedure as it stands, in Python. The fix's model is gone, so none of its code is in mine, though I had read `proofs.md`'s description of it. The final version ran 32 settings to the end. Every figure below is from that run.

- **What it covers:** rows A1 to A4, B1 to B5, C1, C2, D1 and D2, the gate, the epoch, pending entries and the cool-down; pooled connections, `.retry(0)` to `.retry(2)` transactions with savepoints, and held connections in and out of the app's own transaction; behind PgBouncer, three server sessions with a 4 s lifetime and `RECONNECT`. Four statements: an enum cast, a statement whose result type changes, a `DECLARE v st` function and a `%ROWTYPE` function. The oracle is a fresh session, compared by SQLSTATE.
- **Two ways of holding:** T-0319's load, where a held connection makes one call of one or two statements, and a longer one, up to 3 s and several calls.
- **What it leaves out:** calls run one at a time, so it tests no race. Stored text, serialization failures, refused opens, isolation levels, read order and several servers are out too.
- **Corrected while building it:** my first version ordered R-1245's rows wrongly, read the parameter test as EZ6 describes and, behind PgBouncer, blamed the wrong server session for one error. Those runs are thrown away.

What it showed, 256 sequences of 2,000 events per setting unless said:

| Claim | Result |
|---|---|
| The procedure, read as R-1271 means it: I4 per epoch, I7, I8 with its held clause and the `XX000` exception at any time, I9's loud half's "no request, entry or replacement" | Holds in all 14 no-fault settings, both modes, savepoints included |
| **The fix's correction to EY1:** the twenty-first review's "at most once per statement per connection" is wrong | **Right.** With no fault that bound fails 78 of 256 at T-0319's load with the app running again after its `ROLLBACK`, 163 with longer holds, and 253 with longer holds and more of them. The fix measured 116 |
| The same, with no app running again on the held connection | 0 of 256: see EZ4 |
| 32nd switch, row A4 returning the error as it is, at T-0319's load | Caught 246 of 256, and 1,968 of 2,048 (seeds 1,000 to 3,047) |
| 33rd switch, the widened signal | Caught 242 of 256 straight to Postgres, 1,990 of 2,048, and 255 of 256 behind PgBouncer |
| I8 needs its exception for the kept type's function straight to Postgres | Yes: without it I8 outside its held clause fails 25 of 256 at T-0319's load and 145 in the quiet setting |
| R-1294's relabel turns the case into one the bounds cover | Yes: with hand relabels as world events, no bound fails in three settings, both modes |

### Postgres 17.11 (`/opt/pgsql-17`) and 18.6 (`/usr/bin`)

Throwaway clusters under the worktree's `.claude/scratch/`, Unix sockets only under `~/.claude/scratch/`, `TMPDIR` in the scratch, psql 18.6 as the client. Both versions gave the same results:

- **A stale session's `XX000` in the app's own transaction repeats.** A session compiled a `DECLARE v st` function, and another dropped `st` and made it again. Three times in turn, `BEGIN`, a new prepare of `SELECT f($1::text)`, and its run got `XX000` `cache lookup failed for type`, then `ROLLBACK`. Outside a transaction it got the same error on the same backend, and a new session answered. So the correction to EY1 matches Postgres.
- **The kept type's function (EY2).** After `ALTER TYPE st RENAME TO st_old` and a new `st`, the stale function's `'c'` got `22P02` naming `st_old`, outside a transaction and in a savepoint, after whose rollback the transaction went on and committed.
- **The same function given `'zzz'`, a label no type has,** got `22P02` naming `st_old` too (EZ2).
- **A cast cached before the swap,** `SELECT ($1::st)::text` given `'c'` inside `BEGIN`, got `22P02` naming `st_old`. Closed and prepared again after `ROLLBACK`, it answered `c` (EZ2).
- **R-1294's `ALTER TYPE` statements** ran as written, and the stale function's error then named `_morm_retired_m2_st`, for `'c'` and for `'a'`.
- **A table column still on the kept type (EZ3)** read `a` before those statements and `_morm_retired_a` after. `WHERE c = 'a'` and `INSERT ... VALUES ('a')` then got `22P02` naming `_morm_retired_m2_st`.

**Not run:** PgBouncer itself, because no finding rests on it, and Postgres 15 and 16, which T-0311 covers.

## Findings

### EZ1. Should fix. T-0319 (step 20): I8's held clause excepts the `XX000` only "from then on", so its "before then" half fails with nothing broken

- **Where:** T-0319's I8: "from then on a call on a held connection that meets a stale session gets `Error::SchemaChanged`, never the error as it is, **but inside a transaction the app opened, where a stale session's `XX000` from a cached statement ... comes back as it is at each call that meets it; before then** the error comes back as it is only on a connection step 14's epoch test found not old". The PR body's EY1 bullet and `proofs.md` say it the same way.
- **What's wrong:**
  - **The exception sits in the "from then on" half.** The "before then" half has none. But the `XX000` comes back as it is at any time, because row A1's fresh run gets `25P02` and the original error returns before any epoch test runs. No test "found" that connection not old.
  - **R-1271's held bullet has no such limit:** "or its own error when it's an `XX000` from a cached statement" holds at any time. So I8 doesn't check held connections "as R-1271's bullet on them states".
  - **A step 20 session that builds I8 as written gets a failing simulation** with no fault, and then has to guess which text is wrong.
  - **A second, smaller case:** "never the error as it is" doesn't say the error is visible. A client's own label holding `_morm_retired_` that reaches a stale function over a relabelled type gets its `22P02` as it is by R-0981's parameter test, from `tc + W` on too. A fresh session would error as well.
- **Evidence:** my model, no fault. The "before then" half as written fails 135 of 256 sequences at T-0319's load, 85 of them on a connection the test would have found old, and 206 of 256 with longer holds. With the exception at any time it fails none. The second case came up in 3 to 23 of 256 in four settings with longer holds, and in none at T-0319's load.
- **Fix:** no owner choice is needed. Put the exception first, over both halves: "inside a transaction the app opened, a stale session's `XX000` from a cached statement, which no fresh run can tell from a stale statement there, comes back as it is at each call that meets it, at any time, with no epoch test; apart from that, from then on a call on a held connection that meets a stale session never returns its visible error as it is, but `Error::SchemaChanged`, and before then the error comes back as it is only on a connection step 14's epoch test found not old". Say it the same way in the PR body and `proofs.md`.

### EZ2. Should fix. T-0319 (step 20): I9's loud half says a kept type's `22P02` "comes only from plpgsql ... given a label only the new type has", which the simulation's own load breaks

- **Where:** T-0319's I9: "a `22P02` naming a type a hand-made swap kept without a relabel, returned as it is, comes only from plpgsql compiled before that swap and given a label only the new type has, never from a session opened since".
- **What's wrong:** three ordinary cases break it with nothing wrong in the machine.
  - **A label no type has,** 5% of T-0319's labels, sent to the stale function: Postgres names the kept type, and it returns as it is. It isn't visible, since a fresh session errors too, but I9 doesn't say "visible".
  - **A label holding `_morm_retired_`,** another 5%, bound to a cast cached before the swap: R-0981's parameter test returns it as it is. Not plpgsql, and not visible.
  - **A cast cached before the swap, on a held connection inside a transaction the app opened,** given a label only the new type has: row A1's fresh run gets `25P02`, so the caller gets the cast's own `22P02` naming the kept type. Not plpgsql, and it is visible. R-1271's held bullet states it: "a stale statement returns its error as it is ... and that connection's stale copy fails at most once".
- **Evidence:** Postgres 17.11 and 18.6, above, for the first and third. My model, no fault, at T-0319's load: I9 as written fails 254 of 256 sequences on errors that aren't visible, and 70 to 71 of 256 on the third case, 118 of 256 with more held calls.
- **Fix:** no owner choice is needed. Say "a visible `22P02` naming a type a hand-made swap kept without a relabel, returned as it is, comes only from plpgsql compiled before that swap and given a label only the new type has, never from a session opened since, or, on a held connection inside a transaction the app opened, from a statement cached before that swap, I7's exception, at most once per copy". Keep the rest as it is.

### EZ3. Should fix. R-1294 (step 50): the relabel it tells users to run breaks every column and query still on the kept type, and the notice doesn't say so

- **Where:** R-1294. "**When it's right:** a swap that has to keep the old type's labels as they are ... **How to proceed:** rename the kept type and its labels as morm's cutover does, the type to `_morm_retired_<migration>_<type>` and each label to `_morm_retired_<label>`".
- **What's wrong:**
  - **The advice undoes the reason.** The people the notice says are right to keep the labels are the ones told first to rename them. The likeliest reason to keep them is a column still on the old type.
  - **What happens to that column.** Its rows read `_morm_retired_a`, which no `PgEnum` label can be (R-0981), so reading it through a model fails with `Error::UnknownEnumValue` (R-0997). Queries and writes using the old labels get `22P02` naming `_morm_retired_m2_st`, from fresh sessions too, so morm takes them for a stale session, at up to `max` replacements per epoch. SQL that names `st_old` stops parsing. morm's own cutover is safe because it retires the column along with the type (R-0995), and a hand-made swap may not have.
  - **The other tools.** `import` and `verify` leave out a type named `_morm_retired_...` (R-1001), and `check_schema` names its labels through the warning sink (R-0997).
  - **`<migration>` means nothing here,** because a swap made outside morm has no migration. `m2` in the example reads as a migration id, and a hand-picked name that a later morm cutover of `st` generates would make that cutover's rename fail.
- **Evidence:** Postgres 17.11 and 18.6, above: the column read `_morm_retired_a`, and `WHERE c = 'a'` and `INSERT ... VALUES ('a')` got `22P02`.
- **Fix:** no owner choice is needed: the decision's two ways to proceed stay, and the notice says when each works.
  - **The condition:** rename only once no table column, default, constraint, view or function you still use is on the kept type, because its rows then read `_morm_retired_<label>` and its old labels stop working everywhere.
  - **Otherwise** end the sessions.
  - **The name:** any name starting `_morm_retired_` works, so use a word no migration of yours has, such as `_morm_retired_byhand_st`, and expect `check_schema` to mention the labels. R-1271's bullet can say "under a name holding `_morm_retired_`".

### EZ4. Minor. T-0319 (step 20), `proofs.md`: the load doesn't have the app's second run that the fix's model added

- **Where:** T-0319's load: "straight to Postgres 10% of calls are on held connections, 30% of them running two statements up to 10 ms apart, half of them inside a REPEATABLE READ transaction the app opened", with nothing after an error. `proofs.md`: "The load is T-0319's, apart from savepoints ... After an error in its own transaction the app rolls back and runs the statement again half the time".
- **What's wrong:** T-0319 states its load in full (EX1), and says each named setting's counts were measured "in full with the load above". The twenty-first round's weren't. Without the second run, a second statement in the app's failed transaction only gets `25P02`, so nothing in T-0319 ever reaches "comes back as it is at each call that meets it", which is what the correction to EY1 is about.
- **Evidence:** my model at T-0319's load. With the second run, the once-per-statement bound fails 78 of 256. Without it, 0 of 256. The 32nd and 33rd switches don't depend on it: 248 and 245 of 256 without, 246 and 242 with.
- **Fix:** no owner choice is needed. Add to the load: "after an error inside its own transaction the app rolls back and runs the statement again on the same connection half the time".

### EZ5. Minor. `proofs.md` and the PR body: two sentences on the model say more than it shows

- **"253 of 256 behind PgBouncer".** `proofs.md`, and the PR body's "EY2's case is in the model" bullet, say I8 without its exception fails there, "each by a kept type's `22P02` returned as it is from `tc + W` on". I8 has no such clause behind PgBouncer. Its clause there is "never visible after `RECONNECT` or `server_lifetime`", which the model checks as "no stale session is visible on a server session opened since the change". A kept type's error needs a session opened before the swap, so it can't break that clause. **Evidence:** my quiet setting behind PgBouncer breaks I8's own clause in 0 of 256, and has a kept type's error from `tc + W` on in 253 of 256. **Fix:** say the figure counts errors from `tc + W` on, to show how often the case comes up, and that the exception is needed only straight to Postgres.
- **"mostly kept swaps"** in the PR body's quiet setting: `proofs.md` says 4 in 9. **Fix:** "kept swaps 4 in 9 of them".

### EZ6. Minor, older than this round. R-1270 (step 14): the procedure's parameter test isn't limited to a `22P02`

- **Where:** R-1270's `on_error` starts `if param_holds_retired(call): return AsIs`, and nothing defines `param_holds_retired`. R-0981, R-1245's table and T-0153 limit the test to a `22P02` holding `_morm_retired_`.
- **What's wrong:** read as written, any error of a call whose parameter holds `_morm_retired_` returns as it is: a cached statement's `0A000` or `XX000`, or a function's `42804`. Then a stale statement is visible outside a transaction.
- **Evidence:** my model with the line as written, no other fault, at T-0319's load: I7 fails 120 of 256, and I8's held clause 58. With the `22P02` limit neither fails. Step 20's simulation would catch it, so this is a clash of texts, not a hole.
- **Fix:** no owner choice is needed. Write `if err is a 22P02 holding _morm_retired_ and param_holds_retired(call): return AsIs`. R-1270 joins the changed IDs.

## The fix's own choices, 165 to 172

| Choice | Verdict |
|---|---|
| 165, "at each call that meets it" | Right, by my model and on Postgres. Its place in the sentence is EZ1, and its load is EZ4 |
| 166, the 32nd switch | Right. It's caught well above every floor. That the clause's first part alone catches nothing depends on short holds: with longer ones it catches 88 of 256. Naming the whole clause is right either way |
| 167, the 33rd switch | Right. It's caught in both modes |
| 168, R-1293's "other than the one your call's value is for" | Right, and better than the decision's wording: a bad label on a session that isn't stale names `st`. On a stale one it names `st_old`, and releasing is right there too |
| 169, R-1294's place and examples | The place is fine. The example and its condition are EZ3 |
| 170, T-0311's Postgres assertion | Right, as built |
| 171, R-1271's own bullet | Right |
| 172, I4 counts a marked connection at its release | Right. The per-epoch bound held with it in every setting |

## Summary

| Severity | Findings |
|---|---|
| Blocks approval | None |
| Should fix | EZ1, EZ2, EZ3 |
| Minor | EZ4, EZ5, EZ6 |

No finding needs an owner choice.

- **EY1 to EY3 are written in as decided or recommended,** and the fix's correction to EY1 is right: the twenty-first review's once-per-statement bound was wrong. EZ1 carries on that review's own recommended sentence, which had the same scope error.
- **The machine isn't in question.** R-1270 didn't change, and it kept every bound my model checks. EZ1, EZ2 and EZ4 are T-0319's wording, EZ3 is the README notice's advice, EZ5 is the record, and EZ6 is one older line that disagrees with the requirements around it.
- **The 240 rule doesn't apply** to the 32nd switch.
- **The checks agree.** The changed IDs match the INDEX entry ID for ID. The checker reports 1294 requirements, 329 test obligations and 0 problems. CI is green on `93854c2`, 18 of 18 jobs on push and on the pull request.
- **The PR body is accurate for this range,** apart from EZ1's sentence and EZ5.

**PR #3 isn't ready for the owner yet.** The fixes are all wording, then a short check of the edits, the checker and CI.
