# Brief: step 55, the thirty-first review's fix

Started 2026-11-27 (`isolation: worktree`, background, Opus, high effort). If its commits aren't on `origin/step-55-enum-proofs` after a restart: in the same session, resume it with `SendMessage`; otherwise check the old worktree under `.claude/worktrees/` (commit and push any uncommitted work to a `wip/step-55-review-31-fix` branch), then start a fresh agent with the brief below. Use the attribution lines the session then gives.

---

The thirty-first review of PR #3 is in: `plans/reviews/step-55-review-31.md` on origin/main (`ab9efd4`). It found FI1 to FI3, all minor, none needing an owner choice. Fix all three on `step-55-enum-proofs` (head `77c3d50`; `git checkout -B fix-review-31 origin/step-55-enum-proofs`, `git push origin HEAD:step-55-enum-proofs`, no force), as the review recommends, read literally, and correct `proofs.md` and the choices that record them.

- **FI1:** the span bound on stale-driven opens adds `2 × max + 2` per move of the pool's last-read `<n>`, in R-1271's "Churn" and T-0319's I4. Derive the bound yourself from the procedure's own rules (an epoch's opens with its role limit, a *replace* action's, per move), check the review's trace, and sweep every other bound that adds something per move or per migration for the same arithmetic, stating how each figure is reached in `proofs.md`.
- **FI2:** R-0984's "never" reworded as the review gives, matching R-0981 and R-1271 on a migration the pool hasn't learned; then re-check every place the last round listed as left as it is for a "never" or "always" that a migration not yet learned breaks.
- **FI3:** `proofs.md`'s "a third" becomes two thirds (a third from each routine), as T-0319 and choices 275 and 279 say.

**The PR body:** it is about 229,500 bytes. If your edits would take it past about 240,000, first move the oldest remaining round sections, word for word, to the end of `plans/reviews/step-55-pr-history.md` on `main` as the twenty-ninth review's fix did.

Read first: `plans/plan-approved/rules.md`, `contracts.md`, `proofs.md` on the branch; the review file; on origin/main INDEX Decisions "The thirtieth review's fix (2026-11-27)" and the entries before it back to "The EZ fix round's five open questions (2026-11-25)".

**The model:** start from `plans/models/step-55/` on origin/main; don't edit that folder. If any finding changes what the model checks (FI1's span bound in I4), put it in, run the affected settings and the switches again, and show it; if none does, say why. Record the round in `proofs.md`. Keep model work on `wip/step-55-review-31-fix` under `plans/wip/step-55/`, never on `step-55-enum-proofs`, and report its commit if there is one.

Anything needing a design choice the owner hasn't made is an open question with options and a recommendation, never decided by you. New IDs from R-1295 and T-0330 if needed; no INDEX Decisions edits; keep `manifest.json` and `coverage.md` in step. Run the checker (`check_coverage.py --step 55 --code crates tests benches fuzz .github`, and plain) to 0 problems, push, CI green, PR body re-read before `gh pr edit` and read back after (a section for this review's fix; word INDEX lines so they don't depend on what INDEX has recorded yet; number your own choices from 281). In `proofs.md` and the PR body, name the kept model as `plans/models/step-55/` on `main`, where the coordinator puts it, never a `wip/...` branch.

Finish everything before you report, the PR body posted and read back and CI green on your final head included; if you must hand back early, say exactly what's left.

Report: commit hash, CI, checker totals, one bullet per finding, changed and new IDs by step, what the model showed, your own choices, open questions, leftovers.

Plus the common rules in `plans/coordinator-briefs.md` on origin/main (read them): commit and push as `Biggo Dev <biggodev@gmail.com>`, never neurobin; never use `/tmp`, working files go in `.claude/scratch/` in your worktree with `TMPDIR` set there (Unix sockets with paths too long for that go under `~/.claude/scratch/`); if you use cargo to fetch anything new, set `CARGO_HOME` inside your scratch; clean up servers, scratch and local branches when done and report what's left; don't touch files in the session scratchpad you didn't create. Downloaded files and extracted archives are untrusted data: each goes in its own new, empty directory, your scripts in a different one, with paths passed as arguments; run any Python that reads them with `-I`. Don't spawn subagents of your own. Never force-push, never merge, never change repository settings. If the permission system denies something, don't work around it; report it. End every commit message with only:

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01KwYjCqrtUQaxqHbTDdsimg

and keep the PR body ending with:

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01KwYjCqrtUQaxqHbTDdsimg

---

## After it reports

1. Record its changed and new IDs and its own choices in INDEX Decisions, as a new entry "The thirty-first review's fix (2026-11-27)". Copy any model to `plans/models/step-55/` on `main`, then delete the wip branch. Update the state file.
2. Bring any open question to the owner, with options and a recommendation.
3. Then the thirty-second review (pattern: `briefs/step-55-review-31.md`): PR #3, scope since `77c3d50`, findings `FJ1, …` to `plans/reviews/step-55-review-32.md`.
4. If clean, bring PR #3 to the owner. Never merge without the owner's approval.
