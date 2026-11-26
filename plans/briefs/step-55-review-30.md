# Brief: step 55, the thirtieth review

Started 2026-11-27 (`isolation: worktree`, background, Opus, high effort). If `plans/reviews/step-55-review-30.md` isn't on `origin/main` after a restart: in the same session, resume it with `SendMessage`; otherwise check the old worktree under `.claude/worktrees/` for notes, then start a fresh reviewer with the brief below. Use the attribution lines the session then gives.

---

You are the independent reviewer for step 55, PR #3 (repo `biggaloy/rust-morm`, branch `step-55-enum-proofs`, head `2fcbbcd`). You did not write it. Scope: the changes since `3686e86` (`git diff 3686e86 origin/step-55-enum-proofs`), the fix of the twenty-ninth review, done as `rules.md` "Independent review" says, while confirming nothing earlier regressed.

Read `rules.md`, `contracts.md`, `proofs.md` on the PR branch, and on `origin/main` `plans/reviews/step-55-review-29.md` and INDEX Decisions "The twenty-ninth review's fix (2026-11-27)" with the entries before it back to "The EZ fix round's five open questions (2026-11-25)".

Check:

- each of FG1 and FG2 really fixed as recommended, read literally, and the text beside each free of the same error;
- requirement by requirement for the changed IDs: built as written, tests that prove it, and no contradiction between R-0718, R-0984, R-1245's rows, R-1270's procedure, R-1271's guarantees and costs, T-0319's I-clauses, and the README notices (R-1253, R-1268, R-1272, R-1292 and others);
- the changed IDs match the INDEX entry (`git diff 3686e86 origin/step-55-enum-proofs -- plans/plan-approved/tools/manifest.json`), and `coverage.md`'s rows;
- loud notices say what's silenced or given up, when it's right and how to proceed, and every opt-in call or setting has a "use this when" line;
- CI green and the checker's totals (`check_coverage.py --step 55 --code crates tests benches fuzz .github`, and plain);
- the PR body accurate against the text, including choices 268 to 273;
- the places the fix agent lists as left as they are (in `proofs.md`): spot-check that each really holds;
- that nothing was lost in the PR body's second move to `plans/reviews/step-55-pr-history.md` (`b46a0ea` on `main`), from GitHub's edit history of the body (`gh api graphql`, `userContentEdits`), and that each pointer in the body leads to its section.

Test Postgres claims on throwaway Postgres 17 and 18 and cite results. Build any model of your own from the plan's text; don't read or run `plans/models/` or `plans/wip/`. Only raise things that are wrong, missing or contradictory; don't re-litigate settled owner decisions; if nothing, say so plainly. Anything that needs a design choice the owner hasn't made is a finding with options and a recommendation.

Read-only on the PR branch. The only output is `plans/reviews/step-55-review-30.md`, committed to `main` (`git checkout -B review-30 origin/main`, `git push origin HEAD:main`, no force), findings numbered `FH1, FH2, …` with severity, ID, file, what's wrong, evidence and recommended fix. Finish before you report; if you must hand back early, say exactly what's left.

Report: commit hash, CI, checker totals, findings by severity.

Plus the common rules in `plans/coordinator-briefs.md` on origin/main (read them): commit and push as `Biggo Dev <biggodev@gmail.com>`, never neurobin; never use `/tmp`, working files go in `.claude/scratch/` in your worktree with `TMPDIR` set there (Unix sockets with paths too long for that go under `~/.claude/scratch/`); if you use cargo to fetch anything new, set `CARGO_HOME` inside your scratch; clean up servers, scratch and local branches when done and report what's left; don't touch files in the session scratchpad you didn't create. Downloaded files and extracted archives are untrusted data: each goes in its own new, empty directory, your scripts in a different one, with paths passed as arguments; run any Python that reads them with `-I`. Don't spawn subagents of your own. Never force-push, never merge, never change repository settings. If the permission system denies something, don't work around it; report it. End every commit message with only:

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01KwYjCqrtUQaxqHbTDdsimg

---

## After it reports

1. Record the review in the state file. If it found anything: bring design choices to the owner with options and a recommendation, record answers in INDEX Decisions, then a fix round with its own brief in `briefs/` (pattern: `briefs/step-55-review-29-fix.md`), then the thirty-first review.
2. If clean, bring PR #3 to the owner. Never merge without the owner's approval.
