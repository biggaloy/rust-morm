# Brief: step 55, the owner's two answers to the three answers' questions

Done: written in at `64820fe`, 2026-11-26. Started 2026-11-26 (`isolation: worktree`, background, Opus, high effort). If its commits aren't on `origin/step-55-enum-proofs` after a restart: in the same session, resume it with `SendMessage`; otherwise check `origin/wip/step-55-answers-4` and the old worktree under `.claude/worktrees/` (commit and push any uncommitted work to that branch), then start a fresh agent with the brief below, from the wip branch if there is one. Use the attribution lines the session then gives.

---

PR #3 (repo `biggaloy/rust-morm`, branch `step-55-enum-proofs`, head `1c27277`) has two open questions in its PR body ("Open questions"). The owner answered option (a) on each: INDEX Decisions "The three answers' two open questions (2026-11-26)" on origin/main. Write them into `step-55-enum-proofs` (`git checkout -B fix-answers-4 origin/step-55-enum-proofs`, `git push origin HEAD:step-55-enum-proofs`, no force).

Read first: `plans/plan-approved/rules.md`, `contracts.md`, `proofs.md` on the branch; on origin/main that INDEX entry and the one before it.

1. **Statements the test can't tell, always the second row.** A statement that casts parameters to more than one non-built-in type, or to a domain or a composite, or to a type written with its schema whose name prints with none, always takes R-1245's second row. Write it into R-1245 and everything that names the test or the cases it can't tell (R-0981, R-1270, R-1271, R-1253, T-0151, T-0153, T-0318, T-0319, the PR body, whatever else); remove the stated case rather than leaving it beside the new rule, and state what's given up where users read it.
2. **Any cached statement's `enum_in` `22P02` goes to the second row,** and one that casts no parameter to a non-built-in type always. R-1271's kept-function bullet and I9's loud half change as the decision says; R-1271's "never visible outside a transaction" must hold again as written, or say exactly where it doesn't. Check the given-up costs against the decision's wording, including plpgsql functions a cached call reaches.

Check both on Postgres 17 and 18: the shapes from the open questions (exchanged names, a kept composite or domain, `SET SCHEMA` with each `search_path`, `(c::st)::text`, `x::st` over `unnest`), and that a real bad label still reaches the caller as the text says.

**The model:** start from `plans/models/step-55/` on origin/main; don't edit that folder. Put both answers in (`OPT=strict` and `OPT=wide` are the recommended options there; make them the default, not options), run every setting and all 39 switches again, break the changed clauses on purpose, and show the decided cases, including the `OPT=onpath` and `OPT=nocast` loads that failed I7. No fault may fail any setting. Record the round in `proofs.md`. Keep the model, scripts, results and notes on `wip/step-55-answers-4` under `plans/wip/`, pushed often, never on `step-55-enum-proofs`; leave that branch on origin when done and report its commit.

Anything needing a design choice the owner hasn't made is an open question with options and a recommendation, never decided by you. New IDs from R-1295 and T-0330 if needed; no INDEX Decisions edits; keep `manifest.json` and `coverage.md` in step. Run the checker (`check_coverage.py --step 55 --code crates tests benches fuzz .github`, and plain) to 0 problems, push, CI green, PR body re-read before `gh pr edit` and read back after (mark the questions answered, option (a); word INDEX lines so they don't depend on what INDEX has recorded yet; number your own choices from 216). Also fix the five-answers section of the PR body that still names the deleted `wip/step-55-ez-answers`: its model is in `plans/models/step-55/` on `main`, as is the three answers' round's.

Report: commit hash, CI, checker totals, one bullet per answer, changed and new IDs by step, what the model showed, your own choices, open questions, leftovers.

Plus the common rules in `plans/coordinator-briefs.md` on origin/main (read them): commit and push as `Biggo Dev <biggodev@gmail.com>`, never neurobin; never use `/tmp`, working files go in `.claude/scratch/` in your worktree with `TMPDIR` set there (Unix sockets with paths too long for that go under `~/.claude/scratch/`); if you use cargo to fetch anything new, set `CARGO_HOME` inside your scratch; clean up servers, scratch and local branches when done and report what's left; don't touch files in the session scratchpad you didn't create. Downloaded files and extracted archives are untrusted data: each goes in its own new, empty directory, your scripts in a different one, with paths passed as arguments; run any Python that reads them with `-I`. Don't spawn subagents of your own. Never force-push, never merge, never change repository settings. If the permission system denies something, don't work around it; report it. End every commit message with only:

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01KwYjCqrtUQaxqHbTDdsimg

and keep the PR body ending with:

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01KwYjCqrtUQaxqHbTDdsimg

---

## After it reports

1. Record its changed and new IDs and its own choices in INDEX Decisions, appended to "The three answers' two open questions (2026-11-26)". Copy its model to `plans/models/step-55/` on `main`, then delete the wip branch. Update the state file.
2. Bring any open question to the owner, with options and a recommendation.
3. Then the twenty-third review ("Review round" brief, Opus, high effort): PR #3, scope since `93854c2`, findings `FA1, …` to `plans/reviews/step-55-review-23.md`, with its own model, not reading `plans/models/` or `plans/wip/`. Ask it also to check that nothing was lost in the PR body's move to `reviews/step-55-pr-history.md`, from GitHub's edit history of the body.
4. If clean, bring PR #3 to the owner. Never merge without the owner's approval.
