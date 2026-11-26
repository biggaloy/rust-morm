# Step 55 independent review, twenty-seventh round

PR #3 (`step-55-enum-proofs`), head `b98bada`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes.

This round covers `git diff e217a1c origin/step-55-enum-proofs`, three commits: the twenty-sixth review's fix, FD1 (`48d509f`) and FD2 (`41571ec`), and its `proofs.md` record with FD3 (`b98bada`). I also checked that nothing earlier regressed on the paths these commits touch.

## What I read

- **On the PR branch:**
  - `rules.md`, `contracts.md`'s `Error::SchemaChanged` comment, and `proofs.md`'s new parts for this round, "The twenty-sixth review's fix" and "The last round's model, carried on", and the FC1 record FD3 rewrote.
  - **Every changed requirement in full:** R-1271 (step 20); R-1253, R-1268, R-1272 and R-1288 (step 50).
  - **For coherence:** R-0718's rows B1 and D1 and its "Pending entries off", R-0984, R-0982's events E1, E5 and E7, R-1245's rows, R-1246's rows C2 and D2, R-1270's procedure (`before_run`, `on_error`, `session`, `request`, `take`, `on_moved`), R-1249, R-1258, R-1262's `stale_cooldown_max` row, R-1292, T-0312, and T-0319's I8 and I14.
- **On `origin/main`:** `reviews/step-55-review-26.md`, and INDEX's Decisions from "The EZ fix round's five open questions (2026-11-25)" to "The twenty-sixth review's fix (2026-11-26)".
- **The PR body,** read at the head: the twenty-sixth review's section, the FC1 bullet FD3 rewrote, the changed-IDs paragraph since `e217a1c`, choices 247 and 253 to 256, "Open questions" and "End-to-end result".

## What I checked and ran

- **CI on the head.** `b98bada` is green: `ci` on push (run 45861916574) and on the pull request (run 45861922030), 18 of 18 jobs each.
- **The coverage checker,** run on a `git archive` of the head. `check_coverage.py --step 55 --code crates tests benches fuzz .github` reports **1294 requirements, 329 test obligations, 0 problems**, and the plain form gives the same.
- **The manifest against INDEX.** From `e217a1c` to the head:
  - 5 requirements changed, each in `text` and `block`: R-1253, R-1268, R-1271, R-1272 and R-1288.
  - No test obligation changed, no ID is new or gone, no `line` moved, and `master_sha256` and `frame` didn't change.
  - That's exactly the list in INDEX's entry "The twenty-sixth review's fix". `contracts.md` and `coverage.md` didn't change, and they didn't need to: no signature changed, and no changed row's opening changed.
- **FD1 to FD3, each read literally against its recommendation:**
  - **FD1:** R-1253's second bullet has the review's words, "plus the transactions that run the statement before morm has rolled back the first, checked the schema's state and marked the statement pending", and its last bullet "plus the transactions that run it before morm marks it pending". R-1272's bound has the review's words, ending "marks the statement pending, a few more at a high rate". Its "Each failure asks for one repair, once" has "plus the transactions that run it before morm marks it pending", and "What you give up" the same with "a few more at a high rate". Choice 247 follows the new wording. I searched every requirement and test obligation for a gap clause that counts transactions by when they start. Each one left counts calls or runs by the entry's moment: R-0984's two, R-1271's three, and I8. By R-1270, "a run that misses an entry a step has just made counts as one already running when that step ran", so "begin before its entry is made" and "already running when its entry was made" are the same calls.
  - **FD2:** R-1268's "Shorter:" now says "about once per `stale_pending_expiry` per statement", as R-1253 and R-1292 do. Choice 253 leaves out the optional gap clause, which the review allowed. The sweep (choice 254) is right in its own places:
    - R-1271's kept-function bullet now has its exception, and R-1270 gives it, since rows B1 and D1 make the entry after the rollback.
    - R-1271 and R-1272 now say "or about once" in the second half of the several-statements bound.
    - R-1288 now says "once per connection". The sentence is scoped "straight to Postgres", where the evict gives that bound. What the change leaves unstated is FE2.
    - Choice 255 keeps R-1249's "once" behind PgBouncer, and isn't right (FE1).
  - **FD3:** `proofs.md`'s FC1 record and the body's FC1 bullet now quote each of R-1272's three places as they read, word for word.
- **Requirement by requirement, for the changed IDs.**
  - **R-1253's and R-1272's new clauses match R-1270's `on_error`:** `rollback(call.tx)`, then behind PgBouncer D1's `read(conn)`, or straight to Postgres for a `0A000` the gate's own read when the window is closed, then `entry(..)`. A run of the statement that begins before that entry is made takes no pending entry in `before_run` and runs cached. So it's exactly the set the clauses now name, and T-0319's I8 checks the same set.
  - **No contradiction** among R-0718, R-0984, R-1245's rows, R-1270's procedure, R-1271's guarantees and costs, T-0319's I8 and I14, and the README notices R-1253, R-1268, R-1272 and R-1292 on the bounds this round touched. The two exceptions are R-1249's and R-1258's cool-down bound (FE1) and R-1288's missing PgBouncer cost (FE2).
- **Loud notices and "use this when" lines.** R-1253, R-1268, R-1272 and R-1288 still say what's given up, when it's right and how to proceed, apart from FE2's missing clause. No opt-in call or setting is new in this range. R-1263 to R-1266 and R-1291 still send readers to their notices first.
- **The PR body.** The twenty-sixth review's section, the FC1 bullet, the changed-IDs paragraph since `e217a1c` and choices 253, 254 and 256 match the text, the manifest and the INDEX entry. Choice 255's claim is FE1. "End-to-end result" still cites the last head's CI (FE3).

### Postgres 17.11 (`/opt/pgsql-17`) and 18.6 (`/usr/bin`)

These were throwaway clusters under the worktree's `.claude/scratch/`, with the Unix socket under `~/.claude/scratch/`, `TMPDIR` set to the scratch, and psql 18.6 with `VERBOSITY verbose`. Both versions gave the same output, apart from the source line of the `0A000` (`plancache.c:753` on 17.11, `:866` on 18.6).

Session A, standing in for one PgBouncer server session, ran `CREATE FUNCTION f(t text) ... DECLARE v st; BEGIN v := t; ...`, called `f('a')`, and prepared `SELECT * FROM items WHERE id = $1` with `\parse`. A second session then did what R-1249 describes in one transaction:

- renamed `st` to `_morm_retired_st`, and its labels to `_morm_retired_7c1e9a04b52fd386_<label>`;
- created a new `st`;
- ran the reset, an operator made and dropped.

After that it ran `ALTER TABLE items ADD COLUMN b int DEFAULT 7`.

- **The stale function,** back in session A: `22P02` `invalid input value for enum _morm_retired_st: "a"` from `enum_in`, in each of three transactions one after another. A new session's `f('a')` answers `a`. So the server session fails at every call that reaches it, however many times morm has seen it before. How often a transaction gets `Error::SchemaChanged` for it is set only by morm's cool-down (FE1).
- **The kept server statement:** `0A000` `cached plan must not change result type` from `RevalidateCachedQuery`, in each of two transactions. A fresh text prepared after the change answers `1 | _morm_retired_7c1e9a04b52fd386_a | x | 7`. This is the stale statement FE2's sentence leaves without a cost behind PgBouncer.

**Not run:** PgBouncer (no build on this machine), and Postgres 15 and 16. No finding rests on them. FE1 and FE2 come from morm's own procedure, and the server's side is what's shown here.

### My own trace

This is a small Python trace of R-1270's `session()` behind PgBouncer inside a transaction (row D2):

- `in_cooldown`, then `cooldown(call.stmt)` and `SchemaChanged`;
- `take` and `on_moved` remove every pending and cool-down entry (R-0982's E5 and E1);
- a cool-down entry expires `stale_cooldown` after it was made (E7).

One cached statement meets the stale session at every call, as the probe shows. It's called in a transaction every 0.5 s with no `.retry(n)`, at `stale_cooldown` = 60 s. Another statement's `0A000` takes a salt at 10 s, and a migration's read is adopted at 30 s.

In 150 s the statement returns `Error::SchemaChanged` **4 times**: at 0 s, 10 s (after the salt), 30 s (after the adoption) and 90 s (the cool-down's expiry). R-1249 says "once".

## Findings

### FE1. Minor. R-1249 and R-1258 (step 50), `proofs.md` and choice 255: behind PgBouncer, a stale session's `Error::SchemaChanged` inside a transaction isn't "once", because any pool-wide repair or adoption ends the cool-down early

- **Where:**
  - **R-1249, "How morm recovers", last bullet:** "inside one it returns `Error::SchemaChanged` once, and then that statement's `22P02` returns at once for `stale_cooldown`, until PgBouncer replaces the server connection".
  - **R-1258's last paragraph,** which R-1249 points to ("as the PgBouncer notice says"): "inside one returns `Error::SchemaChanged`; after that its error returns at once for `stale_cooldown`, 60 seconds by default".
  - **`proofs.md`'s FD2 record and choice 255:** R-1249's "once" "is exact, because row D2 puts the statement in the cool-down at the failure, before the rollback, so a call of it failing after that returns its error as it is". INDEX's entry repeats it ("R-1249's "once" behind PgBouncer stays, being exact").
- **What's wrong:**
  - **The reason given is right, as far as it goes.** In R-1270's `session()`, `cooldown(call.stmt)` runs in the failure's own step, before `rollback(call.tx)`. So unlike B1 and D1 there's no gap, and no in-flight call adds a second error.
  - **But the cool-down doesn't last `stale_cooldown`.** `take` ("remove every pending and cool-down entry"), `finish_take` and `on_moved` all remove it: R-0982's E5 and E1, and T-0318 tests that "taking an action removes every pending and cool-down entry". The cap can also push it out (R-1262's `stale_cooldown_max`).
  - **Each removal costs another failure.** The server session stays stale, and fails at every call that reaches it (the probe). So after a salt any other statement asks for, after a migration morm reads, or after a push-out, the next such call inside a transaction is out of the cool-down. D2 gives it `Error::SchemaChanged` again, and it enters the cool-down again. Once the cool-down expires, it happens once more.
  - **R-1249's own setting makes this likely.** An online enum swap is a migration. Behind PgBouncer the pool adopts it at the first read a statement-class signal brings (C1 or D1), and the swap's other statements give such signals. So the adoption can come after the stale function's first D2, and ends its cool-down.
  - **R-1258 has the same error without "once".** "For `stale_cooldown`" is a fixed span, where it's really "up to".
  - **The record is wrong too.** FD2's sweep was meant to find exactly these bounds. `proofs.md` and choice 255 record this one as exact.
- **Evidence:**
  - R-1270's `session()` (`remove_entry`, `if in_cooldown: return AsIs`, `cooldown(call.stmt)`, `rollback`, `return SchemaChanged`), `take`, `finish_take` and `on_moved`. R-0982's E1, E5 and E7, and R-1262's `stale_cooldown_max` row.
  - The probe on 17.11 and 18.6 above: the stale function's `22P02` comes in every transaction in its session.
  - My trace above: 4 `Error::SchemaChanged` in 150 s, where R-1249 says one.
  - R-1268 already says the cap's part ("a statement pushed out costs a transaction with no `.retry(n)` one more `Error::SchemaChanged`"). Neither R-1249 nor R-1258 says the repairs' part.
- **Fix:** no owner choice is needed. This words the decided procedure's cost, as R-1253 and R-1272 already do for pending entries ("once more after each repair").
  - **R-1249:** "inside one it returns `Error::SchemaChanged`, and then that statement's `22P02` returns at once for up to `stale_cooldown`, 60 seconds by default, so about once per `stale_cooldown` per statement and pool, and once more after each pool-wide repair morm makes or migration it sees meanwhile, until PgBouncer replaces the server connection, as the PgBouncer notice says".
  - **R-1258:** "after that its error returns at once for up to `stale_cooldown`, 60 seconds by default, ending sooner at morm's next pool-wide repair or a migration it sees, after which the next such call inside a transaction returns `Error::SchemaChanged` again".
  - **`proofs.md` and choice 255:** record that the bound has no gap, but isn't once, and say what the two notices now read.
  - The alternative, keeping cool-down entries across a take or an adoption, would change the procedure and the tests (T-0318). I don't recommend it.

### FE2. Minor. R-1288 (step 50): "What you give up" states a stale statement's in-transaction cost only straight to Postgres, and none behind PgBouncer, where `servers = "several"` is mostly meant to be set

- **Where:** R-1288's "What you give up": "Until then a migration that raises `<n>` back to a value the pool already read is handled like a schema change made by hand: ... behind PgBouncer it keeps its old `g<n>` statement text, and stale state is repaired as it's found, ... Meanwhile, straight to Postgres, a connection opened shortly before that migration can return a stale-session error for up to `stale_cooldown`, and inside a transaction a stale statement can return `Error::SchemaChanged` once per connection, which `.retry(n)` hides."
- **What's wrong:**
  - **Choice 254 reads the sentence as being about straight to Postgres,** and changes "once" to "once per connection". That's right for that sentence.
  - **But then the notice gives no in-transaction cost behind PgBouncer.** Before the change, "a stale statement can return `Error::SchemaChanged` once" could be read for both modes. Now it can't, and nothing replaces it.
  - **Behind PgBouncer the cost is larger.** The notice itself says the pool keeps its old text there, and R-1288's own example, "a PgBouncer database entry listing several hosts", is the case `servers = "several"` is for. By R-1270 ("handled as a change made by hand ... its stale statements are repaired through the gate within `stale_cooldown` of the first confirmation or `0A000`") and R-1271's bullet on stale statements behind PgBouncer, such a statement inside transactions with no `.retry(n)`:
    - with pending checks on, fails about once per `stale_pending_expiry` per process, plus the transactions that run it before morm marks it pending;
    - with pending checks off, fails at every transaction's first attempt that runs it;
    - in both cases, until the salt its `0A000` asks for.
  - **R-1253 states all of this** for a change made by hand. R-1288 neither states it nor points to R-1253.
- **Evidence:**
  - The text above. R-1270's paragraph on a raised-back `<n>`, and R-1271's "Stale statements behind PgBouncer".
  - The probe above: a server statement prepared before a column change fails `0A000` in every transaction until a fresh text is prepared, as a PgBouncer server statement does until the salt.
- **Fix:** no owner choice is needed. After "once per connection" add: "and behind PgBouncer about once per `stale_pending_expiry` per process, plus the transactions that run it before morm marks it pending, or at every transaction's first attempt with pending checks off, until the salt its first failure asks for, within `stale_cooldown`, as the notice on schema changes made outside morm says; `.retry(n)` hides both". Choice 254 and `proofs.md`'s FD2 record should then say so.

### FE3. Minor. The PR body: "End-to-end result" still cites CI on the last round's head, `e217a1c`

- **Where:** the PR body's "End-to-end result", first bullet: "`ci` green on the head `e217a1c`, on push (.../runs/45854696985) and on the pull request (.../runs/45854711326), 18 of 18 jobs each".
- **What's wrong:** the head is `b98bada`. Every earlier round moved this line to its own head, and this round didn't.
- **Evidence:** `gh run list --branch step-55-enum-proofs`: runs 45861916574 (push) and 45861922030 (pull request) on `b98bada`, both `success`, 18 of 18 jobs each.
- **Fix:** name the fix round's head and its two runs, and read the body again after `gh pr edit`.

## Summary

| Severity | Findings |
|---|---|
| Blocks approval | None |
| Should fix | None |
| Minor | FE1, FE2, FE3 |

No finding needs an owner choice.

- **FD1 to FD3 are written in as recommended,** read literally, and the procedure is unchanged.
  - Every gap clause in R-1253 and R-1272 now counts runs before the entry, as R-1271, R-0984 and I8 do, and as R-1270's `before_run` and `on_error` give.
  - R-1268's rate has its "about".
  - `proofs.md` and the body quote R-1272's three places as they read.
- **FD2's sweep is right in the places it changed,** but:
  - **FE1:** it kept R-1249's behind-PgBouncer "once" as exact. A stale session's cool-down ends at every pool-wide repair, adoption or push-out, as well as at its expiry, so the error comes again. R-1258 has the same "for `stale_cooldown`".
  - **FE2:** scoping R-1288's sentence to straight to Postgres left that notice with no in-transaction cost behind PgBouncer.
- **FE3:** the body's CI line names the last head.
- **The checks agree.**
  - The changed IDs match INDEX's entry exactly: 5 requirements. `contracts.md` and `coverage.md` needed no change.
  - The checker reports 1294 requirements, 329 test obligations and 0 problems, in both forms.
  - CI is green on `b98bada`, 18 of 18 jobs on push and on the pull request.
  - Choices 253, 254 and 256 match the text.
- **The probes on 17.11 and 18.6** show a stale function's `22P02` and a kept server statement's `0A000` in every transaction of their session, and a new session or a fresh text answering.

**PR #3 isn't ready for the owner yet.** The fixes are wording only, then a check of the edits, the checker and CI.
