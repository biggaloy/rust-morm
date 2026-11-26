# Brief: step 55, the twenty-fifth review's fix

Done: fixed at `e217a1c`, 2026-11-26. Started 2026-11-26 (`isolation: worktree`, background, Opus, high effort). If its commits aren't on `origin/step-55-enum-proofs` after a restart: in the same session, resume it with `SendMessage`; otherwise check the old worktree under `.claude/worktrees/` (commit and push any uncommitted work to a `wip/step-55-review-25-fix` branch), then start a fresh agent with the brief below. Use the attribution lines the session then gives.

---

The twenty-fifth review of PR #3 is in: `plans/reviews/step-55-review-25.md` on origin/main (`2de67c7`). It found FC1 (should fix) and FC2 to FC4 (minor), none needing an owner choice. Fix every finding on `step-55-enum-proofs` (head `59286dc`; `git checkout -B fix-review-25 origin/step-55-enum-proofs`, `git push origin HEAD:step-55-enum-proofs`, no force), one bullet per finding as the review recommends, read literally, and check the text beside each for the same error. For FC1 take the review's recommended wording, not its alternative (changing when the entry is made needs the owner): the bound counts calls that begin before the pending entry is made, as T-0319's I8 already does. Before finishing, search every requirement, test obligation and notice for a bound that excuses only calls "already running when the first failed", or says "at most once" with no exception, and make each use the moment the entry is made; and for FC2 every entries-off clause behind PgBouncer for whether it leaves out the salt a `0A000` requests.

Read first: `plans/plan-approved/rules.md`, `contracts.md`, `proofs.md` on the branch; the review file; on origin/main INDEX Decisions "The twenty-fourth review's fix (2026-11-26)" and the entries before it back to "The EZ fix round's five open questions (2026-11-25)".

**The model:** start from `plans/models/step-55/` on origin/main; don't edit that folder. If any finding changes what the model checks (FC1's bound with entries on, FC2's salt with entries off), put it in, run the affected settings and the switches again, and show it; if none does, say why. Record the round in `proofs.md`. Keep model work on `wip/step-55-review-25-fix` under `plans/wip/step-55/`, never on `step-55-enum-proofs`, and report its commit if there is one.

Anything needing a design choice the owner hasn't made is an open question with options and a recommendation, never decided by you. New IDs from R-1295 and T-0330 if needed; no INDEX Decisions edits; keep `manifest.json` and `coverage.md` in step. Run the checker (`check_coverage.py --step 55 --code crates tests benches fuzz .github`, and plain) to 0 problems, push, CI green, PR body re-read before `gh pr edit` and read back after (a section for this review's fix; word INDEX lines so they don't depend on what INDEX has recorded yet; number your own choices from 247). In `proofs.md` and the PR body, name the kept model as `plans/models/step-55/` on `main`, where the coordinator puts it, never a `wip/...` branch.

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

1. Record its changed and new IDs and its own choices in INDEX Decisions, as a new entry "The twenty-fifth review's fix (2026-11-26)". Copy any model to `plans/models/step-55/` on `main`, then delete the wip branch. Update the state file.
2. Bring any open question to the owner, with options and a recommendation.
3. Then the twenty-sixth review (pattern: `briefs/step-55-review-25.md`): PR #3, scope since `59286dc`, findings `FD1, …` to `plans/reviews/step-55-review-26.md`.
4. If clean, bring PR #3 to the owner. Never merge without the owner's approval.
