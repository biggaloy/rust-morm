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

**Latest:** step 02 is `in review`: PR #2 head `e78b970` (DH1 to DH9 and A to K written in; matrix re-run on Postgres 15 to 18 at `ca3747b`, spikes deleted again), CI green, checker 1233 R / 296 T, 0 problems; IDs in Decisions. DI1 and DI2 in `reviews/step-02-open-items.md` are with the owner; then the third independent review. Next free IDs: R-1234, T-0297.

## What comes next

1. Step 02: when the review reports, one agent writes DA to DE in and fixes the findings on PR #2; review repeats until clean; then the spikes-deletion commit and PR #2 to the owner. The owner approves the PRs of steps 01, 02, 03, 50 and 55.
2. Then follow the order in `INDEX.md`.

## Open with the owner

- **When bringing PR #1:** point out the choices the BB to BD edit made (see the "Rollback loss and the startup check" entry in Decisions).
- Four collaborators have write access, and `main` needs no review. The owner may want to reduce that list.

## Not proven yet

Moved to "Repo tooling" in `later-defo-build.plan`, by owner decision.

## Housekeeping

- Every agent worktree and its local branch was removed on 2026-11-12 with the owner's go-ahead, after checking each had no uncommitted or unpushed work.
- Still on disk: `target/` in the main checkout (1.2 GB build output, ignored by git), and older files in `.claude/scratch/` (the first session's `session-scratchpad/` copies and a few review scripts), none needed to resume. The remote branch `step-01-bootstrap` is merged and still on GitHub. Ask the owner before removing any of these.
