# Step 55 independent review, nineteenth round

PR #3 (`step-55-enum-proofs`), head `a365aa5`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes.

This round covers `git diff 13bdab6 origin/step-55-enum-proofs`, three commits:

- `e05c8eb`: EV1 to EV6 (the salt set before the raise, no `<n>` read in an open transaction, acquire and release, a failed start read failing the pool, the attempt counts, the switch split);
- `6a42306`: the gap the fix's model found (a take a checkout began just before an adoption);
- `a365aa5`: `proofs.md`'s record of the round and its model.

I also checked that nothing earlier regressed.

## What I read

- **On the PR branch:** `rules.md`, `contracts.md`, `proofs.md` (the new round in full). Every changed ID in full:
  - **Step 14:** R-0718, R-0982, R-1246, R-1270 and T-0318.
  - **Step 20:** R-1271 and T-0319.
  - **Step 48:** R-1269.
  - **For coherence:** R-1245, R-0981, R-0983, R-1289, R-0436, R-0437 to R-0442, R-0984, R-0985, R-1247, T-0312, and step 12's `db.hold()` rows (R-0690 to R-0700, T-0305).
- **On `origin/main`:**
  - `reviews/step-55-review-18.md`;
  - INDEX's Decisions entry "No `<n>` read inside a transaction; a failed start read fails the pool (EV2, EV4)" and the entries it builds on, "The stale-session machine runs its steps under a short pool lock (EU1)", "Three stale-copy cases closed" and "Retries run unnamed; one attempt per stale thing; open in slot after `53300`".
- **The PR body,** at the head.

## What I checked and ran

- **CI on the head.** `a365aa5` is green: `ci` on push (run 44565816221) and on the pull request (run 44565832038), 18 of 18 jobs each.
- **Coverage checker,** run on a `git archive` of the head. `check_coverage.py --step 55 --code crates tests benches fuzz .github` reports **1291 requirements, 329 test obligations, 0 problems**, and the plain form the same. That's the expected total.
- **Manifest against INDEX.** I compared `13bdab6`'s manifest with the head's, ID by ID:
  - **Changed:** R-0718, R-0982, R-1246, R-1269, R-1270 and R-1271 (`text` and `block`), and T-0318 and T-0319 (`text`).
  - **Nothing else moved:** no ID added or removed, and `frame` and `master_sha256` are unchanged.

  That's exactly the INDEX entry's list, and the PR body's "Since `13bdab6`" sentence names the same IDs. Every changed `block` appears word for word in its step file, and no block holds a second `**R-nnnn**` marker.
- **Postgres 17.11 (`/opt/pgsql-17`) and 18.6 (the Manjaro package),** throwaway clusters under my worktree's `.claude/scratch/`, Unix sockets only (`listen_addresses = ''`), the sockets under a short path in `~/.claude/scratch/`, `TMPDIR` in the scratch. Session A ran `BEGIN ISOLATION LEVEL <level>`, read the `_morm` comment (`1`), and session B then committed `COMMENT ON SCHEMA _morm IS 'morm statement generation 2'`. A then ran `SAVEPOINT s`, `SELECT 1/0`, read again (`25P02`), ran `ROLLBACK TO SAVEPOINT s` and read the comment through `obj_description` and from `pg_description`. A second form opened the savepoint first and took the snapshot inside it. On both versions:
  - **READ COMMITTED** read `2` after `ROLLBACK TO SAVEPOINT`;
  - **REPEATABLE READ and SERIALIZABLE** read `1` after `ROLLBACK TO SAVEPOINT`, in both forms, and `2` only after `ROLLBACK`;
  - a statement after the error returned `25P02` (`current transaction is aborted`), with `\set VERBOSITY verbose` showing the SQLSTATE.
- **`unexpected_cfgs`,** with cargo 1.99.1, the workspace's MSRV: a crate whose `lib.rs` holds `#[cfg(morm_loom)]` fails `cargo clippy -- -D warnings` with `unexpected cfg condition name: morm_loom`, and passes once its `Cargo.toml` has `[lints.rust] unexpected_cfgs = { level = "warn", check-cfg = ["cfg(morm_loom)"] }`.
- **tokio-postgres 0.7.18** (the version in `Cargo.lock`), its registry source: every `Message::ReadyForQuery(_)` match drops the transaction status byte, and no public call returns it, so morm can't learn from the driver whether a session is inside a transaction block.
- **Not run:** PgBouncer. No finding here rests on how PgBouncer behaves: the savepoint result is Postgres's, and a transaction keeps one server session behind PgBouncer in transaction mode, as proof 16 has it.
- **My own concurrent model,** below. The servers are stopped and the scratch is deleted.

## My concurrent model

I wrote it in Python from R-1270's procedure as written at the head, without reading `proofs.md`'s model, in my worktree's `.claude/scratch/` with `TMPDIR` there, and deleted it after.

- **Steps.** Each call is a generator. The code between two I/O points runs whole, as the stale lock makes it. The I/O points are a read of `<n>`, a run, an open, a rollback and a checkout's lock wait.
- **Lock-free work** runs inline: a checkout's read and compare-and-swap, the pending check and a prepare's read of the marker. Half the time a step's compare-and-swap on the generation word finds another connection's checkout landed first, whenever a due action's window has ended, and the step then does what the procedure says (`finish_take` and judge again, or `on_moved`'s clearing of the starting bit). A checkout's compare-and-swap is followed by a lock wait of 0 to 0.5 ms before `finish_take`, during which other calls check out, run cached and prepare.
- **Reads of `<n>`** take a ticket when sent and arrive 1 to 200 ms later, 1% failing, the snapshot anywhere in flight. In an open REPEATABLE READ or SERIALIZABLE transaction the snapshot is the attempt's first statement's.
- **The world:** migrations (half with functions left unrefreshed, half with `RECONNECT` behind PgBouncer), lowerings, changes made by hand of five kinds (a swap dropping the type, one keeping it, a column change, a function change, one changing nothing), refused opens straight to Postgres, one server or two with replay up to 0.8 s late.
- **The statements:** an enum cast touching no table, one whose result type a column change alters, and a plpgsql function compiled per session. Labels good, only in the new type, or a client's bad one.
- **Both modes.** Straight to Postgres a connection is its session, with its cached copies and its compiled function. Behind PgBouncer three server sessions hold statements by text, 16 each, replaced after 4 s and by `RECONNECT`, and a client's prepare of a text gets the server statement held for it.
- **The load:** a call about every 20 ms, bursts of three to eight, 35% in `.retry(0)` to `.retry(2)` transactions of one or two statements, half at REPEATABLE READ or SERIALIZABLE, 2% of runs in a transaction ending in a serialization failure, 2% of runs on the no-confirm list.
- **The invariants,** checked at every step: I1 (consecutive actions excused one-for-one by a change of the database's `<n>` after the snapshot the first one's last-read `<n>` came from, or by a rise in a several-server pool), I2 (a checkout that wins the take always gets an action taken), I3 likewise for epochs, I4 (per epoch: at most `max + 2` replacement opens), I7's three clauses, I8's clearing clause (no cached run uses a copy stale from a change once a generation was raised after the change and before the run's read of the word) and I10 (a moving read's snapshot never older than one applied before it, one server).
- **The scale:** 256 sequences of 2,000 events per setting, `stale_cooldown` and `stale_pending_expiry` 1 s, a pool of four, a budget of five. Settings, each straight to Postgres and behind PgBouncer: the defaults, entries off, caps of 1, a short expiry (0.2 s), two servers declared `"several"`, and heavy stress (90% of calls in two-statement transactions with up to 0.3 s between statements, a call every 40 ms, world events at 20%); and straight to Postgres a role limit at the pool's size.
- **Its checkers aren't vacuous:** fault switches of my own, below.
- **It leaves out** several processes, a per-transaction `search_path`, held connections (except one variant below), record-shape functions, stored text holding `_morm_retired_`, and memory orderings, which a sequential model can't show.

**What it shows:**

- **The procedure as written fails nothing** in any of the 13 settings: no I1, I2, I3, I4, I7, I8 or I10 failure in 256 sequences each.
  - **The volume.** Straight to Postgres there were 11,100 to 21,900 pool-wide actions and 7,000 to 19,400 epochs per setting; behind PgBouncer 4,300 to 5,900 actions and 4,600 to 17,900 adoptions. Each setting ran about 245,000 transactions (534,000 under stress), half their attempts at REPEATABLE READ or SERIALIZABLE, and 230 to 850 checkouts landed inside a step's compare-and-swap.
  - **EV2's rule holds:** as written, no read of `<n>` runs in an open transaction, and I10 held.
  - **I7:** no stale statement reached a call outside a transaction, no attempt after `SchemaChanged` ran a cached copy, and no transaction got `SchemaChanged` from stale statements twice.
  - **I8's clearing clause** held behind PgBouncer in every setting, with checkouts landing in the lock wait before `finish_take`.
- **My fault switches are caught,** out of 256:
  - no gate, 256 (I1);
  - reads applied as they arrive, 195 (I10);
  - EV1's, a checkout's compare-and-swap raising the generation itself, 12 behind PgBouncer (I8);
  - a plain store into the word, 20 straight and 27 behind PgBouncer (I2);
  - a `53300` leaving the epoch's later replacements trying the extra, 146 under a role limit, with I4 checked per epoch.

  EV2's switch is caught in 0 at T-0319's settings and 1 with a stress like `proofs.md`'s (EW3). My model has no form of I8's "no no-retry transaction failing twice" clause, so it can't test the entry-removal switch.
- **Two variants the plan doesn't rule out,** each straight to Postgres:
  - **A stale signal inside a savepoint,** the rows' reads made after a rollback to the savepoint, with the outer REPEATABLE READ or SERIALIZABLE snapshot kept, as Postgres keeps it: I10 fails in 1 of 256 at the defaults and 147 of 256 under stress (EW1).
  - **A held connection inside a REPEATABLE READ transaction the app opened:** I10 fails in 16 of 256 (EW2).
- **A sliding-window form of I4 isn't met** by the procedure as written. With no change of `<n>` in the 2 s before, spans of 1 s held up to 11 stale-driven opens in most straight settings and 13 under the role limit, against `2 × max + 2` = 10 (EW7). Checked per epoch, I4 holds everywhere.

## EV1 to EV7, as decided and recommended

| Item | Verdict |
|---|---|
| EV1 | **Fixed as recommended.** A checkout's compare-and-swap sets only the starting bit, once the window has ended and the bit is clear. `finish_take` does the take in one step: a *salt*'s salt, then the raise with the due and starting bits cleared, then the window. `take` sets the salt before its own compare-and-swap, and the rule "a statement generation is raised only after the marker it's raised for is set" is in "Steps and the stale lock". The state table, R-0982's checkout bullet, E4 and R-1246 follow. T-0318 has the threaded hook case, and T-0319 the switch. My model's I8 clearing clause holds behind PgBouncer in every setting, and catches EV1's switch |
| EV2, option (a) | **Written in as decided,** with the owner's choice 130 (row B1's request, after the rollback, still reads). `before_run`'s request passes `unread = call.out`, R-0982's request bullet and the read order state the rule, and the cost line says "per request made outside an open transaction". The premise the fix states, that every read runs "outside a transaction or after its rollback, never in an open transaction's", doesn't hold in two places the procedure counts as rolled back or outside: a savepoint (EW1) and a held connection inside a transaction the app opened (EW2) |
| EV3 | **Fixed,** with the orderings named, the marker's read listed as the fourth lock-free thing, and the "why" bullet. Two gaps in the new text: the sequence lock's write ordering and what the `loom` case can show (EW5), and the pending entry's lock-free lookup, which still has no stated mechanism (EW6) |
| EV4, option (a) | **Written in as decided.** `read` fails the pool's start when the start read fails, the state table's row and R-1246 say so, `Db::connect` returns the error and a pool `db.pool(name)` adds returns it from its first call and reads again at the next (choice 132). T-0318 has the case |
| EV5 | **Fixed.** R-1271's four bullets, T-0319's I7 headline and I8's stale-session clause count the attempts "One attempt per stale thing" counts |
| EV6 | **Fixed.** The switch is split; T-0319 states 28 in all, and I count 8 + 7 + 7 + 3 + 3 = 28 |
| EV7 | **Fixed.** Choices 109 and 115 are marked superseded by choice 120, and the stale-copy section's sentence says "answered since, option (a) each, in the next section" |
| The adoption beside a begun take | **Closed** behind PgBouncer: the step that applies a read runs `finish_take` before `arrive`, and `on_moved`'s compare-and-swap clears the starting bit. Straight to Postgres the same landing inside `on_moved` still finishes the take under the new `<n>` (EW4) |

**R-1270 still reads as one consolidated section:** the state table, "Steps and the stale lock" with its four lock-free things, "Why no other interleaving matters", the procedure with every I/O line marked, the read order and the several-server paragraph. Each rule is stated once, and the history is in `proofs.md` and the PR body. It's self-consistent apart from EW1's unmarked rollback and EW4's sentence.

## Requirement by requirement, the changed IDs

| ID | Built as written? | Tests that prove it |
|---|---|---|
| R-1270 | Yes, apart from EW1, EW2, EW4 and EW6 | T-0318's new cases (no read in an open transaction at each level, B1's read after the rollback, the failed start read, the salt before the raise, the checkout's starting bit, the read's step finishing a begun take, an adoption replacing one, the threaded hook case, `loom`) and T-0319's 28 switches; EW3 and EW5 on how much they prove |
| R-0982 | Yes; its request bullet carries EV2's rule, its checkout bullet EV1's | As R-1270 |
| R-1246 | Yes: the salt in the same step, the failed start read | T-0318's salt and start-read cases |
| R-0718, R-1269 | Yes: "acquire, the same instruction as a plain load on x86-64" | R-1269 is step 48's benchmark |
| R-1271 | Yes (EV5's wording). Its churn bound's "per `W`" (EW7) | T-0319's I4, I7, I8 |
| T-0318 | Yes, apart from what the `loom` case can show (EW5) | — |
| T-0319 | 28 switches, the isolation levels and the lock wait are in. Several switches are caught too rarely to pass at 256 cases, and EV2's lacks the stress its own count needed (EW3) | — |

## Loud notices, "use this when" lines, and the new dev-dependency

- **No new public call or setting** in this range, so no "use this when" line is missing. EV4's start failure refuses nothing and silences nothing, so no notice is needed, as choice 132 says.
- **`loom`** is a dev-dependency with its reason stated in T-0318 and choice 133. `loom` itself is MIT, in `deny.toml`'s list; I didn't resolve its dependency tree. `--cfg morm_loom` rather than `--cfg loom` is right: `tokio` reads `cfg(loom)` for its own tests. The plan doesn't say `morm` declares the new cfg for `unexpected_cfgs`, which `clippy -D warnings` needs (EW5).

## PR body

Accurate for this range: the section "The eighteenth review, EV1 to EV7, and the owner's EV2 and EV4 decisions, since `13bdab6`", choices 126 to 136, the "Since `13bdab6`" ID sentence, the checker line and both CI links (18 of 18 jobs each). Its "Open questions: None" changes with EW1 and EW2.

## Findings

### EW1. Should fix, needs an owner choice. R-1270, R-0982, R-1246 (step 14), R-0984, R-0441, T-0312, T-0319 (step 20): inside `tx.transaction(..)`'s savepoint, "the rollback" the B and D rows read after can be a rollback to the savepoint, whose REPEATABLE READ or SERIALIZABLE snapshot is the outer transaction's, so EV2's fault comes back there; and row B3 discards a connection whose transaction can go on

- **Where:**
  - R-0436: "`tx.transaction(async |tx| ..)` opens a savepoint, released when the closure returns `Ok` and rolled back to when it returns an error".
  - R-0441: "Savepoints clear the failed mark, except after a deadlock or serialization error, which always fails the whole transaction".
  - R-0984: "Postgres has already failed the transaction, so nothing is retried inside it".
  - R-1270's `on_error`: "# from here the transaction has rolled back, so every read below runs outside it"; `session`: "R-0981's read, inside a transaction after its rollback"; "Why no other interleaving matters" and the read order: "every read runs on a snapshot of its own, outside a transaction or after its rollback, never in an open transaction's".
  - R-1270: "the I/O points are the lines marked `# I/O`: a read of `<n>`, a run of the call, and a connection's open or close". No line is a rollback.
  - R-0981's discard: "After the rollback morm closes the connection".
- **What's wrong:**
  - **Which rollback isn't said.** The B and D rows read `<n>` inside `on_error`, before the call returns, and a read in the failed transaction gets `25P02` (`proofs.md`'s probe and mine). So morm rolls something back first. The procedure marks no rollback, and nothing says whether a stale signal inside a savepoint rolls back the whole transaction or only to the savepoint.
  - **Read with R-0441, it's only to the savepoint.** `Error::SchemaChanged` isn't among the errors that fail the whole transaction, so the savepoint's closure returns it, the savepoint is rolled back to, and the outer transaction goes on, as R-0436 and R-0441 describe.
  - **Then the read runs in the open outer transaction.** At REPEATABLE READ or SERIALIZABLE its snapshot is the outer transaction's first statement's. B2's read, B1's request read (choice 130's), R-0981's read in B3 and B4, and D1's read behind PgBouncer can each apply an older `<n>` as a lowering in a one-server pool: EV2's fault, through the rows EV2's decision kept because they run "after the rollback".
  - **Row B3 has no answer there.** `discard(conn)` closes the connection the outer transaction is still on. Nothing says what the outer closure's next statement gets, or whether the close waits for the transaction's end.
- **Evidence:**
  - **Postgres 17.11 and 18.6:** after `ROLLBACK TO SAVEPOINT`, REPEATABLE READ and SERIALIZABLE read the old `<n>`, whether the snapshot was taken before the savepoint or inside it; only `ROLLBACK` gives the committed one. READ COMMITTED reads the new one.
  - **My model,** with the rows' reads made after a rollback to a savepoint in half the attempts (the outer transaction's snapshot kept at REPEATABLE READ and SERIALIZABLE): I10 fails in 1 of 256 sequences at the defaults and 147 of 256 under the stress setting. The first: a read whose snapshot was at 19.4943 s, kept from the outer transaction, was applied after one of 19.6007 s. It moved the last-read `<n>` from 22 back to 23, the value from before a lowering between the two snapshots. The next ordinary read lowers it again, so the gate and the epoch see two moves for one change. With a whole rollback, as in every other setting: no failure.
- **Fix:** whether a savepoint can absorb `Error::SchemaChanged` is a behaviour choice, so it goes to the owner.
  - **(a), recommended: a stale-session `Error::SchemaChanged` fails the whole transaction,** as a deadlock or serialization failure does. R-0441's exception list gains it. The procedure gets a `rollback  # I/O` line, the whole transaction, as the first thing rows B and D do after the signal, before any read, so the stated premise is true. A savepoint's closure returns the error, and the outer transaction's later use returns it, as R-0438 says. B3's discard is then always after the transaction's end. This matches R-0984's "Postgres has already failed the transaction" and the per-transaction `retried` flag.
    - **Tests.** T-0312 adds a case: a stale statement inside `tx.transaction(..)` in a REPEATABLE READ `.retry(1)` transaction, with a migration another call's read adopted meanwhile, fails the whole attempt, reads `<n>` after the whole rollback, and never lowers the last-read `<n>`. T-0319 runs some statements inside savepoints.
    - *You give up:* an app can't catch `SchemaChanged` from a savepoint and carry on in the same transaction; `.retry(n)` runs the whole closure again.
  - **(b) Savepoints stay able to absorb it.** Inside a savepoint, rows B and D make no read of `<n>`: B1's request, B2's and D1's reads and R-0981's test are judged by the pool's last-read `<n>`, as EV2's option (a) does for an open transaction. B2's and B3's discard becomes a mark that replaces the connection when the transaction ends, as A4's `replace_on_release` does. The procedure marks the rollback to the savepoint as I/O.
    - *You give up:* simplicity (a savepoint bit in the call's context and a deferred discard), and a migration first seen there is learned at the next read outside a transaction.

### EW2. Should fix, needs an owner choice. R-1270, R-0982, R-1245 (step 14): on a `db.hold()` connection inside a transaction the app opened itself, the procedure counts the call as outside a transaction, so a confirming run's request reads `<n>` in that transaction's snapshot, and row A1's fresh run in its failed transaction gets `25P02`, which counts as a confirmation

- **Where:**
  - `before_run`: `request(pooled ? Salt : Clear, unread = call.out)`; row A1: `request(Clear, unread = true)`.
  - `ctx.out` and `call.out` mean outside a transaction, which the procedure knows only for morm's own `tx`.
  - A held connection can be inside one the app opened: T-0305 uses "a `db.hold()` connection on the standby, idle in a `REPEATABLE READ` transaction that read a table"; R-0698 lists the session state a held connection carries; step 18's test runs SQL on one through `sql_unchecked`.
  - R-1245's no-confirm list: a lost connection, classes `08`, `40`, `53` and `57`, and `55P03`. Not `25P02`.
- **What's wrong:**
  - **EV2 again.** A confirming run on such a connection succeeds, so the app's transaction isn't failed. When it confirms with the window closed, `request` reads `<n>` on that connection, in a REPEATABLE READ or SERIALIZABLE snapshot that can be older than the pool's last applied read. In a one-server pool it's applied as a lowering.
  - **`25P02` confirms.** A cached statement's signal there fails the app's transaction. Row A1's fresh run then returns `25P02`, "a server error other than the first one and off the no-confirm list", so it confirms: a pool-wide request for nothing, and the caller gets `25P02` in place of the error that happened.
  - **morm can't see the session's state.** tokio-postgres 0.7.18 drops the ReadyForQuery status byte, so the driver can't say whether a held session is in a transaction block.
- **Evidence:**
  - **Postgres 17.11 and 18.6:** an open REPEATABLE READ transaction reads the old `<n>` (the eighteenth review's probe and mine), and a statement after an error returns `25P02`.
  - **My model,** with 20% of the calls outside morm's transactions run on a held connection inside an app's REPEATABLE READ transaction that began up to 0.3 s earlier, straight to Postgres: I10 fails in 16 of 256 sequences. The first: a read whose snapshot was at 36.1633 s, the app's transaction's, was applied after one of 36.4153 s, moving the last-read `<n>` from 22 back to 23. Without that variant: no failure.
- **Fix:** what morm assumes about a held connection's session is a behaviour choice, so it goes to the owner.
  - **(a), recommended: a call on a `db.hold()` connection never reads `<n>` for a request,** judged by the pool's last-read `<n>` as a call in an open transaction is, and `25P02` joins R-1245's no-confirm list, so a fresh run in an app's failed transaction neither confirms nor fails, and row A1 returns the original error when its fresh run gets `25P02`. R-0981's read on a held connection (row A4) can stay: after a session-class failure in an app's transaction it gets `25P02` and applies nothing.
    - **Tests.** T-0318 adds both cases on a held connection inside a REPEATABLE READ transaction opened through `sql_unchecked`.
    - *You give up:* a migration first seen by a held connection's confirming run is learned at the next read elsewhere.
  - **(b) Refuse transaction control on a held connection** (`BEGIN`, `START TRANSACTION`, `COMMIT`, `ROLLBACK` through `sql_unchecked`) and give it a `held.transaction(..)` morm tracks. *You give up:* T-0305's own pattern, and a scanner check an app can get around. Not recommended.
  - **(c) Ask the session before each such read,** a round trip per read. Not recommended.

### EW3. Should fix. T-0319 (step 20): several fault switches are caught so rarely under their stated settings that their tests, which must find a failing sequence in 256 cases, will fail CI on a correct checker; EV2's switch lacks the stress its own figure needed; two switches don't name the mode they're caught in

- **Where:**
  - T-0319: "a unit test per fault runs the simulation with that fault on and asserts that a failing sequence is found within the CI case count, 256 ... A fault that no sequence catches fails its test".
  - Its EV2 switch: "a request made while its call's transaction is open reading `<n>` in that transaction's snapshot, I1 or I10, straight to Postgres with half the attempts at REPEATABLE READ or SERIALIZABLE".
  - `proofs.md`'s figures, out of 256: EV2's switch 2, "with its stress (90% of calls in two-statement transactions, statements up to 0.3 s apart, a call every 40 ms, migrations and kept swaps at 20% of events)"; a confirming run's outcome removing the entry 2; the `53300` switch 3; the horizon set only by a moving read 3 behind PgBouncer; no starting bit 5 behind PgBouncer; a plain store 8 and 7; no stale lock 0 behind PgBouncer.
  - Choice 135: "EV2's switch runs with stated stress".
- **What's wrong:**
  - **The tests are flaky as written,** by `proofs.md`'s own figures. A fault caught in λ of 256 sequences is missed by a fresh run of 256 with probability about e<sup>−λ</sup>: 13.5% for λ = 2 (EV2's, and the entry removal), 5% for 3 (`53300`, the horizon behind PgBouncer), 0.7% for 5. proptest draws a fresh seed each run unless told otherwise, and T-0319 doesn't say the fault tests run fixed sequences.
  - **EV2's switch is weaker still as written.** Its 2 in 256 needed a stress T-0319 doesn't state; choice 135 says the switch "runs with" it, but the requirement doesn't.
  - **No mode named.** "No stale lock" is caught in 0 of 256 behind PgBouncer and "no starting bit" in 5, but T-0319 doesn't say which mode their tests run in.
  - **My model agrees.** EV2's switch: 0 of 256 at T-0319's settings, 1 of 256 with a stress like the one above, and 0 of 64 with two heavier ones (every transaction two statements up to 0.5 s apart, migrations at 50 to 60% of world events, one recurring value). The `53300` switch, which T-0319 names I4 for: 146 of 256 with I4 checked per epoch, at most `max + 2` replacement opens, against `proofs.md`'s 3 with its I4.
- **Fix:** no owner choice is needed.
  - **Each fault test first runs its fault's own shrunk failing sequence,** the one the PR reports, committed under `crates/morm/proptest-regressions/` beside the main test's, so it's caught on every run, and then its random cases. A checker grown too weak still fails on the committed sequence.
  - **T-0319 states what a catch needs:** EV2's switch gets choice 135's stress; "no stale lock" and "no starting bit" name straight to Postgres, as EV1's names behind PgBouncer.
  - **I4 is also checked per epoch and per action** (EW7), which catches the `53300` switch directly.

### EW4. Minor. R-1270, R-0982 (step 14): straight to Postgres, a checkout landing inside `on_moved`'s entry removal is finished under the new `<n>`, against "a begun take is finished under the last-read `<n>` it began under", and holds back the migration's own action

- **Where:**
  - `on_moved(r)`: "last_read_n = r; remove every pending and cool-down entry". Straight to Postgres that removal is a change of the entry count, made by compare-and-swap ("a step that creates or removes a pending entry changes the count").
  - "a step whose compare-and-swap fails because a checkout's came first runs `finish_take` and judges again".
  - "Why no other interleaving matters": "the step that applies a read before `arrive`, so a begun take is finished under the last-read `<n>` it began under".
  - R-0982: "so a migration's stale state isn't held back there".
- **What's wrong:**
  - **The order.** With an action due, its window ended and entries pending, a checkout's compare-and-swap can land in that step. The step then runs `finish_take` after `last_read_n = r`, so the due action is taken under `r` and its window's `<n>` is `r`.
  - **The effect.** The request whose read this was then finds the gate closed and its action due until that window ends. When the request is *replace* and the due action *clear*, the migration's *replace* waits up to `stale_cooldown`; with no landing, `take` takes the max of the two at once.
  - **No bound breaks:** each connection's own repair is local. But the sentence and R-0982's "isn't held back" are false in that interleaving. Behind PgBouncer the fix's own rule, the adoption taking the due action's place, covers it.
- **Fix:** no owner choice is needed. `on_moved` makes its entry removal, its compare-and-swap, before it sets the last-read `<n>`, so a landed take is finished under the old `<n>`, as the read's step does. T-0318's case for the read's step adds the landing inside `on_moved` straight to Postgres.

### EW5. Minor. R-1270, T-0318 (step 14), `ci.yml`: the sequence lock's write ordering is under-specified, the `loom` case asserts wholeness only where it can't fail and checks a model rather than the shipping atomics, and `morm_loom` isn't declared for `unexpected_cfgs`

- **Where:**
  - "a step writes the marker as a sequence lock, making the sequence word odd, storing the `<n>` and salt words and making it even again, with release ordering".
  - T-0318's `loom` case: "a model of the generation word, the marker's sequence lock and the window's end ... a thread that reads the raised generation reads that marker or a newer one, whole"; its fault form makes "the generation word's reads and compare-and-swaps" relaxed.
  - `proofs.md`, the `morm_mgr` refinement: a `cfg` read in `morm` "warns `unexpected_cfgs` ... which breaks `-D warnings`", so `morm` declares it "in its own `[lints.rust] unexpected_cfgs` `check-cfg`".
- **What's wrong:**
  - **The writer.** Read naturally, "with release ordering" goes on the even store. Then the two data stores can become visible before the odd one, and a prepare can read the old even sequence, a new `<n>` with the old salt, and the old even sequence again, and accept a torn marker. A sequence lock's writer needs a release fence after the odd store, or the data words stored with release.
  - **Why a torn read matters.** Only a prepare that read the old generation races a step. Racing `on_moved` from `g<n>.<s>` to `g<r>`, it can read `g<n>` unsalted: a text the pool used before the salt it took to leave it, whose server statements PgBouncer may hold stale. That run breaks "a pool's marker never returns to a text it has used".
  - **The `loom` case can't show it.** In the branch it asserts, the acquire on the generation word already orders the marker's writes before the read, so the sequence lock's correctness never shows there. A torn read is possible only in the branch it doesn't assert, and its fault form leaves the sequence lock alone.
  - **A model, not the module.** The shipping code's orderings go unchecked, unlike T-0319, which drives "the module that ships".
  - **`clippy -D warnings`.** A `#[cfg(morm_loom)]` in `morm` fails it with `unexpected cfg condition name: morm_loom` (shown with cargo 1.99.1), and nothing adds `cfg(morm_loom)` to `morm`'s check-cfg list.
- **Fix:** no owner choice is needed.
  - **The sentence:** "making the sequence word odd, then a release fence, then storing the `<n>` and salt words, then making it even with release".
  - **The `loom` case** also asserts that a prepare reading the marker while a step changes it gets a marker some step wrote, whatever generation it read, with a fault form that drops the fence.
  - **The module's own atomics** come through a small shim, `loom::sync::atomic` under `cfg(morm_loom)` and `std::sync::atomic` otherwise, and the case drives the module's own functions.
  - **`morm`'s `[lints.rust] unexpected_cfgs` check-cfg list** adds `cfg(morm_loom)`.

### EW6. Minor. R-1270 (step 14): the pending check's lock-free lookup of a statement's entry has no stated mechanism or ordering, unlike the marker's read

- **Where:** "while one does, a lookup of the statement's entry as the last step left it, with no lock". An entry holds the text's hash, the confirming bytes' hash, which `end_confirm` clears, the marked and *replace* bits and an expiry, and steps create, change and remove entries.
- **What's wrong:**
  - **A structure is needed.** A table that steps change under the lock and runs read without it needs a structure that makes that safe in Rust.
  - **A torn read is possible.** A lookup racing a step can mix two states of an entry, and so confirm on bytes the entry no longer holds, or miss the *replace* bit row B2 just set.
  - **EV3's own rule.** It names an ordering for every lock-free read. This one has none, it isn't among the four listed things, and the `loom` case doesn't cover it.
- **Fix:** no owner choice is needed: name the mechanism. For example, a fixed array of `stale_pending_max` slots, each written by steps with the marker's sequence-lock protocol (key, bytes, bits, expiry) and read by the lookup the same way, after the count's acquire read. List it among the lock-free things, and give T-0318's `loom` case a lookup racing `end_confirm`.

### EW7. Minor. R-0981, R-1271, T-0319: the churn bounds' "per `stale_cooldown`" hold per epoch and per action, not over every span of that length, so an I4 checked over sliding spans fails on the procedure as written

- **Where:**
  - R-0981: "at most `max` replacements per `stale_cooldown`, plus `max` per migration".
  - R-1271: "at most `2 × max + 2` connection opens from stale handling per `W`".
  - T-0319's I4: "stale-driven connection opens stay within `2 × max + 2` per `stale_cooldown`".
- **What's wrong:**
  - **Replacements trail their cause.** A *replace* action's replacements happen at each connection's next checkout after the action. An epoch's happen at each old connection's next failure, anywhere in the epoch.
  - **The next one can follow at once.** The next action or epoch can come `stale_cooldown` after the last, so one span of that length can hold the tail of one and the head of the next.
- **Evidence:** my model, straight to Postgres, a role limit at the pool's size, `max` 4, so a bound of 10:
  - a *replace* at 23.689 s replaced two idle connections at 23.861 and 23.944 s;
  - an epoch at 24.331 s made four opens: a discarded slot's, a `53300` beside the old connection, its slot, and another slot;
  - the next *replace*, at 24.747 s, 1.058 s after the first, made four at 24.770 to 24.811 s.

  That's 11 opens within 0.95 s, with no change of `<n>` in the 2 s before. Sliding spans with no change reached 11 or 12 in most straight settings.
- **Fix:** no owner choice is needed. State the bounds per epoch and per action: each epoch makes at most `max` replacements and `max + 2` opens, each *replace* action at most `max`, plus `2 × max` per migration as now, so any span of `stale_cooldown` holds at most two of each. T-0319's I4 checks per epoch and per action, which also catches EW3's `53300` switch.

## Summary

| Severity | Findings |
|---|---|
| Blocks approval | None |
| Should fix | EW1 (needs an owner choice: (a) recommended); EW2 (needs an owner choice: (a) recommended); EW3 |
| Minor | EW4, EW5, EW6, EW7 |

What's written in:

- **EV1 to EV7 are fixed as decided and recommended.** That includes the owner's options (a) on EV2 and EV4 with choice 130. The model's own gap, an adoption beside a begun take, is closed behind PgBouncer.
- **R-1270 still reads as one consolidated section,** self-consistent apart from EW1's unmarked rollback and EW4's sentence.
- **The checks agree.** The changed IDs match the INDEX entry ID for ID. The checker reports 1291 requirements, 329 test obligations and 0 problems. CI is green on `a365aa5`, 18 of 18 jobs on push and on the pull request. The PR body is accurate for this range.
- **My model agrees with the plan where the plan's premises hold.** In 13 settings, both modes, REPEATABLE READ and SERIALIZABLE attempts, checkouts landing inside steps and before `finish_take`, it found:
  - no I1, I2, I3, per-epoch I4, I7, I8 or I10 failure;
  - R-0982's bound held;
  - no stale statement reached a caller under `.retry(n)` beyond what R-1271 states.

**PR #3 isn't ready for the owner yet.** Three should-fix findings remain:

- **EW1** is a question for the owner, with options. EV2's stated premise, that every read of `<n>` runs outside a transaction or after its rollback, fails inside `tx.transaction(..)`'s savepoint. There a rollback is only to the savepoint and keeps the outer snapshot, which is shown on Postgres 17.11 and 18.6.
- **EW2** is a question for the owner, with options. The same premise fails on a held connection inside a transaction the app opened, and there `25P02` counts as a confirmation.
- **EW3** needs no owner choice. Several of T-0319's fault switches are caught too rarely under their stated settings for their 256-case tests to pass reliably: 13.5% misses for two of them by the PR's own figures, and EV2's switch was caught in 0 of 256 in my model at T-0319's settings.

The four minor findings are text and test fixes with no owner choice. Once EW1 and EW2 are answered and written in with EW3 to EW7, a short check of those edits, the checker and CI should be enough for PR #3 to go to the owner.
