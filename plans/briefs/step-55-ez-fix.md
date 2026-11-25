# Brief: step 55, the EZ fix round

Started 2026-11-25 as a background agent (worktree `.claude/worktrees/agent-a409538c3bab26380`, local branch `fix-ez`). If its commits aren't on `origin/step-55-enum-proofs` after a restart:

1. If the same session was reopened (`claude --resume`), resume the agent with `SendMessage` to `a409538c3bab26380`.
2. Otherwise check `origin/wip/step-55-ez` and the old worktree for uncommitted work (`git status`, `.claude/scratch/`), commit and push any to `wip/step-55-ez`, then start a fresh agent (`isolation: worktree`, background) with the brief below, unchanged. If the wip branch exists, tell it to start from there.

Update the attribution line to whatever the session then gives.

---

The twenty-second review of PR #3 (repo `biggaloy/rust-morm`, branch `step-55-enum-proofs`, head `93854c2`) is in: `plans/reviews/step-55-review-22.md` on origin/main (`55a720a`). It needs no owner decision. Fix every finding, EZ1 to EZ6, on `step-55-enum-proofs` (`git checkout -B fix-ez origin/step-55-enum-proofs`, `git push origin HEAD:step-55-enum-proofs`, no force), one bullet per finding, as the review recommends.

Read first: `plans/plan-approved/rules.md`, `contracts.md`, `proofs.md` on the branch; on origin/main the INDEX Decisions entries "A hand-made enum swap without relabel is stated, not cured (EY2)", "`SchemaChanged` fails the whole transaction; held connections never read `<n>` (EW1, EW2)" and "A stale-session error inside a savepoint stays catchable (B4)"; and the review.

Beyond each finding's own sentence, check the neighbouring text for the same error, because the last two rounds each fixed one sentence and left its sibling wrong:
- EZ1 and EZ2: read each invariant in T-0319 (I1 to I11) and R-1271's bullets as a step 20 session would build them literally, and check each clause that says "only", "never", "at most" or "from then on" against the procedure in R-1270: is it limited to visible errors where it should be, does its exception cover every half it should, and does T-0319's stated load reach it? The wording of the XX000 exception must agree in T-0319, R-1271, R-1293, the PR body and `proofs.md`.
- EZ3: the owner's decision keeps both ways to proceed (relabel, or end the sessions). The notice says when each works, as the review recommends; don't drop either one.
- EZ6: the parameter test's `22P02` limit must agree in R-1270, R-0981, R-1245 and T-0153.

Write EZ1's and EZ2's final wording as literal checks in a model with T-0319's load as stated after EZ4, and show: no fault gives no failure in any setting, straight to Postgres and behind PgBouncer, savepoints and held connections included; each clause you changed can fail (a fault switch or a deliberate break catches it); the 32nd and 33rd switches' counts at the stated load. If the review's recommended sentence fails in your model, correct it, say so and say why. Record the round in `proofs.md`. Probe Postgres 17 and 18 for any new claim about Postgres. If anything needs a design choice the owner hasn't made, write it as an open question with options and a recommendation; never decide it.

New IDs from R-1295 and T-0330 if needed; no INDEX Decisions edits; keep `manifest.json` and `coverage.md` in step. Run the checker (`check_coverage.py --step 55 --code crates tests benches fuzz .github`, and plain) to 0 problems, push, CI green, PR body re-read before `gh pr edit` and read back after (word INDEX lines so they don't depend on what INDEX has recorded yet; number your own choices from 173). Push work in progress to `wip/step-55-ez` often; if that branch already exists, start from it. Delete it once the work is on `step-55-enum-proofs`.

Report: commit hash, CI, checker totals, one bullet per finding, changed and new IDs by step, what the model showed, your own choices, open questions, leftovers.

Plus the common rules in `plans/coordinator-briefs.md` on origin/main (read them): commit and push as `Biggo Dev <biggodev@gmail.com>`, never neurobin; never use `/tmp`, working files go in `.claude/scratch/` in your worktree with `TMPDIR` set there (Unix sockets with paths too long for that go under `~/.claude/scratch/`); if you use cargo to fetch anything new, set `CARGO_HOME` inside your scratch so the shared `~/.cargo` cache stays as it is; clean up servers, scratch and branches when done and report what's left; don't touch files in the session scratchpad you didn't create. End every commit message with only:

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

and keep the PR body ending with `🤖 Generated with [Claude Code](https://claude.com/claude-code)`, with no session link added.

## After it reports

1. Record its changed and new IDs and its own choices in INDEX Decisions as a new entry, "Twenty-second review fixed (EZ1 to EZ6)". Update the state file.
2. Bring any open question to the owner, with options and a recommendation.
3. Then a short twenty-third review ("Review round" brief): PR #3, scope since `93854c2`, findings `FA1, …` to `plans/reviews/step-55-review-23.md`, with its own model, independent of `proofs.md`'s.
4. If clean, bring PR #3 to the owner. Never merge without the owner's approval.
