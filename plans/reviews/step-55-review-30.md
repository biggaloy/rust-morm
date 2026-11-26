# Step 55 independent review, thirtieth round

PR #3 (`step-55-enum-proofs`), head `2fcbbcd`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes.

This round covers `git diff 3686e86 origin/step-55-enum-proofs`, two commits: the twenty-ninth review's fix of FG1 and FG2 (`e61b7c1`) and its `proofs.md` record (`2fcbbcd`), and the PR body's second move to `plans/reviews/step-55-pr-history.md` (`b46a0ea` on `main`). I also checked that nothing earlier regressed on the paths these commits touch.

## What I read

- **On the PR branch:**
  - `rules.md`; `proofs.md`'s new sections "The twenty-ninth review's fix" and "The last round's model, carried on", and the FF sweep record it corrects.
  - **Every changed ID in full:** R-0981, T-0152, T-0311 (step 14), R-0984, T-0312, T-0319 (step 20), R-1249, R-1250 (step 50) and R-1040 (step 59).
  - **For coherence:** step 14's R-0718, R-1245's rows, "Which row decides", "The second row reads no message text" and "A confirmation" with its no-confirm list, R-1270's state table, steps, `before_run`, `on_error`, `session`, `request`, `take`, `finish_take` and `checkout`, and T-0318's confirmation clauses; step 20's R-1271 ("Churn" and the second row's costs) and T-0319's I1 to I8; step 50's R-1253, R-1268, R-1272 and R-1292; step 11's R-0975.
- **On `origin/main`:** `reviews/step-55-review-29.md`, and INDEX's Decisions from "The EZ fix round's five open questions (2026-11-25)" to "The twenty-ninth review's fix (2026-11-27)".
- **The PR body** at the head, all of it, and the version before it from GitHub's edit history.

## What I checked and ran

- **CI on the head.** `2fcbbcd` is green: `ci` on push (run 45904289451) and on the pull request (run 45904296921), 18 of 18 jobs each, both on head SHA `2fcbbcd39e2b…`.
- **The coverage checker,** run on a `git archive` of the head. `check_coverage.py --step 55 --code crates tests benches fuzz .github` reports **1294 requirements, 329 test obligations, 0 problems**. The plain form gives the same.
- **The manifest against INDEX.** From `3686e86` to the head:
  - 5 requirements changed, each in `text` and `block`: R-0981, R-0984, R-1040, R-1249 and R-1250.
  - 4 test obligations changed, in `text`: T-0152, T-0311, T-0312 and T-0319.
  - No ID is new or gone (1,294 and 329 either side), and `master_sha256` and `frame` didn't change.
  - That's exactly the list in INDEX's entry "The twenty-ninth review's fix" and in the PR body's changed-IDs paragraph. Only `proofs.md`, the four step files and `manifest.json` differ. `contracts.md` and `coverage.md` didn't change and didn't need to: no signature and no row's opening changed, as the checker confirms.
- **FG1, read literally against its recommendation:**
  - **R-0981** is the review's text, with "with another SQLSTATE off R-1245's no-confirm list" added after "or the function fails another way" (choice 268). The addition is right and needed: by R-1245 a run that ends in the first error's SQLSTATE, or on the list, doesn't confirm.
  - **R-0984** and **R-1040** say the run "confirms, as step 14 defines a confirmation", answering or failing with another SQLSTATE off the no-confirm list. Both are right.
  - **R-1249** and **R-1250** are right on what answers: a good label, `NULL`, a row that's gone. But each states "fails another way" wider than a confirmation (FH1).
  - **"A client changes" is gone** from R-0981 and R-0984. R-1250 and R-1040 now name the next attempt too.
  - **`proofs.md`, choice 265 and the twenty-eighth section of the body** now name the case so, and each says it first named only a good label.
- **FG2, read literally:** R-1249 now says "every connection at its next checkout, the one that run was on included". That holds against R-1270:
  - In `session()`, a `22P02`'s request is `if !sure and confirms(r, err): request(Replace)`, after `replace(conn, call)` opened the new connection.
  - The new connection saw the connection generation as it was when it opened. So whether `take` raises the generation at once, or a later checkout's `finish_take` does, that connection is behind it.
  - Choice 269's "the one that run was on" in place of "the one that just answered" is right, because by FG1 the run may fail another way.
- **FG2's sweep to three tests**, each against `session()`'s order (`if sure: request(Replace)` before `replace`, a `22P02`'s request after its run):
  - **T-0311:** the first case is a `22P02`, whose run requests the action after the new connection opened, so "the other three and the new one" is right.
  - **T-0312's B3:** the next attempt runs on a connection opened in the discarded one's slot before its own confirming run requests the action, so "the one that attempt ran on included" is right.
  - **T-0152's function cases:** "every connection opened before it's taken … and the new one too when it opened first, as it always does for the `22P02`" is exact for both orders. A `42804` requests at the failure, so its new connection is behind only when the action was due rather than taken.
- **The places listed as read and left as they are.** I spot-checked each:
  - **T-0152's first sentence** (`XX000` from a trigger): `sure`, so it requests before `replace`. On the test's pool no action has been taken, so the window is open, `take` raises the generation at once, and the new connection opens current. "The pool's other connections" holds.
  - **T-0152's held-connection sentence**, R-0975's "the other connections don't each have to fail first", and R-1040's "the rest of the pool" beside "each stale connection is replaced once" each hold. A held connection is marked for replacement on release anyway (A4), and a new connection isn't a stale one.
  - **R-1271's "Churn" and T-0319's I4** count the action whatever requests it, and T-0311's new-connection cases (`57014` requests nothing; `23505` and a command tag request *replace*) already test both sides of a confirmation on Postgres.
  - **The model's span-check correction,** and the claim that "the text's 'plus `max` per migration' needs no change: it counts the migrations the pool sees, whenever they were committed". The bound is right when read that way, but no text says it, and the other reading fails (FH2).
- **Requirement by requirement, for the changed IDs.** Apart from FH1 and FH2 I found no contradiction among R-0718, R-0981, R-0984, R-1245's rows and its confirmation, R-1270's procedure (unchanged this round), R-1271's guarantees and costs, T-0319's I-clauses and the README notices R-1249, R-1250, R-1253, R-1268, R-1272 and R-1292.
  - T-0319's world now drives `NULL`, no row and a function failing another way, beside a good label.
  - T-0318 asserts the same-SQLSTATE rule in the unit test ("the other way round after a `22P02`").
  - So each behaviour the new wording states is tested.
- **Loud notices and "use this when" lines.** R-1249 and R-1250 changed only inside their bound sentences. Each still says what it does, when it's right, what's given up and how to proceed. No opt-in call or setting is new in this range, and R-1263 to R-1266 still give each stale-session setting its line.
- **The PR body:**
  - **It matches the text, the manifest and INDEX's entry:** the section "The twenty-ninth review, since `3686e86`", the changed-IDs paragraph, choices 268 to 273, the corrected choice 265 and twenty-eighth section, "Open questions" ("None") and "End-to-end result", which names `2fcbbcd` and runs 45904289451 and 45904296921.
  - **The model's numbers add up.** The body's 2,049 / 1,057 / 1,067 / 1,044 are `proofs.md`'s A2 plus B3 counts at "stored text only", and its 12, and 249, 256 and 223 of 256, match `proofs.md`.
  - **Choice 268's record of R-1249** carries FH1's omission. It names the no-confirm errors, but not the same-SQLSTATE rule that the same choice gives for R-0981.
- **The PR body's second move,** checked against GitHub's edit history (`gh api graphql`, `userContentEdits`, 52 edits):
  - **The edits:** the edit at 2026-11-26T21:25:59Z is the body at `3686e86`, 224,086 bytes. The latest, at 22:24:58Z, is byte for byte the live body, 218,022 bytes.
  - **Nothing was lost.** The four sections "The owner's answer to the two answers' question, from `64820fe` to `e417bc8`" through "The owner's five answers to the twenty-second fix round's questions, from `19be048` to `a1a6d0b`" are lines 141 to 223 of the older body. They're identical, by `diff`, to lines 529 to the end of `plans/reviews/step-55-pr-history.md` on `main`.
  - **The rest of the diff:** apart from those 83 lines, it holds only the new pointers and paragraph, the twenty-ninth section, choices 268 to 273, the changed-IDs paragraph, and the edits to choice 265, the twenty-eighth section, "Open questions" and "End-to-end result".
  - **The pointers lead to their sections.** Each of the four links under "The earlier rounds" has an anchor equal to GitHub's slug of its heading in the history file. Each slug occurs once there, so none gets a `-1` suffix.
  - **"Above" and "below".** In the moved sections they point only at one another, or at the body's "Open questions" ("answered, below"), which is still in the body. The body's "written in above" for a section that has left is covered by "The earlier rounds", which sends the reader to the history file.

### Postgres 17.11 (`/opt/pgsql-17`) and 18.6 (`/usr/bin`)

I ran throwaway clusters under the worktree's `.claude/scratch/`, with Unix sockets under `~/.claude/scratch/`, `TMPDIR` set to the scratch, `lc_messages = C` and psql with `VERBOSITY verbose`. Both versions gave the same results, apart from one source line.

The setup is an enum `st ('a', 'b')` and a table `src (id int, txt text, num text)` holding `(1, '_morm_retired_7c1e9a04b52fd386_a', '5')`. The function `f(p int)` does `DECLARE v st; n int; BEGIN SELECT txt INTO v FROM src WHERE id = p; SELECT num::int INTO n FROM src WHERE id = p; RETURN …`.

| The row holds | `f(1)` |
|---|---|
| `txt` = `_morm_retired_…_a` | `22P02` `invalid input value for enum st: "_morm_retired_7c1e9a04b52fd386_a"`, `enum_in`, `enum.c:129` |
| `txt` = `zzz`, another bad label | `22P02` `invalid input value for enum st: "zzz"`, `enum_in`, `enum.c:129` |
| `txt` = `a`, `num` = `x` | `22P02` `invalid input syntax for type integer: "x"`, `pg_strtoint32_safe` (`numutils.c:616` on 17.11, `:615` on 18.6) |
| `txt` = `a`, `num` = `5` | answers `a 5` |

So after a change the run can stop failing with the `22P02` it gave and fail another way, with the same SQLSTATE. By R-1245 ("The same error means the same SQLSTATE") that run fails the same way and doesn't confirm, so nothing is requested.

**Not run:**
- PgBouncer: there's no build on this machine, and nothing in this range is about it.
- Postgres 15 and 16.
- A model of my own. FH1 rests on R-1245's definition, R-1270's `session()` and the probe above. FH2 rests on R-1270's `old()` and epoch rules, and on the scenario below, which follows from the text alone.

## Findings

### FH1. Minor. R-1249 and R-1250 (step 50): the run that "fails another way" is said to be taken for a cured stale session even when it fails with another `22P02`, and in R-1250 even when it ends on the no-confirm list

- **Where:**
  - **R-1249** (line 395): "so that run no longer fails with the `22P02` it gave, as when the row now holds a good label or `NULL`, is gone, or your function fails another way, with an error other than one that can stop it before it reaches the value, such as a lost connection, a timeout, a cancel, a deadlock or a lack of resources, is taken for a cured stale session, so it also replaces every connection at its next checkout".
  - **R-1250** (line 435): "so that run no longer fails with the `22P02` it gave, and answers or fails another way, which morm takes for a cured stale session".
  - **Their records:** choice 268's account of R-1249, and `proofs.md`'s FG1 bullet, which gives R-1250's wording without remark.
- **What's wrong:**
  - **What confirms.** R-1245's "A confirmation" counts a run that ends in a server error only when its SQLSTATE "isn't the first one's" and it's off the no-confirm list, and adds: "The same error means the same SQLSTATE … a fresh run that ends in the same error fails". So in R-1270's `session()`, `confirms(r, err)` is false for any `22P02`, and A2 requests nothing. The same goes for `expect_confirm` at B3's next attempt.
  - **R-1249 leaves out the SQLSTATE rule.** A function that now fails with another `22P02`, such as a different bad label in the row or a later `num::int` of bad text (the probe above), "no longer fails with the `22P02` it gave". It "fails another way", with an error not on the no-confirm list, so R-1249 says morm takes it for a cured stale session and replaces every connection. It doesn't: the call gets that `22P02` as it is, and nothing is requested.
  - **R-1250 leaves out both rules.** "Answers or fails another way" also takes in a timeout, a cancel, a lost connection or a deadlock, which R-1245's list says neither confirm nor fail. T-0311 asserts the first of these on Postgres: the new connection's run "cancelled by `statement_timeout` before the conversion gives the caller `57014`, requests nothing".
  - **Why it matters.** R-0981, R-0984 and R-1040 state the case exactly ("with another SQLSTATE off R-1245's no-confirm list"), and choice 268 gives the reason: "a run that ends on that list, or in another `22P02`, the same error by SQLSTATE, doesn't confirm". The two README notices, which users read, state what morm does wider than the requirements do. A reader whose stored text now holds another bad label is told that morm replaces the pool, and it doesn't. The bound beside it is an upper bound, so the numbers aren't wrong, only the statement of when morm acts.
- **Evidence:**
  - R-1245's "A confirmation" (step 14, line 107);
  - R-1270's `session()` (`if !sure and confirms(r, err): request(Replace)`) and `before_run` (`if r ends in the same error: end_confirm(e)`);
  - T-0311's `57014` case and T-0318's "the other way round after a `22P02`";
  - choice 268's own reason;
  - the probe on 17.11 and 18.6: `22P02` from `enum_in` for another bad label, and from `pg_strtoint32_safe` for a bad number, each after the same call failed with the stored text's `22P02`.
- **Fix:** no owner choice is needed. This words the decided procedure.
  - **R-1249:** "… or your function fails another way, with an error other than another `22P02` and other than one that can stop it before it reaches the value, such as a lost connection, a timeout, a cancel, a deadlock or a lack of resources, is taken for a cured stale session …". Then one sentence after it: "One that fails with another `22P02`, such as another bad label or bad text for a number, isn't: you get that error as it is, and nothing more is replaced."
  - **R-1250:** "… so that run no longer fails with the `22P02` it gave, and answers, or fails with an error other than a `22P02` and other than one that can stop it before it reaches the value, which morm takes for a cured stale session, as the enum notice says".
  - **The records:** choice 268 and `proofs.md`'s FG1 bullet say what the text now reads.
  - **Optional, so the simulation drives the case that doesn't confirm:** T-0319's world's "a value its function fails on another way" also fails sometimes with another `22P02`. T-0318 already asserts the rule in the unit test.

### FH2. Minor. T-0319's I4 and R-1271's "Churn" (step 20), and R-0981's span bound (step 14): "plus `max` per migration" doesn't say which migrations a span counts, and counted by commit time the bound fails

- **Where:**
  - **T-0319's I4:** "in any span of `stale_cooldown` at most `4 × max + 4` stale-driven opens, plus `2 × max` per migration".
  - **R-1271's "Churn":** "in any span of `W`, … `4 × max + 4` stale-driven opens, plus `2 × max` per migration".
  - **R-0981:** "at most `2 × max` replacements in any span of `stale_cooldown`, plus `max` per migration".
  - **The fix's claim:** `proofs.md` and choice 272 say the span checks now count "the larger of the changes committed from `W` before it and the pool's moves of `<n>` from `W` before it", and that "the text's 'plus `max` per migration' needs no change: it counts the migrations the pool sees, whenever they were committed". INDEX records "which the text already says".
- **What's wrong:**
  - **No text says which migrations count.** I1 and I3 each say exactly which change of `<n>` excuses a pair, by the snapshot of the read the pool's `<n>` came from. I4, "Churn" and R-0981 don't say it for a span: neither by commit time, nor by when the pool learns of the change, nor over which stretch around the span. R-1271's sentences on lowered `<n>`, several servers and overlapping reads say what counts as a migration, not when.
  - **The natural reading fails.** Count the migrations committed in or just before the span, as the model's own check did until this round, and the bound fails. The fix's model met this once with no fault: three migrations committed 1.4 s before a span of `W`, learned inside it, and 9 of an epoch's replacements in the span, against `2 × max` = 8.
  - **The text alone allows more.** An epoch can start inside a savepoint with no read, judged by the pool's last-read `<n>` (R-1270's `session()`, `if !ctx.sp: read(conn)`, and the epoch row of the state table). So a pool that hasn't read `<n>` since a migration committed long before can have this, with `W` = 1 s, `max` = 4 and the migration committed at `t = −2 s`:
    - epoch E0 starts inside a savepoint at `t = −0.9`, under the old `<n>`. Its four old connections fail, with stored text holding `_morm_retired_`, late in its life, from `0.05` to `0.1`, so each is discarded and replaced then;
    - E1 starts inside a savepoint at `0.1`, when E0 has ended, still under the old `<n>`. It replaces up to 4 more, E0's replacements among them, since they opened before its start;
    - E2 starts at `0.4`, from a read outside a savepoint that learns the new `<n>`, which lets an epoch start inside E1. It replaces up to 4 more.

    That's up to 12 epoch replacements in the span from `0.05` to `1.05`, with no migration committed within `W` of it. A check of I4 or R-0981 written by commit time reports a failure, since it allows 8. That reading is as good as any the text gives. Counted by the move at `0.4`, the bound is `2 × max + max` = 12, and it holds.
  - **Read the other way, the bound holds.** Count, for a span, the moves of the pool's last-read `<n>` from `stale_cooldown` before its start to its end:
    - each move lets one more epoch start, of up to `max` replacements over the next `stale_cooldown`, and one *replace* through a closed window;
    - a move earlier than that can't add an epoch's replacements to the span, and an action it let through adds nothing past the one replacement per slot that "Churn" already counts for an action before the span.

    That's what the fix's corrected checks count. But T-0319's I4 is what a test implementer writes from, and it has to say it.
- **Evidence:**
  - T-0319's I1, I3 and I4 (step 20, line 128), R-1271's "Churn" (line 108) and R-0981's closing paragraph (step 14, line 421);
  - R-1270's state table ("The epoch … started by `old`, in the step that applies a session-class failure's read, or inside a savepoint, where there's no read, in that failure's own step") and `session()`;
  - `proofs.md`'s "One correction to the last round's span checks", whose seed 2,889 at switch 16's setting fails the commit-time reading with no fault.
- **Fix:** no owner choice is needed. This states the decided bound's terms; the numbers don't change.
  - **T-0319's I4:** "… in any span of `stale_cooldown` at most `4 × max + 4` stale-driven opens, plus `2 × max` per move of the pool's last-read `<n>` from `stale_cooldown` before the span's start to its end, whenever the change was committed, …". If the fix agent keeps the model's larger-of count, the text should say that instead.
  - **R-1271's "Churn":** the same words after "plus `2 × max` per migration": "that the pool learns, by a move of its last-read `<n>`, from `W` before the span to its end, whenever it was committed".
  - **R-0981:** "plus `max` per migration the pool learns from `stale_cooldown` before the span to its end".
  - **The README notices** (R-1249, R-1250, R-1268, R-1294 and R-1040) can keep "per migration" for users. Optionally add once, in the notice on the stale-session settings, "counted when the pool first sees it, which can be after it ran".
  - **The records:** `proofs.md`'s correction and choice 272 then say the text now names the count, not that it already did.

## Summary

| Severity | Findings |
|---|---|
| Blocks approval | None |
| Should fix | None |
| Minor | FH1, FH2 |

No finding needs an owner choice.

- **FG1 and FG2 are written in as recommended, read literally.** R-0981, R-0984 and R-1040 state the confirming case exactly, and the *replace* action now replaces every connection, the one whose run requested it included, in R-1249 and in the three tests the sweep found. R-1270's procedure is unchanged.
- **Two wording gaps remain:**
  - **FH1:** R-1249 and R-1250 say a run that fails another way is taken for a cured stale session without the rule that another `22P02` doesn't confirm, and R-1250 without the no-confirm list either.
  - **FH2:** the span bounds' "per migration" doesn't say which migrations count. Read by commit time it fails, as the fix's own model found and as an epoch started inside a savepoint shows from the text alone.
- **The checks agree:**
  - The changed IDs match INDEX's entry exactly: 5 requirements and 4 test obligations, with no change to `contracts.md` or `coverage.md`.
  - The checker reports 1294 requirements, 329 test obligations and 0 problems, in both forms.
  - CI is green on `2fcbbcd`, 18 of 18 jobs on push and on the pull request.
  - The PR body matches the text, apart from choice 268's record of R-1249.
  - The PR body's second move lost nothing: the four sections are byte for byte the body at `3686e86`, and each pointer leads to its section.
- **The probes on 17.11 and 18.6** show a stored-text function that failed with the renamed label's `22P02` failing next with another `22P02`, a bad label's or a bad number's, which doesn't confirm.

**PR #3 isn't ready for the owner yet.** Both fixes are wording only, apart from FH1's optional addition to T-0319's world, followed by a check of the edits, the checker and CI.
