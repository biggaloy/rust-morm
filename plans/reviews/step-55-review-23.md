# Step 55 independent review, twenty-third round

PR #3 (`step-55-enum-proofs`), head `e417bc8`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes.

This round covers `git diff 93854c2 origin/step-55-enum-proofs`, 19 commits in five rounds: the twenty-second review's fix (EZ1 to EZ6, `b86c5d6`, `19be048`), then the owner's five answers (to `a1a6d0b`), three answers (to `1c27277`), two answers (to `64820fe`) and the last answer (to `e417bc8`). I also checked that nothing earlier regressed on the paths these rounds touch.

## What I read

- **On the PR branch:**
  - `rules.md` in full, `contracts.md`'s conventions and `Error` comment, and the five rounds' parts of `proofs.md`.
  - **Every changed requirement in step 14 in full:** R-1245, R-1270 and its procedure, R-0718 with rows A1 to D1, R-0981 with rows A2 to B4, and R-1246's row C2 and its second checkout.
  - **Step 14's changed test obligations:** T-0151, T-0153 and T-0318.
  - **Step 20:** R-0984, R-1271 in full, T-0312 and T-0319 in full.
  - **Step 50:** R-1249, R-1253, R-1258, R-1268, R-1293 and R-1294, and R-1292 for coherence.
  - **Elsewhere:** T-0309 (step 38), and grep checks of every step file for what the answers removed.
- **On `origin/main`:** `reviews/step-55-review-22.md`, and INDEX's five Decisions entries for these rounds: the twenty-second review's, "The EZ fix round's five open questions", "The five answers' three open questions", "The three answers' two open questions" and "The two answers' open question".
- **The PR body:** read at the head, with choices 187 to 234, and GitHub's edit history of it.

## What I checked and ran

- **CI on the head.** `e417bc8` is green: `ci` on push (run 45794551123) and on the pull request (run 45794566576), 18 of 18 jobs each.
- **The coverage checker,** run on a `git archive` of the head. `check_coverage.py --step 55 --code crates tests benches fuzz .github` reports **1294 requirements, 329 test obligations, 0 problems**, and the plain form gives the same.
- **The manifest against INDEX, round by round.** From `93854c2` to the head, 19 requirements and 12 test obligations changed. For requirements both `text` and `block` changed. No ID is new or gone, and `master_sha256` and `frame` didn't change. Split at each round's write-in commit, the manifest's changed IDs match each INDEX entry's list exactly:

  | Range | Round | Changed IDs |
  |---|---|---|
  | `93854c2..19be048` | The EZ fix | 8 |
  | `19be048..a1a6d0b` | The five answers | 30 |
  | `a1a6d0b..1c27277` | The three answers | 12 |
  | `1c27277..64820fe` | The two answers, with R-1270 unchanged as the entry says | 10 |
  | `64820fe..e417bc8` | The last answer | 11 |

  `contracts.md` changed in two places only, line 21 (`c9ebdd6`) and the `SchemaChanged` comment (`1b12465`), as the entries say. `coverage.md` didn't change.
- **EZ1 to EZ6, each against its finding:**
  - **EZ1:** I8's held exception now stands over both halves, at any time.
  - **EZ2:** I9's loud half says "visible", with the held connection's cached statement as I7's exception.
  - **EZ3:** R-1294 keeps both ways to proceed. It adds the condition (nothing still on the kept type), the `byhand` name, and a paragraph on what breaks if you rename anyway.
  - **EZ4:** T-0319's load has the app's second run, half the time.
  - **EZ5:** the PR body's history section says "kept swaps 4 in 9", and that I8 needs no exception behind PgBouncer.
  - **EZ6:** R-1270's `on_error` limits the parameter test to a `22P02` holding `_morm_retired_`.

  All six are in as recommended.
- **The answers, read literally:**
  - **Five answers:**
    - (1) A fresh run's session-class error goes on in A1 and C1, and the row order puts the seventh before the second.
    - (2) `_morm_retired_<random>_<label>` is used everywhere I looked: `contracts.md`, steps 38, 41, 44, 50, 52, 56 and 58. `_morm_retired_a` stays in T-0309 only as text a session must refuse, which is right.
    - (4) Row C2 gives its connection back first.
    - (5) The `XX000` bit.
  - **Three answers:** one row order in both modes. The copied-text case is loud in R-1249 and R-1253.
  - **Two answers and the last answer:** the bound-texts test and then the name test are gone.
    - I found none of the value's mark, the colon, `LATIN2`, "more than one non-built-in type", the name comparison or the language readings in any step file, `contracts.md` or the manifest's texts.
    - What's left of an exchange of names, a cast written `app.st` and the `entrada` case are T-0151 cases that still test a real repair.
    - The choices that only served the test are marked dropped, with their numbers kept.
- **Requirement by requirement, for the changed IDs:**
  - R-1245's rows and order, R-1270's `class` and rows A1, B1 and D1, R-0718's table, R-1271's bullets, T-0319's I7, I8, I9 and I14, and R-1253 and R-1294 agree on what the second row does in each context.
  - FA1 to FA4 are where they don't.
  - T-0319's fault list counts 7 + 1 + 9 + 7 + 3 + 3 + 1 + 1 + 1 + 6 = 39, as it says, and switch 36's new wording matches its setting.
- **Loud notices and "use this when" lines.**
  - R-1253's new bullet has all four parts: what morm does, what you give up, when it's right and how to proceed.
  - R-1294 has all four too, with EZ3's condition.
  - R-1249 says loudly what copied text costs, and R-1253 says how to proceed.
  - No opt-in call or setting is new in this range, so no "use this when" line is due.
  - FA2 and FA3 are about two notices that didn't follow the second row's change.
- **The PR body.** Its four answer sections, the changed-IDs paragraphs and choices 187 to 234 match the text and the INDEX entries:
  - Each round's ID list matches the manifest split above.
  - Choices 195, 196, 204 to 208, 210, 211, 217, 218 and 227 are marked dropped.
  - Choice 232's lowest catch, 54, is `proofs.md`'s and INDEX's figure.
  - FA2 also touches choice 231's wording.
- **The move of earlier rounds to `plans/reviews/step-55-pr-history.md`, checked from GitHub's edit history.** `gh api graphql` lists 43 `userContentEdits` on PR #3, and I compared the bodies section by section.
  - **The last body before the move,** the edit of 2026-11-25 15:15:41Z, 261,450 bytes: each of its 23 earlier rounds' sections, from "The consolidated stale-session rewrite" to "The twenty-second review", is word for word in the history file (`de22c3e`).
  - **Every other section** is word for word in today's body, apart from deliberate updates: T-0176 added to step 52's row, the five questions marked answered, and the head's CI line.
  - **The body before that,** at 10:34:33Z: its differences are EZ1's and EZ5's own corrections, which the EZ round made to the twenty-first round's section before the move.
  - **The body at `64820fe`,** 06:58:05Z: its differences from today's are the questions marked answered and the dropped choices.
  - **So nothing was lost in the move.**

### Postgres 17.11 (`/opt/pgsql-17`) and 18.6 (`/usr/bin`)

Throwaway clusters under the worktree's `.claude/scratch/`, Unix sockets only under `~/.claude/scratch/`, `TMPDIR` in the scratch, psql 18.6 with `VERBOSITY verbose` for the routine. Both versions gave the same results, apart from line numbers:

- **R-1245's "Nothing else takes it."**
  - A bad label to `'zz'::st`, to a domain over `st`, to a composite over it and to `st[]` each gave `22P02` with routine `enum_in` (`enum.c:129`). A label over 63 bytes gave the same at `enum.c:119`, the second error site.
  - A plpgsql handler's bare `RAISE` kept `22P02` and `enum_in`.
  - A function's `RAISE ... USING ERRCODE = '22P02'` of the same words gave `exec_stmt_raise`. `'zz'::bool` gave `boolin` and `'zz'::int4` gave `pg_strtoint32_safe`.
  - A label added in an open transaction gave `55P04` from `check_safe_enum_use`.
- **In other languages** (18.6, whose package has translations, under `es_ES.utf8` and `de_DE.utf8`): the message was translated, and the SQLSTATE and the routine `enum_in` were unchanged, the re-raise's included. The `/opt` build of 17.11 has no message catalogs, so it answered in English.
- **The cases the second row now takes,** with each statement prepared before the change:
  - **Bad stored text, nothing changed:** a cached `(c::st)::text` and a freshly prepared one both gave the same `22P02`, as did a bad bound label. So the fresh run returns it as it is.
  - **After a hand-made swap keeping the old type:** cached `(c::st)::text` and `(($1::text)::st)::text` gave `22P02` naming `st_old`, and fresh statements in the same session answered.
  - **A kept type's stale function:** a cached `SELECT f(c)` and a freshly prepared one both gave `22P02` naming `st_old`, and a new session answered. Inside `BEGIN` the next statement got `25P02`.
  - **After an exchange of the names of the second `st` and `kind`:** the cached cast got `invalid input value for enum kind`, naming a current type and not its own cast's, and a fresh statement answered.

  Every one is as R-1245, R-1271 and R-1294 state.

**Not run:** PgBouncer and Postgres 15 and 16. No finding rests on them, and the second row's signal comes from Postgres unchanged.

### My own model

It's a small one in Python, written from R-0718's rows A1 and B1, its "Entry", R-1270's `before_run` and pending-entry table, and `take`. It covers one cached statement straight to Postgres that gets calls in transactions with no `.retry(n)`, 30% of them with a bad label, and nothing stale. It drew 64 runs of 20 s for each setting, with a call every 20 ms on average. It counts the `Error::SchemaChanged` the statement returns for a bad label in each `stale_pending_expiry`:

| Setting | `SchemaChanged` per `E`, mean and max |
|---|---|
| Entries on, `E` = `W` = 1 s, nothing else | 0.96, 1.00 |
| Entries on, `E` = `W` = 1 s, a pool-wide action for another statement each `W` | 1.02, 1.05 |
| Entries on, `E` = 2 s, `W` = 1 s, the same actions | 2.03, 2.10 |
| Pending entries off | 14.91, 16.80 |

R-1271's bullet bounds the first three, as it says: once per `E`, and once more after each thing that ends an entry sooner. The fourth is FA2. It's the only result I rest on the model, and it follows from R-0718's "Entry: nothing, if pending entries are off" read with B1.

## Findings

### FA1. Should fix. R-1245 and T-0318 (step 14): "only the seventh row reads the message" contradicts R-1245's own table

- **Where:**
  - R-1245's paragraph "The second row reads no message text": "Of the stale-session signals, only the seventh row reads the message, for the identifier `_morm_retired_`".
  - T-0318: "the matcher is given the SQLSTATE, the routine, whether the statement is cached and, for the seventh row alone, the message, never a bound text".
  - INDEX's entry for the last answer says the same: "the seventh row is the only one that reads the message".
- **What's wrong:** four other signals are recognised by the message, in R-1245's own table and the paragraph after it.
  - The fourth and fifth rows: `XX000` "message starting `cache lookup failed for type`". The fifth is a session-class signal.
  - The ninth: "message starting `could not open relation with OID`".
  - The English second signals for `0A000` and `42804`.

  These are `elog` texts, or a fallback, never translated, so R-1245's language rule holds. But as written, T-0318 asserts a matcher interface that can't recognise the `XX000` rows. A step 14 session that builds the matcher to pass T-0318 literally can't build R-1245's table.
- **Evidence:** both sentences came in with `a28b1fd`. The table's fourth, fifth and ninth rows, and the paragraph "An English message is a second signal", are unchanged in the same file.
- **Fix:** no owner choice is needed.
  - R-1245: "Of the `22P02` rows, only the seventh reads the message, for the identifier `_morm_retired_`, which no translation changes; the `XX000` rows read only `elog` text, which is never translated".
  - T-0318: "the message, which of the `22P02` rows only the seventh reads, never a bound text".
  - The coordinator may want INDEX's record to say "the only `22P02` row".

### FA2. Should fix. R-1253 and R-1268 (step 50), R-1271 and T-0319 (step 20): the second row's cost inside a transaction is stated as at most once per `stale_pending_expiry`, but the settings change it, and with pending entries off it's every call

- **Where:**
  - R-1253's loud bullet: "inside one an `Error::SchemaChanged` before its own `22P02`, ... **at most** once per `stale_pending_expiry`, 60 seconds by default, per statement".
  - R-1271's "What the second row costs": "So a statement given such text inside transactions returns `Error::SchemaChanged` for it at most once per `E`, ...", and its kept-function bullet's "once per pending entry".
  - I9's loud half in T-0319: "`Error::SchemaChanged` and a pending entry inside one".
  - R-1268, the settings notice: its `stale_pending_expiry` bullet and "What `"off"` turns off".
- **What's wrong:**
  - **With pending entries off,** `stale_pending_expiry = "off"` or `"0s"`, or `stale_pending_max = 0`, R-0718's "Entry" makes nothing. B1 evicts the statement, so its next run prepares it cached again. Every bad label, and every bad stored text, that any cached statement meets in a transaction then returns `Error::SchemaChanged` in place of its `22P02`, at every such call, in both modes. R-1271's "at most once per `E`" and I9's "a pending entry" don't hold there. R-1268's paragraph on what `"off"` turns off, which ends "Leave them on unless you've read that and accept it", doesn't mention this, though it's now the largest cost of turning them off.
  - **R-1253's "at most" is not a bound.** R-1271 states the true bound: once per `E`, "once more after anything that ends that entry sooner, a pool-wide action, a moved `<n>`, the cap", calls already running, and behind PgBouncer one per call whose read found `<n>` moved. With `E` longer than `W`, which R-1268 allows, a pool-wide action for any other statement each `W` makes it about once per `W`. The loud notice gives a smaller bound than the requirement it's loud about.
  - **R-1268's `stale_pending_expiry` bullet** gives as the cost of a shorter value only stale statements after a change made by hand. Since the second row's change, a shorter value also means more `Error::SchemaChanged` for bad labels and bad stored text, once per `E` per statement.
- **Evidence:**
  - From the procedure: entries off, `entry(..)` does nothing, and B1 returns `SchemaChanged` and evicts. At the next call, `before_run` finds no entry and runs the statement cached, so the second row takes it again.
  - My model above: 14.91 `SchemaChanged` per `E` with entries off, against 0.96 on; 2.03 per `E` with `E` = 2 s and `W` = 1 s.
- **Fix:** no owner choice is needed: this states the decided rule's cost where it's read.
  - **R-1253:** "about once per `stale_pending_expiry` per statement, once more after each repair morm makes for any statement, and at every such call with pending checks off".
  - **R-1271's bullet and the kept-function bullet:** "with pending entries off, at each such call inside a transaction".
  - **I9:** "and a pending entry, unless entries are off".
  - **R-1268:** in the `stale_pending_expiry` bullet, "Shorter: ... and a bad enum value inside a transaction gets `Error::SchemaChanged` more often". In "What `"off"` turns off", "every bad enum value a cached statement meets inside a transaction then returns `Error::SchemaChanged` in place of its `22P02`, at every call, in both modes; `.retry(1)` gets the `22P02`".

### FA3. Should fix. R-1292 (step 50): the savepoint notice still says `SchemaChanged` comes from a schema change and every other error rolls back only to the savepoint, which a bad enum value in a savepoint now breaks

- **Where:** R-1292, beside transactions and `tx.transaction(..)`:
  - "Not `Error::SchemaChanged`, which a schema change made outside morm gives inside a transaction: like a deadlock or a serialization failure, it fails the whole transaction."
  - "Every other error inside a savepoint, such as a unique violation, still rolls back only to the savepoint."
- **What's wrong:**
  - Since the two answers and the last answer, the second row takes every cached statement's `22P02` from `enum_in`. Inside `tx.transaction(..)`, B1 or D1 rolls the whole transaction back and returns `Error::SchemaChanged`, with no schema change at all: T-0312 says so, "inside a transaction, a savepoint included, the decision is `Error::SchemaChanged`".
  - A common pattern now loses its whole transaction: a savepoint that tries a user-supplied enum value and catches its error. So does one around a cached statement that converts a column holding bad text.
  - R-1292 tells that user the opposite. R-1253 and R-1271 state the cost but don't name the savepoint, and R-1292 is the notice a reader of savepoints reads.
- **Evidence:** R-1270's `on_error` takes a cached `22P02` to `cls == Statement`. Then `!ctx.out` leads to `rollback(call.tx)`, which "rolls the whole of it back, savepoints included", and `return SchemaChanged`. R-1292 is unchanged since `93854c2`.
- **Fix:** no owner choice is needed. In R-1292, say that `Error::SchemaChanged` also comes, with no schema change, from a bad enum value that a statement morm caches meets: a label your code sends, or text a column holds. Inside a savepoint it fails the whole transaction. To catch it there, check the label first, or give the transaction `.retry(1)`, whose next attempt returns the `22P02` itself, which the savepoint then clears. Keep "every other error" for the rest.

### FA4. Minor. T-0151 and T-0153 (step 14), T-0312 (step 20): repeated bad labels are asserted as if no pending entry followed the first

- **Where:**
  - T-0151: "With nothing changed, a label no type has, `zz` and the type's own name among them, given to each of those statements, and bad stored text ..., gets `Error::SchemaChanged` inside a transaction with no `.retry(n)`, its `22P02` as it is under `.retry(1)`, and outside one its `22P02` as it is after one more run, in each language."
  - T-0153: "a bad label on a cast that isn't stale, whatever other text the call binds, ... inside one `Error::SchemaChanged` and then its `22P02` as it is on `.retry(1)`'s second attempt".
  - T-0312: `conv`'s case, "gives `SchemaChanged` and then its `22P02` as it is ... on the second attempt ..., ten times". That sentence is older than this range, unchanged since `93854c2`, but it now asserts the second row's cost.
- **What's wrong:**
  - The first in-transaction failure makes the statement's pending entry. For `stale_pending_expiry`, 60 s by default, every run of that statement is then unnamed, so a bad label returns its `22P02` at the first attempt, with no `Error::SchemaChanged`.
  - Outside a transaction it comes back from the unnamed run, with no "one more run".
  - So the second of two languages, the second of the `zz` and type-name labels, the call with other bound text, and the second of `conv`'s ten each get a different outcome than the one asserted, unless each runs on a new pool or after the entry ends.
  - T-0312's own D1 case ("the right answer unnamed until the entry expires") and R-1271's bullet have it right. The test obligations don't.
- **Evidence:** R-0718's "pending entry" bullet ("Before any row, every run of a statement whose entry exists is unnamed"), `before_run`'s note "an error from an unnamed run goes to on_error as one from a statement that isn't cached", and the second row's "from a cached statement".
- **Fix:** no owner choice is needed. Have each case run on a pool with no entry for its statement, a new pool or after a pool-wide action, or state what follows it. For example, in T-0312: "the first of ten such transactions gives `SchemaChanged` and then its `22P02` on the second attempt, and the other nine, while the entry lasts, its `22P02` as it is at the first attempt, unnamed, with nothing requested".

## Summary

| Severity | Findings |
|---|---|
| Blocks approval | None |
| Should fix | FA1, FA2, FA3 |
| Minor | FA4 |

No finding needs an owner choice.

- **EZ1 to EZ6 are written in as recommended,** and the owner's answers are written in as decided, read literally. The bound-texts test and the name test are gone everywhere they should be.
- **The machine isn't in question.** R-1270's procedure does what the answers decided. My Postgres checks on 17.11 and 18.6 match R-1245's "Nothing else takes it" and every stale case the second row now takes.
- **The findings are about the text around the procedure:**
  - FA1 is a sentence of R-1245, and T-0318's matcher interface, that contradict R-1245's own table.
  - FA2 and FA3 are notices and bounds that didn't follow the second row's cost into the settings and the savepoint.
  - FA4 is test obligations that leave out the pending entry.
- **The checks agree.**
  - The changed IDs match the five INDEX entries, round by round.
  - The checker reports 1294 requirements, 329 test obligations and 0 problems.
  - CI is green on `e417bc8`, 18 of 18 jobs on push and on the pull request.
  - The PR body is accurate for this range, and its move of earlier rounds lost nothing.

**PR #3 isn't ready for the owner yet.** The fixes are all wording, then a short check of the edits, the checker and CI.
