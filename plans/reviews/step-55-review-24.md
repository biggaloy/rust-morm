# Step 55 independent review, twenty-fourth round

PR #3 (`step-55-enum-proofs`), head `018b3fb`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes.

This round covers `git diff e417bc8 origin/step-55-enum-proofs`, six commits: the twenty-third review's fix, FA1 to FA4 (`2576e35`, `0eafc9e`, `155b350`, `3f8814a`, `5c91e7c`) and its `proofs.md` record (`018b3fb`). I also checked that nothing earlier regressed on the paths these commits touch.

## What I read

- **On the PR branch:**
  - `rules.md` in full, and `contracts.md`'s `Tx::transaction` and `Error::SchemaChanged` comments.
  - `proofs.md`'s new part for this round, and the parts before it that it carries on.
  - **Every changed requirement in full:** R-0718 with its rows and "Pending entries off", R-1245 with its table and paragraphs (step 14); R-1271 in full (step 20); R-1253, R-1268 and R-1292 (step 50).
  - **Every changed test obligation in full:** T-0151, T-0153 and T-0318 (step 14); T-0312 and T-0319 (step 20).
  - **For coherence:** R-0981's parameter test and R-1270's `on_error` (step 14); R-0984 (step 20); R-1249, R-1265, R-1266, R-1272, R-1293 and R-1294 (step 50); T-0306, T-0311, T-0314 and T-0317.
- **On `origin/main`:** `reviews/step-55-review-23.md`, and INDEX's Decisions from "The EZ fix round's five open questions (2026-11-25)" to "The twenty-third review's fix (2026-11-26)".
- **The PR body,** read at the head: the twenty-third review's section, the changed-IDs paragraphs and choices 235 to 240.

## What I checked and ran

- **CI on the head.** `018b3fb` is green: `ci` on push (run 45812968986) and on the pull request (run 45812979304), 18 of 18 jobs each.
- **The coverage checker,** run on a `git archive` of the head. `check_coverage.py --step 55 --code crates tests benches fuzz .github` reports **1294 requirements, 329 test obligations, 0 problems**, and the plain form gives the same.
- **The manifest against INDEX.** From `e417bc8` to the head, 6 requirements changed (R-0718, R-1245, R-1253, R-1268, R-1271, R-1292), each in `text` and `block`, and 5 test obligations changed (T-0151, T-0153, T-0312, T-0318, T-0319).
  - No ID is new or gone. The keys `master_sha256` and `frame` didn't change.
  - That's exactly the list in INDEX's entry "The twenty-third review's fix".
  - `contracts.md` didn't change. `coverage.md` changed only in the rows for T-0151 and T-0153, which now begin with their new openings, as the entry says.
- **FA1 to FA4, each read literally against its recommendation:**
  - **FA1:** R-1245 and T-0318 have the recommended sentences, and R-1245 adds what the `XX000` rows and the English second signals read. The new sentence still says "only the seventh" reads the message, against the eighth row (FB5).
  - **FA2:** R-1253, R-1271's two bullets, I9 and R-1268 have the recommended wording, and R-0718's "Pending entries off" was added. The fix covered the second row only. The first row's `search_path` case carries the same unqualified bound in four places (FB1).
  - **FA3:** R-1292's opening names the bad enum value, and it has a new loud paragraph with the recommended advice. Two problems are left. The paragraph states the cost as if it came at every call, against R-1253 (FB3). The contract's comment on `Error::SchemaChanged` still describes only stale things (FB4).
  - **FA4:** T-0151, T-0153 and T-0312 now run each case on a new pool and state what follows while the entry lasts. T-0312 has the cases it repeats marked. T-0151's and T-0153's head rules contradict their own follow-ups (FB6).
- **Requirement by requirement, for the changed IDs:**
  - With pending entries on, these agree on what the second row costs in each context, in both modes and inside a savepoint: R-0718's rows B1 and D1, "Entry" and "Pending entries off"; R-1245's rows and order; R-1270's `on_error` and `before_run`; R-1271's bullets; T-0319's I9 and I14; and R-1253, R-1268 and R-1292.
  - **With entries off,** the second row's cost is now stated alike in R-0718, R-1271, R-1253 and R-1268, and T-0319 checks it (I14's new clauses, in its "pending entries off" setting). The load includes `.retry(0)` transactions, so the clause is exercised. No test on real Postgres asserts it, nor R-1292's new advice (FB2).
  - T-0319's fault list still counts 39, as its text says.
- **Loud notices and "use this when" lines.**
  - R-1292's new paragraph has all four parts: what morm does, what you give up (the transaction's work), when it's right and how to proceed.
  - R-1268's "The largest cost" says what turning pending checks off gives up and how `.retry(1)` gets the `22P02` back.
  - No opt-in call or setting is new in this range, so no "use this when" line is due. R-1265's and R-1266's lines still send readers to the notice before they turn checks off.
- **The PR body.** The twenty-third review's section, the changed-IDs paragraph since `e417bc8` and choices 235 to 240 match the text, the manifest and the INDEX entry. The one exception is the model's location: the body names `plans/models/step-55/` on `main`, which is right, and `proofs.md` names a branch that's gone (FB7).

### Postgres 17.11 (`/opt/pgsql-17`) and 18.6 (`/usr/bin`)

Throwaway clusters under the worktree's `.claude/scratch/`, Unix sockets only under `~/.claude/scratch/`, `TMPDIR` in the scratch, psql 18.6 with `VERBOSITY verbose`. Named statements went through `\parse` and `\bind_named`, unnamed runs through `\bind`. Both versions gave the same results, apart from line numbers.

- **FA3's advice:** inside `BEGIN`, a savepoint ran a statement prepared before the transaction, `SELECT (($1::text)::st)::text`, with `zz`. It got `22P02` `invalid input value for enum st: "zz"` from `enum_in` (`enum.c:129`).
  - After `ROLLBACK TO SAVEPOINT`, an unnamed run of the same text in a second savepoint, as `.retry(1)`'s next attempt runs it, got the same `22P02`. That savepoint was rolled back too, the transaction inserted again, and it committed both rows.
  - Without the rollback to the savepoint, the next statement got `25P02`.
- **FA2's "at every such call" straight to Postgres:** three rounds of closing the named statement, preparing it again and running it with `zz` each gave the same `22P02` from `enum_in`. Bad stored text gave the same with a cached `SELECT (c::st)::text FROM s WHERE id = $1` and with an unnamed run of it.
- **FA1's sentence on what is translated,** 18.6 with `lc_messages` set per session to `de_DE.utf8` and to `es_ES.utf8`:
  - **Translated, with the routine unchanged:** the enum message (`ungültiger Eingabewert für Enum e2: »zz«`, `la sintaxis de entrada no es válida para el enum e2: «zz»`, routine `enum_in`), the same message naming the kept type after a hand-made swap that keeps it, and `0A000` (`gecachter Plan darf den Ergebnistyp nicht ändern`, `el plan almacenado no debe cambiar el tipo de resultado`, routine `RevalidateCachedQuery`). So the `0A000` English second signal is only an English message, as the sentence says.
  - **Untranslated in both languages:** the `XX000` `cache lookup failed for type <oid>` after the kept type was dropped, routine `getTypeOutputInfo`, an `elog`.
  - **17.11:** the `/opt` build has no message catalogs, so it answered in English, with the same routines.
- **FB1's `search_path` case:** two schemas with tables of different shapes, and a named `SELECT * FROM items WHERE id = $1` run in transactions that each set `search_path` with `SET LOCAL`. Six alternations, closing and preparing again after each failure as row B1's evict does with pending entries off, gave:
  - `0A000` `cached plan must not change result type` from `RevalidateCachedQuery` at every switch;
  - an answer at each run prepared again under the new path.

  So an evict and a new prepare cure a switch only until the path switches back.

**Not run:** PgBouncer, and Postgres 15 and 16. No finding rests on them: FB1's frequency comes from morm's procedure, and the signal from Postgres is the same.

### My own model

A small Python model, written from R-0718's row B1, its "Entry" and its "Pending entries off", from R-1245's rule that a `0A000` requests *clear* once, at the failure, and from R-1271's gate: one pool-wide action per `W`, a due action taken by the first checkout after the window ends, and an action removing every pending entry.

It covers one cached statement straight to Postgres under a per-transaction `search_path`, two paths taken at random, on a pool of four connections, with a transaction with no `.retry(n)` about every 20 ms and no migrations. It counts `Error::SchemaChanged`:

| Setting | Runs | `SchemaChanged` |
|---|---|---|
| Entries on, `E` = `W` = 1 s | 64 of 20 s | 0.94 per `E`, at most 0.95 |
| Entries off, `W` = 1 s | 64 of 20 s | 15.7 per second, at most 17.1 |
| Entries on, `E` = `W` = 60 s, the defaults | 16 of 600 s | 1.00 per `E` |
| Entries off, `W` = 60 s | 16 of 600 s | 16.7 per second, about 1,000 per 60 s |

With entries on, R-1271's "at most one failed transaction per statement per `E`" holds. With entries off, about a third of the 50 transactions a second fail. FB1 rests on this result, and the procedure gives it directly: B1 evicts and makes no entry, so the next transaction on that connection prepares the statement under its own path, and the next switch fails again.
