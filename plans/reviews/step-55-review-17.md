# Step 55 independent review, seventeenth round

PR #3 (`step-55-enum-proofs`), head `8b45b7e`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes.

This round covers `git diff 7ed642b origin/step-55-enum-proofs`, eight commits:

- `4799c27` and `9805559`: ET1's fix, and `proofs.md` recording it;
- `0a13cda` and `1212993`: the owner's decision "overlapping reads of `<n>` are ordered by ticket", and its record;
- `975d290` and `8695718`: the owner's decisions "an overlapping read never counts" and "multi-server pools only rise", and their record;
- `e8eaf75` and `8b45b7e`: the owner's decision "a `servers` pool setting", and its record.

I also checked that nothing earlier regressed.

## What I read

- **On the PR branch:** `rules.md`, `contracts.md`, and `proofs.md` (step 55's ET1 bullets and the three decision rounds after it). I read every changed ID in full:
  - **Changed:** R-0060 (and its quoted copies in steps 11, 12, 27 and 47), R-0981, R-0982, R-1246, R-1262, R-1270, R-1271, R-1273, T-0318, T-0319 and T-0320.
  - **New:** R-1288, R-1289, R-1290, R-1291 and T-0329.
  - **For coherence:** R-0718, R-1245, the rows A1 to D3, the events E1 to E8, R-1116's start read, R-1275's unknown-key rule and R-0074's `target_session_attrs`.
- **On `origin/main`:**
  - `reviews/step-55-review-16.md`;
  - INDEX's Decisions entries "Sixteenth review fixed (ET1)", "Overlapping reads of `<n>` are ordered by ticket", "An overlapping read never counts; multi-server pools only rise" and "A `servers` pool setting", with their choices 74 to 100;
  - the entries they build on: "Fifteenth review fixed (ES1 to ES5)", "`0A000` requests a repair once" and the stale-session machine's MA to ME.
- **The PR body.**

## What I checked and ran

- **CI on the head.** `8b45b7e` is green: `ci` on push (run 44368213220) and on the pull request (run 44368232065), 18 of 18 jobs each, 36 of 36. The diff touches only `plans/plan-approved/`: `coverage.md`, `proofs.md`, `manifest.json` and steps 10, 11, 12, 14, 20, 27, 47 and 50.
- **Coverage checker,** run on a `git archive` of the head, `.github` included. `check_coverage.py --step 55 --code crates tests benches fuzz .github` reports **1291 requirements, 329 test obligations, 0 problems**, and the plain form reports the same. That's the expected total.
- **Manifest against INDEX.** I compared `git diff 7ed642b origin/step-55-enum-proofs -- plans/plan-approved/tools/manifest.json` ID by ID:
  - **Changed:** R-0060, R-0981, R-0982, R-1246, R-1262, R-1270, R-1271 and R-1273 (`text` and `block`), and T-0318, T-0319 and T-0320 (`text`).
  - **Added:** R-1288, R-1289, R-1290, R-1291 and T-0329.
  - **Nothing else moved:** no ID removed, and `frame` and `master_sha256` are unchanged.

  That's exactly the union of the four INDEX entries' lists. The PR body's four "Since ..." sentences name the same IDs round by round. Every requirement's `block` appears word for word in its step file, `contracts.md` or INDEX's record from the frozen plan, and no block holds a second `**R-nnnn**` marker.
- **R-0060's copies.** The `servers = "auto"` line is in step 10's block and in the quoted copies in steps 11, 12, 27 and 47, word for word. No other file quotes the block.
- **A model of the read and gate path.** I wrote a small model of R-1270's `read_n` (sent and arriving apart), `arrive`, `on_moved`, `request`, `take` and R-0981's epoch test, straight to Postgres, exactly as written at the head. Run as sequential steps, it agrees with `proofs.md`'s traces: ET1's trace gives one action, a late read from before a migration or a lowering is refused, and standbys replaying a migration 0.6 s apart give one move in a several-server pool. Then I split each function where the procedure doesn't say it runs as one step, and let a second call run in the gap. That gives EU1. A third trace gives EU2. The traces are under those findings. The model was in `.claude/scratch/` in my worktree and is deleted.
- **PgBouncer with a multi-host entry, on Postgres 17.** The last round ran this on Postgres 18.6 only. I ran it on Postgres 17.11 (`/opt/pgsql-17`), in my worktree's `.claude/scratch/` with `TMPDIR` there:
  - **Setup.** PgBouncer 1.26.1, built from pgbouncer.org's `pgbouncer-1.26.1.tar.gz`. Its SHA-256, `00886a2994b4844cd37110970df777772cfc42a74c400522c82fba509a3f42d9`, matches the published `.sha256`. It ran in transaction mode with one database entry, `host=127.0.0.1,127.0.0.2`, over two clusters whose `_morm` comments say `morm statement generation 7` and `6`.
  - **Default (`load_balance_hosts` unset, so `round-robin`):** three clients reading the comment 20 times each got `7` 38 times and `6` 22 times.
  - **`load_balance_hosts=disable`:** all 60 reads got `7`.
  - **One client connection,** reading it 30 times in a row: `7` each time in both modes.

  This confirms R-1288's and R-1289's "a PgBouncer database entry listing several hosts, which PgBouncer takes in turn by default". The servers are stopped and the scratch is deleted.
- **`target_session_attrs`.** I read `tokio-postgres` 0.7.18's `src/config.rs`, the version the workspace's `Cargo.lock` holds (EU3).
- **Not run:**
  - PgBouncer 1.22;
  - a load balancer;
  - a DNS name with several addresses.

  Nothing else in this range rests on how Postgres or PgBouncer behaves.

## ET1, as recommended

**Fixed as recommended.**

- **The rule.** `request` begins with `if n is not Unread: n = last_read_n`, with a comment saying why.
- **The mark.** `expect_confirm` lost its `<n>` argument. The mark's request uses `last_read_n` and makes no read.
- **The prose.** R-1270's prose has the replacement sentence the review gave. `proofs.md`'s ES2 bullet notes the refinement, and choices 67 and 70 say it.
- **R-0982.** Its request bullet and its bound both say the `<n>` judged is the pool's last-read one.
- **The tests:**
  - T-0319's unit test has row B3's trace, and row B2's with entries off.
  - T-0318 has a request carrying an older `<n>` inside a window, due and not taken, and row A2's case.
  - T-0319 has the eighth fault switch, caught by I1.
- **The model.** My model of the head gives ET1's trace one action, with B3's *replace* due.

## The owner's decisions, as decided

| Decision | Verdict |
|---|---|
| Overlapping reads ordered by ticket | **Written in.** `read_n` takes its ticket when the read is sent, with one atomic add. `arrive` is one step. The start read takes the first ticket and sets the last-read `<n>` (and the marker behind PgBouncer) with no `on_moved` (choice 78). Every applied read sets the horizon (79). A failed read applies nothing (80). R-0981's `T` is the read's arrival (81). `read_n` calls `on_moved` through `arrive` at all four call sites (82). I1 is stated per change of the database's `<n>` (83). Choices 84 and 85 are marked superseded |
| An overlapping read never counts | **Written in.** `arrive` returns `(last_read_n, false)` for `t < horizon`, greater or not. T-0319's span and exception are gone: I1, I3 and I4 are checked "with no span left out". The fifth read-order fault switch is the `1212993` procedure |
| Multi-server pools only rise | **Written in.** `arrive` adopts in such a pool only when `r > last_read_n`, and a lower `r` still sets the horizon. The servers bit is set once at pool start (87). Every pool's start read is taken as it is (88). The give-up is stated as "handled as a change made by hand" (89). R-1288 is the loud notice, and its "What to do" is choice 90. T-0319 models two- and three-server pools and has six read-order switches |
| A `servers` pool setting | **Written in.** R-1289 gives the three values, with `"auto"`'s rule unchanged. `"one"` wins with no refusal or warning (94). Values are matched exactly, with `Error::Config` naming the key, its table or the variable, and the three values (95). The setting is inherited from `[pool]` (96). `MORM_POOL_SERVERS` is R-1290 (97). `server` is refused naming `servers` (98). T-0319 is unchanged (99). T-0329 is new, and `contracts.md` is unchanged (100). R-0060's line and its copies are in place. R-1273 and T-0320 add the `# WARNING:` line for `"one"`. R-1262 says `servers` isn't a tunable. R-1288 tells pools behind a multi-host host to set `"several"`. R-1291 is the "use this when" line |

Three gaps:

- R-0981's prose and T-0319's unit test say an overlapping read "starts no epoch", which the procedure doesn't do (EU2).
- Choice 92's several-server I3 isn't in T-0319's text (EU4).
- The concurrency the decisions assume is stated for `arrive` alone (EU1).

## Loud notices and "use this when" lines

- **R-1288** has all four parts:
  - what it does (`<n>` only rises, `servers = "auto"`'s rule);
  - when it's right, and what morm can't see (`"several"`, `"one"`);
  - what you give up;
  - what to do.

  One sentence overstates what a move does straight to Postgres (EU5).
- **R-1291** is the setting's "use this when" line, and it's listed in the PR body's end-to-end paragraph.
- **The shipped file's `# WARNING:` line** for `servers = "one"` is required by R-1273 and checked by T-0320.
- No other new opt-in setting or call lacks a line.

## Still faithful

- **No regressions.** Outside the IDs above, nothing changed. The rows A1 to D3, R-1245's table, the pending-entry and cool-down rules, the `0A000`-once rule and ES1's *replace* bit read as before. `before_run` is unchanged.
- **R-1271** (step 20). The churn bounds now count a lowered `<n>` as a migration, say overlapping reads don't loosen them, and treat a several-server pool's raise back to an already-read `<n>` as a change made by hand. Its memory line adds the read order's two words and the servers bit. Its figures are unchanged.
- **The checker is clean.**

## PR body

These parts are accurate:

- the sections "The sixteenth review, ET1", "The owner's answer on overlapping reads of `<n>`", "The owner's answers on overlapping reads and multi-server pools" and "The owner's answer on a `servers` pool setting";
- choices 74 to 100;
- the four per-round ID sentences;
- "Open questions: None open";
- the end-to-end result: the checker line, both CI links, 18 of 18 jobs each, and the use-when list with R-1291.

One sentence repeats EU2's error: "R-0981's epoch test: a read that overlapped the last applied one, or in a several-server pool one finding a lower `<n>`, starts no epoch". Choice 92 describes an I3 that T-0319 doesn't state (EU4).

## Findings

### EU1. Should fix, needs an owner choice. R-1270 (step 14), R-0981, R-0982, T-0318, T-0319: only `arrive` is stated to run as one step, so with concurrent calls the gate can take two actions at once and an epoch can be set back

- **Where:**
  - R-1270's procedure: `arrive(t, r): # the read order, one step: nothing else sets the last-read <n> or the horizon meanwhile`.
  - The read-order paragraph: "`arrive` applies its result as one step; nothing waits".
  - `request`: `if window_open() or (n is not Unread and n != window.n): take(max(action, due), n)`.
  - `session()`: `(n, mv) = read_n(conn)`, then `if !old(conn, n)`.
  - `on_error`'s row B2: `(n, mv) = read_n(conn)` ... `old(conn, n)`.
  - R-0982: "a checkout ... a due action whose window has ended is taken".
  - T-0319: "generates random event sequences ... checking every invariant after every event".
- **What's wrong:**
  - **No other step is stated atomic.** The plan states no other function, and no stretch of a call's work, as one step on the pool's state. Calls run on a multi-threaded runtime, and "nothing waits" reads as lock-free. So a builder can write `request`'s window test and its `take` as separate reads and writes. The same goes for `old`'s epoch test and its start, and for a checkout's taking of the due action. Each pair then races.
  - **Two traces from my model,** straight to Postgres, with each function split where the plan doesn't say it's one step:
    1. **Two requests at once.** The window is open, or it's closed after a read found `<n>` moved from 1 to 2. Two calls on two threads both run `request(Replace, ..)`: both find the gate passing, and both `take`. That's two *replace* actions at the same instant, one pair I1 doesn't excuse. With the window open it needs no migration at all. This is the common case: after a hand-made change, every connection's cached statement fails at about the same moment, and R-1245 lists `0A000` and `42804` as signals a client can cause.
    2. **An epoch set back.** Call A's session-class read arrives at 0.010 s with `<n>` = 1 and is applied, unmoved, and A's thread is preempted before `old(conn, 1)`. A migration commits at 0.011 s. Call B's read is sent after A's arrived, so it doesn't overlap. It arrives at 0.013 s with 2, is applied, and B's `old(conn, 2)` starts an epoch under 2. A resumes at 0.014 s: `old(conn, 1)` finds 1 ≠ 2 and starts an epoch under 1, with `T` back at 0.010 s. Call C at 0.020 s reads 2 and starts a third. That's three epoch starts in 7 ms for one migration, against I3. Each epoch may replace up to `max` connections, against I4 and R-0981's "plus `max` per migration". With `old` judging by the pool's last-read `<n>`, as `request` now does, the same trace starts one epoch.
  - **The tests can't find it, and the text claims more than it gives.** T-0319's events are sequential, so they can't interleave inside a function. Its model is only faithful if the plan says what a step is. Meanwhile R-0982's headline bound ("at most one pool-wide action per `stale_cooldown`, plus one per migration") and the read-order paragraph ("sets back neither the last-read `<n>`, the marker nor the epoch") are stated without that condition.
  - **The cause is older than this range.** The gate's test-and-take was never stated atomic. But this range is the first to state an atomicity rule, for `arrive` alone, and the brief to this round asks for concurrent calls to be modelled.
- **Evidence:** the lines quoted above. My model prints three epochs (0.013 s under 2, 0.014 s under 1 with `T` = 0.010 s, 0.020 s under 2) and two *replace* actions at one instant in each gate case, all with the head's procedure. It prints one epoch with `old` judging by the last-read `<n>`, and one action with the gate's test and take as one step.
- **Fix:** this is a concurrency mechanism the plan doesn't settle, and it touches "nothing waits", so it goes to the owner.
  - **(a), recommended: the machine's work between two I/O points is one step.** Each stretch of a call's machine work with no I/O in it runs as one step on the pool's stale-session state, under a short per-pool synchronous critical section that holds no I/O and no `.await`:
    - from `arrive` through `old`, `entry`, `request` and `take`;
    - `on_moved`;
    - `expect_confirm`;
    - the confirming run's request.

    It's taken only on the error and read paths, which are already rare and cost a catalog read. A checkout stays one atomic read of the generation word when nothing is due, and takes a due action by one compare-and-swap that clears the due bits and raises the generation, so two checkouts can't both take it. A read's I/O is never inside the step, so no read waits for another, which is what the owner's "nothing waits" was about (against the async lock of the ticket question's option (b)). `old(conn, n)` also judges by the pool's last-read `<n>`, as `request` does, with `T` no earlier than the step's start. R-1270 states the rule beside `arrive`'s. R-0982's bound and the read-order paragraph cite it.
    - **Tests.** T-0318 adds threaded cases: eight threads requesting at once with the window open take one action and leave the rest due; the same after a read found `<n>` moved, with the window closed; eight checkouts at once after a window ends take the due action once; and EU1's trace 2, using a test hook between `arrive` and `old`, starts one epoch. T-0319 says its events are the machine's steps.
    - *You give up:* nothing measurable on the hot path. Concurrent error paths queue for nanoseconds on the critical section.
  - **(b) Lock-free.** Each test-and-act is its own compare-and-swap on a packed word:
    - the window's time, its `<n>` and the due bits for the gate;
    - the epoch's `T`, its `<n>` and the suspended bit for the epoch, with `old` judging by the last-read `<n>`;
    - the generation word for a checkout.

    Each loop rereads the last-read `<n>`. *You give up* simplicity: the window's time and `<n>` must fit one atomic word, or a 128-bit compare-and-swap is needed. Every function needs its own proof of the same bound, and the same threaded tests.
  - **(c) One step per call into the module, stated as a constraint on the pool code,** with no mechanism named. *You give up* nothing now, but step 14's builder has to choose (a) or (b) anyway, and per `rules.md` would stop and ask.

### EU2. Should fix. R-0981 (step 14), T-0319 (step 20), PR body: an overlapping read, or a lower read in a several-server pool, is said to start no epoch, but the procedure starts one whenever the last-read `<n>` differs from the epoch's

- **Where:**
  - R-0981's old-connection bullet: "a read that overlapped the last applied read counts as the last-read `<n>` and starts no epoch of its own, so a read sent before a migration or a lowering that arrives after a newer one starts none, nor, in a pool that reaches several servers, does a read that finds a lower `<n>`".
  - T-0319's unit test: "a row B2 or D1 read sent before a migration that arrives after another call's read of the new `<n>` counts as the new `<n>`, so its call ... calls no `on_moved` and starts no epoch, and a row B3 read that arrives the same way starts no epoch".
  - The PR body: "R-0981's epoch test: a read that overlapped the last applied one, or in a several-server pool one finding a lower `<n>`, starts no epoch".
- **What's wrong:**
  - **The procedure starts one.** Such a read returns `(last_read_n, false)`, and `old(conn, n)` starts an epoch "if there's none, it has ended, or the `<n>` the read counts as ... differs from the epoch's". Straight to Postgres, `on_moved` starts no epoch. So when the newer read that moved `<n>` was the gate's own read in `request`, the last-read `<n>` is already new and the epoch is still under the old one. A late overlapping read on a session-class failure then counts as the new `<n>` and starts an epoch, as event E1 wants ("its next session-class failure starts a new epoch").
  - **The same holds with no epoch.** With no epoch, or an ended one, every session-class failure must start one, overlapping read or not.
  - **Building R-0981 literally would break healing.** "Starts no epoch" would leave that call's old connection not old, its stale session returned as it is, until a later read starts the epoch. T-0319's B3 case, with "another call's read" left open, would assert against the procedure whenever that read was a gate read.
- **Evidence:** the lines quoted. R-0982's event E1. `on_moved` straight to Postgres touches no epoch. My model:
  - An epoch is in force under 1. A session-class failure's read is sent.
  - After a migration to 2, a gate read is sent, arrives first and is applied: `<n>` moved, and the epoch is still under 1.
  - The session read then arrives. It overlapped, so it counts as 2, and `old(conn, 2)` starts an epoch under 2, with `T` at its arrival, which is after the applied read's.
- **Fix:** no owner choice is needed. This states what the procedure does.
  - **R-0981.** Replace the clause with: "a read that overlapped the last applied read, or in a pool that reaches several servers one that finds a lower `<n>`, counts as the last-read `<n>`, so it starts an epoch only as any read does, when there's none, it has ended or the last-read `<n>` differs from the epoch's, and never under the older `<n>` it found; so a read sent before a migration or a lowering that arrives after a newer one never sets the epoch back".
  - **T-0319's unit case.** Name the other call's read as a session-class failure's, which started the epoch under the new `<n>`. Add the case where the newer read was the gate's own (row B1's `0A000` with the window closed): the late row B3 read starts the epoch under the new `<n>`, with `T` at its arrival.
  - **The PR body.** Correct the sentence.

### EU3. Minor. R-1270, R-1289 (step 14), R-1288 (step 50), T-0318, `proofs.md`: `target_session_attrs = "primary"` isn't a value morm's driver takes

- **Where:** `"auto"`'s rule says "its `target_session_attrs` isn't `"read-write"` or `"primary"`". It's in R-1270's state table, its paragraph "A pool that reaches several servers" and R-1289's table, and it's repeated in R-1288's first paragraph and in T-0318.
- **What's wrong:**
  - **The driver refuses it.** `tokio-postgres` 0.7.18, the version `Cargo.lock` holds, takes only `"any"`, `"read-write"` and `"read-only"` (`src/config.rs`, lines 677 to 686). Any other value is `config_parse(InvalidValue("target_session_attrs"))`. Step 10 has `target_session_attrs` "checked on each [host] as the driver checks it", and nothing in the plan maps libpq's other names.
  - **So the clause is dead, and the README points the wrong way.** A pool given `"primary"` fails at the driver's config parse and never starts. A reader of R-1288 who sets it to find the one primary gets an error at start. T-0329 tests only `"read-write"`, so nothing shows this.
  - **It's libpq's value, not the driver's.** `"primary"`, `"standby"` and `"prefer-standby"` are libpq's values.
- **Evidence:** the source lines above, at the locked version.
- **Fix:** no owner choice is needed, since the behaviour doesn't change. Drop `"primary"` from the rule in R-1270, R-1289, R-1288 and T-0318, leaving "isn't `"read-write"`". Note in `proofs.md` that the driver refuses `"primary"`. If the owner would rather accept libpq's names, that's a new feature (mapping `"primary"` to `ReadWrite`) and a question of its own. I don't recommend it in this PR.

### EU4. Minor. T-0319 (step 20): I3 has no several-server form, though choice 92 and INDEX say it does

- **Where:**
  - T-0319: "I3, two epochs start at least `stale_cooldown` apart unless `<n>` moved, and an epoch replaces at most the connections open at its start", unchanged from `7ed642b`.
  - Choice 92: "I3's 'unless `<n>` moved' means a rise there".
  - INDEX's "multi-server pools only rise" entry: "several-server forms of I1, I3 and I10".
- **What's wrong:** I1 says precisely which change excuses a pair, in one-server and several-server pools. I3 still says "unless `<n>` moved". In a pool that reaches several servers the database's `<n>` differs by server, so a builder can't tell whether a standby's replay excuses a second epoch. The choice the owner approves through the PR isn't in the text the builder reads.
- **Evidence:** the lines quoted.
- **Fix:** state I3 as I1 is stated:
  - **One server:** two epochs start at least `stale_cooldown` apart unless the database's `<n>` changed after the snapshot of the read the last-read `<n>` came from at the first start and no later than the second, each change excusing one such pair.
  - **Several servers:** unless the pool's last-read `<n>` rose between them.

### EU5. Minor. R-1288 (step 50): straight to Postgres a move of `<n>` doesn't itself clear every connection's caches

- **Where:** R-1288's "What morm can't see": "each move is handled as a migration: the pool clears every connection's statement caches, straight to Postgres starts a new epoch and lets a replacement of every connection through, and behind PgBouncer moves its statements to new text".
- **What's wrong:** `on_moved` raises the statement generation only behind PgBouncer. Straight to Postgres it sets the last-read `<n>` and removes every entry. By R-0982's event E1, the move then lets "its next request [pass] a closed window once", and "its next session-class failure starts a new epoch". So straight to Postgres a move clears nothing itself. It lets the next requested repair through at once, a *clear* or a *replace*, and the next stale-session error start an epoch. A loud notice should say what happens.
- **Evidence:** `on_moved` in R-1270, and E1 in R-0982.
- **Fix:** "each move is handled as a migration: behind PgBouncer the pool moves its statements to new text, so every connection clears its statement caches; straight to Postgres the next repair any call asks for passes at once, clearing every connection's caches or replacing every connection, and the next stale-session error starts a new epoch". The "seven moves" figure stands.

## Summary

| Severity | Findings |
|---|---|
| Blocks approval | None |
| Should fix | EU1 (needs an owner choice: (a) recommended), EU2 |
| Minor | EU3, EU4, EU5 |

What's written in:

- **ET1** is fixed as the sixteenth review recommended.
- **The owner's three decisions** in this range are written in as decided, with their choices 78 to 100, apart from EU2's wording and EU4's missing I3 form.
- **The checks agree.** The changed and new IDs match the four INDEX entries ID for ID. The checker reports 1291 requirements, 329 test obligations and 0 problems. CI is green on `8b45b7e`, 36 of 36.
- **The PgBouncer evidence holds on Postgres 17.11:** a multi-host entry round-robins by default.

**PR #3 isn't ready for the owner yet.** Two should-fix findings remain:

- **EU1** is a question for the owner, with options. Modelled with concurrent calls, the gate and the epoch need a stated atomic step, without which R-0982's bound and the read order's "sets back ... the epoch" don't hold.
- **EU2** is a contradiction between R-0981's prose and the procedure, fixable with no owner choice.

The three minor findings are wording and test-text fixes. Once EU2 to EU5 are fixed and EU1 is answered and written in, a short check of those edits, the checker and CI should be enough for PR #3 to go to the owner.
