# Brief: consolidated stale-session design (step 55)

Started 2026-11-15 by the coordinator after the owner chose one redesign over more patching (INDEX Decisions: "Step 55 twelfth review, and one consolidated stale-session design"). If `plans/reviews/stale-session-state-machine.md` isn't on `main`, start a fresh agent (worktree, background) with this brief. If a partial version is there, tell the agent to continue it.

---

You are designing one part of the rust-morm plan for the owner to approve. You don't edit the plan itself. Repo: `biggaloy/rust-morm`.

**Rules**
- Never use `/tmp`; it is memory-backed. Keep working files, throwaway Rust crates, Postgres servers and PgBouncers in `.claude/scratch/` in your worktree, with `TMPDIR` set there. Stop servers and delete scratch when done.
- Commit as `Biggo Dev <biggodev@gmail.com>`, never neurobin, ending every commit message with the session's attribution lines.
- Your only output file is `plans/reviews/stale-session-state-machine.md`, committed to `main` (`git fetch origin && git checkout -B stalesm origin/main`, then `git push origin HEAD:main`, no force; rebase and retry if rejected). Push in small steps.
- No gaps left open; nothing marked "accepted". Anything that needs a choice the owner hasn't made becomes a lettered choice with a recommendation.

**Why this exists.** PR #3 (step 55, branch `step-55-enum-proofs`, head `0f26ba7`) has had twelve independent reviews. Each found new gaps in one subsystem: how morm recognises and heals stale sessions (statements and compiled functions left stale by a type change). Fixes added rules and the rules interacted badly. The owner chose to stop patching and have the whole rule set redesigned once, as a single coherent state machine.

**Read first.** On `origin/main`: `plans/plan-approved/INDEX.md` "Decisions since the split" from "Step 55, EA and the stale-statement design" through "Step 55 twelfth review, and one consolidated stale-session design" (every owner decision there stands, in particular: stale sessions heal by themselves; the reset and relabel at cutover; SB, SC, SD, SE, SF, SG; the `22P02` narrowing; EF1; EG1 to EG3; EH1 to EH3; EI1; EJ1; EJ5's `RECONNECT` and `server_lifetime`; EK1; EL1; EL2; EM1; EM3; EN1; EN5 and the per-pool settings with defaults, floors and ceilings; EO1 to EO4); `plans/reviews/stale-statement-design.md`; `plans/reviews/step-55-review.md` through `step-55-review-12.md` (the history of every gap found, which the design must close); `plans/reviews/step-55-open-items.md`. On the PR branch `origin/step-55-enum-proofs`: `contracts.md`, `proofs.md` (step 55 section), and the requirements that make up the current rule set: R-0718, R-0981, R-0982, R-0984, R-1040, R-1245, R-1246, R-1249, R-1250, R-1253, R-1256, R-1258, R-1262 to R-1268, and tests T-0306, T-0311, T-0312, T-0314, T-0316, T-0317.

**Design** one state machine for the whole stale-session subsystem:
1. **The signals:** every error or event morm reacts to (`0A000`, each `22P02` kind, `42804`, `XX000`, lost connections and R-0981's listed classes, refused opens and `53300`, migrations and the statement generation, hand-made schema changes, `RECONNECT`).
2. **The state** morm keeps per pool and per connection (markers, generations, salts, cool-down, pending checks, first-failure records, the one-clear window), each with its bound and clearing rule stated once.
3. **One table:** for every signal × context (outside or inside a transaction; with or without `.retry(n)`; straight to Postgres or behind a pooler; within or outside the one-clear window), exactly what morm does, what it returns to the caller, what it may clear or raise, and what it costs.
4. **Guarantees, argued in writing:** (a) every stale session heals within a stated time bound in every row, expressed in the settings; (b) churn is bounded: no client-controlled input (values, stored text, row changes, timing, `search_path`) can cause more than a stated number of cache clears, pool replacements or new connections per unit time; (c) no growth past `max` or the budget; (d) bounded memory; (e) no silent wrong answer except those the README states, listed exactly.
5. **EP1 to EP8** of `step-55-review-12.md` closed inside the design (EP1: fresh connections under the window; EP2: `0A000` from `search_path` with no schema change; EP3 to EP8 as listed there).
6. **A randomised simulation test** (property-based, in outline for the building step): a model of pools, connections, server sessions, PgBouncer routing, migrations, hand-made changes and client inputs, driven by random sequences, asserting the guarantees above. Say which step builds it and how it runs in CI.
7. **The rewrite plan:** which requirements and tests are replaced, merged or removed, and the new requirement and test outline (no new IDs), so one fix agent can rewrite them from this design; and the README notices, each with what it must cover.

Keep every owner decision; where an earlier decision can't be kept exactly in a coherent machine, say so and give the owner a lettered choice with a recommendation. Prefer fewer rules over more. Test every Postgres and PgBouncer claim the design rests on, on throwaway Postgres 17 and 18 and PgBouncer 1.20.1 and 1.26.1 (built from pgbouncer.org's tarballs, checked by SHA-256), with pools larger than 1; you may prototype the simulation in a scratch crate on stable Rust. Cite results; say plainly what you couldn't run.

**Report:** the commit hash, the shape of the machine in brief, any lettered choices with recommendations, what you tested and on which versions, and confirmation that servers are stopped and scratch removed.

## After the design

Bring its choices to the owner and record the answers in INDEX's Decisions. Then one fresh fix agent ("Fix round" brief in `coordinator-briefs.md`) rewrites the requirements and tests on PR #3 from the design's rewrite plan, next free IDs R-1270 and T-0318. Then one fresh review ("Review round" brief), scoped to the changes since `0f26ba7`.
