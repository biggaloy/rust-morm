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

## Findings

### FB1. Should fix. R-0718 (step 14), R-0984 and R-1271 (step 20), R-1268 and R-1272 (step 50): the `search_path` case's bound, "at most once per `stale_pending_expiry`", is stated with no exception for pending entries off, under which about every switch fails

- **Where:**
  - R-1271's bullet "A `search_path` set per transaction": "inside one, without `.retry(n)`, at most one failed transaction per statement per `E`, and one more after each thing that ends its entry sooner". The bullet also says "So one such statement fails about once per `E`".
  - R-0984's bullet "With no `.retry(n)`": "in both modes a statement that fails on a `search_path` switch, R-1271's case, fails one transaction per `stale_pending_expiry`, and one more after each pool-wide action".
  - R-1272, the loud notice beside `tx.set_local`:
    - "Inside one you get `Error::SchemaChanged` at most once per `stale_pending_expiry`, 60 seconds by default, per statement and pool";
    - "What you give up. Without `.retry(n)`, about one failed transaction per statement per `stale_pending_expiry`".
  - R-0718's "Pending entries off" and R-1268's "What `"off"` turns off": both list what turning entries off costs, the second row's bad values included since FA2, but neither names this case.
- **What's wrong:**
  - This is FA2's error, in the first row's case beside it.
  - The bound comes from the pending entry. While an entry lasts, the statement runs unnamed and no switch fails it.
  - With `stale_pending_expiry = "off"` or `"0s"`, or `stale_pending_max = 0`, R-0718's "Entry" makes nothing. Row B1 evicts the statement on that connection, and the next transaction there prepares it again under its own path. The next switch on that connection then fails it again, and the same holds for each connection.
  - Behind PgBouncer, row D1 makes no entry either. Each later transaction's first attempt runs the shared server statement, planned under whichever path last prepared it.
  - So with entries off, a statement under a per-transaction `search_path` fails about every transaction that switches relative to its connection's copy. The bound of once per `E` doesn't hold.
  - T-0319 already has it right: "so with pending entries on it fails at most once per `stale_pending_expiry`". The requirements and the notices don't.
  - Choice 236 says the FA2 fix states the cost "so every place that says what turning entries off does says it". But R-1268's "What `"off"` turns off" still doesn't warn a `search_path`-per-tenant user, who is the reader R-1272's notice is for.
- **Evidence:**
  - My model above: with entries off, 15.7 and 16.7 `Error::SchemaChanged` per second out of 50 transactions a second, against about one per `E` with entries on.
  - On 17.11 and 18.6, a statement closed and prepared again after each failure got `0A000` from `RevalidateCachedQuery` at every one of six switches.
  - From the procedure: B1's "entry, unless ..." and "Entry: ... Nothing, if pending entries are off". Also R-1270's `before_run`, which runs a statement cached when no entry exists.
- **Fix:** no owner choice is needed. This states the decided procedure's cost where it's read.
  - **R-1271's bullet:** say "inside one, without `.retry(n)`, with pending entries on, at most one failed transaction per statement per `E`, ...". Then add: "with pending entries off, no entry is made, so in both modes every transaction whose first attempt meets a copy planned under the other path fails, about every switch, while `.retry(1)`'s next attempt, unnamed, answers".
  - **R-0984's bullet:** add "with pending entries on" to its `search_path` clause, and "about every switch with them off".
  - **R-0718's "Pending entries off":** add "and a statement under a `search_path` set per transaction fails inside a transaction about every time the path switches, in both modes".
  - **R-1272:** "at most once per `stale_pending_expiry` ..., and about every switch with pending checks off, as the notice on the stale-session settings says". Make the same change in "What you give up".
  - **R-1268's "What `"off"` turns off":** add "and a statement under a `search_path` set per transaction, over tables of different shapes, fails inside a transaction about every time the path switches; `.retry(1)` hides it".

### FB2. Minor. R-0718 (step 14), R-1271 and T-0312 (step 20), R-1292 (step 50): the entries-off cost of the second row, and R-1292's new advice, aren't asserted on real Postgres

- **Where:**
  - R-0718's new sentence in "Pending entries off" and R-1271's new sentence in "What the second row costs": "every such call inside a transaction returns `Error::SchemaChanged` in place of its `22P02`, in both modes".
  - R-1292's new paragraph: `.retry(1)`'s next attempt "returns the `22P02` itself, as `Error::Postgres`, which the savepoint then clears, so your closure can catch it and go on".
- **What's wrong:**
  - Both are new rules of a code requirement. R-1292's is the loud notice's way to proceed.
  - Only T-0319's simulation checks them, with I14's new clauses, which have no Postgres.
  - T-0312, which runs the stale rows inside transactions and already has entries-off cases, tests neither:
    - its entries-off case covers only a stale statement behind PgBouncer and straight to Postgres;
    - its `tx.transaction(..)` cases cover only stale statements and functions, never a bad label.
  - So a build that keeps an entry with entries off, or that returns a bad label's `Error::SchemaChanged` from `.retry(1)`'s second attempt inside a savepoint, passes every integration test. `rules.md`: "A requirement that holds several rules needs tests for each of them".
- **Evidence:**
  - T-0312's text has no bad label with `stale_pending_expiry = "off"`, and none inside `tx.transaction(..)`.
  - T-0151 and T-0153 run only at the defaults.
- **Fix:** no owner choice is needed. Add to T-0312, in both modes:
  - **Entries off, every call:** "with `stale_pending_expiry = "off"`, and again with `stale_pending_max = 0`, a cached `SELECT (($1::text)::st)::text` given `zz` in five transactions with no `.retry(n)` gives `Error::SchemaChanged` in each, and under `.retry(1)` its `22P02` on the second attempt, with nothing requested".
  - **The savepoint, on a new pool:** "a closure that tries `zz` inside `tx.transaction(..)` and catches its error gets `Error::SchemaChanged` from the transaction, its outer `INSERT` not committed. With `.retry(1)`, the second attempt's savepoint gets the `22P02` as `Error::Postgres`, the closure catches it and the transaction commits. While the entry lasts, the savepoint catches it at the first attempt".

### FB3. Minor. R-1292 (step 50): the new loud paragraph states the bad enum value's `Error::SchemaChanged` as if it came at every call, against R-1253

- **Where:** R-1292's "A bad enum value inside a savepoint fails the whole transaction too":
  - "so inside a transaction such a value gives `Error::SchemaChanged` with no schema change";
  - "Inside a savepoint that fails the whole transaction, so a savepoint that tries a user's label and catches its error loses the transaction's work".
- **What's wrong:**
  - With pending entries on, the defaults, the first bad value makes the statement's pending entry. While the entry lasts, the statement runs unnamed, and a bad label in a savepoint returns its `22P02` as `Error::Postgres`, which the savepoint clears.
  - R-1253 states this frequency: "about once per `stale_pending_expiry` ... per statement, ..., and at every such call with pending checks off". This round's FA4 tests now assert the same thing at once, unnamed, while the entry lasts.
  - R-1292 states it as unconditional. A reader who tests the pattern sees the savepoint catch the error on the second try. They can take the first failure for a fluke and ship code that loses work about once a minute per statement.
  - The advice is right either way. The statement of what happens isn't.
- **Evidence:** R-1253's FA2 wording; R-0718's "pending entry" bullet; T-0151's and T-0153's new follow-ups; I14's first clause.
- **Fix:** no owner choice is needed. Write: "such a value gives `Error::SchemaChanged` with no schema change: the first time a statement meets one, about once per `stale_pending_expiry` per statement after that, and at every such call with pending checks off, as that notice says; in between, the statement runs unnamed and the `22P02` comes as `Error::Postgres`, which the savepoint clears. So a savepoint that tries a user's label can lose the transaction's work at any call, and your code can't count on either outcome". The way to proceed is unchanged.

### FB4. Minor. `contracts.md`, `Error::SchemaChanged`'s comment: it describes only stale things, and one of its lines breaks its first sentence in two

- **Where:** `contracts.md`'s `Error` enum, the `SchemaChanged` comment, lines 279 and 280:
  - "a stale statement or session inside a transaction or on a held connection, after a type swap;"
  - "inside a transaction it fails the whole transaction, a savepoint's included (R-0441)"
  - "or a column type change, recognised by SQLSTATE ..."
- **What's wrong:**
  - **The broken sentence.** The second line, added by `aac1a51` (the EW round), sits in the middle of the first sentence. As written, "or a column type change" follows R-0441, and the sentence reads as nonsense. Reviews since then haven't raised it.
  - **The definition.** The comment says the variant means a stale statement or session after a type swap or a column type change. FA3 corrected exactly this belief in R-1292: the variant also comes with no schema change at all, from a bad enum value a cached statement meets inside a transaction (R-1245's second row) and from a `search_path` switch (its first).
  - The comment's own signal list ends "22P02 from enum_in ... or from any cached statement", so it contradicts its first sentence.
  - Step 03 freezes this comment as the variant's documentation, which users read in rustdoc.
- **Evidence:** `git log -S"a savepoint's included (R-0441)"` on `contracts.md` gives `aac1a51`. R-1292's opening after FA3 says the variant comes "with no schema change at all".
- **Fix:** no owner choice is needed, and there's no signature change. Rewrite the comment's start: "a stale statement or session inside a transaction or on a held connection, after a type swap or a column type change, or, with no schema change, a cached statement's 0A000 on a `search_path` switch or its 22P02 from enum_in for a bad enum value (R-1245's first and second rows); inside a transaction it fails the whole transaction, a savepoint's included (R-0441); recognised by SQLSTATE ...". The coordinator will want INDEX to list the `contracts.md` change.

### FB5. Minor. R-1245 (step 14): FA1's new sentence still says only the seventh of the `22P02` rows reads the message, but the eighth reads it too

- **Where:** R-1245's "The second row reads no message text": "Of the `22P02` rows, only the seventh reads the message, for the identifier `_morm_retired_` ...; and the parameter test, R-0981's, on the eighth row, alone looks at what the call binds, and only at a message holding `_morm_retired_`".
- **What's wrong:**
  - The eighth row is a `22P02` row: "`22P02` holding `_morm_retired_` that a bound parameter's text holds", recognised by R-0981's parameter test.
  - R-0981 says the test "takes only a `22P02` whose message holds `_morm_retired_`", and R-1270's `on_error` begins `if err is a 22P02 holding _morm_retired_ and param_holds_retired(call)`. So the eighth row reads the message for the same identifier, and the sentence's own end says so.
  - This is FA1's error, an exclusive claim that the table contradicts, carried into the fix's wording, which was the twenty-third review's own recommendation.
  - T-0318's "which of the `22P02` rows only the seventh reads" is right for the matcher, because `on_error` runs the parameter test before `class(..)`, so it can stay.
- **Evidence:** R-1245's table, eighth row; R-0981's "The parameter test"; R-1270's `on_error`.
- **Fix:** no owner choice is needed. Write: "Of the `22P02` rows, only the seventh and the eighth read the message, both only for the identifier `_morm_retired_`, which no translation changes".

### FB6. Minor. T-0151 and T-0153 (step 14): the new head rule contradicts each obligation's own follow-up

- **Where:**
  - T-0151's head: "each case, a statement in one context, outside a transaction or inside one, with one label or stored text and in one language, runs on a new pool". Its end: "and on the pool whose first transaction made that statement's pending entry, ... the same call gets its `22P02` as it is at once, unnamed, inside a transaction with no `.retry(n)` and outside one".
  - T-0153's head: "each case, a statement in one context and with one set of values, runs on a new pool". Its follow-up: "the same call, or one with another bad label, gets its `22P02` as it is at once, unnamed, inside a transaction with no `.retry(n)` and outside one".
- **What's wrong:**
  - Each follow-up runs on the first case's pool, in the other context and, in T-0153, with other values. That's what FA4 asked for, but the head rule, read literally, forbids it.
  - T-0312's head has the carve-out these two lack: "and a case that repeats a call states what follows while the entry its first call made lasts".
  - A step 14 session that follows the head rule runs each follow-up on a new pool, where it gets `Error::SchemaChanged` and not the stated outcome.
- **Evidence:** the two heads and follow-ups quoted above; T-0312's head.
- **Fix:** no owner choice is needed. Add T-0312's clause to both heads: "..., and a case that repeats a call states what follows, on the same pool, while the entry its first call made lasts".

### FB7. Minor. `proofs.md`: this round's model is said to be on a branch that's gone

- **Where:** `proofs.md`, "The answer's model, carried on": "(Rust, kept with its scripts and outputs on `wip/step-55-review-23-fix` under `plans/wip/step-55/`)".
- **What's wrong:**
  - The coordinator copied the model to `plans/models/step-55/` on `main` (`cc49146`) and deleted the branch, as `coordinator-briefs.md` says it does. `git ls-remote origin 'refs/heads/wip/*'` lists nothing.
  - The files this round names are all in `plans/models/step-55/`: `results/fa2.txt`, `fa2breaks.txt`, `probe9-17.txt`, `probe9-18.txt`, `breaks.txt`, `switches.txt` and `probes/run9.sh`.
  - The earlier rounds' entries in `proofs.md` name `plans/models/step-55/` on `main`, and so does the PR body.
- **Evidence:** `git ls-tree origin/main plans/models/step-55/results/` lists the files. `proofs.md`'s entries for the EZ round, the five answers, the three answers and the two answers all name `plans/models/step-55/`.
- **Fix:** write "kept with its scripts and outputs in `plans/models/step-55/` on `main`".

## Summary

| Severity | Findings |
|---|---|
| Blocks approval | None |
| Should fix | FB1 |
| Minor | FB2, FB3, FB4, FB5, FB6, FB7 |

No finding needs an owner choice.

- **FA1 to FA4 are written in as recommended,** read literally, and the procedure is unchanged.
- **The machine isn't in question.** My probes on 17.11 and 18.6 match FA1's sentence on what Postgres translates, and FA3's advice inside a savepoint.
- **Every finding is in the text around the procedure:**
  - **FB1** is FA2's error in the first row's case beside it: the `search_path` bound in R-1271, R-0984 and R-1272, which pending entries off break, and which R-0718 and R-1268 don't mention.
  - **FB3, FB4 and FB5** are the text beside FA3 and FA1 carrying the same kind of error. FB4's broken sentence is older than this range.
  - **FB2** is the tests the new rules lack on real Postgres.
  - **FB6** is FA4's head rule contradicting its follow-ups.
  - **FB7** is a stale pointer.
- **The checks agree.**
  - The changed IDs match INDEX's entry exactly, with 6 requirements and 5 test obligations, and `coverage.md`'s two rows.
  - The checker reports 1294 requirements, 329 test obligations and 0 problems, in both forms.
  - CI is green on `018b3fb`, 18 of 18 jobs on push and on the pull request.
  - The PR body is accurate for this range, choices 235 to 240 included.

**PR #3 isn't ready for the owner yet.** The fixes are all wording, plus two short T-0312 cases, then a check of the edits, the checker and CI.
