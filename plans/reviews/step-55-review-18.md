# Step 55 independent review, eighteenth round

PR #3 (`step-55-enum-proofs`), head `13bdab6`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes.

This round covers `git diff 8b45b7e origin/step-55-enum-proofs`, seven commits:

- `c2f8356` and `fd79625`: the owner-chosen consolidated rewrite of R-1270's read and gate path under EU1 option (a), with EU2 to EU5 folded in, and `proofs.md` recording it;
- `6d765fb` and `7c12f96`: the three stale-copy answers (a generation check before a cached run, a fresh salt on a non-rising adoption, an entry kept to its expiry), and their record;
- `4643977`, `9291415` and `13bdab6`: the retry round (retries run unnamed after `Error::SchemaChanged`, one attempt per stale thing, a replacement in its slot after `53300`), its tests and its record.

I also checked that nothing earlier regressed.

## What I read

- **On the PR branch:** `rules.md`, `contracts.md`, and `proofs.md` (the three new rounds). I read every changed ID in full:
  - **Step 14:** R-0718, R-0981, R-0982, R-1246, R-1270, R-1289, T-0311 and T-0318.
  - **Step 20:** R-0984, R-1271, T-0312 and T-0319.
  - **Step 48:** R-1269.
  - **Step 50:** R-1249, R-1253, R-1258, R-1265, R-1268, R-1272 and R-1288.
  - **For coherence:** R-1245, R-0983, R-1262 and the rows A1 to D3.
- **On `origin/main`:**
  - `reviews/step-55-review-17.md`;
  - INDEX's Decisions entries "The stale-session machine runs its steps under a short pool lock (EU1), in one consolidated rewrite", "Three stale-copy cases closed" and "Retries run unnamed; one attempt per stale thing; open in slot after `53300`";
  - the entries they build on, from "Sixteenth review fixed (ET1)" through "A `servers` pool setting".
- **The PR body.**

## What I checked and ran

- **CI on the head.** `13bdab6` is green: `ci` on push (run 44517187168) and on the pull request (run 44517199888), 18 of 18 jobs each.
- **Coverage checker,** run on a `git archive` of the head. `check_coverage.py --step 55 --code crates tests benches fuzz .github` reports **1291 requirements, 329 test obligations, 0 problems**. The plain form reports the same. That's the expected total.
- **Manifest against INDEX.** I compared the two manifests ID by ID:
  - **Changed:** R-0718, R-0981, R-0982, R-0984, R-1246, R-1249, R-1253, R-1258, R-1265, R-1268, R-1269, R-1270, R-1271, R-1272, R-1288 and R-1289 (`text` and `block`), and T-0311, T-0312, T-0318 and T-0319 (`text`).
  - **Nothing else moved:** no ID added or removed, and `frame` and `master_sha256` are unchanged.

  That's exactly the union of the three INDEX entries' lists, and the PR body's three "Since ..." sentences name the same IDs round by round. Every changed `block` appears word for word in its step file. No block holds a second `**R-nnnn**` marker.
- **My own concurrent model** of R-1270's procedure as written at the head. It's Python, a discrete-event scheduler, written without reading `proofs.md`'s model, in my worktree's `.claude/scratch/` with `TMPDIR` there, and deleted after. How it works:
  - **Steps.** Each call is a coroutine. The code between two I/O points runs whole, as the stale lock makes it. A step's judging of the generation word and its own compare-and-swap are split, so a lock-free checkout can land between them.
  - **A checkout's take.** It's the compare-and-swap, then `finish_take` after a lock wait of 0 to 0.5 ms, during which other calls' checkouts and cached runs go on.
  - **Reads of `<n>`.** Each takes a ticket and arrives 1 to 200 ms later, with its snapshot anywhere in flight. Inside a REPEATABLE READ attempt, the snapshot is the attempt's first statement's.
  - **The world:** migrations, lowerings and changes made by hand of two statements (a `0A000` kind and a `22P02` kind).
  - **The two modes.** Straight to Postgres, a connection's cached copies have their versions. Behind PgBouncer, three server sessions hold statements by text, replaced over `server_lifetime`.
  - **The load:** calls outside transactions and `.retry(0)` to `.retry(2)` transactions of one or two statements, with bursts and serialization failures.
  - **The invariants:** I1 (each pair excused by one change of the database's `<n>` after the snapshot the last-read `<n>` came from), I3, I7 (no cached run in an attempt after `SchemaChanged`, and no transaction with two stale-statement `SchemaChanged`), I8's clearing clause and I10.
  - **The scale:** seven settings per mode (the defaults, entries off, caps of 1, a short expiry, heavy stress, slow lock, REPEATABLE READ attempts), 32 sequences of 20 s each.
  - **Its checkers aren't vacuous.** Four fault switches of my own are caught: no gate (I1, 30 and 32 of 32), no ticket (I10 and I1, 22 and 15), a retried attempt running cached (I7, 32 and 32), and a checkout taking by a plain read and write (I1, 17 and 21).
  - **It leaves out** stale sessions, epochs' replacements, several servers and several processes, which `proofs.md`'s model covers. That's where it found nothing new to look at.
- **What the model shows:**
  - **Clean everywhere but two places.** No I1, I3, I7 or I10 failure in any setting, apart from EV2's setting. That includes no transaction under `.retry(n)` with two stale-statement `SchemaChanged`, and no cached run in a retried attempt.
  - **Two failures, both from text the procedure states:** EV1 (I8, behind PgBouncer) and EV2 (I10 and I1, straight to Postgres, REPEATABLE READ).
- **Postgres 17.11 (`/opt/pgsql-17`) and 18.6 (the Manjaro package)** were throwaway clusters under my worktree's `.claude/scratch/`, on Unix sockets only (`listen_addresses = ''`), with `TMPDIR` there. One session ran `BEGIN ISOLATION LEVEL <level>; SELECT ...`, another committed `COMMENT ON SCHEMA _morm IS 'morm statement generation 2'`, and the first then read the comment from `pg_description` and through `obj_description`. On both versions:
  - **READ COMMITTED** read `2`;
  - **REPEATABLE READ and SERIALIZABLE** read `1`, and `2` after `COMMIT`.

  The servers are stopped and the scratch is deleted.
- **Not run:** PgBouncer. EV1 rests only on PgBouncer handing every client the server statement prepared under a text, which is proof 16's. Nothing else in this range rests on how PgBouncer behaves.

## EU1 to EU5, as decided and recommended

| Item | Verdict |
|---|---|
| EU1, option (a) | **Written in.** One per-pool synchronous lock, held for one step, never across I/O or an `.await`. Every `# I/O` line ends a step. `old` judges by the last-read `<n>` in the step that applies the read. A checkout is one atomic read when nothing is due, and takes a due action by one compare-and-swap, with `finish_take` and the starting bit (choices 101, 102). T-0318 has the threaded cases the review listed, and more. T-0319's events are the machine's steps. One gap is in the salt path (EV1) |
| EU2 | **Fixed as recommended.** R-0981's clause is the review's text. T-0319 names the session-class read and adds the gate's own read (row B1's `0A000` with the window closed) |
| EU3 | **Fixed.** `"primary"` is gone from R-1270, R-1289, R-1288 and T-0318 (only the historical record in `proofs.md` still names it) |
| EU4 | **Fixed.** T-0319's I3 has one-server and several-server forms, stated as I1's |
| EU5 | **Fixed.** R-1288's sentence is the review's text |

**R-1270 reads as one consolidated section:**

- the state table;
- "Steps and the stale lock";
- "Why no other interleaving matters";
- the procedure with every I/O line marked;
- the read order;
- the several-server paragraph.

Each rule is stated once, and the history is in `proofs.md` and the PR body, not in the requirement. Nothing reads as a patch on a patch. The exceptions are a missing step for the salt (EV1) and the lock-free reads' wording (EV3).

## The owner's answers, as decided

| Decision | Verdict |
|---|---|
| A cached run checks the generation word | **Written in.** The pending entry count is in the generation word. `before_run` clears both caches on a connection behind either generation. T-0318 and the I8 clearing clause check it (choices 110, 111) |
| A fresh salt on a non-rising adoption | **Written in.** The highest adopted `<n>`, the start read's included, and `on_moved`'s two branches. R-1258's `RECONNECT` paragraph has all four parts of a loud notice (choices 112, 113) |
| A confirming run ends only the confirming role | **Written in.** `end_confirm` clears the bytes only if they're still the run's, and the entry stays to its expiry, an action or a moved `<n>` (choice 114) |
| Retries run unnamed | **Written in.** The per-transaction `retried` flag, every later attempt (choice 118), everything through `query_typed` with an unnamed run's error classed as uncached (119), R-0718's bullet, R-0984, R-1253 with its cost, R-1269's benchmark, and T-0312's and T-0319's cases. My model finds no stale-statement `SchemaChanged` on a second attempt in any setting |
| One attempt per stale thing | **Written in** R-1253's README notice and R-1271's bullet, qualified by mode (choice 122). Three earlier R-1271 bullets still say the unqualified thing (EV5) |
| In the slot after `53300` | **Written in.** The epoch's `extras_ended`, set by `no_extra` for the deciding epoch only, in the step that gives the extra back. R-0981's replace bullet, R-1249's README line, T-0311's five-attempt case and T-0318's case cover it. The churn bound stays `2 × max + 2` per `W` (choice 123) |

## Loud notices and "use this when" lines

- **R-1258's new `RECONNECT` paragraph** has all four parts:
  - what it does;
  - when it's right;
  - what you give up;
  - how to proceed.
- **R-1253, R-1268 and R-1272** say what retries cost now.
- **R-1265's "use this when" line** names transactions without `.retry(n)` and an app's own retry loop.
- **No new opt-in setting or call** is added in this range, so no line is missing.

## PR body

- **Accurate:** the three new sections, choices 101 to 125, the per-round ID sentences, the checker line and both CI links.
- **Two flaws:** choices 109 and 115 aren't marked superseded (EV7), and the fault-switch count, 26, differs from T-0319's 25 (EV6).

## Findings

### EV1. Should fix. R-1270 (step 14), R-0982, R-1246, T-0318, T-0319: a *salt*'s new marker isn't set anywhere in the procedure, and when a checkout takes a due *salt* the statement generation rises before it can be set, so connections prepare the old text again under the new generation

- **Where:**
  - R-1270's `take(action)`: "compare-and-swap the generation word: raise a's generation, clear the due bits; window = ...; remove every pending and cool-down entry". No marker.
  - `checkout(conn)`: "compare-and-swap(w -> due bits cleared, the due action's generation raised, starting bit set): finish_take()".
  - `finish_take()`: "window = (now, last_read_n); remove every pending and cool-down entry; clear the starting bit". No marker.
  - The state table's marker row: "given a new salt by the gate's *salt* action".
  - "A checkout changes the state only by its compare-and-swap".
- **What's wrong:**
  - **The salt has no place.** The procedure never sets the salt. R-0982, R-1246 and E5 say a *salt* adds a fresh salt to the marker and raises the statement generation, in that order. In a step, the order is only implied.
  - **A checkout can't follow that order.** A checkout's take is lock-free and changes only the generation word, so the only step where the salt could reach the marker is `finish_take`, after the compare-and-swap has already raised the statement generation.
  - **The gap.** Between the two, any connection's checkout or cached run sees the raised generation, clears its caches, and prepares its statements under the old marker. The checkout that won the compare-and-swap does so itself, before it reaches the lock.
  - **The old text is the stale one.** That text is exactly the one whose server statements PgBouncer still holds stale, the reason the salt was asked for. Those connections now cache it under the new generation, so nothing clears it until the next pool-wide action, at least `stale_cooldown` later.
  - **What breaks:**
    - R-1271's "a cached run never uses a copy a pool-wide action was taken to clear";
    - T-0319's I8 clearing clause;
    - with a type swap made by hand that kept the old type, I9's bound on silent wrong answers ("within `stale_cooldown` of the pool's first confirmation").
  - **No test catches it.** T-0318's threaded checkout case uses a due *clear*. T-0319 has no fault switch for it. `proofs.md`'s model reports no I8 failure, so it can't have set the salt where the text allows.
- **Evidence:**
  - **The trace,** behind PgBouncer, from my model of the head:
    - every connection holds `S` prepared under `g1.<s1>`;
    - a change made by hand at 0.5 s leaves those server statements stale;
    - a confirmation at 0.6 s inside the window makes *salt* due;
    - the window ends at 1.1175 s. Checkout C wins the compare-and-swap at 1.1180 s and checkout D lands at 1.1183 s, both before `finish_take`;
    - both clear their caches and prepare `S` under `g1.<s1>` again, and get the stale server statement's `0A000`;
    - `finish_take` then makes the marker `g1.<s2>`;
    - at 1.5 s D still runs `g1.<s1>`, gets `0A000` again, and *salt* is due again. That's I8's clearing clause broken three times.
  - **Random runs,** 32 sequences of 20 s per setting, behind PgBouncer: I8 fails in 7 sequences with the defaults, 25 with entries off, 20 with a short expiry and 8 with caps of 1.
  - **With the fix below,** the trace gives D the new text at 1.5 s, and every setting fails no sequence.
- **Fix:** no owner choice is needed. It completes choice 101's mechanism.
  - **The rule, in R-1270's "Steps and the stale lock":** a statement generation is raised only after the marker it's raised for is set.
  - **`take`:** for a *salt*, `marker = g<last_read_n>.<fresh salt>` before its compare-and-swap.
  - **`checkout`:** its compare-and-swap, once the window has ended and the starting bit is clear, sets the starting bit and leaves the due bits and both generations as they are. So two checkouts still can't both take the action.
  - **`finish_take`:** in that one step, set the fresh salt for a due *salt*. Then, by compare-and-swap, raise the due action's generation and clear the due and starting bits. Then set the window and remove the entries.

  Raising *clear*'s and *replace*'s generations there too keeps one rule. The action takes effect microseconds later, and I1 is unchanged, because the window still starts in `finish_take`.
  - **The prose.** R-0982's checkout bullet and E4, and the state table's generation-word row, follow.
  - **T-0318 adds a threaded case:** a due *salt* taken by a checkout, with a test hook holding `finish_take` while another thread checks out and runs a cached statement. That run doesn't clear its caches, and once `finish_take` ends, the next cached run prepares the new salted text.
  - **T-0319 adds a fault switch:** "a checkout's take raising the statement generation before `finish_take` sets the salt", caught by I8 behind PgBouncer.

### EV2. Should fix, needs an owner choice. R-1270, R-0982 (step 14), T-0319 (step 20): straight to Postgres, the gate's own read can run inside the app's transaction, whose REPEATABLE READ or SERIALIZABLE snapshot is older than the read, so a read sent after the last applied one adopts an older `<n>`

- **Where:**
  - `before_run`'s confirming run: `request(pooled ? Salt : Clear, unread = true)`.
  - `request`: `if unread and !pooled: mv = read(conn)`.
  - R-0982: "Straight to Postgres, a request whose call hasn't read `<n>`, made while the window is closed, reads `<n>` on that connection".
  - T-0319: "straight to Postgres inside a transaction `<n>` is read only for row B2's old-connection test and by a request while the window is closed".
  - "Why no other interleaving matters": "a read whose ticket is at or above the horizon was sent after the last applied read was applied, and its snapshot is newer than that read's".
  - The read order: "applied reads' snapshots come in order".
- **What's wrong:**
  - **The read runs inside the attempt.** With retries now unnamed, an entry's confirming run is often a statement of `.retry(n)`'s later attempt, inside the app's open transaction. When it confirms with the window closed, the gate reads `<n>` on that connection, in that transaction.
  - **Its snapshot can be old.** Under REPEATABLE READ or SERIALIZABLE, the step 20 pattern `db.isolation(Serializable).retry(3)`, a query on `pg_description` sees the transaction's snapshot, taken at the attempt's first statement. That's even true of the confirming run itself, when it's the first statement, whose snapshot is its own start.
  - **The ticket order's premise fails.** The read can be sent after the last applied read and still carry an older snapshot. In a pool that reaches one server, if the pool has already applied a newer `<n>`, this read is applied as a lowering: `on_moved` sets the last-read `<n>` back.
  - **Then the bounds break:**
    - the gate passes, since the last-read `<n>` differs from the window's;
    - the next ordinary read moves `<n>` up again and passes the gate once more;
    - each move also lets the next session-class failure start an epoch;
    - so one migration gives three pool-wide actions within `stale_cooldown`, against R-0982's "plus one per migration", I1 and I10, and extra epochs, against I3 and R-0981's "plus `max` per migration".
  - **Which reads are affected.** Reads after a rollback (rows B2, B3 and D1) and outside transactions are unaffected. READ COMMITTED takes a fresh snapshot per statement and is unaffected. Several-server pools refuse a lower `<n>`.
  - **No model can show it.** T-0319 doesn't model isolation levels, so neither model could see this.
- **Evidence:**
  - **Postgres 17.11 and 18.6:** inside REPEATABLE READ and SERIALIZABLE, the `_morm` comment read after another session committed `2` returns `1`, both from `pg_description` and through `obj_description`. READ COMMITTED returns `2`.
  - **My model's trace,** straight to Postgres, one server:
    - an action at 0 s under 1, so the window is closed to 1 s;
    - `.retry(1)`'s second attempt of a REPEATABLE READ transaction T starts at 0.2 s;
    - a migration at 0.25 s;
    - another call's request reads and applies 2 at 0.2717 s and takes *clear*;
    - a change made by hand at 0.29 s, and call X's `22P02` at 0.30 s makes the entry with T's values;
    - T's confirming run at 0.353 s confirms, and the gate's read inside T finds 1. It's applied (`<n>` 2 to 1), and *clear* is taken at 0.3531 s;
    - the next request's read at 0.4012 s finds 2 (`<n>` 1 to 2), and *clear* is taken again.

    That's three actions in 130 ms for one migration: I10 fails (a snapshot of 0.200 s applied after one of 0.271 s), and I1 fails twice. Under READ COMMITTED the same trace takes one action and leaves T's request due.
  - **Random runs** with half the attempts REPEATABLE READ and stress: I10 and I1 fail in 1 of 32 sequences. With READ COMMITTED: none.
- **Fix:** which read to drop or move is a design choice, so it goes to the owner.
  - **(a), recommended: a request made inside a transaction never reads `<n>`.** It's judged by the pool's last-read `<n>`, as every request is behind PgBouncer. R-0982's list of reads and T-0319's sentence change to "and by a request outside a transaction while the window is closed".
    - **Unchanged:** the reads after a rollback (rows B2, B3 and D1), and every read outside a transaction.
    - **Tests.** T-0318 adds a case: a confirming run inside a REPEATABLE READ attempt, with the window closed, makes no read. T-0319 runs half its attempts as REPEATABLE READ, with the snapshot at the attempt's first statement.
    - *You give up:* when the first thing in the pool to see a migration's moved `<n>` is a confirming run inside a transaction, its *clear* is due until the window ends, or until the next read outside a transaction adopts the move, not taken at once. R-1271's "cleared within `W` of the first confirmation" still holds.
  - **(b) The read is moved after the transaction ends.** The confirming run inside a transaction records its request. When the transaction commits or rolls back, before its connection goes back to the pool, morm reads `<n>` on it and judges the request in the step that applies that read.
    - **Keeps** "a migration's stale state is never held back".
    - *You give up* simplicity: a deferred request carried per transaction, a catalog read after the commit, and the commit's latency by one round trip on that path.
  - **(c) Keep the read, and treat any read made inside a transaction as overlapping:** it counts as the last-read `<n>`. *You give up:* the same as (a), plus a wasted round trip. I don't recommend it.

### EV3. Minor. R-1270 (step 14), R-0718, R-1269 (step 48): the lock-free reads are called "relaxed", and the marker's read isn't among them, though what they guard needs acquire and release

- **Where:**
  - `before_run`: "one relaxed atomic read, no lock".
  - "Three things run with no lock": a ticket, the pending check ("one relaxed atomic read of the generation word"), and a checkout (the word, then the window's end).
  - The same "relaxed" is in R-0718's cost paragraph and R-1269.
  - "The procedure below is the only code that reads or changes the pool's stale-session state". Yet every prepare behind PgBouncer reads the marker, with no lock.
- **What's wrong:**
  - **Relaxed doesn't order other memory.** A connection that sees a raised statement generation by a relaxed load has no happens-before with the step that set the marker before raising it. Under Rust's memory model, and in practice on aarch64, it can prepare under the old marker, which is EV1's stale copy with no checkout involved.
  - **The window's end too.** A checkout that sees due bits by a relaxed load can read an older window's end, and take an action inside the new window, against I1.
  - **Built literally, it's a bug.** A builder following "relaxed" writes `Ordering::Relaxed`, and the model in T-0319, being sequential, can't show it.
  - **The marker read is missing from the list.** The state table and "Steps and the stale lock" don't list it among the lock-free reads, though the procedure's rule says only steps read the state.
- **Fix:** no owner choice is needed.
  - **Name the orderings.** Every lock-free read of the generation word, in a cached run's pending check and in a checkout, is an acquire read. Every compare-and-swap on it, in a step or a checkout, is acquire-release.
  - **List the marker read.** "Three things" becomes four: a prepare's read of the marker, behind PgBouncer, after the cached run's read of the generation word.
  - **Say why.** One sentence in "Why no other interleaving matters": a connection that sees a raised generation sees the marker and window it was raised for.
  - **The cost wording.** R-0718's and R-1269's "relaxed" become "acquire". On x86-64 an acquire load is the same instruction, so R-1269's "within noise" stands.

### EV4. Minor, needs an owner choice. R-1270 (step 14): a start read that fails leaves the last-read `<n>` unset, against "set before any call reaches the gate"

- **Where:**
  - The state table's last-read row: "set by the start read, ... so it's set before any call reaches the gate".
  - `read`: "if the read failed: return false # it applies nothing; its call goes on, judged by the last-read `<n>`" (choice 108).
  - `arrive`: "if last_read_n is unset: # the start read, before any call".
- **What's wrong:** nothing says what a pool does when its start read fails (a statement timeout, a cancel, a recovery conflict on a standby) while its first connection works.
  - **Read literally,** the pool starts with the last-read `<n>` unset, and calls reach the gate and `old` with it unset.
  - **The first read that later arrives** takes `arrive`'s start-read branch: it skips the ticket test and calls no `on_moved`, even though calls have run.
  - **`old` then starts a second epoch at once,** because the epoch's `<n>` (unset) differs from the last-read `<n>`, against I3. The gate's "`<n>` differs from the window's" behaves the same way.
- **Evidence:** the lines quoted. No requirement or test covers a failed start read.
- **Fix:** what a pool does when its start read fails is a behaviour choice, so it goes to the owner.
  - **(a), recommended:** a failed start read fails the pool's start with its error, as a failed first connection or startup statement does, so the state table's sentence is true. T-0318 adds the case. *You give up:* a pool that can't run one catalog query at start doesn't start; a retry by the app starts it.
  - **(b) The pool starts with the last-read `<n>` unknown.** Until a read applies:
    - every request is judged by the window alone;
    - `old` treats unknown as equal to any epoch's `<n>`;
    - the first applied read sets the last-read `<n>` as the start read does, with no `on_moved`.

    *You give up* simplicity: a third value in the state, and a migration in that gap isn't seen as a move.

### EV5. Minor. R-1271, T-0319 (step 20): the per-kind bullets still say "none is visible under `.retry(n)` with `n` at least 1", against the bullet "One attempt per stale thing"

- **Where:**
  - R-1271's bullets on stale statements straight to Postgres, stale sessions straight to Postgres, stale statements behind PgBouncer and a per-transaction `search_path` each say none is visible under `.retry(n)` with `n` at least 1.
  - "One attempt per stale thing": "a deadlock or serialization failure costs its own attempt ... So straight to Postgres `.retry(1)` hides a stale statement or a stale session, and `.retry(2)` hides a stale statement and a stale session together".
  - T-0319's I8: "a stale session straight to Postgres is never visible ... under `.retry(n)`".
- **What's wrong:**
  - **By R-1271's own definition, `SchemaChanged` can be visible.** An error is "visible" when it reaches the caller and a fresh session wouldn't return it. Under `.retry(1)`, a transaction whose first attempt gets a serialization failure and whose second meets a stale statement returns `Error::SchemaChanged`, and so does one that meets a stale statement and a stale session. Both are visible.
  - **The bullets contradict "One attempt per stale thing",** which states this correctly, as the owner's decision (2) has it. T-0319's I7 is stated precisely; the bullets aren't.
- **Fix:** no owner choice is needed.
  - **In each of those bullets,** replace "none is visible under `.retry(n)` with `n` at least 1" with "none is visible under `.retry(n)` with `n` at least the attempts the bullet below counts, one for a transaction's stale statements together".
  - **The same in T-0319's I8.**

### EV6. Minor. T-0319 (step 20), `proofs.md`, INDEX, PR body: T-0319 has 25 fault switches, while the record says 26

- **Where:**
  - **T-0319:** "for five broken forms of the stale lock". The fourth of the five is "a step that writes the generation word with a plain store and never finishes a checkout's take". The total is 8 + 6 + 5 + 3 + 3 = 25.
  - **The other three:** `proofs.md`'s retry round lists "no starting bit, 66 and 7, I1; a plain store into the word, 66 and 7, I1" as two switches. INDEX and the PR body say "all 26 fault switches are caught".
- **What's wrong:**
  - **The model ran two switches where T-0319 has one.** The model justified the stale lock with two separate switches; T-0319 merges them.
  - **A merged switch proves less.** One that breaks two rules at once doesn't show the checker catches either break alone, so a builder following T-0319 builds a weaker test than the model ran.
  - **The counts disagree:** 25 in T-0319, 26 in the record.
- **Fix:** no owner choice is needed. Split T-0319's switch into "a checkout's take never finished by a step, no starting bit, I1" and "a step that writes the generation word with a plain store, I1", and say "six broken forms of the stale lock".

### EV7. Minor. PR body: choices 109 and 115, and a sentence in the stale-copy section, still read as current

- **Where:**
  - **Choice 109:** "R-1271's I7 condition `stale_pending_expiry` at least `stale_cooldown`". Its added note says only the "three other cases" are answered.
  - **Choice 115:** "R-1271 states what's left as the procedure stands, pending the open questions below ... R-1268's says an expiry below `stale_cooldown` stops `.retry(1)` always hiding a stale statement".
  - **The section "The owner's answers on the three stale-copy cases":** "They're the open questions below" and the same R-1268 sentence.
- **What's wrong:**
  - **Choice 120 supersedes them.** I7's conditions are dropped; R-1271 no longer states what's left; R-1268 now speaks of transactions without `.retry(n)`; and there are no open questions.
  - **The convention isn't followed.** Earlier rounds marked such choices superseded (84 and 85, ER7, ES5). These three aren't marked, so the owner approves text that no longer describes the plan.
- **Fix:** mark choices 109's condition and 115 "superseded by choice 120", and the section's sentence "(answered since, option (a) each, in the next section)", as the EU1 section's are.

## Summary

| Severity | Findings |
|---|---|
| Blocks approval | None |
| Should fix | EV1; EV2 (needs an owner choice: (a) recommended) |
| Minor | EV3, EV5, EV6, EV7; EV4 (needs an owner choice: (a) recommended) |

What's written in:

- **EU1 to EU5** are fixed as decided and recommended.
- **The owner's six answers** in this range are written in as decided.
- **R-1270 reads as one consolidated section.**
- **The checks agree.** The changed IDs match the three INDEX entries ID for ID. The checker reports 1291 requirements, 329 test obligations and 0 problems. CI is green on `13bdab6`, 36 of 36 jobs.
- **My model agrees with R-1271 and T-0319's I7 where it reaches.** No stale statement reaches a caller outside a transaction, no attempt after `SchemaChanged` runs a cached copy, and no transaction gets `SchemaChanged` from stale statements twice, in any setting. I1, I3 and I10 hold everywhere except EV2's isolation case.

**PR #3 isn't ready for the owner yet.** Two should-fix findings remain:

- **EV1** is a gap in the salt path, fixable with no owner choice. As written, a checkout's take of a due *salt* raises the statement generation before the salt can reach the marker, so behind PgBouncer connections re-prepare the stale text under the new generation.
- **EV2** is a question for the owner, with options. Straight to Postgres, the gate's read inside a REPEATABLE READ or SERIALIZABLE attempt sees an old snapshot, and one migration can then pass the gate three times. That's shown on Postgres 17.11 and 18.6.

The five minor findings are wording, test-text and PR-body fixes, two of them small owner questions. Once EV1 and EV3 to EV7 are fixed and EV2 and EV4 are answered and written in, a short check of those edits, the checker and CI should be enough for PR #3 to go to the owner.
