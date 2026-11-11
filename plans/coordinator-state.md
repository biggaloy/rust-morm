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

**Latest:** step 60 is written in at `ae0498f` (CI green, checker 1130 R / 236 T, 0 problems); its IDs are in Decisions, and the key-type section is removed from `later-defo-build.plan` on `main`. The owner answered BE to BH (see Decisions). BE opened three gaps, designed in `reviews/step-60-families-design.md` (`c18de68`) as choices BI to BR, each with a recommendation, now with the owner. Next free IDs: R-1131, T-0237.

## What comes next

1. Bring the BE gaps design to the owner, record the answers, then one edit agent writes BE to BH in on the branch.
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
