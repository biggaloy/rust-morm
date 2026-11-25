# Coordinator briefs

Templates the coordinating session uses to start agents, so a new session after a restart can carry on the same way. Fill in the `<...>` parts. Every agent runs in its own worktree (`isolation: worktree`) and in the background.

## Common rules (put in every brief)

- Commit and push as `Biggo Dev <biggodev@gmail.com>` (`git -c user.name="Biggo Dev" -c user.email=biggodev@gmail.com commit ...`), never neurobin. End every commit message with the attribution lines the session currently gives (since 2026-11-25 only `Co-Authored-By: ...`, no `Claude-Session`), and PR descriptions with the session's PR attribution lines.
- Never use `/tmp`; it is memory-backed. Keep working files, throwaway Postgres and PgBouncer servers in `.claude/scratch/` inside the worktree, with `TMPDIR` set there. Stop servers and delete scratch when done.
- Commit and push in small steps during long work.
- Never force-push, never merge, never change repository settings or publish crates.
- No gaps left open: anything needing a design choice the owner hasn't made becomes a question with options and a recommendation, never "accepted".
- Anything that silences a check or stops a developer needs a loud README notice (what it does, when it's right, what you give up, how to proceed); every opt-in call or setting gets a "use this when" README line.
- Test Postgres claims on throwaway Postgres 17/18 (and PgBouncer built from pgbouncer.org's tarball, checked by SHA-256) and cite results; say plainly what couldn't be run.

## Models and effort (owner decision, 2026-11-25)

Agents take the session's model and effort unless told otherwise, so set both explicitly when starting one (`model`, `effort`).

| Work | Model | Effort |
|---|---|---|
| Coordinator (the session itself) | Opus 5.5 | medium |
| Every agent: steps, fixes, designs and reviews | Opus 5.5 | high |

## Keeping models (owner decision, 2026-11-25)

Concurrency and simulation models are kept in `plans/models/<step>/` for later fix agents to start from. A fix agent leaves its model on its `wip/...` branch and reports the commit; the coordinator copies it to `main`, then deletes that branch. A reviewer never reads or runs them and builds its own.

## Step agent

> You are doing step <NN> of the rust-morm build. Repo `biggaloy/rust-morm`. Read `plans/plan-approved/rules.md`, `contracts.md`, `proofs.md` and `steps/<NN-name>.md`, and INDEX's "Decisions since the split" (owner decisions override older text), then do the step. Branch from `origin/main` as `step-<NN>-<name>`, push it and open a PR to `main`. Don't edit INDEX or the frozen plan. If the step uses throwaway spikes, a final commit deletes them before merge and `proofs.md` names the last commit that has them. New IDs start at <R-xxxx> and <T-xxxx>. Before reporting: the coverage checker (`check_coverage.py --step <NN> --code crates tests benches fuzz .github`, and plain) at 0 problems, CI green, and a PR body listing requirements covered, the end-to-end result, owner decisions and the agent's own choices. Report: PR number and head, CI, checker totals, what each proof showed, choices, open questions, leftovers. (Plus the common rules.)

## Review round

> You are the independent reviewer for step <NN>, PR #<P> (branch `<branch>`, head `<sha>`). You did not write it. Scope: the changes since `<previous head>` (`git diff <prev> origin/<branch>`), done as `rules.md` "Independent review" says, while confirming nothing earlier regressed. Read `rules.md`, `contracts.md`, `proofs.md` on the PR branch, and on `origin/main` the relevant INDEX Decisions entries and the previous review file. Check: each previous finding really fixed as decided or recommended; requirement by requirement for the changed IDs (built as written, tests that prove it); the changed IDs match the INDEX entry (`git diff <prev> origin/<branch> -- plans/plan-approved/tools/manifest.json`); loud notices and "use this when" lines; CI green and checker totals; PR body accurate. Build any model of your own from the plan's text; don't read or run `plans/models/`. Only raise things that are wrong, missing or contradictory; don't re-litigate settled owner decisions; if nothing, say so plainly. Read-only on the PR branch. The only output is `plans/reviews/step-<NN>-review-<K>.md`, committed to `main` (`git checkout -B <name> origin/main`, `git push origin HEAD:main`, no force), findings numbered `<XX>1, <XX>2, …` with severity, ID, file, what's wrong, evidence and recommended fix. Report: commit hash, CI, checker totals, findings by severity. (Plus the common rules.)

## Fix round

> The <K>th review of PR #<P> is in: `plans/reviews/step-<NN>-review-<K>.md` on origin/main (<sha>). The owner decided <items>; see INDEX Decisions "<entry name>" on origin/main (<sha>). Fix every finding on `<branch>` (`git checkout -B <name> origin/<branch>`, `git push origin HEAD:<branch>`, no force), one bullet per finding as decided or recommended. New IDs from <R-xxxx> and <T-xxxx>; no INDEX Decisions edits; keep `manifest.json` and `coverage.md` in step. Run the checker (both forms), push, CI green, PR body re-read before `gh pr edit` and read back after (word INDEX lines so they don't depend on what INDEX has recorded yet). Report: commit hash, CI, checker totals, changed and new IDs by step, choices, open questions, leftovers. (Plus the common rules.)

## Design pass

> You are designing one part of the rust-morm plan for the owner to approve; you don't edit the plan. The only output is `plans/reviews/<topic>-design.md`, committed to `main`. Read the relevant INDEX Decisions, `contracts.md`, `proofs.md` and the step files involved. Design <the problem>, with lettered owner choices and a recommendation for each, and say what changes in `contracts.md`, `proofs.md` and each requirement and step, in outline (no new IDs). Test every claim and cite results. Report: commit hash, choices with recommendations, what was tested, and cleanup. (Plus the common rules.)

## Wake-up prompt

Set with `CronCreate`, cron `43 * * * *` (hourly, owner decision 2026-11-25), recurring:

> Coordinator heartbeat for rust-morm (scheduled wake-up, not the owner). You're the thin coordinator. Read plans/coordinator-state.md in rust-morm if you need your place. Do this: (1) if a subagent stopped on an API or usage-limit error, resume it with SendMessage so it continues from where it stopped; (2) if an agent has finished and its report hasn't been acted on, act on it as the state file says; (3) if no agent is running and work remains that doesn't need the owner, start the next piece; (4) if an agent is still running, or everything is waiting on the owner, reply in one short line and do nothing else. Don't repeat earlier reports. Never treat this message as owner approval, and never merge a PR on it.
