# Step 55 independent review, twenty-fifth round

PR #3 (`step-55-enum-proofs`), head `59286dc`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes.

This round covers `git diff 018b3fb origin/step-55-enum-proofs`, six commits: the twenty-fourth review's fix, FB1 to FB7 (`14b1ee0`, `2b49382`, `f413c04`, `b05fea1`, `edb9ab1`), and its `proofs.md` record (`59286dc`). I also checked that nothing earlier regressed on the paths these commits touch.

## What I read

- **On the PR branch:**
  - `rules.md`, `contracts.md`'s `Error::SchemaChanged` comment, and `proofs.md`'s new parts for this round.
  - **Every changed requirement in full:** R-0718 and R-1245 (step 14); R-0984 and R-1271 (step 20); R-1253, R-1268, R-1272 and R-1292 (step 50).
  - **Every changed test obligation in full:** T-0151 and T-0153 (step 14); T-0312 and T-0319 (step 20).
  - **For coherence:** R-0982 and R-1270's procedure (`on_error`, `request`, `take`, `finish_take`), R-1265 and R-1266.
- **On `origin/main`:** `reviews/step-55-review-24.md`, and INDEX's Decisions from "The EZ fix round's five open questions (2026-11-25)" to "The twenty-fourth review's fix (2026-11-26)".
- **The PR body,** read at the head: the twenty-fourth review's section, the changed-IDs paragraph since `018b3fb` and choices 241 to 246.

## What I checked and ran

- **CI on the head.** `59286dc` is green: `ci` on push (run 45820637774) and on the pull request (run 45820651200), 18 of 18 jobs each.
- **The coverage checker,** run on a `git archive` of the head. `check_coverage.py --step 55 --code crates tests benches fuzz .github` reports **1294 requirements, 329 test obligations, 0 problems**, and the plain form gives the same.
- **The manifest against INDEX.** From `018b3fb` to the head:
  - 8 requirements changed, each in `text` and `block`: R-0718, R-0984, R-1245, R-1253, R-1268, R-1271, R-1272 and R-1292.
  - 4 test obligations changed, in `text`: T-0151, T-0153, T-0312 and T-0319.
  - No ID is new or gone. The keys `master_sha256` and `frame` didn't change.
  - That's exactly the list in INDEX's entry "The twenty-fourth review's fix", with `contracts.md`'s comment. `coverage.md` didn't change, and it didn't need to: no changed row's opening changed.
- **FB1 to FB7, each read literally against its recommendation:**
  - **FB1:** written in at all five places, in the review's words: R-1271's bullet with its new entries-off sentence, R-0984, R-0718's "Pending entries off", R-1272's bound and "What you give up", and R-1268's "What `"off"` turns off". The sweep beyond them (choice 241) added "with pending entries on" to the other entry bounds. Two problems are left in the text around it:
    - the `search_path` bound with entries on still doesn't hold as R-1272 states it (FC1);
    - the sweep's new entries-off clauses for stale statements behind PgBouncer leave out the salt (FC2).
    - R-1268's new sentence also took the antecedent of the sentence after it (FC3).
  - **FB2:** T-0312 has both recommended cases: the entries-off bad label with `"off"` and `stale_pending_max = 0`, and the savepoint case on a new pool with its follow-up. It also has FB1's case with entries off on one connection.
  - **FB3:** R-1292 has the recommended sentence. The two additions (choice 244) match R-1253's wording. The way to proceed is unchanged.
  - **FB4:** the comment has the recommended text. Its first sentence reads as one sentence, and it names R-1245's first and second rows. I checked both row numbers against R-1245's table. No signature changed.
  - **FB5:** R-1245 has the recommended sentence, and T-0318 is unchanged, as the review said.
  - **FB6:** T-0151's and T-0153's heads have the clause. Each follow-up now runs on the same pool, as its head allows.
  - **FB7:** `proofs.md` names `plans/models/step-55/` on `main`.
- **Requirement by requirement, for the changed IDs.**
  - These agree with each other in both modes, with entries on and off: what the second row costs in R-0718, R-1271's "What the second row costs", R-1253, R-1268, R-1292, T-0319's I14 and T-0312's new cases.
  - With entries off, they also agree on the `search_path` case's rate and on `.retry(1)`: R-0718, R-0984, R-1271, R-1268, R-1272, I8's new clause and T-0312's new case. I checked this on Postgres and in my own model (below).
  - **R-1270's order matters for T-0312's "where the defaults give one".** In `on_error`, `request(..)`, whose `take` removes every pending entry, runs before `entry(..)`. So the first failure's own clear doesn't remove the entry it makes, and one failure in ten quick transactions is right.
  - T-0319's fault list still counts 39, as its text says.
- **Loud notices and "use this when" lines.**
  - R-1292's paragraph has all four parts. R-1268's "What `"off"` turns off" and R-1272's "What you give up" now name the entries-off `search_path` cost and `.retry(1)`.
  - No opt-in call or setting is new in this range. R-1265's and R-1266's lines still send readers to the notice before they turn pending checks off.
- **The PR body.** The twenty-fourth review's section, the changed-IDs paragraph since `018b3fb` and choices 241 to 246 match the text, the manifest and the INDEX entry, with one exception (FC4).

### Postgres 17.11 (`/opt/pgsql-17`) and 18.6 (`/usr/bin`)

Throwaway clusters under the worktree's `.claude/scratch/`, Unix sockets only under `~/.claude/scratch/`, `TMPDIR` in the scratch, psql 18.6 with `VERBOSITY verbose`. Named statements went through `\parse`, `\bind_named` and `\close_prepared`, unnamed runs through `\bind`. Both versions gave the same output, apart from source line numbers (`plancache.c:753` on 17.11, `:866` on 18.6).

- **FB1's case, T-0312's new test.** Two schemas each hold `items`, `b`'s with an extra column. `SELECT * FROM items WHERE id = $1` was prepared under `a`, then run in ten transactions that each set `search_path` with `SET LOCAL`, alternating, `b` first.
  - **Statement kept,** as a server statement behind PgBouncer is: `0A000` `cached plan must not change result type` from `RevalidateCachedQuery` in each of the five under `b`, and the right row in each of the five under `a`.
  - **Statement closed after each failure and prepared again by the next transaction under its own path,** as row B1's evict does with entries off: the same, five failures and five answers.
  - **An unnamed run under `b` in a new transaction,** as `.retry(1)`'s next attempt runs it, answered with the extra column.
- **FB2's savepoint case.**
  - A statement prepared before the transaction, `SELECT (($1::text)::st)::text`, given `zz` inside `SAVEPOINT`, got `22P02` from `enum_in` (`enum.c:129`). After `ROLLBACK TO SAVEPOINT`, an `INSERT` committed: that's the outcome T-0312 states for an unnamed run while the entry lasts.
  - Five transactions that each prepared it fresh, and an unnamed run, each got the same `22P02`. So with entries off every call reaches the second row again, as T-0312's new case says.

**Not run:** PgBouncer, and Postgres 15 and 16. No finding rests on them. FC1 and FC2 come from morm's procedure, and Postgres's signal is the one shown above.

### My own model

These are two small Python models, written from:

- R-0718's rows B1 and D1, its "Entry" and its "Pending entries off";
- R-1270's `on_error` order: the whole rollback; then, straight to Postgres, the evict and `request(..)`, with the gate's own read of `<n>` when the window is closed; behind PgBouncer, D1's read of `<n>` first; then `entry(..)`;
- R-0982's gate: taken at once when the window is open, otherwise due and taken by the first checkout after the window ends, a take removing every pending entry and raising the generation.

**The first model is sequential:** one transaction at a time, one cached statement.

| Case | Straight to Postgres | Behind PgBouncer |
|---|---|---|
| T-0312's new case: one connection or server session, a copy planned under the first path, ten transactions alternating, the second first (F is `SchemaChanged`) | entries on `F.........`; off `F.F.F.F.F.` | the same |
| Two paths at random, 50 transactions a second, `E` = `W` = 1 s, 40 s, 16 seeds, entries off | 15.7 a second, 31.5% | 23.1 a second, 46.2% |
| The same with `.retry(1)` | 0 | 0 |
| A column change made by hand, every copy stale, transactions only, different values, `W` = 1 s, entries off, window open at the failure | 1 failure | 1 failure |
| The same, but with a window another action opened, which ends 0.5 s after the change | 3, one per connection, as R-1271 says | 16, the last at 0.455 s, then none: the due salt, taken at the window's end, heals it (FC2) |

The entries-off rates match the fix's own (15.6 and 22.3 a second).

**The second model is concurrent.** It's event-driven, one cached statement under a per-transaction `search_path`, two paths at random, on a pool of four. It uses T-0319's load where T-0319 states it: a transaction about every 20 ms, a run of 1 to 10 ms, a read of `<n>` of 1 to 200 ms. I chose 1 to 10 ms myself for the gap before the run and for the rollback. Each `Error::SchemaChanged` is classed by when its run began, against the first failure since the last entry ended:

| Entries on | Per `E` | The first | Runs begun before the first's error | Runs begun after it, before its entry was made | Runs begun before the last entry, in flight |
|---|---|---|---|---|---|
| `E` = `W` = 1 s, straight to Postgres, 16 seeds of 40 s | 2.32 | 0.98 | 0.08 | **1.18** | 0.08 |
| `E` = `W` = 1 s, behind PgBouncer | 2.49 | 0.98 | 0.09 | **1.34** | 0.08 |
| `E` = `W` = 60 s, the defaults, straight to Postgres, 8 seeds of 600 s | 1.57 | 1.00 | 0.05 | **0.47** | 0.05 |
| `E` = `W` = 60 s, behind PgBouncer | 2.46 | 1.00 | 0.10 | **1.29** | 0.07 |
| `E` = `W` = 60 s, reads of 1 to 5 ms, straight to Postgres | 1.16 | 1.00 | 0.05 | 0.03 | 0.09 |
| `E` = `W` = 60 s, reads of 1 to 5 ms, behind PgBouncer | 1.20 | 1.00 | 0.05 | 0.05 | 0.10 |

The fix's own model gives the same order of result: 2.00 and 2.20 a second at `E` = 1 s (`proofs.md`). FC1 rests on this.

## Findings

### FC1. Should fix. R-1271 (step 20), R-1272 (step 50), and R-0984 (step 20): the `search_path` bound with pending entries on excuses only calls "already running when the first failed", and R-1272 excuses none, but the calls that begin before the entry is made fail too, so at the defaults about 1.5 to 2.5 fail per `stale_pending_expiry`, not at most one

- **Where:**
  - R-1271's bullet "A `search_path` set per transaction": "with pending entries on, at most one failed transaction per statement per `E`, and one more after each thing that ends its entry sooner, ..., beyond calls of that statement already running when the first failed".
  - R-1272, the loud notice beside `tx.set_local`: "Inside one you get `Error::SchemaChanged` at most once per `stale_pending_expiry`, 60 seconds by default, per statement and pool". The notice names no exception.
  - R-0984's bullet "With no `.retry(n)`": "with pending entries on, fails one transaction per `stale_pending_expiry`".
- **What's wrong:**
  - **When the entry is made.** R-1270's `on_error` makes the entry only after the whole rollback. Straight to Postgres, when the window is closed, `request(Clear, unread = true)` first reads `<n>`. Behind PgBouncer, row D1 always reads `<n>` first, after the rollback.
  - **What runs meanwhile.** Every run of the statement that begins in that span finds no entry and runs its connection's cached copy, or behind PgBouncer the server statement. Each one under the other path fails.
  - **Those calls weren't running when the first failed.** They began after its error, so R-1271's exception doesn't cover them. R-1272's notice has no exception at all.
  - **The windows are often closed.** At `E` = `W`, another call that fails in that span makes a request the closed window defers. The due action is then taken at the window's end and opens a new window. So in my model, after the first cycle, the next failure usually meets a window that's just been opened, and the span holds a catalog read in most cycles.
  - **The other bounds use another moment.** T-0319's I8 states this same case's bound "beyond calls of it already running when its entry was made". R-1271's own "What the second row costs" and its PgBouncer stale-statement bullet say the same. So R-1271's `search_path` bullet contradicts I8 and its own neighbours.
  - **The fix's own model shows it.** `proofs.md` reports 2.00 a second straight to Postgres and 2.20 behind PgBouncer at `E` = 1 s with entries on, "about once per `E` beside calls already running when the first failed, which the bound leaves out". The bound, as written, doesn't leave those calls out, and the README states "at most once".
  - FB1's fix rewrote exactly these sentences to add "with pending entries on", so the corrected bound is the one now read.
- **Evidence:**
  - My concurrent model above. With the defaults, `E` = `W` = 60 s, per `E`:
    - straight to Postgres, 1.57 `Error::SchemaChanged`, 0.47 of them from runs begun after the first's error and before its entry;
    - behind PgBouncer, 2.46 and 1.29.
  - With 1 to 5 ms reads, 1.16 and 1.20 per `E`. That's still more than one, because calls in flight at the first failure, or at the entry, fail too.
  - In the text: R-1270's `on_error`:

    ```
    rollback(call.tx)
    if !pooled: evict(conn, call.stmt)
    if pooled or err is XX000: mv = read(conn)
    if !mv and err == 0A000: mv = request(pooled ? Salt : Clear, unread = !pooled)
    if !mv: entry(call.stmt, call.bytes, marked = err == 0A000, xx = err is XX000)
    ```

    And `request`'s `if unread and !pooled: mv = read(conn)` when the gate is closed.
  - I8: "so with pending entries on it fails at most once per `stale_pending_expiry`, beyond calls of it already running when its entry was made".
- **Fix:** no owner choice is needed for the wording, which states the decided procedure's cost, as FB1 did.
  - **R-1271's bullet:** replace "beyond calls of that statement already running when the first failed" with "beyond calls of that statement that begin before its entry is made, which comes after the whole rollback and, behind PgBouncer or with the window closed, after a catalog read, so at a high rate of such transactions a few more fail with each entry".
  - **R-1272:**
    - Replace "at most once per `stale_pending_expiry`" with "about once per `stale_pending_expiry`, 60 seconds by default, per statement and pool, plus the transactions that start while morm checks the schema's state after that failure, before it marks the statement pending, a few more at a high rate".
    - Make the same change in "Each failure asks for one repair, once" and in "What you give up".
  - **R-0984:** "fails about one transaction per `stale_pending_expiry`, beyond calls that begin before its entry is made".
  - **T-0312 needs no change:** its alternations run one after another.
  - **If the owner prefers to keep "at most once" true,** that's a procedure change. For example, make the entry before D1's and the gate's read, and drop it if the read finds `<n>` moved. That needs the owner's choice. I recommend the wording, because the extra failures are few, `.retry(1)` hides them, and the read's result decides the entry.

### FC2. Minor. R-1271 and R-0984 (step 20), R-1253 (step 50), and the older R-0718 (step 14) and R-1268 (step 50): with pending entries off, a stale statement behind PgBouncer is said to fail every transaction's first attempt until a call outside a transaction, `RECONNECT` or `server_lifetime`, but a salt ends it too, and a `0A000` requests one at the failure

- **Where:**
  - **R-1271's bullet "Stale statements behind PgBouncer":**
    - "with them off every transaction's first attempt that runs it";
    - "with pending entries off it returns `Error::SchemaChanged` on every transaction's first attempt that runs it, whatever its values, until the same things end it", those being "it runs outside a transaction, `RECONNECT`, or `L`".
  - **R-0984:** "and with them off on every transaction's first attempt that runs it, ..., until it runs outside a transaction or PgBouncer replaces the server connection through `RECONNECT` or `server_lifetime`".
  - **R-1253:** "and there on every transaction's first attempt that runs the statement with pending checks off".
  - **Older text that the new clauses point to:**
    - R-0718's "Pending entries off": "D1 returns `Error::SchemaChanged` on every transaction's first attempt that runs the statement, until a call outside a transaction confirms it, C1, or PgBouncer replaces the server connection";
    - R-1268: "returns `Error::SchemaChanged` on every transaction's first attempt, until a call outside a transaction repairs it, PgBouncer gets `RECONNECT`, or its `server_lifetime` passes".
- **What's wrong:**
  - **A column change asks for its own salt.** R-0718's row D1 has "`0A000` requests *salt*, once, at the failure", and R-1270's `on_error` makes that request whether or not pending entries are on (`if !mv and err == 0A000: mv = request(..)`, before `entry(..)`, which does nothing with entries off).
  - **So it heals.** A statement a column change made by hand left stale gets `0A000` at D1, and the salt comes at once, or at the first checkout after the window ends. The fresh text is prepared under the current schema, and it heals within `W`, with entries on or off.
  - **Any statement heals at any salt.** A salt another statement's request brings moves every statement to fresh text, so it ends the case for `22P02` and `XX000` too.
  - **The bullet contradicts itself.** Its first sentence says "the salt, or an adoption in its place, comes within `W` of the first request a confirmation or a `0A000` makes after the change". T-0312's D1 case says "the same column change ... requests the salt at once".
  - **Only the case T-0312 tests behaves as stated.** Its entries-off case is a swap that leaves a statement on another enum, "with no salt". There, nothing but those three things ends it.
  - Read as an upper bound, the guarantee isn't broken. But the README notice R-1253, and R-1268's "What `"off"` turns off", state it as what happens, and tell a reader that a column change heals only at `RECONNECT`.
  - The sweep of choice 241 wrote the new clauses in R-1271, R-0984 and R-1253 in this range.
- **Evidence:** my sequential model above. With entries off, behind PgBouncer:
  - a column change gives 1 failure with the window open;
  - it gives 16 failures when another action's window is still running, ending 0.5 s after the change: the last at 0.455 s, and then none once the due salt is taken at the window's end.
- **Fix:** no owner choice is needed.
  - **R-1271:** write "with pending entries off it returns `Error::SchemaChanged` on every transaction's first attempt that runs it, whatever its values, until the next salt or adoption, which a `0A000` requests at its failure, so a column change heals within `W`, or until it runs outside a transaction, `RECONNECT`, or `L`".
  - Make the same change in R-0984, in R-1253's clause ("until the next repair, which a column change asks for itself"), and in R-0718's and R-1268's sentences.

### FC3. Minor. R-1268 (step 50): FB1's new sentence took the antecedent of the sentence after it

- **Where:** R-1268's "What `"off"` turns off".
  - **At `018b3fb`:** "Straight to Postgres a statement a change made by hand left stale still heals, as each connection's copy fails once. Behind PgBouncer such a statement, run inside transactions, returns `Error::SchemaChanged` on every transaction's first attempt, ... and while an old type is kept, a value both types share can run on the old type's order meanwhile".
  - **Now:** the `search_path` sentence stands between the two: "And a statement under a `search_path` set per transaction, ..., fails inside a transaction about every time the path switches, ...; `.retry(1)` hides it. Behind PgBouncer such a statement, ...".
- **What's wrong:**
  - "Such a statement" now reads as the `search_path` statement. For that statement, "until a call outside a transaction repairs it" is false: nothing repairs a path switch.
  - The end of the sentence, "while an old type is kept", only makes sense for the stale statement two sentences back.
  - This is a loud notice, and it's what a reader checks before turning pending checks off.
- **Fix:** move the `search_path` sentence after the PgBouncer one, or write "Behind PgBouncer a statement a change made by hand left stale, run inside transactions, ...". Fold in FC2's change to the same sentence.

### FC4. Minor. The PR body: FB1's bullet says R-1272's "Each failure asks for one repair" bullet gained "about every switch with pending checks off", but it gained only "with pending checks on"

- **Where:** the PR body, "The twenty-fourth review, since `018b3fb`", FB1's "The review's places": "R-1272's notice beside `tx.set_local` adds "about every switch with pending checks off" to its bound, its "Each failure asks for one repair" bullet and "What you give up"".
- **What's wrong:**
  - R-1272's "Each failure asks for one repair, once" bullet now reads "So, with pending checks on, one such statement fails ..." and "When more than one statement switches, with them on, ...". It doesn't say "about every switch".
  - Choice 241 describes it rightly, as one of the places that now say "with pending entries on".
- **Fix:** write "adds "about every switch with pending checks off" to its bound and "What you give up", and "with pending checks on" to its "Each failure asks for one repair" bullet". Read the body again after FC1 to FC3's fix.

## Summary

| Severity | Findings |
|---|---|
| Blocks approval | None |
| Should fix | FC1 |
| Minor | FC2, FC3, FC4 |

No finding needs an owner choice. FC1 has an optional procedure alternative, which I don't recommend.

- **FB1 to FB7 are written in as recommended,** read literally, and the procedure is unchanged.
- **The new entries-off claims hold.** The `search_path` case with entries off matches my probes on 17.11 and 18.6, and both of my models: five failures in ten alternating transactions, about every switch at random, none under `.retry(1)`. So do the second row's cost and T-0312's new savepoint cases.
- **Every finding is in the text around the fix:**
  - **FC1** is the `search_path` bound with entries on. FB1 kept it and qualified it, but R-1271 excuses the wrong calls and R-1272 excuses none. The fix's model and mine both measure more than one failure per `E`.
  - **FC2** is the new entries-off clauses behind PgBouncer, which leave out the salt that a `0A000` requests.
  - **FC3** is a broken antecedent in R-1268.
  - **FC4** is one misdescription in the PR body.
- **The checks agree.**
  - The changed IDs match INDEX's entry exactly, with 8 requirements, 4 test obligations and `contracts.md`'s comment. `coverage.md` needed no change.
  - The checker reports 1294 requirements, 329 test obligations and 0 problems, in both forms.
  - CI is green on `59286dc`, 18 of 18 jobs on push and on the pull request.
  - The PR body is accurate for this range apart from FC4, choices 241 to 246 included.

**PR #3 isn't ready for the owner yet.** The fixes are wording only, then a check of the edits, the checker and CI.
