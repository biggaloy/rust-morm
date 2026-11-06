# Coordinator state

Read this first when resuming. It's what the coordinating session needs that isn't in `plan-approved/INDEX.md`. The coordinator updates and pushes it after each milestone.

**Last updated:** 2026-11-06, during step 01's third review fix round.

## Where working files live

Nothing is kept in `/tmp`: it's memory-backed here, so a power cut wipes it. Agents and the coordinator keep working files in the repo's `.claude/scratch/` or their own worktree, with `TMPDIR` set there for commands that create temp files. That folder is on this disk but not on GitHub, so anything needed to resume is also pushed under `plans/`. Put this in every agent's prompt until the `rules.md` line lands.

## How to resume

1. Read `plan-approved/INDEX.md` and follow "The coordinating session". Stay thin: don't read code or the frozen plan.
2. Run `gh pr view 1` and `git log origin/step-01-bootstrap`, to see which fixes below landed. Commits on that branch are the truth.
3. Redo any fix round that didn't land, with a fresh agent, from the files named here. Unpushed agent work is lost in a crash, and nothing else is.
4. Agents can't reach the owner. Bring design questions to the owner, and don't decide them.

## Where step 01 stands

- PR #1 is open and unmerged. `INDEX.md` shows `in review`. Base head for this round: `79d7c42`.
- All repo settings are live: required `ci` check, `crates-io` environment, tag ruleset.
- The third review found problems in two parts, and each has a fix round running on `step-01-bootstrap`:
  - **Plan fixes:** apply `reviews/step-01-review-3-plan.md`, all blockers and minors, with owner answers O1 to O5 at the reviewer's recommendation and all five splits (new steps 55 to 59). `rules.md` gains step 55 in the owner's list and the manifest check in "Independent review". The `INDEX.md` rows, depends-on cells and "Order" change on the branch, because the checker needs them with the step files. Decisions and statuses stay on `main`.
  - **Code fixes:** done at `57e2c20`, with CI green. The rewritten `fetch_last_bench.sh` and `fuzz` step have only run against stubs. `nothing_switches_a_job_or_step_off` is still tagged R-0957, so the next code review should judge that tag.

## What comes next

1. When both rounds land, run a fresh independent review of what changed since `79d7c42`, one for code and one for plans. Repeat until clean.
2. Bring PR #1 to the owner. The owner approves it, and no agent merges it without that.
3. Merge, mark step 01 `done`, and pull. Check `INDEX.md` on `main` has both its Decisions and the branch's rows and order.
4. Start the next step as a fresh agent, then an independent review. The owner approves the PRs of steps 01, 02, 03, 50 and, once the plan edit lands, 55.

## Open with the owner

- Four collaborators have write access to the repo, and `main` needs no review. The owner may want to reduce that list.

## Not proven yet

- `actionlint` has never run. The permission system refused it for agents, so the owner would have to run it.
- `verify`, `publish` and `github-release` haven't run on a real tag, and `CARGO_REGISTRY_TOKEN` hasn't been tested.
- A genuinely expired bench artifact hasn't been seen.

## Housekeeping

- Old agent worktrees under `.claude/worktrees/` are locked by the harness, and one holds `step-01-bootstrap`. Remove them once no agent needs them, and don't force a lock while an agent is running.
- `.claude/` and `target/` show as untracked on `main` until PR #1 brings `.gitignore`. Don't commit them.
