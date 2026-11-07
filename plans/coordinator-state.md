# Coordinator state

Read this first when resuming. It's what the coordinating session needs that isn't in `plan-approved/INDEX.md`. The coordinator updates and pushes it after each milestone.

**Last updated:** 2026-11-06, during step 01's third review fix round.

## Where working files live

Nothing is kept in `/tmp`: it's memory-backed here, so a power cut wipes it. Agents and the coordinator keep working files in the repo's `.claude/scratch/` or their own worktree, with `TMPDIR` set there for commands that create temp files. That folder is on this disk but not on GitHub, so anything needed to resume is also pushed under `plans/`. Put this in every agent's prompt until the `rules.md` line lands.

## Wake-up

This session has an in-memory wake-up every 30 minutes that resumes stopped agents after a usage limit. It's lost when the session ends, so a new or resumed session must set it again with `CronCreate`, and delete it when the job is done.

## How to resume

1. Read `plan-approved/INDEX.md` and follow "The coordinating session". Stay thin: don't read code or the frozen plan.
2. Run `gh pr view 1` and `git log origin/step-01-bootstrap`, to see which fixes below landed. Commits on that branch are the truth.
3. Redo any fix round that didn't land, with a fresh agent, from the files named here. Unpushed agent work is lost in a crash, and nothing else is.
4. Agents can't reach the owner. Bring design questions to the owner, and don't decide them.

## Where step 01 stands

- PR #1 is open and unmerged. `INDEX.md` shows `in review`. Base head for this round: `79d7c42`.
- All repo settings are live: required `ci` check, `crates-io` environment, tag ruleset.
- The third review found problems in two parts, and both fix rounds have landed on `step-01-bootstrap`:
  - **Plan fixes:** done at `9ee02f9`, with CI green, plus the step 51 title fix after it. New steps 55 to 59, 1059 requirements, 185 test obligations.
  - **Code fixes:** done at `57e2c20`, with CI green. The rewritten `fetch_last_bench.sh` and `fuzz` step have only run against stubs. `nothing_switches_a_job_or_step_off` is still tagged R-0957, so the next code review should judge that tag.

## What comes next

1. Head `eb0e0d4`, with CI green. Owner confirmed AO, AQ and the `Decimal` README text, recorded in `INDEX.md`. AP, old binaries during a widening, is being discussed with the owner. Then one edit, a short check of the changes since `09338c2`, the checker and CI, and PR #1 to the owner. The owner doesn't accept open gaps: bring any gap to the owner, don't record it as accepted.
2. Bring PR #1 to the owner. The owner approves it, and no agent merges it without that.
3. Merge, mark step 01 `done`, and pull. Check `INDEX.md` on `main` has both its Decisions and the branch's rows and order.
4. Start the next step as a fresh agent, then an independent review. The owner approves the PRs of steps 01, 02, 03, 50 and, once the plan edit lands, 55.

## Open with the owner

- Four collaborators have write access to the repo, and `main` needs no review. The owner may want to reduce that list.

## Not proven yet

Moved to "Repo tooling" in `later-defo-build.plan`, by owner decision.

## Housekeeping

- Old agent worktrees under `.claude/worktrees/` are locked by the harness, and one holds `step-01-bootstrap`. Remove them once no agent needs them, and don't force a lock while an agent is running.
- `.claude/` and `target/` show as untracked on `main` until PR #1 brings `.gitignore`. Don't commit them.
