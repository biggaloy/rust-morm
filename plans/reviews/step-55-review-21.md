# Step 55 independent review, twenty-first round

PR #3 (`step-55-enum-proofs`), head `f25aefe`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes.

This round covers `git diff 5d111d3 origin/step-55-enum-proofs`, two commits: `bbf95e3`, the fix for the twentieth review's EX1 to EX6, and `f25aefe`, which records it in `proofs.md`. I also checked that nothing earlier regressed.

## What I read

- **On the PR branch:**
  - `rules.md`, `contracts.md`, and the new round of `proofs.md` in full.
  - **Every changed ID in full:** R-0981 and R-1270 (step 14), R-1271 and T-0319 (step 20), and R-1293 (step 50).
  - **For coherence:** R-1245, R-0698 (step 12), and R-1270's procedure: `request`, `gate_open`, `take`, `finish_take`, `on_error`, `session`, `read` and `on_moved`.
- **On `origin/main`:**
  - `reviews/step-55-review-20.md`.
  - INDEX's Decisions entry "Twentieth review fixed (EX1 to EX6)" and the entries it builds on.
  - `reviews/stale-session-state-machine.md`'s "Silent wrong answers, exactly".
- **The PR body,** at the head.

## What I checked and ran

- **CI on the head.** `f25aefe` is green: `ci` on push (run 45562664777) and on the pull request (run 45562676141), 18 of 18 jobs each.
- **The coverage checker,** run on a `git archive` of the head. `check_coverage.py --step 55 --code crates tests benches fuzz .github` reports **1293 requirements, 329 test obligations, 0 problems**, and the plain form gives the same. That's the expected total.
- **The manifest against INDEX.** From `5d111d3` to the head, the `text` and `block` of R-0981, R-1270, R-1271 and R-1293 changed, and the `text` of T-0319. That's exactly the EX entry's list, and nothing else moved.
- **Each fix against its finding,** word by word against the step files at `5d111d3`:
  - **EX1:** I10's snapshot clause is written in, and EV2's, EW1's and EW2's switches name it. EW2's setting now adds up: 40% held, 36% in morm's transactions, 24% outside one. The load is stated in full. The floor sentence gives 50 of 256, and the step's PR measures over 2,048 with 240 as the stop.
  - **EX2:** R-1293 and R-1271 now say release and take a new connection on a stale function's errors, and roll back and run again on a stale statement's.
  - **EX3:** both places say a begun take is finished under the step's starting `<n>`, and the step's own take uses the `<n>` after any move. That matches `take` (`last_read_n`) and `finish_take` (`n0`).
  - **EX4:** R-1271 has its own held-connection bullet, and the stale-session bullet names row A4 as its exception. I8 excepts held connections. See EY1.
  - **EX5:** a fault file holds the event list itself, replayed with no strategy and no random source. The main test's regression file is a proptest seed, which is what proptest writes.
  - **EX6:** R-0981's wording matches R-1271's savepoint bullet and R-1270's paragraph on savepoints.
- **R-1270 is still one consolidated section.** The only edits are the step-start rule's two sentences, and they agree with the procedure.
- **The five restated settings in choice 159,** checked against the procedure:
  - **No replacement:** straight to Postgres, `on_moved` requests nothing, and a `22P02` holding `_morm_retired_` isn't one of the `sure` signals. So with this switch, nothing heals a stale session, and with the design, A2's confirmation requests *replace*.
  - **EV1's:** a lock wait of 20 ms widens the gap between the checkout's compare-and-swap and `finish_take`. With every call outside a transaction, runs stay cached instead of going unnamed through pending entries.
  - **Two momentary extras:** the stored-text toggles now reach the function that's called.
  - **A confirming run removing the entry:** one recurring value gives same-bytes confirming runs.
  - **No starting bit:** a landing at every eligible compare-and-swap.

  Each setting gives its switch the path it needs. Choice 158's measurement in the step's own simulation, a fault under 240 of 2,048 stopping the step for the owner, means CI's reliability doesn't rest on `proofs.md`'s model alone. So I didn't rebuild a concurrent model this round. The previous round's model went with its scratch.
- **Postgres 17.11 (`/opt/pgsql-17`) and 18.6 (the Manjaro package `18.6-1`).** Throwaway clusters under my worktree's `.claude/scratch/`, Unix sockets only (`listen_addresses = ''`), sockets under `~/.claude/scratch/`, `TMPDIR` in the scratch, psql 18.6 with `VERBOSITY=verbose`. Both versions gave the same results:
  - **A stale function after a hand-made swap that keeps the old type without a relabel (EY2).** Session A compiled `f(t text)`, a plpgsql function with `DECLARE v st` that assigns `t` to `v`, by calling `f('a')`. Session B ran `ALTER TYPE st RENAME TO st_old; CREATE TYPE st AS ENUM ('a','b','c')`.
    - Session A's `f('c')` then got `22P02` `invalid input value for enum st_old: "c"`, outside a transaction, in a REPEATABLE READ transaction, and again in a second one after `ROLLBACK`.
    - `f('a')` answered `a`.
    - A new session's `f('c')` answered `c`.
  - **A Close inside a failed transaction.** psql's `\parse` and `\bind_named` prepared and ran `SELECT c FROM tp`, and another session changed `tp.c` to `bigint`. In a REPEATABLE READ transaction the bind got `0A000`. `\close_prepared` there went through, though the `SELECT 1` after it got `25P02`. After `ROLLBACK`, `pg_prepared_statements` was empty and the bind got `26000`. So morm can drop a stale statement inside the app's failed transaction, which R-1293's "roll back and run again on the same connection" for `0A000` depends on.
- **Not run:** PgBouncer, because no finding rests on it. The servers are stopped, and the clusters, sockets and scratch are deleted.

## Findings

### EY1. Should fix. T-0319 (step 20), R-1271: I8's new held-connection check can't fail, so nothing asserts R-1271's held-connection bound any more

- **Where:**
  - T-0319's I8: "held connections apart, which R-1271's bullet on held connections bounds instead: a held connection's stale session ends at its release, no call running on that session after it".
  - R-1271's held bullet: "On a connection step 14's epoch test finds old, row A4, a stale session returns `Error::SchemaChanged` at each call that meets it until the connection is released, from `t0 + W` on too ... On one the test finds not old, its error returns as it is, before `t0 + W` only ... These are the only visible cases on a held connection".
  - R-1271's opening: "this step's simulation asserts them".
  - R-0698: "Held connections are closed on release, never reused."
- **What's wrong:**
  - **The check holds for any machine.** R-0698 closes every held connection on release, so a held session always ends at its release. That's the pool's code, not the stale-session machine.
  - **What a broken machine would pass.** Say a machine returns a held connection's stale-session error as it is after `t0 + W`, because it skips row A4 or never judges a held connection old. It passes I8 as now written.
  - **Before EX4, I8 caught that,** because it counted a held connection as outside a transaction. That also flagged row A4's correct `Error::SchemaChanged`, which was EX4's point. The fix, as the twentieth review recommended, removed the check instead of restating it for held connections.
  - **Nothing else covers it.** I7 also excepts held connections inside the app's transaction. So the held bullet's list of "the only visible cases" has no invariant behind it.
- **Evidence:** the texts above. `replace_on_release` only counts the replacement for the epoch, and the release closes the connection either way.
- **Fix:** no owner choice is needed. Make I8's held-connection clause R-1271's held bullet:
  - **From the last change, or the end of the last refusal, plus `stale_cooldown`:** a call on a held connection that meets a stale session gets `Error::SchemaChanged`, never the error as it is. The one exception is inside a transaction the app opened: a cached statement's `XX000` can come back as it is, at most once per statement per connection.
  - **Before then,** the error comes back as it is only on a connection R-0981's test found not old.
  - **At release,** a held connection that met a stale session counts as its epoch's replacement, and the next `db.hold()` is a new session.
  - **Record the check in `proofs.md`:** run it at the held settings with no fault. Also run, as a fault form, row A4 returning the error as it is, to show the clause catches it.

### EY2. Should fix, owner choice. R-1293 (step 50), R-1271 (step 20): a stale function's `22P02` after a hand-made swap that keeps the old type without a relabel names the kept type, not `_morm_retired_`. It repeats on its session, which R-1293's advice and R-1271's bound both miss

- **Where:**
  - R-1293's how to proceed: "on `0A000`, or any other `22P02`, roll back and run your transaction again on the same connection, because morm has already dropped the stale statement".
  - R-1271's stale-session bullet: "from `t0 + W` on, never visible outside a transaction or inside a savepoint, a held connection's row A4 apart".
  - R-1271's silent wrong answers: a hand-made swap that keeps the old type without renaming its labels lets plpgsql compiled before it convert a label both types share "by the old type's order, with no error".
  - R-1245: only a `22P02` holding `_morm_retired_` is session class. A `22P02` from a statement that casts no parameter to a non-built-in type isn't a signal at all.
- **What's wrong:**
  - **The loud half of the case is missing.** The plan covers the silent half: a label both types share. A label only the new type has fails inside the stale function with `22P02` naming the kept type, here `st_old`. That error comes from compiled code in the session, so it repeats on that session in every transaction and outside one, and a new session answers.
  - **On a held connection,** R-1293 sends that error down the "any other `22P02`" branch: roll back and run again on the same connection. That fails the same way every time.
  - **On a pooled connection,** R-1245 doesn't recognise the error, so morm returns it as it is at every call that meets it. That lasts until the connection is replaced for some other reason, long after `t0 + W`. A fresh session wouldn't return it, so by R-1271's own definition it's a visible stale session outside a transaction. That contradicts the stale-session bullet.
  - **The model left it out.** `proofs.md`'s model describes a kept swap without a relabel only for casts: "gives a cast a label only the new type has `22P02` not naming the cast's type". So the "fails nothing" result doesn't cover it.
  - **Scope.** R-1293's wording is new this round. R-1271's gap is older, but R-1271 is a changed ID here.
- **Evidence:** Postgres 17.11 and 18.6, above. `f('c')` gave `22P02` `invalid input value for enum st_old: "c"` outside a transaction and in two REPEATABLE READ transactions in turn, on the session that compiled `f` before the swap. A new session answered `c`.
- **Options:**
  - **(a) State it and steer users to the handled form.**
    - **R-1271** lists the case as a visible one: a stale function's `22P02` naming the kept type after a hand-made swap that keeps the old type without a relabel. It lasts until the session ends: straight to Postgres, until the connection is replaced; behind PgBouncer, until `RECONNECT` or `L`. The stale-session bullet names it as an exception.
    - **The README** gets a loud notice beside changes made outside morm:
      - *What it does:* the case above.
      - *When it's right:* a swap that has to keep the old type's labels as they are.
      - *What you give up:* those calls fail until each session ends.
      - *How to proceed:* relabel the kept type as morm's cutover does (`_morm_retired_<migration>_<type>` and `_morm_retired_` labels), which turns it into the case morm already handles; or end the sessions after the swap (restart the process, or `RECONNECT` behind PgBouncer).
    - **R-1293's release branch** adds a `22P02` that names a type other than the one the call passed, after a schema change.
    - **T-0319's I8 and I9,** and the model, cover it as stated.
  - **(b) Make morm cure it.** R-1245 widens the session class to any `22P02` from `enum_in` that isn't on its other rows. On an old connection it gets row A2's run on a new connection, and a confirmation requests *replace*.
    - **Cost:** a client's own bad label converted to an enum costs up to the connections open at an epoch's start, per epoch, as stored `_morm_retired_` text does now.
    - **Work:** R-0981, R-1245, R-1270's procedure, T-0151, T-0318 and T-0319's load and fault list all change.
  - **(c) Refuse the form.** Say hand-made swaps must relabel the kept type, and leave the case out of every bound. The README notice states that, with the four parts.
- **Recommendation: (a).**
  - **morm can't tell this error from a client's bad label** without a new session. (b) would make client input churn connections for a case only a hand-made swap causes.
  - **A hand-made swap that relabels is already handled** end to end. Steering users there costs one notice.
  - **(c) leaves a bound that a supported command can still break.**

### EY3. Minor. PR body: the EX1 section says four settings are restated, and choice 159 says five plus a widened invariant name

- **Where:**
  - The PR body's EX1 bullet: "Four named settings were too weak in the model as T-0319 stated them, and are stated to what catches (choice 159)", listing four.
  - Choice 159: "Five named settings are restated to what catches, and one invariant name widened".
- **What's wrong:** both are true as far as they go. The fifth setting, no starting bit, caught 40 of 256 at the defaults, which is above the old floor of 30 but below the new floor of 50. The EX1 section omits it and the no-stale-lock switch's I10, so its count disagrees with choice 159's.
- **Fix:** no owner choice is needed. In the EX1 bullet, add: "and no starting bit gets a landing at every eligible compare-and-swap (40 of 256 at the defaults, 60 at the new setting), to clear the new floor; no stale lock also names I10".

## Summary

| Severity | Findings |
|---|---|
| Blocks approval | None |
| Should fix | EY1, EY2 |
| Minor | EY3 |

EY2 needs an owner choice: (a), (b) or (c), with (a) recommended. EY1 and EY3 don't.

- **EX1 to EX6 are written in as the twentieth review recommended.** EX4's own recommendation left I8 with a check that can't fail (EY1).
- **R-1270 is still one consolidated, self-consistent section.**
- **The checks agree.** The changed IDs match the INDEX entry ID for ID. The checker reports 1293 requirements, 329 test obligations and 0 problems. CI is green on `f25aefe`, 18 of 18 jobs on push and on the pull request.
- **R-1293 has every part of a loud notice:** what it does, when it's right, what you give up and how to proceed. Its release and rerun lists match Postgres for every case the plan models: a stale function's `XX000`, `42804` and `_morm_retired_` `22P02` repeat on their session, and a stale statement's `0A000` heals once it's closed, which works inside the failed transaction. The case that doesn't match is EY2's.
- **T-0319's switch settings** each give their switch its path, and choice 158 makes the step's own simulation prove each rate before merge. The fault-file form can be replayed as stated: every random choice is written out, and the replay uses no random source.
- **The PR body is accurate for this range,** apart from EY3.

**PR #3 isn't ready for the owner yet.** EY1 is a test-wording fix. EY2 needs the owner's choice, then a short check of the edits, the checker and CI.
