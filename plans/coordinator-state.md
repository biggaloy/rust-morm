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

**Latest:** BS to BU are written in at `63775d9` (checker 1156 R / 246 T, 0 problems), IDs in Decisions. CI: `pooler` failed again on the Ubuntu mirror DNS error; rerun 1 of 3 in progress (runs 43041915348, 43041929695). Four follow-ups, BV to BY in `reviews/step-01-open-items.md`, are with the owner. Next free IDs: R-1157, T-0247.

## What comes next

1. Confirm CI green on `63775d9` (rerun `pooler` if it hits the mirror DNS error), get the owner's answers to BV to BY, record them, and have one edit agent write them in.
2. Run a short check of the changes since `09338c2`, the checker and CI, with one fresh reviewer. By owner decision, this is the last round.
3. Bring PR #1 to the owner, who approves it. No agent merges without that.
4. Merge, mark step 01 `done`, pull, and check `INDEX.md` on `main` keeps both its Decisions and the branch's rows and order.
5. Start step 02 as a fresh agent, then an independent review. The owner approves the PRs of steps 01, 02, 03, 50 and 55.

## Open with the owner

- **When bringing PR #1:** point out the choices the BB to BD edit made (see the "Rollback loss and the startup check" entry in Decisions).
- Four collaborators have write access, and `main` needs no review. The owner may want to reduce that list.

## Not proven yet

Moved to "Repo tooling" in `later-defo-build.plan`, by owner decision.

## Housekeeping

- Agent worktrees under `.claude/worktrees/`, about a dozen and some locked, hold only finished scratch work, including 1.4 GB from the `plpgsql_check` builds. Once no agent is running, list them and ask the owner before removing any.
- `.claude/` and `target/` show as untracked on `main` until PR #1 brings `.gitignore`. Don't commit them.
- The rust-morm repo's `.claude/scratch/session-scratchpad/` holds copies of the first session's scratch files. They're not needed to resume.
