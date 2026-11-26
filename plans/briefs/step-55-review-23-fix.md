# Brief: step 55, the twenty-third review's fix

Started 2026-11-26 (`isolation: worktree`, background, Opus, high effort). If its commits aren't on `origin/step-55-enum-proofs` after a restart: in the same session, resume it with `SendMessage`; otherwise check the old worktree under `.claude/worktrees/` (commit and push any uncommitted work to a `wip/step-55-review-23-fix` branch), then start a fresh agent with the brief below. Use the attribution lines the session then gives.

---

The twenty-third review of PR #3 is in: `plans/reviews/step-55-review-23.md` on origin/main (`b3d89cf`). It found FA1 to FA4, none needing an owner choice. Fix every finding on `step-55-enum-proofs` (head `e417bc8`; `git checkout -B fix-review-23 origin/step-55-enum-proofs`, `git push origin HEAD:step-55-enum-proofs`, no force), one bullet per finding as the review recommends, read literally, and check the text beside each for the same error:

- **FA1:** "only the seventh row reads the message" becomes true of the `22P02` rows only, in R-1245, T-0318 and wherever else it's said; the `XX000` rows and the English second signals still read the message. (The coordinator has already fixed INDEX's wording.)
- **FA2:** the second row's cost inside a transaction stated as it really is, in R-1253 (loud), R-1271's bullets, T-0319's I9 and R-1268: once per `stale_pending_expiry` per statement plus what R-1271 already lists, more often with a shorter expiry, and at every call with pending entries off, in both modes, with `.retry(1)` getting the `22P02`. Check R-1268's "What `"off"` turns off" says this plainly, as the largest cost of turning them off.
- **FA3:** R-1292's savepoint notice says `Error::SchemaChanged` also comes, with no schema change, from a bad enum value a cached statement meets (a label sent, or text a column holds), that inside a savepoint it fails the whole transaction, and how to proceed (check the label first, or `.retry(1)`).
- **FA4:** T-0151, T-0153 and T-0312's repeated bad labels either run with no pending entry for their statement (a new pool, or after a pool-wide action) or state what follows while the entry lasts.

Read first: `plans/plan-approved/rules.md`, `contracts.md`, `proofs.md` on the branch; the review file; on origin/main INDEX Decisions "The two answers' open question (2026-11-26)" and the entries before it back to "The EZ fix round's five open questions (2026-11-25)".

**The model:** start from `plans/models/step-55/` on origin/main; don't edit that folder. If any finding changes what the model checks (FA2's entries-off cost, FA4's outcomes), put it in, run the affected settings and the switches again, and show it; if none does, say why. Record the round in `proofs.md`. Keep model work on `wip/step-55-review-23-fix` under `plans/wip/step-55/`, never on `step-55-enum-proofs`, and report its commit if there is one.

Anything needing a design choice the owner hasn't made is an open question with options and a recommendation, never decided by you. New IDs from R-1295 and T-0330 if needed; no INDEX Decisions edits; keep `manifest.json` and `coverage.md` in step. Run the checker (`check_coverage.py --step 55 --code crates tests benches fuzz .github`, and plain) to 0 problems, push, CI green, PR body re-read before `gh pr edit` and read back after (a section for this review's fix; word INDEX lines so they don't depend on what INDEX has recorded yet; number your own choices from 235).

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

1. Record its changed and new IDs and its own choices in INDEX Decisions, as a new entry "The twenty-third review's fix (2026-11-26)". Copy any model to `plans/models/step-55/` on `main`, then delete the wip branch. Update the state file.
2. Bring any open question to the owner, with options and a recommendation.
3. Then the twenty-fourth review ("Review round" brief, Opus, high effort): PR #3, scope since `e417bc8`, findings `FB1, …` to `plans/reviews/step-55-review-24.md`, with its own model, not reading `plans/models/` or `plans/wip/`, confirming nothing earlier regressed.
4. If clean, bring PR #3 to the owner. Never merge without the owner's approval.
