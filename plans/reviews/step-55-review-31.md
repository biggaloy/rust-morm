# Step 55 independent review, thirty-first round

PR #3 (`step-55-enum-proofs`), head `77c3d50`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes.

This round covers `git diff 2fcbbcd origin/step-55-enum-proofs`, two commits: the thirtieth review's fix of FH1 and FH2 (`1ae0657`) and its `proofs.md` record (`77c3d50`). I also checked that nothing earlier regressed on the paths these commits touch.

## What I read

- **On the PR branch:**
  - `rules.md`; `proofs.md`'s new sections "The thirtieth review's fix" and "The last round's model, carried on", and the FG1 bullet and span-check correction they amend.
  - **Every changed ID in full:** R-0975 (step 11); R-0981, R-0982, R-1246 and R-1270's gate bullet and summary (step 14); R-1271 and T-0319 (step 20); R-1249, R-1250, R-1253, R-1268, R-1272, R-1288 and R-1294 (step 50); R-1040 (step 59).
  - **For coherence:** step 14's R-0718, R-1245's rows and "A confirmation", R-1270's state table (the epoch row), its read order and steps, rows A2 to B4 and the replacement rules (beside the old connection, in its slot, `53300`); step 20's R-0984, R-1271's other bullets and T-0319's I1 to I14; step 50's R-1258 and R-1292.
- **On `origin/main`:** `reviews/step-55-review-30.md`, and INDEX's Decisions from "The EZ fix round's five open questions (2026-11-25)" to "The thirtieth review's fix (2026-11-27)".
- **The PR body** at the head, all of it.

## What I checked and ran

- **CI on the head.** `77c3d50` is green: `ci` on push (run 45925921910) and on the pull request (run 45925927944), 18 of 18 jobs each, both on head SHA `77c3d508c7a6…`.
- **The coverage checker,** run on a `git archive` of the head. `check_coverage.py --step 55 --code crates tests benches fuzz .github` reports **1294 requirements, 329 test obligations, 0 problems**. The plain form gives the same.
- **The manifest against INDEX.** From `2fcbbcd` to the head:
  - 14 requirements changed, each in `text` and `block`: R-0975, R-0981, R-0982, R-1040, R-1246, R-1249, R-1250, R-1253, R-1268, R-1270, R-1271, R-1272, R-1288 and R-1294.
  - 1 test obligation changed, in `text`: T-0319.
  - No ID is new or gone (1,294 and 329 either side), no `line` moved, and `master_sha256` and `frame` didn't change.
  - That's exactly the list in INDEX's entry "The thirtieth review's fix" and in the PR body's changed-IDs paragraph and FH2 bullet. Only `proofs.md`, five step files and `manifest.json` differ. `contracts.md` and `coverage.md` didn't change and didn't need to, as the checker confirms.
- **FH1, read literally against its recommendation:**
  - **R-1249** reads "with an error other than another `22P02` and other than one that can stop it before it reaches the value, …", followed by the review's sentence "One that fails with another `22P02`, such as another bad label or bad text for a number, isn't: you get that error as it is, and nothing more is replaced." Both are word for word. The sentence holds against the procedure: outside a transaction row A2's caller gets the new connection's run, and a cached statement's `22P02` from `enum_in` there costs one more run and returns as it is; inside one, `.retry(n)`'s next attempt runs unnamed and returns it as it is; `confirms(r, err)` is false for the same SQLSTATE, so nothing is requested.
  - **R-1250** reads "and answers, or fails with an error other than a `22P02` and other than one that can stop it before it reaches the value, which morm takes for a cured stale session", word for word.
  - **The text beside them** is free of the same error. I searched every step file for "another way" and "cured stale session": R-0981, R-0984 and R-1040 state the case by SQLSTATE and the no-confirm list, and T-0319's world line is the only other hit.
  - **T-0319** takes the optional change: its world now fails stored text with another `22P02` from `enum_in` or from another routine, "which confirms nothing", and I13 judges row A2's run and B3's next attempt against the session-class error before them. That's consistent with A2's and B3's "requests nothing more" for a `42804` or `XX000`, since I13 says what confirms, not what requests.
  - **The records:** `proofs.md`'s FG1 bullet, choice 268 and the PR body's twenty-ninth section now say what the text reads. One record of the model's world contradicts T-0319 (FI3).
- **FH2, read literally:**
  - **T-0319's I4** is the review's words: "plus `2 × max` per move of the pool's last-read `<n>` from `stale_cooldown` before the span's start to its end, whenever the change was committed".
  - **R-1271's "Churn"** is the review's words, with the reason the review gave. It holds for replacements, but the bound it states is on opens, and an epoch's opens are `max + 2` (FI1).
  - **R-0981** has "plus `max` per migration the pool learns from `stale_cooldown` before the span to its end", the same for its `2 × max`, and the sentence defining a learned migration. I checked both bounds against the epoch and gate rules. Collapsing what causes before the span replace in it, one replacement per slot, and counting one more epoch and one more action per move inside the span, gives at most `2 × max` plus `max` per move for epochs and `4 × max` plus `2 × max` per move with actions. Both hold.
  - **R-1268's optional sentence** says when a migration counts. "morm reads it only when the pool starts and after a stale error" matches R-0982 and step 14's list of reads in both modes (start; R-0981's test and row B2's `XX000`; an unread request while the window is closed, which follows a stale signal; behind PgBouncer rows C1 and D1).
  - **The records:** `proofs.md`'s span-check correction and choice 272 now say the text needed the change.
- **FH2's sweep,** every "per migration", "after each migration", "migration … starts" and "once per migration" in the step files and `contracts.md`. Every pool bound now names the migrations the pool learns or sees. The hits left unscoped aren't pool bounds: preflight "before each migration" (steps 33, 34, 40), R-1258's `RECONNECT`, the decision "per migration" (R-1085, step 59), the checks around each migration (step 50, line 631), and R-0984's "a migration starts a new epoch", which is wrong as it stands (FI2).
- **The places listed as read and left as they are.** I spot-checked each:
  - **The per-epoch bounds** count no migration. That holds.
  - **T-0311's span bound** ("at most twice that in any span of `stale_cooldown`") is in a test with no schema change. That holds.
  - **I1 and I3** state the change that excuses a pair by snapshot. That holds, and it doesn't conflict with I4's moves.
  - **R-1258's "after each migration"** is `migrate`'s `RECONNECT`. That holds.
  - **"a migration it sees"** in R-1249's PgBouncer bullet and R-1268 already scopes. That holds.
  - **R-1292's "for up to `stale_cooldown` after a migration"** is a time from the commit, and it holds. The epoch in force at the commit started before it, so it ends within `stale_cooldown` of it. Any later epoch starts after the commit, so every connection holding older code is old in it.
  - **R-0984's "a migration starts a new epoch, so its stale sessions are never taken for that text"** doesn't hold (FI2).
  - **R-0981, R-0984 and R-1040 for FH1:** each states the case by SQLSTATE. That holds.
- **Requirement by requirement, for the changed IDs.** Apart from FI1 and FI2, I found no contradiction among R-0718, R-0984, R-1245's rows, R-1270's procedure (unchanged this round apart from its gate bullet's bound), R-1271's guarantees and costs, T-0319's I-clauses, and the README notices R-1249, R-1250, R-1253, R-1268, R-1272, R-1288, R-1292 and R-1294. Each behaviour the new wording states is driven by T-0319's world and asserted by I4 or I13, and the same-SQLSTATE rule is also asserted by T-0318.
- **Loud notices and "use this when" lines.** The notices changed only inside their bounds, plus R-1268's one sentence. Each still says what it does, when it's right, what's given up and how to proceed. No opt-in call or setting is new in this range, and R-1263 to R-1266 still give each stale-session setting its line.
- **The PR body:**
  - **It matches the text, the manifest and INDEX's entry:** the section "The thirtieth review, since `2fcbbcd`", the changed-IDs paragraph and table row for step 11, choices 274 to 280, the corrected choices 268 and 272 and the twenty-ninth section, "Open questions" and "End-to-end result".
  - **Its quotations** of R-1249, R-1250, I4, "Churn", R-0981 and R-1268 are word for word.
  - **Its numbers** match `proofs.md`: 572 is 289 plus 283, 130 is 71 plus 59, 47, 213 and 22 of 256 for break 129, 69 to 256 and 422 to 2,048 for the switches, and seed 2,480.
  - **Choice 276** says I4 counts moves alone, which is what the text and INDEX say.
  - **Choice 277** groups R-1246 with the rate bounds "between two of them". R-1246 actually says "per migration the pool learns, by a move of its last-read `<n>`, whenever it was committed". `proofs.md` describes that exactly, and the bound is right either way, so I don't raise it.
  - **The body is 229,513 bytes,** under GitHub's 262,144.

### Postgres 17.11 (`/opt/pgsql-17`) and 18.6 (`/usr/bin`)

I ran throwaway clusters under the worktree's `.claude/scratch/`, with Unix sockets under `~/.claude/scratch/`, `TMPDIR` set to the scratch, `lc_messages = C` and psql with `VERBOSITY verbose`. Both versions gave the same results, apart from source lines.

The setup is an enum `st ('a', 'b')` and a table `src (id int, txt text, num text)`. The function `f(p int)` does `DECLARE v st; n int; BEGIN SELECT txt INTO v …; SELECT num::int INTO n …; RETURN …`. It was called once to compile it, then `ALTER TYPE st RENAME VALUE 'a' TO '_morm_retired_7c1e9a04b52fd386_a'`.

| The row holds | `f(1)` on 17.11 and 18.6 |
|---|---|
| `txt` = `_morm_retired_7c1e9a04b52fd386_x` | `22P02` `invalid input value for enum st: "_morm_retired_…_x"`, `enum_in`, `enum.c:129` |
| `txt` = `zzz` | `22P02` `invalid input value for enum st: "zzz"`, `enum_in`, `enum.c:129` |
| `txt` = `b`, `num` = `x` | `22P02` `invalid input syntax for type integer: "x"`, `pg_strtoint32_safe` (`numutils.c:616` on 17.11, `:615` on 18.6) |
| `txt` = `b`, `num` = `_morm_retired_q` | `22P02` `invalid input syntax for type integer: "_morm_retired_q"`, `pg_strtoint32_safe` |
| `txt`, `num` = `NULL` | answers `null null` |
| any, under `statement_timeout = 10ms` with `pg_sleep(1)` | `57014`, `ProcessInterrupts` |

So the errors FH1 names as not confirming all carry SQLSTATE `22P02`, whichever routine raises them, and the timeout is on the no-confirm list. R-1249 and R-1250 now say both.

**Not run:**
- PgBouncer: nothing in this range is about it, and there's no build on this machine.
- Postgres 15 and 16.
- A coded model of my own. FI1 rests on a hand trace of the text's own rules, given in full below. FI2 rests on R-0981's and R-1271's own sentences.

## Findings

### FI1. Minor. R-1271's "Churn" and T-0319's I4 (step 20): the span bound on stale-driven opens adds `2 × max` per move, but the move's epoch can make `max + 2` opens. With enough moves in one span, the bound fails under the role limit I4 names.

- **Where:**
  - **"Churn":** "in any span of `W`, at most two epochs' and two *replace* actions' worth, `4 × max + 4` stale-driven opens, plus `2 × max` per migration that the pool learns, …, because each such move lets a new epoch of up to `max` replacements start and one *replace* through a closed window". The same bullet says: "per epoch, at most `max` stale-driven replacements, … and at most `max + 2` connection opens".
  - **T-0319's I4:** "per epoch at most `max` stale-driven replacements and `max + 2` stale-driven connection opens, … in any span of `stale_cooldown` at most `4 × max + 4` stale-driven opens, plus `2 × max` per move …, under a role limit at the pool's size too".
- **What's wrong:**
  - **The arithmetic isn't consistent.** The base `4 × max + 4` is two epochs' `max + 2` and two actions' `max`. By the same count, a move's extra epoch and extra action are worth `2 × max + 2` opens. The text allows `2 × max`, and its reason counts replacements, not opens.
  - **Why the base hides it at first.** In practice the base has slack: what a span inherits from before it collapses to one replacement per slot. With the role limit, each epoch makes one failed open beside, with its first replacement. So each move adds up to `2 × max + 1` opens against the `2 × max` allowed, and the slack runs out after `2 × max + 3` moves in one span.
  - **A sequence the text allows,** with `max` = 1, a process budget of two, a role limit of one (one of T-0319's settings, under the role limit I4 names), `W` = 1 s, and stored text toggled between a renamed label and a good one:
    - **At 0.5,** with no epoch in force and the window open, the one connection, opened before 0.5, fails with the stored text's `22P02` holding `_morm_retired_`. The read is unmoved, epoch E1 starts, and the connection is old. Row A2 opens beside it and gets `53300` (open 1), which ends E1's extras. It closes the old connection and opens in its slot (open 2). The text has changed, so the run there answers and confirms. Its *replace* is taken, because the window is open. That connection is behind, so the next checkout replaces it (open 3).
    - **At each of 0.6, 0.65, 0.7, 0.75, 0.8 and 0.85,** `<n>` has been raised just before, by a migration or a comment set by hand. The connection, opened at the previous step, fails with the same `22P02`. Its read finds `<n>` moved, so a new epoch starts (the last-read `<n>` differs from the epoch's) with fresh extras, and the connection is old in it. The beside open gets `53300` (one open) and the slot open succeeds (one). The text has changed again, so the run confirms. Its *replace* passes the closed window, because the last-read `<n>` differs from the window's. The next checkout replaces the connection (one). That's three opens per move.
  - **The result.** The span `[0.5, 1.5)` holds 3 + 6 × 3 = 21 stale-driven opens. The stated bound, with six moves from `W` before the span to its end, is 4 + 4 + 2 × 6 = 20, so I4 as written fails. Each epoch stays within its own `max + 2` (2 opens), and each action within its `max` (1).
  - **What still holds.** The replacement bounds are right. There are 14 replacements against R-0981's `4 × max` plus `2 × max` per move, which is 16, and 7 epoch replacements against `2 × max` plus `max` per move, which is 8. The README states replacements only (R-1249, R-1250, R-1268, R-1294, R-1040), so it's unaffected.
  - **Is it older than this round?** The numbers predate this round. But FH2's fix rewrote this clause and gave it the replacement-only reason, so it's a changed ID that doesn't hold as written.
- **Evidence:**
  - "Churn"'s per-epoch "`max + 2` connection opens" and per-action "`max` connections replaced";
  - its sentence that a role limit at the pool's size "keeps this bound, because the first `53300` ends that epoch's extras";
  - R-1270's epoch row ("its extras ended for the rest of its life by `no_extra`"), so each new epoch tries beside again;
  - row A2 ("a `22P02`'s request comes from the new connection's run, if it confirms");
  - R-0982's gate ("taken now when the window is open, or when the last-read `<n>` differs from the window's");
  - R-0981 ("starts an epoch … if … the pool's last-read `<n>` differs from its own"; "An epoch ends `stale_cooldown` after `T`");
  - T-0318's case "four replacements decided one after another in that epoch make five opens".
- **Fix:** no owner choice is needed. This corrects a stated upper bound, and the procedure doesn't change.
  - **"Churn":** "… `4 × max + 4` stale-driven opens, plus `2 × max + 2` per migration that the pool learns, …, because each such move lets a new epoch of up to `max` replacements and `max + 2` opens start, and one *replace* of up to `max` through a closed window, …".
  - **T-0319's I4:** "plus `2 × max + 2` per move of the pool's last-read `<n>` …".
  - **The simulation's I4 check** follows. The records (`proofs.md`, the PR body's choices) say why.
  - **An alternative** is `2 × max + 1`, with a sentence that an epoch whose first open beside fails makes at most `max + 1` opens. That's tighter, but it needs the argument written in. `2 × max + 2` follows from the per-epoch bound the text already states, so I recommend it.

### FI2. Minor. R-0984 (step 20): "a migration starts a new epoch, so its stale sessions are never taken for that text" contradicts R-0981's new sentence and the cases it and R-1271 state

- **Where:** R-0984 (step 20, line 90): "straight to Postgres step 14's epoch decides which connections a failure may discard, so stored or transformed text holding `_morm_retired_` costs at most the connections open at the epoch's start, and a migration starts a new epoch, so its stale sessions are never taken for that text". `proofs.md`'s FH2 sweep lists it as "read and left as they are … which counts nothing".
- **What's wrong:**
  - **What R-0981 now says.** "A migration the pool learns is a move of its last-read `<n>` … and a migration it hasn't learned yet starts no epoch". An epoch starts only in the step of a session-class failure's read, or inside a savepoint with no read, so a migration never starts one by itself.
  - **Until the pool learns it, a migration's stale session is judged by the epoch the text started.**
    - Inside a savepoint, R-0981 says "a migration first seen inside a savepoint leaves a connection opened between the start of the epoch in force and the migration not old there, until that epoch ends or a read outside a transaction starts a new one". R-1271 says the same, and that the error "can reach each savepoint call that meets it on that connection".
    - Outside a savepoint, a read that overlapped the last applied one "counts as the last-read `<n>`", so the migration stays unlearned, and such a connection's failure takes row A3 and returns as it is.
    - In both cases the migration's stale session is treated as the text's epoch treats it, not replaced. That's what "never taken for that text" denies.
  - **The claim isn't only descriptive.** It doesn't count anything, but it states a guarantee in exactly the terms FH2 corrected elsewhere ("a migration does X", when only a migration the pool has learned does). The sweep's reason for leaving it ("counts nothing") doesn't cover a false "never".
- **Evidence:**
  - R-0981's "Old connection" bullet (the epoch's start, the savepoint case, an overlapping read) and its closing paragraph's new sentence and "Before then …";
  - R-1271's "Stale sessions straight to Postgres" ("Inside a savepoint, … that case holds after a migration too, until that epoch ends or a read of `<n>` outside a transaction applies the migration and the next failure starts a new epoch");
  - R-1270's epoch row ("started by `old`, in the step that applies a session-class failure's read, or inside a savepoint …").
- **Fix:** no owner choice is needed. This words the decided procedure.
  - **R-0984:** "… and a migration the pool learns lets the next session-class failure start a new epoch, so once it's learned its stale sessions are never taken for that text; until then, inside a savepoint or after a read that overlapped another, a connection opened since the epoch's start returns such an error as it is, as step 14's R-0981 and R-1271 say".
  - **The record:** `proofs.md`'s list of places left as they are drops R-0984's line, or says it was corrected.

### FI3. Minor. `proofs.md` (the thirtieth round's model record): the world's share of another `22P02` contradicts T-0319 and choice 279

- **Where:** `proofs.md`, "The last round's model, carried on", **The world:** "a third of the stored-text values its function fails on another way now fail with another `22P02`, a third each from `enum_in`, another bad label, and from another routine, bad text for a number."
- **What's wrong:** the sentence reads as one third failing with another `22P02`, and then as a third from each routine, which would be two thirds. T-0319 says "with a SQLSTATE off the no-confirm list, or with another `22P02`, from `enum_in` … or from another routine …, a third of the time each", so two thirds fail with another `22P02`. The FH1 bullet a few lines above in the same file says the same ("a third of the time with another `22P02` from `enum_in` and a third with one from another routine"), and so do choices 275 and 279.
- **Evidence:** the three sentences quoted, in `proofs.md`, step 20's T-0319 and the PR body.
- **Fix:** "two thirds of the stored-text values its function fails on another way now fail with another `22P02`, a third from `enum_in`, another bad label, and a third from another routine, bad text for a number; the rest fail with a SQLSTATE off the no-confirm list".

## Summary

| Severity | Findings |
|---|---|
| Blocks approval | None |
| Should fix | None |
| Minor | FI1, FI2, FI3 |

No finding needs an owner choice.

- **FH1 and FH2 are fixed as recommended, read literally:**
  - R-1249 and R-1250 now leave out another `22P02` and the no-confirm list, in the review's words;
  - T-0319's world and I13 drive and assert the case;
  - every per-migration pool bound names the migrations the pool learns, or for users sees;
  - R-1268 says once when a migration counts.
- **Three things remain:**
  - **FI1:** the opens bound in "Churn" and I4 adds `2 × max` per move, though a move's epoch can make `max + 2` opens, and a sequence the text allows exceeds it.
  - **FI2:** R-0984's "a migration starts a new epoch, so its stale sessions are never taken for that text" contradicts R-0981's new sentence and its savepoint and overlapping-read cases.
  - **FI3:** one `proofs.md` sentence miscounts the model's share of another `22P02`.
- **The checks agree:**
  - The changed IDs match INDEX's entry exactly: 14 requirements and 1 test obligation, with no change to `contracts.md` or `coverage.md`.
  - The checker reports 1294 requirements, 329 test obligations and 0 problems, in both forms.
  - CI is green on `77c3d50`, 18 of 18 jobs on push and on the pull request.
  - The PR body matches the text, including choices 274 to 280.
- **The probes on 17.11 and 18.6** show every error FH1 names as not confirming carrying `22P02`, from `enum_in` or `pg_strtoint32_safe`, and the timeout's `57014`.

**PR #3 isn't ready for the owner yet.** All three fixes are wording, followed by a check of the edits, the checker and CI. FI1 also changes the simulation's I4 check.
