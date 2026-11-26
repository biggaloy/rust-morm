# Step 55 independent review, twenty-ninth round

PR #3 (`step-55-enum-proofs`), head `3686e86`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes.

This round covers `git diff c458987 origin/step-55-enum-proofs`, three commits: the twenty-eighth review's fix of FF1 and FF2 (`7d825f4`), its sweep of replacement bounds and one-mode costs (`79c0a02`), and its `proofs.md` record (`3686e86`). I also checked that nothing earlier regressed on the paths these commits touch.

## What I read

- **On the PR branch:**
  - `rules.md`, `contracts.md`'s `Error::SchemaChanged` comment, and `proofs.md`'s new section "The twenty-eighth review's fix", "The last round's model, carried on", and the FE1 sweep record it corrects.
  - **Every changed requirement in full:** R-0981 (step 14), R-0984 (step 20), R-1249, R-1250, R-1268, R-1288, R-1292 and R-1294 (step 50), and R-1040 (step 59).
  - **For coherence:**
    - Step 14: R-0718; R-0982 and its gate; R-1245's rows, "Which row decides" and "A confirmation"; R-1246's bullets; R-1262's table; R-1270's state table, steps and procedure (`before_run`, `on_error`, `session`, `request`, `take`, `finish_take`, `checkout`, `read`); T-0311.
    - Step 20: R-1271's bullets, "Churn" in full; T-0319's I3, I4 and its load.
    - Step 50: the README notices R-1252, R-1253, R-1255, R-1258, R-1272 and R-1293, and the "use this when" rows R-1263 to R-1266.
    - Step 12: R-0673.
- **On `origin/main`:** `reviews/step-55-review-28.md`, and INDEX's Decisions from "The EZ fix round's five open questions (2026-11-25)" to "The twenty-eighth review's fix (2026-11-27)".
- **The PR body,** read at the head: "The twenty-eighth review, since `c458987`", the changed-IDs paragraph since `c458987`, choices 258, 260 and 263 to 267, "Open questions" and "End-to-end result".

## What I checked and ran

- **CI on the head.** `3686e86` is green: `ci` on push (run 45897743374) and on the pull request (run 45897750135), 18 of 18 jobs each, both on head SHA `3686e86eb0f1…`.
- **The coverage checker,** run on a `git archive` of the head. `check_coverage.py --step 55 --code crates tests benches fuzz .github` reports **1294 requirements, 329 test obligations, 0 problems**. The plain form gives the same.
- **The manifest against INDEX.** From `c458987` to the head:
  - 9 requirements changed, each in `text` and `block`: R-0981, R-0984, R-1040, R-1249, R-1250, R-1268, R-1288, R-1292 and R-1294. No test obligation changed.
  - No ID is new or gone (1,294 requirements and 329 test obligations either side), no `line` moved, and `master_sha256` and `frame` didn't change.
  - That's exactly the list in INDEX's entry "The twenty-eighth review's fix". Only `proofs.md`, the four step files and `manifest.json` differ between the two trees. `contracts.md` and `coverage.md` didn't change and didn't need to, because no signature and no row's opening changed, as the checker confirms.
- **FF1 and FF2, each read literally against its recommendation:**
  - **FF1:** R-1268's "Shorter:" is the review's text word for word, with ", straight to Postgres," added before "up to the pool's `max` more replacements" (choice 263). The addition is right, because behind PgBouncer nothing is replaced. The first sentence's "or, straight to Postgres, replaces every connection" is right too. R-1268 now counts both kinds of replacement, up to four times `max` in any span plus a round of each per migration. That's what R-1271's "Churn" gives: two epochs' and two *replace* actions' worth, `4 × max` replacements of the `4 × max + 4` opens, plus `2 × max` per migration. The text beside it doesn't repeat FF1's error: R-1268's other bullets state no replacement bound.
  - **FF2:** R-1294's sentence on something still on the kept type is the review's text word for word (choice 264). It holds against R-1270's `session()`:
    - Behind PgBouncer a call outside a transaction not in the cool-down runs once more on a connection checked out again. Inside a transaction it puts the statement in the cool-down and returns `Error::SchemaChanged`. Calls in the cool-down return the error as it is.
    - Straight to Postgres the query's own label fails on the new connection too, so no *replace* is requested, and the epoch's bound is the right one.
  - **`proofs.md` and choices 258 and 260** now say which places R-0981's epoch-only wording fits, and which R-1294 sentence "for up to `stale_cooldown`" belongs to.
- **The sweep of replacement bounds** is right that a *replace* is requested by text holding `_morm_retired_` whose call's next run doesn't fail the same way (R-1270's `session()`: `if !sure and confirms(r, err): request(Replace)`, and B3's `expect_confirm(call, Replace)`). R-0981, R-0984, R-1249, R-1250 and R-1040 now scope the epoch's bound to text that "stays as it is". But two things are wrong in how they word the case that adds a *replace*:
  - They name only one cause, text changed to a good label (FG1).
  - R-1249 says the action replaces "every other connection" (FG2).
- **The one-mode sweep** is right in each place it changed:
  - R-1249's four recovery bullets now start "Straight to Postgres", and its PgBouncer bullet follows them.
  - R-1292's paragraph, "What you give up" and "How to proceed" now give the PgBouncer cost. That cost follows from R-1270's `session()`: in the cool-down the error comes back as it is, which inside a savepoint is an `Error::Postgres` the savepoint clears. Outside the cool-down, D2 rolls the whole transaction back.
  - R-1288 now scopes "replace connections" and gives the PgBouncer cost.
- **The fix agent's "read and left as they are" lists.** I spot-checked each:
  - **Replacement bounds:**
    - R-1271's "Churn" and T-0319's I4 give the full bound.
    - T-0311's eight-client case reads a stored `_morm_retired_x` that doesn't change during the test, so the epoch's bound is exact there. Its record-shape case states per epoch and per *replace* action.
    - R-1294's bound, as above.
    - R-1040's "the rest of the pool at most once per `stale_cooldown`, plus once per migration", the gate's bounds, and the size bounds.
  - **One-mode costs:**
    - Each bullet of R-1253 and R-1272 names its mode or holds in both, and R-1258 is about PgBouncer throughout.
    - R-1263 to R-1266 name their mode or hold in both, and R-1262's rows are each scoped.
    - R-1293 is right to be left: `db.hold()` returns `PoolerMode` behind PgBouncer (R-0673).
    - R-1252 and R-1255 hold in both modes.

  Each holds.
- **Requirement by requirement, for the changed IDs.** Apart from FG1 and FG2 I found no contradiction among:
  - R-0718, R-0981, R-0984, R-1245's rows and its confirmation, and R-1270's procedure, which didn't change this round;
  - R-1271's guarantees and costs ("Churn", the PgBouncer bullets, the kept-type bullets) and T-0319's I3, I4 and I8;
  - the README notices R-1249, R-1250, R-1253, R-1258, R-1268, R-1272, R-1288, R-1292 and R-1294, and R-1040's decision text.

  T-0311 proves the confirming run's *replace* on Postgres (a `23505` and an `UPDATE`'s command tag on the new connection each request *replace*). T-0319's load toggles stored text "between `.retry(n)`'s attempts too", and I4 bounds the result, so the new bounds are tested.
- **Loud notices and "use this when" lines.** Every changed notice still says what it does, when it's right, what's given up and how to proceed. No opt-in call or setting is new in this range, and R-1263 to R-1266 still give each stale-session setting its line.
- **The PR body.** These match the text, the manifest and INDEX's entry:
  - the section "The twenty-eighth review, since `c458987`";
  - the changed-IDs paragraph since `c458987`;
  - choices 263 to 267, and 258 and 260 as corrected;
  - "Open questions" ("None");
  - "End-to-end result", which names `3686e86` and runs 45897743374 and 45897750135, the head's.

  Choice 265's record of the new case says "stored text changed to a good label", which is FG1's narrowing.

### Postgres 17.11 (`/opt/pgsql-17`) and 18.6 (`/usr/bin`)

I ran throwaway clusters under the worktree's `.claude/scratch/`, with Unix sockets under `~/.claude/scratch/`, `TMPDIR` set to the scratch, and psql with `VERBOSITY verbose`. Both versions gave the same results, apart from source lines.

- **Stored text holding `_morm_retired_`, then changed in other ways than to a good label.** The setup is an enum `st ('a', 'b')` and a table `src (id int, txt text)` holding `(1, '_morm_retired_7c1e9a04b52fd386_a')`. It has two functions:
  - `f(p int)`, `DECLARE v st; BEGIN SELECT txt INTO v FROM src WHERE id = p; RETURN coalesce(v::text, '<null>'); END`;
  - `g(p int)`, the same with `INTO STRICT`.

  Results:

  | Row `1` holds | `f(1)` | `g(1)` |
  |---|---|---|
  | `_morm_retired_7c1e9a04b52fd386_a` | `22P02` `invalid input value for enum st: "_morm_retired_7c1e9a04b52fd386_a"` from `enum_in` (`enum.c:129`): R-1245's seventh row, since no parameter holds it | same |
  | nothing (row deleted) | answers `<null>` | `P0002` `query returned no rows` (`exec_stmt_execsql`, `pl_exec.c:4378` on 17.11, `:4384` on 18.6) |
  | `NULL` | answers `<null>` | answers, `NULL` |
  | `a` | answers `a` | (not run) |

  By R-1245's "A confirmation", a run that succeeds confirms, and so does a run that ends in another SQLSTATE off the no-confirm list, as `P0002` is. So a deleted row, a `NULL` and a good label each make the same call's next run confirm, and R-1270's `session()` then requests *replace* (A2), as `expect_confirm` does for B3's next attempt. T-0311 already asserts the "other SQLSTATE" half on Postgres: a new connection's run that "raises `23505` … requests *replace*".

**Not run:**
- PgBouncer: there's no build on this machine.
- Postgres 15 and 16.
- A model of my own. FG1 and FG2 rest on the procedure's text, R-1270's `session()`, `expect_confirm`, `take` and the per-connection generations, and on the probe above. The bound itself is unchanged, and the fix agent's model already counts it.

## Findings

### FG1. Minor. R-0981 (step 14), R-0984 (step 20), R-1249 and R-1250 (step 50), R-1040 (step 59), `proofs.md` and choice 265: the case that adds a *replace* action is named as text "changed to a good label", but any change that makes the next run stop failing with that `22P02` does it

- **Where:**
  - **R-0981:** "stored text a client changes to a good label between a failure and that run, or the next attempt's run inside a transaction, which morm takes for a cured stale session, also bring a *replace* action", and before it "a `22P02` does when that run answers".
  - **R-0984:** "stored text a client changes to a good label before the next attempt's run requests it when that run answers".
  - **R-1249:** "A value read from a table that changes to a good label between a call's failure and its run on the new connection, or its transaction's next attempt, makes that run answer".
  - **R-1250 and R-1040:** "when stored text changes to a good label between a call's failure and its run on the new connection".
  - **`proofs.md` and choice 265:** they record the same, "stored text changed to a good label".
- **What's wrong:**
  - **The procedure requests *replace* on any confirmation.** In R-1270's `session()`, row A2 runs `if !sure and confirms(r, err): request(Replace)`, and B3's `expect_confirm(call, Replace)` makes the next attempt's confirming run request it.
  - **R-1245 says what confirms.** A confirmation is a run "that succeeds, with or without rows, … or that ends in a server error whose SQLSTATE isn't the first one's and that's off the no-confirm list".
  - **So more than a good label confirms.** A row that's deleted, or set to `NULL`, between the failure and that run, or anything else that makes the function stop reaching that conversion, makes the run answer or fail another way (the probe above). That includes a `STRICT` read's `P0002`, or any error the function raises further on, as T-0311's `23505` case shows. Each of these also brings a *replace* action.
  - **"Answers" is narrower than "confirms",** in R-0981's "when that run answers", R-0984's "when that run answers" and R-1249's "makes that run answer".
  - **"A client changes" is narrower than the procedure.** The change can be made by any writer, another service, a trigger or a job, as R-1249 and R-1250 already say ("A value read from a table that changes"). R-0981 and R-0984 say "a client changes".
  - **R-1250 and R-1040 name only "its run on the new connection".** R-1249 and R-0981 name the next attempt inside a transaction too.
  - **Why it matters.** The epoch's smaller bound is scoped to text that "stays as it is", so a reader whose function's rows are deleted or nulled is told that bound doesn't hold. But no sentence tells them that the larger one does. In a loud notice the case that costs more is stated narrower than it is.
- **Evidence:**
  - R-1270's `session()` and `expect_confirm`;
  - R-1245's "A confirmation" and its no-confirm list;
  - T-0311's new-connection cases, where `23505` requests *replace*, and so does an `UPDATE`'s command tag;
  - the probe above on 17.11 and 18.6: a deleted row gives `<null>` from `f` and `P0002` from `g`, and a `NULL` gives `<null>`, each where the same call failed with the `22P02`.
- **Fix:** no owner choice is needed. This words the decided procedure.
  - **R-0981:** "… and a `22P02` does when that run confirms, so values a client chooses that give a `42804` with no schema change, such as a record-shape function fed rows of tables of different shapes, and stored text that changes between a failure and that run, or the next attempt's run inside a transaction, so that run no longer fails with that `22P02`, as when the row now holds a good label or `NULL`, is gone, or the function fails another way, which morm takes for a cured stale session, also bring a *replace* action …".
  - **R-0984:** "and stored text that changes before the next attempt's run, so that run no longer fails with that `22P02`, requests it when that run confirms".
  - **R-1249:** "A value read from a table that changes between a call's failure and its run on the new connection, or its transaction's next attempt, so that run no longer fails with that `22P02`, as when the row now holds a good label or `NULL`, is gone, or your function fails another way, is taken for a cured stale session, so …".
  - **R-1250 and R-1040:** "when stored text changes between a call's failure and its run on the new connection, or its transaction's next attempt, so that run no longer fails with that `22P02`, which morm takes for a cured stale session".
  - **`proofs.md` and choice 265:** say what the text now reads.
  - **The sweep:** check every other place that names this case. I found none beyond these.

### FG2. Minor. R-1249 (step 50): the *replace* action "replaces every other connection", but it replaces every connection, the one that just answered included

- **Where:** R-1249's bullet on text holding `_morm_retired_`: "which morm takes for a cured stale session, so it also replaces every other connection at its next checkout, at most once per `stale_cooldown`".
- **What's wrong:**
  - **What the action does.** *Replace* raises step 11's connection generation, "so every connection is replaced at its next checkout" (R-0982; R-1270's `take` and `checkout`).
  - **Why the new connection is behind.** Each connection holds "the two generations it last saw". The connection A2's `replace(conn, call)` opens, or B3's next attempt opens, sees the generation as it was when it opened. Then `request(Replace)` follows that connection's own run, so its take, at once or at a later checkout, raises the generation past it. It's replaced at its next checkout too, and so is every connection the epoch replaced before.
  - **"Every other" undercounts by one.** The span bound after it, four times `max`, is right, because it counts the action at up to `max`. But the sentence before it misstates what the pool does. R-0981, R-0984, R-1250 and R-1040 don't say "other".
- **Evidence:** R-0982's *replace*; R-1270's state table ("The generation word", "Per connection: … the two generations it last saw"), `session()` (`r = replace(conn, call)` then `if !sure and confirms(r, err): request(Replace)`), `take` and `checkout`.
- **Fix:** no owner choice is needed. R-1249: "so it also replaces every connection at its next checkout, the one that just answered included, at most once per `stale_cooldown`". This can go into FG1's rewording of the same sentence.

## Summary

| Severity | Findings |
|---|---|
| Blocks approval | None |
| Should fix | None |
| Minor | FG1, FG2 |

No finding needs an owner choice.

- **FF1 and FF2 are written in as recommended, read literally.** R-1268 counts the epoch's and the *replace* action's replacements together, as R-1271's "Churn" does. R-1294 scopes its replacements to straight to Postgres and gives the PgBouncer cost. R-1270's procedure is unchanged.
- **The sweeps are right where they changed the bounds and the modes,** and each place they read and left as it was holds. But they have two wording errors:
  - **FG1:** the case that adds a *replace* action is named as text changed to a good label, where any change that makes the next run confirm does it: a deleted row, a `NULL`, or another error further on.
  - **FG2:** R-1249 says the action replaces "every other connection". It replaces every connection.
- **The checks agree:**
  - The changed IDs match INDEX's entry exactly: 9 requirements, no test obligation, and no change to `contracts.md` or `coverage.md`.
  - The checker reports 1294 requirements, 329 test obligations and 0 problems, in both forms.
  - CI is green on `3686e86`, 18 of 18 jobs on push and on the pull request.
  - The PR body matches the text, apart from choice 265's record of FG1's case.
- **The probes on 17.11 and 18.6** show a stored `_morm_retired_` value's `22P02` giving way to an answer or a `P0002` when its row is deleted or nulled, with no good label involved.

**PR #3 isn't ready for the owner yet.** Both fixes are wording only, followed by a check of the edits, the checker and CI.
