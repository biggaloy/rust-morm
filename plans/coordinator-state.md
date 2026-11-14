# Coordinator state

Read this first when resuming. It's what the coordinating session needs that isn't in `plan-approved/INDEX.md`. The coordinator updates and pushes it after each milestone.

**Last updated:** 2026-11-11, after BB to BD landed at `9f4ee7a`.

## Where working files live

Nothing is kept in `/tmp`: it's memory-backed here, so a power cut wipes it. Agents and the coordinator keep working files in the repo's `.claude/scratch/` or their own worktree, with `TMPDIR` set there for commands that create temp files. That folder is on this disk but not on GitHub, so anything needed to resume is also pushed under `plans/`. Put this in every agent's prompt until the `rules.md` line lands.

## Wake-up

The 30-minute wake-up lived only in the session, so a restart removes it. After resuming, set it again with `CronCreate`, so a usage limit doesn't stall the work, and delete it when the job is done. A session started after 2026-11-06 15:50 also loads `autoContinueAtUsageLimit`.

## How to resume

1. Read `plan-approved/INDEX.md` and follow "The coordinating session". Stay thin: don't read code or the frozen plan.
2. Run `gh pr view 1` and `git log origin/step-01-bootstrap`, to see which fixes below landed. Commits on that branch are the truth.
3. Redo any fix round that didn't land, with a fresh agent, from the files named here. Unpushed agent work is lost in a crash, and nothing else is.
4. Agents can't reach the owner. Bring design questions to the owner, and don't decide them.

## Where step 01 stands

**Last saved:** 2026-11-07, before a PC restart, with the weekly usage limit nearly reached.

- PR #1 is open and unmerged, at `acf82a2` when saved. `INDEX.md` shows `in review`. All repo settings are live.
- Step 01's code is reviewed and fixed. PR #1 now also carries the plan redesign that came out of the reviews: managed enums, steps 51 to 59, the general walk, `plpgsql_check`, `--decide`, and type changes that never break a running app.
- Every owner decision is in `INDEX.md` under "Decisions since the split". `reviews/step-01-open-items.md` has every open item, A to AV, each answered.

**Latest:** step 55 is `in review`: PR #3 head `0501425`, EH1 to EH8 written in, CI green, mergeable, checker 1255 R / 314 T, 0 problems; IDs in Decisions. The fifth independent review is running on the changes since `baf82f0`, findings to `reviews/step-55-review-5.md`. Next free IDs: R-1256, T-0315. Asked the owner whether to delete the stray `~/.claude/scratch/ci-watch-a1834.log` (2026-11-11).

## What comes next

1. Step 55: get the owner's answers to EA to ED and the review's findings, have one fix agent write both into PR #3, review until clean, then the owner approves. The owner approves the PRs of steps 01, 02, 03, 50 and 55.
2. Then follow the order in `INDEX.md`.

## Open with the owner

- **When bringing PR #1:** point out the choices the BB to BD edit made (see the "Rollback loss and the startup check" entry in Decisions).
- Four collaborators have write access, and `main` needs no review. The owner may want to reduce that list.

## Not proven yet

Moved to "Repo tooling" in `later-defo-build.plan`, by owner decision.

## Housekeeping

- Every agent worktree and its local branch was removed on 2026-11-12 and again after step 02's merge, with the owner's go-ahead, after checking each had no uncommitted or unpushed work.
- `target/` in the main checkout and the merged remote branches `step-01-bootstrap` and `step-02-proofs` were removed on 2026-11-13 at the owner's request. The older files in `.claude/scratch/` were removed the same day at the owner's request; the folder is empty. After each future merge, delete the merged remote branch and the finished worktrees, as the owner asked.
