# Step 55 independent review, twentieth round

PR #3 (`step-55-enum-proofs`), head `5d111d3`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes.

This round covers `git diff a365aa5 origin/step-55-enum-proofs`, ten commits in two parts:

- **The nineteenth review's fix, `aac1a51` to `81f53c3`:** EW1 and EW2 as the owner decided (option (a) each), EW3 to EW7, the held connection's `XX000` inside the app's own transaction, and the new loud notices R-1292 and R-1293.
- **The owner's B4 decision, `6c22bc6` to `5d111d3`:** option (b). Inside a savepoint rows B3 and B4 make no read of `<n>`, `old` judges by the last-read `<n>`, and B4's stale-session error stays catchable there. It adds T-0319's I11 and a 31st fault switch.

I also checked that nothing earlier regressed.

## What I read

- **On the PR branch:**
  - `rules.md`, `contracts.md`, and the two new rounds of `proofs.md` in full.
  - **Every changed ID in full.** Step 14: R-0718, R-0981, R-0982, R-1245, R-1246, R-1270 and T-0318. Step 20: R-0436, R-0441, R-0984, R-1271, T-0312 and T-0319. Step 50: R-1253, R-1292 and R-1293.
  - **For coherence:** R-0438, R-0442, R-0983, R-0985, R-1262, R-1289, `contracts.md`'s `Conn`, `Tx` and `Error` blocks, and step 12's R-0698.
- **On `origin/main`:**
  - `reviews/step-55-review-19.md`.
  - INDEX's Decisions entries "`SchemaChanged` fails the whole transaction; held connections never read `<n>` (EW1, EW2)" and "A stale-session error inside a savepoint stays catchable (B4)".
  - The entries those build on, from "The stale-session machine runs its steps under a short pool lock (EU1)" to "No `<n>` read inside a transaction; a failed start read fails the pool (EV2, EV4)".
- **The PR body,** at the head.

## What I checked and ran

- **CI on the head.** `5d111d3` is green: `ci` on push (run 45550919857) and on the pull request (run 45550928623), 18 of 18 jobs each. The `wip/step-55-b4` run on the same commit (45550875919) is green too, and that branch is deleted.
- **Coverage checker,** run on a `git archive` of the head. `check_coverage.py --step 55 --code crates tests benches fuzz .github` reports **1293 requirements, 329 test obligations, 0 problems**, and the plain form the same. That's the expected total.
- **Manifest against INDEX.** I compared the manifests at `a365aa5`, `81f53c3` and the head, ID by ID:
  - **`a365aa5` to `81f53c3`:** R-0436, R-0441, R-0718, R-0981, R-0982, R-0984, R-1245, R-1246, R-1253, R-1270 and R-1271 (`text` and `block`), T-0312, T-0318 and T-0319 (`text`), and new R-1292 and R-1293. That's exactly the EW entry's list.
  - **`81f53c3` to the head:** R-0441, R-0981, R-0984, R-1270, R-1271 and R-1292 (`text` and `block`), and T-0312, T-0318 and T-0319 (`text`). That's exactly the B4 entry's list.
  - **Nothing else moved,** and `frame` and `master_sha256` are unchanged.

  Every changed `block` appears word for word in its step file, and none holds a second `**R-nnnn**` marker. The PR body's two "Since" sentences name the same IDs.
- **Postgres 17.11 (`/opt/pgsql-17`) and 18.6 (the Manjaro package),** throwaway clusters under my worktree's `.claude/scratch/`, Unix sockets only (`listen_addresses = ''`), the sockets under a short path in `~/.claude/scratch/`, `TMPDIR` in the scratch, and `\set VERBOSITY verbose`. I used psql 18's `\parse`, `\bind_named` and `\close_prepared` for the extended protocol, as a held connection's driver sends it. Session A called a plpgsql `DECLARE v st` function, compiling it, and prepared `SELECT c FROM tt`. Session B dropped and recreated `st`, changed `tt.c` to `bigint` and raised the `_morm` comment. On both versions:
  - **A stale function in A's own REPEATABLE READ transaction:** `XX000` `cache lookup failed for type`. A Parse of a fresh statement then got `25P02`, from `exec_parse_message`, and a Close of the stale statement went through.
  - **After `ROLLBACK`, the stale function on the same session:** `XX000` again in a new transaction, and again outside any transaction. A new session's call answered. So running a held connection's transaction again doesn't cure a stale session; only a new session does (EX2).
  - **A stale prepared statement in A's own transaction:** `0A000`, and a Parse after it `25P02`. After `ROLLBACK` a fresh Parse and Bind answered. So a stale statement does heal on the same session once morm has dropped it, as R-1293 says.
- **proptest 1.12.0,** its registry source (`src/test_runner/runner.rs`): a failure is persisted as the seed taken before the failing case was generated, and the replay regenerates that case through the strategy. So a `proptest-regressions` file holds a seed of the original, unshrunk case, not a shrunk sequence (EX5).
- **Not run:** PgBouncer. No finding here rests on how PgBouncer behaves.
- **My own concurrent model,** below. The servers are stopped, and the clusters, the sockets and the model are deleted.

## My concurrent model

I wrote it in Python, in my worktree's `.claude/scratch/` with `TMPDIR` there, from R-1270's procedure as written at the head. I didn't read `proofs.md`'s model first.

- **Steps.** Each call is a generator, and the code between two I/O points runs whole, as the stale lock makes it. The I/O points are:
  - a read of `<n>`, a run and an open;
  - the whole transaction's rollback, a `ROLLBACK TO SAVEPOINT`, a `SAVEPOINT`, a `COMMIT`;
  - a checkout's lock wait of 0 to 0.5 ms before `finish_take`.
- **Every step's starting last-read `<n>`** (and the snapshot it came from) is recorded when the step starts, and `finish_take` uses it. Half the time, a step's compare-and-swap that finds a due action's window ended has a checkout's compare-and-swap land first, at `take`, `set_due`, `on_moved`'s removal and an entry's creation. A further setting makes every such compare-and-swap land.
- **Reads of `<n>`.** A ticket is taken when the read is sent; it arrives 1 to 200 ms later, and 1% fail. Its snapshot is:
  - anywhere in flight;
  - in an open REPEATABLE READ or SERIALIZABLE transaction (morm's, or the app's on a held connection), the snapshot of that transaction's first statement;
  - in a failed transaction, none: the read gets `25P02` and applies nothing.
- **The world:**
  - migrations, each ending with the reset and a relabel, half refreshing functions, half changing the column, behind PgBouncer half followed by `RECONNECT`;
  - lowerings;
  - changes made by hand: a swap dropping the type, one keeping it, a column change, one changing nothing;
  - stored text holding `_morm_retired_` toggled;
  - straight to Postgres, refused opens for up to 0.5 s, and a role limit at the pool's size;
  - one server, or two with replay up to 0.8 s late.
- **The statements:**
  - an enum cast touching no table and a function call through a cast, with labels good, only in the new type, bad, or holding `_morm_retired_`;
  - a statement whose result type a column change alters;
  - a stored-text function;
  - plpgsql `DECLARE v st` and `%ROWTYPE` functions compiled per session.

  The oracle is a fresh session with a fresh statement at the run's instant. A run in flight while a change commits is left out.
- **Both modes.** Straight to Postgres a connection is its session. Behind PgBouncer three server sessions hold statements by text, 16 each, replaced after 4 s and by `RECONNECT`.
- **The load:**
  - a call about every 20 ms, a fifth in bursts of two to eight with the same statement and values;
  - 55% outside a transaction, 35% in `.retry(0)` to `.retry(2)` transactions of one or two statements, attempts at READ COMMITTED (half), REPEATABLE READ and SERIALIZABLE;
  - 2% of runs in a transaction end in a serialization failure, and 1% of runs in a statement timeout.
- **Savepoints.** 30% of statements in a transaction run inside `tx.transaction(..)`'s savepoint, nested one to three deep. An error returned as it is unwinds level by level, and each level's closure catches it half the time and carries on, committing.
- **Held connections:** 10% of calls straight to Postgres, half inside a REPEATABLE READ transaction the app opened, whose statements after an error get `25P02`, closed on release.
- **The invariants,** checked at every step:
  - I1 and I3: consecutive pairs excused one for one by a change of `<n>` after the first one's last-read snapshot, or by a rise in a several-server pool;
  - I2: a checkout after a window with an action due leaves none due;
  - I4: per epoch (`max` replaced, `max + 2` opens), per *replace* action (`max`), and over every span of `stale_cooldown` (`4 × max + 4`, plus `2 × max` per migration in the two windows before);
  - I5;
  - I7: no stale statement visible outside a transaction or on a held connection outside the app's transaction, no cached run after `SchemaChanged`, no transaction with `SchemaChanged` from stale statements twice;
  - I8: from `W` after the last change and the last refusal, no stale session visible outside a transaction, inside a savepoint, or under `.retry(2)` with no serialization failure;
  - I10, both forms;
  - I11: no error returned as it is inside a savepoint after a whole rollback or a session-class read, every whole rollback for a savepoint call returning `SchemaChanged`, and no savepoint rollback or commit after a whole rollback.
  - **A diagnostic beside I10,** not one of the plan's invariants, counts reads that are applied from a snapshot older than their sending: the premise R-1270 states.
- **The scale:** 256 sequences of 2,000 generated events per setting, `stale_cooldown` and `stale_pending_expiry` 1 s, a pool of four and a budget of five. The settings:
  - **in both modes:** the defaults, `servers = "several"` with one server, two servers, entries off, caps of 1, a short expiry (0.2 s), heavy stress (90% of calls in two-statement transactions up to 0.3 s apart, a call every 40 ms, world events at 20%), and savepoints (60% of calls in transactions, 90% of statements inside a savepoint);
  - **straight to Postgres:** savepoints at READ COMMITTED only, at REPEATABLE READ only and at SERIALIZABLE only; savepoints under stress; savepoints with two servers, entries off, and a role limit at the pool's size at two call rates; stress under a role limit; held connections (30% of calls held, 80% of those inside the app's transaction), under stress, with a role limit and with two servers; a role limit at the pool's size; a budget of eight; and every eligible compare-and-swap landing, in both modes, with savepoints and with held connections;
  - **each fault switch's own setting,** below, with no fault.
- **It leaves out** several processes behind one PgBouncer, a per-transaction `search_path`, several pools, a standby's recovery conflict, the transient `XX000`, a failing start read, pipelined statements, savepoints an app opens itself on a held connection, I6 (the caps are structural there), I9, and memory orderings.

**What it shows:**

- **The procedure as written fails nothing** in any of the 45 settings, with 256 sequences each.
- **The volume.** Each base setting ran 220,000 to 670,000 transactions and 104,000 to 1,150,000 statements inside a savepoint:
  - **Straight to Postgres:** 6,400 to 24,700 pool-wide actions and 8,100 to 25,300 epochs per setting.
  - **Behind PgBouncer:** 5,200 to 6,500 actions and 3,200 to 14,700 adoptions.
  - **Checkouts landing:** 9 to 540 per setting at a step's compare-and-swap.
- **The savepoint rule holds.** In the savepoints setting straight to Postgres:
  - **The decisions inside a savepoint:** rows B3 and B4 decided 9,900 and 31,000 times, about a quarter of each at REPEATABLE READ and a quarter at SERIALIZABLE, with no read.
  - **Errors returned as they are there:** 102,000, which closures caught 72,000 times, carrying on and committing.
  - **Stale sessions among them:** 5,700, every one before `t0 + W` or `W` after a refusal.
  - **I11 and I8's savepoint clause** held, and no rollback to a savepoint followed a whole rollback.
- **Held connections hold as R-1271 says.** In the held setting, 203,000 held calls ran inside the app's transaction. No read of `<n>` applied from that transaction's snapshot, and no stale statement was visible on a held connection outside one.
- **No stale statement or session reached a caller** beyond what R-1271 states. I7 and I8 held with entries off, caps of 1, a short expiry, two servers and a role limit.
- **R-0981's and R-0982's bounds hold.** I1 to I5 held in every setting, the span form of I4 included, under a role limit at the pool's size.
- **One model correction, not a plan finding.** My first I8 failed in 2 of 256 sequences with savepoints under a role limit:
  - **What happened:** a replacement's open in its slot got `53300`, because another replacement's momentary extra was open beside its old connection. That suspended the epoch, and a stale session there was returned as it is inside a savepoint 1.02 s after the change.
  - **Why it isn't a plan finding:** R-1271's "When Postgres refuses connections, healing comes within `W` of the first moment it accepts one again" covers it. Postgres accepted the next open 6 ms later, and the failure came within `W` of that.
  - **The correction:** my model now counts a `53300` as a refusal for I8, and every setting passes.
- **Fault switches of my own are caught,** out of 256, in the mode and at the setting T-0319 names, which shows the checkers aren't vacuous:
  - no gate, 256 straight and 256 behind PgBouncer (I1);
  - reads applied with no ticket, 176 (I10);
  - EV2's, a request in an open transaction reading `<n>`, 27 (I10); over 2,048 sequences 278, about 35 per 256;
  - EW1's, a row B1 or B2 read after a rollback only to the savepoint, 38 (I10);
  - the savepoint bit dropped, 256 (I11);
  - a `53300` leaving the epoch's extras on, 183 (I4 per epoch);
  - an epoch that never ends, at the savepoints setting, 256, by I8's savepoint clause;
  - the literal reading of EX3's sentence, which takes the move's own request under the `<n>` the step started with: 249 straight to Postgres and 256 under stress (I1).
- **EW2's switch, a request on a held connection reading `<n>` in the app's transaction, isn't reliably caught by I10** (EX1):
  - **At T-0319's setting:** 1 and 3 of 256, under the two readings of its wording below. Over 2,048 sequences it was 27 and 22, about 3.4 and 2.8 per 256.
  - **Longer app transactions** (statements up to 1 or 2 s apart) caught it in 0 or 1 of 256.
  - **The snapshot diagnostic** flagged the same switch in 254 and 255 of 256 (2,017 and 2,029 of 2,048), and EV2's and EW1's in 256 of 256, while no setting without a fault flagged anything.

## EW1 to EW7, and the B4 decision, as decided and recommended

| Item | Verdict |
|---|---|
| EW1, option (a) | **Written in as decided.** Every row that returns `Error::SchemaChanged` inside a transaction rolls the whole transaction back with a `rollback(call.tx)  # I/O` line, never `ROLLBACK TO SAVEPOINT`, before any read: B1 to B3, D1, D2 outside the cool-down and the transient `XX000`. "A rollback of the call's whole transaction" is among the I/O points. R-0441 adds `SchemaChanged` to the errors that fail the whole transaction and says nothing more is sent on it. `contracts.md`'s comments, R-0436, R-0984, R-1271's savepoint bullet, R-1292 and T-0312's savepoint case follow. The premise is restated in "Why no other interleaving matters" and in the read order |
| EW2, option (a) | **Written in as decided.** The confirming run passes `unread = call.out and !ctx.held`, and row A1 passes `unread = !ctx.held`. `25P02` is on R-1245's no-confirm list. Row A1's `if r is 25P02: return err` and the transient `XX000`'s retry return the original error. The interleaving note states that R-0981's read on a held connection gets `25P02` in the app's failed transaction. R-0718's A1 row, R-0982's request bullet and cost line, R-0981's A4 row, R-1271's held bullet, R-1293 and T-0318's held cases follow. R-1293's advice for a stale session is wrong (EX2) |
| EW3 | **Written in, but not reliable for EW2's switch.** Each fault test first runs a committed sequence under `crates/morm/proptest-regressions/stale_faults/`, one file per fault and mode, then 256 random cases at a named mode and setting. EV2's switch has its stress, and "no stale lock" and "no starting bit" name straight to Postgres. The claim that every named setting caught its switch in at least 30 of 256 is contradicted by `proofs.md` for EW2's switch, and my model agrees (EX1). What a committed sequence is isn't stated (EX5) |
| EW4 | **Fixed, in the wider form of choice 141.** `finish_take` uses the step's starting `<n>`, and `on_moved`'s landing case is covered. My model's I1 held with every eligible compare-and-swap landing. One sentence reads wider than the procedure means (EX3) |
| EW5 | **Fixed as recommended.** The writer is "sequence odd, a release fence, the data words relaxed, sequence even with a release store", and the reader matches. The `loom` case drives the module's own atomics through a shim and asserts a whole marker whatever generation was read, with a fault form that drops the fence. `cfg(morm_loom)` is in `morm`'s check-cfg list |
| EW6 | **Fixed as recommended:** a fixed array of `stale_pending_max` four-word sequence-locked slots, read as the marker is. The lookup is listed among the lock-free things, R-0718's cost line states the scan, and the `loom` case races `end_confirm` and a slot's reuse |
| EW7 | **Fixed as recommended.** R-0981 and R-1271 state the bounds per epoch, per *replace* action and over any span of `stale_cooldown`, and T-0319's I4 checks all three. My model's span check held, under a role limit too |
| The held connection's `XX000` (`7fc9175`) | **Fixed.** R-1271 and R-1293 say a cached statement's `XX000` on a held connection in the app's transaction returns as it is, because no fresh run can tell it there |
| B4, option (b) | **Written in as decided.** Each call's context carries `ctx.sp`, set at any depth. In `session`, B3 and B4 outside a savepoint roll the whole transaction back and then read; inside one they read nothing and `old` judges by the last-read `<n>`. B4 then returns as it is with only the savepoint rolled back, and B3 rolls the whole transaction back after its step and closes the connection after that. R-0981, R-0441, R-0984, R-1271, R-1292, the state table's epoch row and the interleaving note follow. So do T-0318's no-read `old` case, T-0312's savepoint cases, and T-0319's I11 and 31st switch. My model's I11 and I8 savepoint clause held at all three isolation levels, caught and not caught, nested up to three deep, and the dropped bit is caught in 256 of 256 |

**R-1270 still reads as one consolidated section.** It has the state table, "Steps and the stale lock" with its five lock-free things, "Why no other interleaving matters", the procedure with every I/O line marked, the paragraph on transactions, held connections and the start read, the read order, and the several-server paragraph. Each rule is stated once, and the history is in `proofs.md` and the PR body. It's self-consistent apart from EX3's sentence.

## Requirement by requirement, the changed IDs

| ID | Built as written? | Tests that prove it |
|---|---|---|
| R-1270 | Yes, apart from EX3's sentence | T-0318: the savepoint bit and no-read `old`, held connections in an app transaction, `25P02` as a no-confirm outcome, the step-start `<n>`, `loom` on the shim, the pending slots. T-0319's simulation and 31 switches, with EX1 and EX5 on how much they prove |
| R-0718, R-1245 | Yes: A1's held case, `25P02` on the no-confirm list, the slot scan's cost | T-0318's held and no-confirm cases, T-0312 |
| R-0981 | Yes. One phrase about when an epoch starts is wrong (EX6) | T-0318's no-read `old` case, T-0312's savepoint and role-limit cases, T-0319's I3, I4 and I8 |
| R-0982, R-1246 | Yes: no read on a held connection; the whole rollback before row D1's read | T-0318, T-0312 |
| R-0436, R-0441, R-0984 | Yes: `SchemaChanged` fails the whole transaction, nothing more is sent, and inside a savepoint B4 is catchable while B3 rolls back | T-0312's savepoint cases (`ROLLBACK` and no `ROLLBACK TO SAVEPOINT` by the server's log, the outer `INSERT` unsent, B4's `INSERT` committed), T-0319's I11 and row-B unit test |
| R-1271 | Yes, apart from the held-connection sentence (EX4) | T-0319's I4, I7, I8 and I11 |
| R-1253 | Yes: one bullet pointing to R-1292 and R-1293 | `doc` |
| R-1292 | Yes. All four parts of a loud notice: what it does, when it's right, what you give up, how to proceed | `doc` |
| R-1293 | All four parts are there, but the how-to-proceed is wrong for a stale session (EX2) | `doc` |
| T-0312, T-0318 | Yes | — |
| T-0319 | The savepoints, held connections, I11 and the 31 switches are in. EW2's switch fails its 256-case assertion often enough to make CI flaky, and the committed sequences' form isn't stated (EX1, EX5); I8 doesn't except held connections (EX4) | — |

## Loud notices and "use this when" lines

- **R-1292** has every part: what morm does inside a savepoint and why, when it's right, what you give up (the lost transaction, and a stale function's `Error::Postgres` inside a savepoint for up to `stale_cooldown` after a migration), and how to proceed. Its claims match the procedure and my model.
- **R-1293** has every part, but its how-to-proceed tells an app to roll back and run its transaction again on the held connection on `Error::SchemaChanged` or `XX000`. A stale session repeats on that connection (EX2).
- **No new public call or setting,** so no "use this when" line is missing.

## PR body

The PR body is accurate for this range:

- the sections "The nineteenth review, EW1 to EW7, and the owner's EW1 and EW2 decisions, since `a365aa5`" and "The owner's answer on B4, option (b), since `81f53c3`";
- choices 137 to 155, the two "Since" ID sentences, the checker line, and both CI links (18 of 18 jobs each);
- the changed and new IDs table.

The EW section's "All 30 fault switches are caught in their named mode and setting, in 30 to 256 of 256" describes the first rebuild. The B4 section then reports EW2's switch at 1 of 256, and "Open questions: None open" leaves it unresolved. That changes with EX1.

## Findings

### EX1. Should fix. T-0319 (step 20), `proofs.md`, PR body: EW2's fault switch isn't reliably caught, so its test makes CI flaky on a correct checker; T-0319's claim that every named setting caught its switch in at least 30 of 256 is contradicted for that switch

- **Where:**
  - T-0319: each fault test "first runs that fault's own shrunk failing sequence ... and then runs the CI case count, 256, of random sequences in the mode and at the setting named for the fault ... and asserts that a failing sequence is found among them".
  - T-0319: "each named setting is one under which `proofs.md`'s model caught the fault in at least 30 of 256 sequences, so a correct checker misses it in a fresh 256 with odds below one in 10<sup>13</sup>".
  - T-0319: "A fault whose committed sequence passes, or that no random sequence catches, fails its test".
  - EW2's switch: "a request on a `db.hold()` connection reading `<n>`, inside the transaction the app opened there, I10, straight to Postgres with 40% of calls on held connections, every one inside a REPEATABLE READ transaction the app opened, and the rest as for the open-transaction switch".
  - `proofs.md`'s B4 round: "EW2's switch, below the floor in this model: ... caught in 1 of 256, and in at most 2 of 256 at every setting tried". That round measured every switch "at a setting of the model's own", not at T-0319's.
- **What's wrong:**
  - **The committed sequence is reliable; the random half isn't.** Replaying a committed failing case catches the fault on every run, as long as the generator doesn't change (EX5). But the test also asserts a catch in 256 fresh cases, and fails if there's none.
  - **The odds of a miss.** A switch caught in λ of 256 is missed in a fresh 256 with probability about e<sup>−λ</sup>:
    - 37% at the fix's model's 1 of 256;
    - about 3% to 6% at my model's rate;
    - negligible at the nineteenth round's 70.
  - **Why it's rare.** I10 fails only when the app transaction's old snapshot lowers the last-read `<n>`. That needs a long chain inside one app transaction:
    - a migration after the snapshot;
    - an applied read of it;
    - an action under the new `<n>`, closing the window;
    - an unmarked entry made after that action;
    - then the held call's confirming run with the same bytes.
  - **The stated floor is false for this switch,** by `proofs.md`'s own latest figures, and the latest round doesn't measure at T-0319's named settings at all.
  - **The setting is ambiguous.** "40% of calls on held connections ... and the rest as for the open-transaction switch" can mean 60% of the remaining calls in transactions (24% outside one) or 60% of all calls (none outside one). It doesn't say how many statements an app transaction runs or how far apart they are.
- **Evidence:**
  - **My model at T-0319's setting:** 1 and 3 of 256 for the two readings, 27 and 22 of 2,048, and 0 or 1 of 256 with app transactions whose statements are up to 1 or 2 s apart.
  - **The premise, checked directly.** A check that no read of `<n>` is applied from a snapshot older than its sending flagged the switch in 254 and 255 of 256 (2,017 and 2,029 of 2,048), and EV2's and EW1's switches in 256 of 256. It flagged nothing in any of the 45 settings without a fault.
  - **The other two snapshot switches are caught often enough in my model,** though EV2's sits near the floor: 27 of 256 in one run and about 35 per 256 over 2,048, so a miss in a fresh 256 is under one in 10<sup>11</sup>; EW1's 38.
- **Fix:** no owner choice is needed.
  - **I10 gains the premise R-1270 states:** "every read of `<n>` the pool applies ran on a snapshot of its own, taken after it was sent: never an open transaction's, never one kept after a rollback only to a savepoint, never a held connection's app transaction's". The simulation knows each read's snapshot, so this can be checked.
  - **Name it for the three snapshot switches.** EV2's, EW1's and EW2's switches name this clause of I10, and each catches on any such read, whatever `<n>` it finds.
  - **State EW2's setting in full:** the share of held calls, the shape of an app transaction (statements and gaps), and the share of the other calls in transactions.
  - **Correct T-0319's floor sentence** to the measurement behind it. Rebuild the model's count for every switch at T-0319's own named settings, with the new clause, and record it in `proofs.md` and the PR body.

### EX2. Should fix. R-1293 (step 50), R-1271 (step 20): the held-connection notice tells an app to roll back and run its transaction again on the same held connection, which doesn't cure a stale session; only a new connection does

- **Where:**
  - R-1293's how to proceed: "on a held connection, roll back and run your transaction again on `Error::SchemaChanged`, or on `Error::Postgres` with SQLSTATE `0A000`, `22P02` or `XX000` after a schema change".
  - R-1293: "A stale function's error there returns `Error::SchemaChanged`, or, when it's `cache lookup failed for type` ..., its own error; either way the connection is closed when you release it".
  - R-0981's row A4: "an old connection is replaced on release".
- **What's wrong:**
  - **A stale session lives in the session.** A stale plpgsql function's compiled code stays until the session ends, and a held connection keeps its session until it's released.
  - **Running again repeats it.** Following the advice on the same held connection gets the same error at every call:
    - `Error::SchemaChanged` again while the connection is old (A4);
    - or `XX000` again: after the first fresh run evicted the cached statement, the call is uncached and goes to `session`, which returns it as it is when the connection isn't old.
  - **Only stale statements heal there:** `0A000`, a `22P02` naming the old type, or a cached statement's `XX000` that came from a dropped type rather than plpgsql. morm has dropped the statement, so the next run prepares it fresh.
  - **The same advice is missing for held connections outside an app transaction.** No README text tells an app that a held connection's `SchemaChanged` (row A4) needs the connection released and taken again.
- **Evidence:** Postgres 17.11 and 18.6, as above.
  - The stale function's `XX000` came back in a new transaction after `ROLLBACK` on the same session, and again outside any transaction.
  - A new session's call answered.
  - A stale prepared statement healed on the same session after `ROLLBACK`.
- **Fix:** no owner choice is needed. In R-1293's how to proceed (and, to match it, R-1271's held bullet):
  - **On `Error::SchemaChanged`, or on `XX000` `cache lookup failed for type`,** release the held connection, take a new one with `db.hold()`, set its session state up again, and run the transaction there.
  - **On `0A000` or `22P02` after a schema change,** roll back and run the transaction again on the same connection.
  - **What you give up** gains the session state a held connection loses when it's released for a stale function.
  - **The same sentence covers row A4** outside an app transaction.

### EX3. Minor. R-1270 (step 14), choice 141: "every take a step finishes ... is taken under the last-read `<n>` that step started with" reads as covering a step's own `take` after its own move, which the procedure takes under the new `<n>`; taken literally it breaks I1

- **Where:**
  - "Steps and the stale lock": "Every take a step finishes, a checkout's begun take included, is taken under the last-read `<n>` that step started with, so a take a checkout begins while a step that moves `<n>` runs counts as begun before the move, and the move's own request is judged against the window that take started".
  - "Why no other interleaving matters": "every take a step finishes is taken under the last-read `<n>` the step started with".
  - The procedure: `take` uses `last_read_n`, and only `finish_take` uses `n0`. R-0982's checkout bullet says "under the last-read `<n>` the finishing step started with".
- **What's wrong:**
  - **"A checkout's begun take included"** says the rule covers more than begun takes.
  - **The move's own take.** A read's step that moves `<n>` then takes its own request with `take`, under the new `<n>`. The EW4 trace in `proofs.md` needs exactly that: "the *replace* at once under 2".
  - **The literal reading breaks the bound.** A builder following the sentence would start that window under the old `<n>`. The next request in the window would then pass it again, because the last-read `<n>` differs from the window's.
- **Evidence:** my model, with `take` after a move made under the step's starting `<n>`: I1 fails in 249 of 256 sequences straight to Postgres and 256 under stress.
- **Fix:** no owner choice is needed. Say "every begun take a step finishes, a checkout's whose compare-and-swap landed during the step included, is taken under the last-read `<n>` that step started with, while a take the step makes for its own request uses the last-read `<n>` as the step leaves it", in both places, and in choice 141's wording in the PR body.

### EX4. Minor. R-1271, T-0319 (step 20): R-1271 now says held connections count as outside a transaction for every other bound, but row A4 makes a stale session visible on a held connection at every call until it's released, from `t0 + W` on too; I8 doesn't except held connections

- **Where:**
  - R-1271's held bullet: "a stale session returns `Error::SchemaChanged`, as on any held connection ... Those are the only visible cases on a held connection beyond the bullets below, which count it as outside a transaction".
  - The stale-session bullet: "from `t0 + W` on, never visible outside a transaction or inside a savepoint".
  - T-0319's I8: "a stale session straight to Postgres is never visible outside a transaction, inside a savepoint, or under `.retry(n)` ...".
- **What's wrong:**
  - **Row A4 is a visible case.** On a held connection outside any app transaction, a stale session on an old connection returns `Error::SchemaChanged` at every call until release. That's "an error a fresh session wouldn't return", and it can come long after `t0 + W`.
  - **The sentences contradict it.** Counting the held connection as outside a transaction makes the stale-session bullet say that case can't happen, and I8 as written would fail on it.
- **Fix:** no owner choice is needed.
  - **R-1271's held bullet** states row A4's case for every held connection: `Error::SchemaChanged` at each call on an old connection until it's released, as EX2's advice says. The bullets below count a held connection as outside a transaction apart from that.
  - **T-0319's I8** excepts held connections, checking instead that each held connection's stale session ends at its release.

### EX5. Minor. T-0319 (step 20): the "committed shrunk sequence" under `proptest-regressions/` isn't what proptest commits; its form isn't stated, so a generator change can silently turn it into a different case

- **Where:**
  - T-0319: "a unit test per fault first runs that fault's own shrunk failing sequence, committed under `crates/morm/proptest-regressions/stale_faults/`, one file per fault and mode"; "the step's PR finds each fault's failing sequence, shrinks it, commits it and reports its length".
  - The main test: "A failing sequence shrinks to a shortest one, saved under `crates/morm/proptest-regressions/` and committed".
- **What's wrong:**
  - **What proptest saves.** proptest persists the seed taken before the failing case was generated, and its replay regenerates that case through the strategy (proptest 1.12.0, `runner.rs`: `let seed = self.rng.gen_get_seed()`, then `save_persisted_failure2(source_file, PersistedSeed(seed), value)`, and on replay `self.rng.set_seed(persisted_seed)` and `gen_and_run_case`). The shrunk value appears only as a `# shrinks to` comment.
  - **So a proptest-regressions file holds** a seed of the original, unshrunk case, which changes whenever the event generator changes.
  - **The plan doesn't say which it means.** For a switch as rare as EW2's, a changed generator turns the committed case into an arbitrary one, so the test fails until someone finds a new failing case. T-0319 accepts that, but the "shrunk sequence" it describes isn't what would be committed.
- **Fix:** no owner choice is needed.
  - **State that each `stale_faults` file holds the shrunk event list itself,** in a stated text form the test parses, one event per line. The test replays it straight through the simulation's driver, with no strategy, so the case it runs doesn't depend on the generator.
  - **The main test's regressions** can stay proptest seeds; say so in place of "shrinks to a shortest one, saved".

### EX6. Minor. R-0981 (step 14): "a migration first seen there starts its epoch at the next read outside a transaction"; an epoch starts only at a session-class failure's test, after such a read

- **Where:** R-0981's old-connection bullet: "Inside a `tx.transaction(..)` savepoint, rows B3 and B4, morm reads nothing, ..., so a migration first seen there starts its epoch at the next read outside a transaction".
- **What's wrong:**
  - **Epochs start only in `old`,** at a session-class failure. A read outside a transaction, a request's or row C1's or D1's, applies the migration but starts no epoch.
  - **R-1271 says it right:** "until that epoch ends or a read of `<n>` outside a transaction applies the migration and the next failure starts a new epoch". So does T-0312's case.
- **Fix:** no owner choice is needed: "so a migration first seen there is learned at the next read of `<n>` outside a transaction that counts, and the next session-class failure after it starts its epoch".

## Summary

| Severity | Findings |
|---|---|
| Blocks approval | None |
| Should fix | EX1, EX2 |
| Minor | EX3, EX4, EX5, EX6 |

None of the six needs an owner choice.

What's written in:

- **EW1 to EW7 are written in as decided and recommended.** That's the owner's option (a) on EW1 and on EW2, and EW4 in its wider form. So is the owner's option (b) on B4: inside a savepoint rows B3 and B4 read nothing, `old` judges by the last-read `<n>`, B4 stays catchable, and B3 still fails the whole transaction.
- **R-1270 is still one consolidated, self-consistent section,** apart from EX3's sentence.
- **The checks agree.** The changed IDs match the two INDEX entries ID for ID. The checker reports 1293 requirements, 329 test obligations and 0 problems. CI is green on `5d111d3`, 18 of 18 jobs on push and on the pull request. R-1292 and R-1293 have every part of a loud notice, and the PR body is accurate for this range.
- **My model agrees with the plan** in 45 settings, both modes, with savepoints at READ COMMITTED, REPEATABLE READ and SERIALIZABLE, caught and not caught, nested up to three deep, held connections inside the app's transactions, two servers and role limits with `53300`. It found:
  - no I1, I2, I3, I4, I5, I7, I8, I10 or I11 failure;
  - R-0981's and R-0982's bounds held;
  - no stale statement or session reached a caller beyond what R-1271 states.

**EW2's fault switch, the question INDEX left for this round:**

- **The committed sequence is reliable.** Replaying it catches the fault on every run while the generator is unchanged.
- **The random half isn't.** The same test asserts a catch in 256 random cases at T-0319's setting, and there the switch is caught in about 1 to 3 of 256 (1 in the fix's model). So the test fails CI on a correct checker in roughly 3% to 37% of runs.
- **The fix (EX1):** check the premise directly, as part of I10. In my model that catches the switch in 254 to 255 of 256, and EV2's and EW1's in 256.

**PR #3 isn't ready for the owner yet.** Two should-fix findings remain, neither needing an owner choice:

- **EX1:** EW2's switch makes CI flaky as T-0319 states it.
- **EX2:** R-1293's loud notice gives advice that doesn't cure a stale session on a held connection.

The four minor findings are text and test fixes. Once EX1 to EX6 are written in, a short check of those edits, the checker and CI should be enough for PR #3 to go to the owner.
