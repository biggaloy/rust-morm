# Brief: step 55, the owner's five answers to the EZ round

Started 2026-11-25 as a background agent. If its commits aren't on `origin/step-55-enum-proofs` after a restart: in the same session, resume it with `SendMessage`; otherwise check `origin/wip/step-55-ez-answers` and the old worktree under `.claude/worktrees/` (commit and push any uncommitted work to that branch), then start a fresh agent (`isolation: worktree`, background) with the brief below, telling it to start from the wip branch if there is one. Use the attribution line the session then gives.

---

PR #3 (repo `biggaloy/rust-morm`, branch `step-55-enum-proofs`, head `19be048`) has five open questions in its PR body ("Open questions"). The owner answered option (a) on each: INDEX Decisions "The EZ fix round's five open questions (2026-11-25)" on origin/main. Write them into `step-55-enum-proofs` (`git checkout -B fix-ez-answers origin/step-55-enum-proofs`, `git push origin HEAD:step-55-enum-proofs`, no force).

Read first: `plans/plan-approved/rules.md`, `contracts.md`, `proofs.md` on the branch; on origin/main the INDEX Decisions entries from "`SchemaChanged` fails the whole transaction (EW1, EW2)" to that one, and `plans/reviews/step-55-review-22.md`.

**First, the PR body.** It's 261,451 bytes, and GitHub's limit is 262,144. Move the older rounds' sections, word for word, to `plans/reviews/step-55-pr-history.md` on `main` (the only file you may push to `main`), and link it from the body. The round being added, every one of the fix rounds' own choices (1 to the last), the open and answered questions, and the closing line stay in the body. Check nothing is lost: every moved line is in the file. Leave at least 30 KB free.

**Then the five answers,** one bullet each. Keep R-1270 one consolidated section with no patch-on-patch wording. Follow each answer into everything that depends on it, and check the neighbouring text for the same error, read literally:
1. A fresh run's session-class error outside a transaction, and the seventh row before the second. The case given up (a kept type renamed by hand without the reset) is stated in R-1271 and loudly in R-1294. Drop the "as the procedure stands" text in R-1271 and T-0319.
2. The random part: the cutover's crash recovery and resume reuse the recorded one, and its inverse, `down`, retire and purge, `import`, `verify` and `check_schema` find the labels by it; label and name length limits still hold at 63 bytes with it, the refusal for a label too long included; R-1294's hand relabel uses one too; everything that detects `_morm_retired_` still does. R-0995, R-1023, R-1050, R-1249, R-1253, R-1294, R-1271, I9 and T-0309 at least. Say what happens to an enum retired by an earlier plan version, or state that none can exist before 0.1.
3. The second row's test with the bound texts taken out: say exactly what is taken out (which bound forms, arrays and composite values included), and that no removal can make a stale error look not stale or the reverse.
4. Row C2 outside a transaction. Held connections and the app's own transactions are not given back. No call may wait while holding a connection: check every other place in the procedure that takes a second connection for the same deadlock, with T-0319's load reaching `max` 1.
5. The extra bit.

**Weak checks to strengthen,** or say why they can't be: a deliberate break of EZ2's "at most once per copy" is caught in only 9 of 256; the no-stale-lock switch is only partly split (91 of 256, 597 of 2,048, and that figure counts I3 alone).

**The model.** The EZ round's model (Rust), probe and notes are in this machine's local branch `step-55-model-local` (`c3cbeb4`), under `plans/wip/`: `git archive step-55-model-local plans/wip | tar -x -C <your scratch>`. Start from it instead of rebuilding. Add the five answers, then run every setting and all 33 switches at the stated load, each changed clause broken on purpose, and the five questions' own cases, straight to Postgres and behind PgBouncer. No fault may fail any setting. Probe Postgres 17 and 18 for any new claim about Postgres. Record the round in `proofs.md`. Keep the model, scripts and notes on `wip/step-55-ez-answers` under `plans/wip/`, pushed often (after each answer), never on `step-55-enum-proofs`; when done, leave that branch on origin and report its commit, so the coordinator can keep the model.

Anything needing a design choice the owner hasn't made is an open question with options and a recommendation, never decided by you. New IDs from R-1295 and T-0330 if needed; no INDEX Decisions edits; keep `manifest.json` and `coverage.md` in step. Run the checker (`check_coverage.py --step 55 --code crates tests benches fuzz .github`, and plain) to 0 problems, push, CI green, PR body re-read before `gh pr edit` and read back after (mark the questions answered, option (a); word INDEX lines so they don't depend on what INDEX has recorded yet; number your own choices from 187).

Report: commit hash, CI, checker totals, one bullet per answer, changed and new IDs by step, what the model showed, your own choices, open questions, leftovers.

Plus the common rules in `plans/coordinator-briefs.md` on origin/main (read them): commit and push as `Biggo Dev <biggodev@gmail.com>`, never neurobin; never use `/tmp`, working files go in `.claude/scratch/` in your worktree with `TMPDIR` set there (Unix sockets with paths too long for that go under `~/.claude/scratch/`); if you use cargo to fetch anything new, set `CARGO_HOME` inside your scratch; clean up servers, scratch and local branches when done and report what's left; don't touch files in the session scratchpad you didn't create. End every commit message with only:

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

and keep the PR body ending with `🤖 Generated with [Claude Code](https://claude.com/claude-code)`, with no session link.

---

## After it reports

1. Record its changed and new IDs and its own choices in INDEX Decisions, appended to "The EZ fix round's five open questions (2026-11-25)". Update the state file.
2. Bring any open question to the owner, with options and a recommendation.
3. Then the twenty-third review ("Review round" brief): PR #3, scope since `93854c2`, findings `FA1, …` to `plans/reviews/step-55-review-23.md`, with its own model, independent of `proofs.md`'s and of `plans/wip/`. Ask it to check nothing was lost in the PR body's move.
4. If clean, bring PR #3 to the owner. Never merge without the owner's approval.
