# Brief: step 55, the twenty-seventh review's fix

Done: fixed at `c458987`, 2026-11-26. Started 2026-11-26 (`isolation: worktree`, background, Opus, high effort). If its commits aren't on `origin/step-55-enum-proofs` after a restart: in the same session, resume it with `SendMessage`; otherwise check the old worktree under `.claude/worktrees/` (commit and push any uncommitted work to a `wip/step-55-review-27-fix` branch), then start a fresh agent with the brief below. Use the attribution lines the session then gives.

---

The twenty-seventh review of PR #3 is in: `plans/reviews/step-55-review-27.md` on origin/main (`2e3d636`). It found FE1 to FE3, all minor wording, none needing an owner choice. Fix every finding on `step-55-enum-proofs` (head `b98bada`; `git checkout -B fix-review-27 origin/step-55-enum-proofs`, `git push origin HEAD:step-55-enum-proofs`, no force), one bullet per finding as the review recommends, read literally, and check the text beside each for the same error. For FE1 take the review's wording, not its alternative (keeping cool-down entries across a repair or an adoption changes the procedure and needs the owner), and correct choice 255 and `proofs.md`. Then sweep every bound on the cool-down and on a stale session's `Error::SchemaChanged`, straight to Postgres and behind PgBouncer, for "once", "for `stale_cooldown`" or similar that a salt, an adoption, the cap or the expiry can break, so nothing of this kind is left. For FE2, add the cost behind PgBouncer to R-1288. For FE3, bring the PR body's "End-to-end result" to your final head.

Read first: `plans/plan-approved/rules.md`, `contracts.md`, `proofs.md` on the branch; the review file; on origin/main INDEX Decisions "The twenty-sixth review's fix (2026-11-26)" and the entries before it back to "The EZ fix round's five open questions (2026-11-25)".

**The model:** start from `plans/models/step-55/` on origin/main; don't edit that folder. If any finding changes what the model checks (FE1's cool-down count behind PgBouncer), put it in, run the affected settings and the switches again, and show it; if none does, say why. Record the round in `proofs.md`. Keep model work on `wip/step-55-review-27-fix` under `plans/wip/step-55/`, never on `step-55-enum-proofs`, and report its commit if there is one.

Anything needing a design choice the owner hasn't made is an open question with options and a recommendation, never decided by you. New IDs from R-1295 and T-0330 if needed; no INDEX Decisions edits; keep `manifest.json` and `coverage.md` in step. Run the checker (`check_coverage.py --step 55 --code crates tests benches fuzz .github`, and plain) to 0 problems, push, CI green, PR body re-read before `gh pr edit` and read back after (a section for this review's fix; word INDEX lines so they don't depend on what INDEX has recorded yet; number your own choices from 257). In `proofs.md` and the PR body, name the kept model as `plans/models/step-55/` on `main`, where the coordinator puts it, never a `wip/...` branch.

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

1. Record its changed and new IDs and its own choices in INDEX Decisions, as a new entry "The twenty-seventh review's fix (2026-11-26)". Copy any model to `plans/models/step-55/` on `main`, then delete the wip branch. Update the state file.
2. Bring any open question to the owner, with options and a recommendation.
3. Then the twenty-eighth review (pattern: `briefs/step-55-review-27.md`): PR #3, scope since `b98bada`, findings `FF1, …` to `plans/reviews/step-55-review-28.md`.
4. If clean, bring PR #3 to the owner. Never merge without the owner's approval.
