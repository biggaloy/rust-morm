# Brief: step 55, the owner's three answers

Started 2026-11-25 (`isolation: worktree`, background, Opus, high effort). If its commits aren't on `origin/step-55-enum-proofs` after a restart: in the same session, resume it with `SendMessage`; otherwise check `origin/wip/step-55-answers-3` and the old worktree under `.claude/worktrees/` (commit and push any uncommitted work to that branch), then start a fresh agent with the brief below, from the wip branch if there is one. Use the attribution line the session then gives.

---

PR #3 (repo `biggaloy/rust-morm`, branch `step-55-enum-proofs`, head `a1a6d0b`) has three open questions in its PR body ("Open questions"). The owner answered option (a) on each: INDEX Decisions "The five answers' three open questions (2026-11-25)" on origin/main. Write them into `step-55-enum-proofs` (`git checkout -B fix-answers-3 origin/step-55-enum-proofs`, `git push origin HEAD:step-55-enum-proofs`, no force).

Read first: `plans/plan-approved/rules.md`, `contracts.md`, `proofs.md` on the branch; on origin/main that INDEX entry and the one before it.

1. **Changes the text.** R-1245's second row reads only what stands before the message's value. It replaces the bound-texts test everywhere (R-1245 itself, R-0981, R-1270, R-1271, R-1253, T-0153, T-0318, T-0319, the PR body and whatever else names it): remove the old rule rather than patching it, and drop choices and text that only served it. State what's left (another language, a type named as a word of its message) in R-1271 and R-1253. The parameter test for `_morm_retired_` is a different test and stays; check nothing confuses the two. Check the rule on every message Postgres 15 to 18 ships, in every language: which texts mark the value, quoted type names, schema-qualified names, names holding a colon or a quote mark, and messages that aren't an enum's; a message the rule can't read must fall on the safe side, said plainly. Probe Postgres 17 and 18.
2. **Keeps the text:** confirm R-1271 and R-1294 state the case behind PgBouncer as the decision does, in how long it lasts and what cures it.
3. **Keeps the text:** confirm R-1271, I9, R-1249 and R-1253 state it as the decision does, loudly in the last two.

**The model:** start from `plans/models/step-55/` on origin/main; don't edit that folder. Put the new test in, run every setting and all 39 switches again, break the changed clauses on purpose, and show the decided cases. No fault may fail any setting. Record the round in `proofs.md`. Keep the model, scripts, results and notes on `wip/step-55-answers-3` under `plans/wip/`, pushed often, never on `step-55-enum-proofs`; leave that branch on origin when done and report its commit.

Anything needing a design choice the owner hasn't made is an open question with options and a recommendation, never decided by you. New IDs from R-1295 and T-0330 if needed; no INDEX Decisions edits; keep `manifest.json` and `coverage.md` in step. Run the checker (`check_coverage.py --step 55 --code crates tests benches fuzz .github`, and plain) to 0 problems, push, CI green, PR body re-read before `gh pr edit` and read back after (mark the questions answered, option (a); word INDEX lines so they don't depend on what INDEX has recorded yet; number your own choices from 204).

Report: commit hash, CI, checker totals, one bullet per answer, changed and new IDs by step, what the model showed, your own choices, open questions, leftovers.

Plus the common rules in `plans/coordinator-briefs.md` on origin/main (read them): commit and push as `Biggo Dev <biggodev@gmail.com>`, never neurobin; never use `/tmp`, working files go in `.claude/scratch/` in your worktree with `TMPDIR` set there (Unix sockets with paths too long for that go under `~/.claude/scratch/`); if you use cargo to fetch anything new, set `CARGO_HOME` inside your scratch; clean up servers, scratch and local branches when done and report what's left; don't touch files in the session scratchpad you didn't create. End every commit message with only:

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

and keep the PR body ending with `🤖 Generated with [Claude Code](https://claude.com/claude-code)`, with no session link.

---

## After it reports

1. Record its changed and new IDs and its own choices in INDEX Decisions, appended to "The five answers' three open questions (2026-11-25)". Copy its model to `plans/models/step-55/` on `main`, then delete the wip branch. Update the state file.
2. Bring any open question to the owner, with options and a recommendation.
3. Then the twenty-third review ("Review round" brief, Opus, high effort): PR #3, scope since `93854c2`, findings `FA1, …` to `plans/reviews/step-55-review-23.md`, with its own model, not reading `plans/models/` or `plans/wip/`. Ask it also to check that nothing was lost in the PR body's move to `reviews/step-55-pr-history.md`, from GitHub's edit history of the body.
4. If clean, bring PR #3 to the owner. Never merge without the owner's approval.
