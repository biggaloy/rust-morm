# Brief: step 55, the B4 savepoint fix round

Started 2026-11-20 as a background agent. If its commit isn't on `origin/step-55-enum-proofs` after a restart, start a fresh agent (`isolation: worktree`, background) with the brief below, unchanged. Before that, check the remote branch `wip/step-55-b4`, where the agent pushes its work in progress, and the old worktree under `.claude/worktrees/` for uncommitted work (`git status`). If `wip/step-55-b4` exists, tell the new agent to start from it (`git checkout -B fix-b4 origin/wip/step-55-b4`) and to keep pushing work in progress there; it deletes the branch once its work is on `step-55-enum-proofs`.

---

PR #3 (repo `biggaloy/rust-morm`, branch `step-55-enum-proofs`, head `81f53c3`) has one open question in its PR body from the EW fix round: should a stale-session error morm returns as it is inside a savepoint (row B4) fail the whole transaction? The owner chose option (b); see INDEX Decisions "A stale-session error inside a savepoint stays catchable (B4)" on origin/main (`a6c6812`), and "`SchemaChanged` fails the whole transaction; held connections never read `<n>` (EW1, EW2)" before it. Write it into `step-55-enum-proofs` (`git checkout -B fix-b4 origin/step-55-enum-proofs`, `git push origin HEAD:step-55-enum-proofs`, no force): inside a nested `tx.transaction(..)` (a savepoint), rows B3 and B4 make no read of `<n>`; R-0981's epoch test judges by the pool's last-read `<n>`, so B4 needs no whole-transaction rollback and the app can catch its error at the savepoint and carry on; each call carries a savepoint bit; a migration first seen there is learned at the next read outside a transaction. Rows that return `SchemaChanged` still roll back the whole transaction. Update R-1270 (keep it one consolidated section, no patch-on-patch wording), R-0981, R-1271, R-1292's loud notice (what an app sees inside a savepoint now), and anything else that depends on it; T-0318 / T-0319 cases and a fault switch for the savepoint bit, with a committed shrunk sequence as for the other fault tests.

Re-run the concurrent model against the procedure as written (all settings in `proofs.md`'s latest record, savepoints at REPEATABLE READ / SERIALIZABLE included), every trace and fault switch, and record results in `proofs.md`. Every remaining case must be closed in the plan or stated with a loud README notice; anything needing a design choice the owner hasn't made becomes an open question with options and a recommendation, never decided by you.

Read `plans/plan-approved/rules.md`, `contracts.md`, `proofs.md` on the branch and the relevant INDEX Decisions on origin/main first. New IDs from R-1294 and T-0330 if needed; no INDEX Decisions edits; keep `manifest.json` and `coverage.md` in step. Run the checker (`check_coverage.py --step 55 --code crates tests benches fuzz .github`, and plain) to 0 problems, push, CI green, PR body re-read before `gh pr edit` and read back after (mark the question answered, option (b); word INDEX lines so they don't depend on what INDEX has recorded yet). Report: commit hash, CI, checker totals, changed and new IDs by step, what the model showed, your own choices, open questions, leftovers.

Don't touch files in the session scratchpad you didn't create.

Plus the common rules in `plans/coordinator-briefs.md`, with the session's attribution lines on every commit and the PR body.

## After it reports

1. Record its changed and new IDs and its own choices in INDEX Decisions, appended to "A stale-session error inside a savepoint stays catchable (B4)".
2. Bring any open question to the owner, with options and a recommendation.
3. Then the twentieth review ("Review round" brief): PR #3, scope since `a365aa5`, findings `EX1, …` to `plans/reviews/step-55-review-20.md`. Ask it to build its own concurrent model, independent of `proofs.md`'s, as the eighteenth and nineteenth reviews did, savepoints and held connections included.
4. If clean, bring PR #3 to the owner.
