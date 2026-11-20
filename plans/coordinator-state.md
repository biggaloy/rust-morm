# Coordinator state

Read this first when resuming. It's what the coordinating session needs that isn't in `plan-approved/INDEX.md`. The coordinator updates and pushes it after each milestone. Agent briefs to reuse are in `coordinator-briefs.md`.

**Last updated:** 2026-11-20, B4 fix round running, before a weekly usage limit.

## Where things stand

- **Steps done:** 01 (PR #1, merged `ccd4413`) and 02 (PR #2, merged `34c81aa`), both approved by the owner.
- **Step 55 is `in review`:** PR #3, branch `step-55-enum-proofs`, head `81f53c3`, CI green, checker 1293 R / 329 T, 0 problems. The owner chose (b) on row B4 (INDEX Decisions "A stale-session error inside a savepoint stays catchable (B4)"). A fix agent was started on 2026-11-20 to write it in; its full brief and what to do after it reports are in `briefs/step-55-b4-savepoint-fix.md`. **On a restart:** if `origin/step-55-enum-proofs` has no commit after `81f53c3`, check `.claude/worktrees/agent-ad3c483f7083a9543` for uncommitted work, then start a fresh agent with that brief. If it has, read the PR body's newest section to see what landed and whether the agent finished. Next: record its IDs and choices in INDEX, then the twentieth review (scope since `a365aa5`, with its own model), then PR #3 to the owner if clean.
- **Next free IDs:** R-1294, T-0330.
- Step 55's open items, reviews and designs: `reviews/step-55-open-items.md`, `reviews/step-55-review.md` and `-review-2.md` to `-review-12.md`, `reviews/stale-statement-design.md`.

## How to resume after a restart

1. Read `plan-approved/INDEX.md` ("The coordinating session" and "Decisions since the split") and this file. Stay thin: don't read code or the frozen plan.
2. Set the wake-up again with `CronCreate`: cron `13,43 * * * *`, recurring, prompt as in `coordinator-briefs.md`. A restart removes it, and it expires after 7 days anyway. The current one (`ca486a50`, set 2026-11-19) expires about 2026-11-26.
3. Check what landed: `gh pr view 3`, `git log origin/step-55-enum-proofs`, and `git log origin/main -- plans/reviews/`. Commits are the truth; unpushed agent work is lost in a crash.
4. Background agents don't survive a restart. Restart whatever the "Where things stand" line names as running, from its brief in `briefs/` (or the matching template in `coordinator-briefs.md`), after checking its old worktree for uncommitted work.
5. Remove stale agent worktrees under `.claude/worktrees/` only after checking each has no uncommitted or unpushed work.

## The review loop (how step PRs go)

1. A step agent opens the PR. A fresh independent reviewer reviews it ("Review round" brief) and pushes `reviews/step-NN-review-K.md` to `main`.
2. The coordinator brings design choices to the owner, with options and a recommendation, and records each answer in INDEX's Decisions on `main`.
3. One fix agent writes the findings and decisions into the PR ("Fix round" brief). The coordinator records the IDs and the fix's own choices in Decisions; those choices are approved by the owner through the PR.
4. Repeat until a review finds nothing to fix. Then bring the PR to the owner. The owner approves the PRs of steps 01, 02, 03, 50 and 55; no agent merges without that.
5. After approval: merge with a merge commit, mark the step `done`, check INDEX on `main` keeps its Decisions, delete the merged remote branch and finished worktrees, and start the next step.

## What comes next

1. Step 55: finish the B4 fix round, then the twentieth review, repeat until clean, then PR #3 to the owner.
2. Step 03 (freeze the contracts) depends on 55. Then follow the order in `INDEX.md`.

## Rules that hold for every agent

- Commit and push as `Biggo Dev <biggodev@gmail.com>`, never neurobin, with the session's attribution lines.
- Never use `/tmp`; keep working files in `.claude/scratch/` with `TMPDIR` set there. Clean up when done, and report what's left.
- No gaps left open: close each one, or bring it to the owner. Never record one as "accepted".
- Anything that silences a check or stops a developer's work gets a loud README notice; every opt-in call or setting gets a "use this when" line.
- Never force-push, never merge, never change repository settings without the owner.

## Usage limits

When an agent stops on a usage limit, resume it with `SendMessage` once the limit lifts. Wake-ups can't run during a limit either; they queue and arrive together after it. On a heartbeat, resend a stopped agent even when unsure the limit has lifted: an early resend does no harm.

## Open with the owner

- Four collaborators have write access, and `main` needs no review. The owner may want to reduce that list.

## Housekeeping

- Agent worktrees and their local branches were removed after steps 01 and 02 merged, with the owner's go-ahead. `target/`, old scratch files and the merged remote branches were removed on 2026-11-13. After each future merge, delete the merged remote branch and the finished worktrees, as the owner asked.
