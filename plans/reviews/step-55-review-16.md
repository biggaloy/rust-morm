# Step 55 independent review, sixteenth round

PR #3 (`step-55-enum-proofs`), head `7ed642b`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes.

This round covers `git diff 80a671e origin/step-55-enum-proofs`, four commits:

- `ba8379c`: ES1 and ES2;
- `0cbe4d5`: ES3 and ES4;
- `30adccd`: `proofs.md` records the round;
- `7ed642b`: ES2's wording, the start read's example and T-0319's B3 case.

I also checked that nothing earlier regressed.

## What I read

- **On the PR branch:** `rules.md`, `contracts.md`, and `proofs.md` (step 55's ES bullets and the `0A000` decision's bullet they qualify). I read every changed ID in full:
  - **Changed:** R-0718, R-1116, R-1245, R-1270, R-1271, R-1272, R-1277 (manifest only), R-1280, R-1282, R-1283, T-0312, T-0318, T-0319 and T-0325.
  - **For coherence:** R-0060's block and the shipped-file rules under it, R-0062, R-0066 to R-0079 (the `MORM_DB_*` and `MORM_MIGRATION_*` variables), R-0111, R-0981, R-0982 with its events, R-1278 and R-1284.
- **On `origin/main`:**
  - `reviews/step-55-review-15.md`;
  - INDEX's Decisions entry "Fifteenth review fixed (ES1 to ES5)", with its "written in at `7ed642b`" list and the fix's choices 67 to 73;
  - the entries it builds on: "`init` writes a found socket loudly (ER2)", "`0A000` requests a repair once", "`sslmode` defaults to `require` (EQ1)", "Per-transaction `search_path` (EQ2)" and the stale-session machine's MA to ME.
- **The PR body.**

## What I checked and ran

- **CI on the head.** `7ed642b` is green: `ci` on push (run 44319201215) and on the pull request (run 44319214025), 18 of 18 jobs each, 36 of 36. The diff touches only `plans/plan-approved/`: `coverage.md`, `proofs.md`, `manifest.json` and steps 10, 14, 20, 41, 42 and 50.
- **Coverage checker,** run on a `git archive` of the head, `.github` included. `check_coverage.py --step 55 --code crates tests benches fuzz .github` reports **1287 requirements, 328 test obligations, 0 problems**, and the plain form reports the same. That's the expected total.
- **Manifest against INDEX.** I compared `git diff 80a671e origin/step-55-enum-proofs -- plans/plan-approved/tools/manifest.json` ID by ID:
  - **Changed:** R-0718, R-1116, R-1245, R-1270, R-1271, R-1272, R-1277, R-1280, R-1282 and R-1283 (`text` and `block`), and T-0312, T-0318, T-0319 and T-0325 (`text`).
  - **Nothing else moved:** no ID added or removed, and `frame` and `master_sha256` are unchanged.

  That's exactly INDEX's list for `7ed642b`, and the PR body's "Since the fifteenth review's head `80a671e`" sentence names the same fourteen.
- **R-1277's manifest-only change (choice 73).** At `80a671e`, R-1277's `text` and `block` both ran on into R-1282's whole paragraph. At the head they hold R-1277 alone. I checked the whole manifest:
  - every requirement's `block` appears word for word in its step file or `contracts.md`;
  - no block holds a second `**R-nnnn**` marker.

  So the error is gone and has no twin, and R-1277's step text is unchanged in the diff. R-1283's `text` now matches its new block. The PR body explains the change as choice 73, and it's accurate.
- **ES3's model figures.** I wrote my own small model of the gate, the window and the pending entries: in-transaction `0A000`s, switches every 100 ms, the request at the failure, the entry after it, and an action ending every entry. It gives what `proofs.md` records:

  | Case | Failures | Actions |
  |---|---|---|
  | Two statements, `stale_cooldown = "3s"`, `stale_pending_expiry = "1s"`, 30 s | 30 each | 10 |
  | Two statements, `"1s"` and `"3s"`, 30 s | 30 each | 30 |
  | Two statements, `"3s"` and `"3s"`, 30 s | 10 each | 10 |
  | Two statements, `"10s"`/`"1s"`, 5 s | 5 each | |
  | One statement repeating its `id`, 5 s | 1 | |
  | One statement, `"1s"`/`"3s"`, 6 s | 6 | |

  T-0312's bounds stand.
- **ET1's trace.** I wrote a second small model: R-1270's `request`, `take` and `on_moved` straight to Postgres, exactly as written at the head. It reproduces ET1. Applying the recommended fix removes the second and third actions.
- **Not run:** nothing in this range rests on how Postgres or PgBouncer behaves. The texts that changed are the machine's procedure, rates derived from it, a notice's wording, and test specs. So I started no Postgres or PgBouncer. The fifteenth round's runs (PgBouncer 1.26.1 from the checked tarball, Postgres 18.6, Cargo 1.99.1) cover the evidence this range leaves unchanged. The model files and the archive were in `.claude/scratch/` in my worktree and are deleted.

## Each ES, as recommended

| Finding | Verdict |
|---|---|
| ES1 | **Fixed as recommended.** In `on_error`'s row B2, `entry(..)` runs first and clears the *replace* bit. `expect_confirm(call, Replace, n)` then finds the entry with the call's bytes and sets the bit. `before_run`'s confirming run makes one request, *replace* if the bit is set and *clear* otherwise. *Replace* and the `0A000` mark can't both be set: the mark comes only from a `0A000`, and the bit only after an `XX000`'s unmarked `entry(..)`. With no entry, because the read found `<n>` moved or entries are off, the mark goes to the next attempt. The texts agree: R-0718's row B2 ("one or the other, never both") and its pending-entry and entry bullets, R-1245's sentence, and R-1270's state table and prose. T-0319's unit test asserts one *replace* and nothing due on an old connection, and one *clear* on a connection that isn't old. T-0312's B2 asserts no *clear* beside the *replace*. One new defect comes with the `<n>` the mark carries (ET1) |
| ES2 | **Fixed as recommended.** R-1270's last-read row reads `<n>` when every pool starts, caches on or off, straight and behind PgBouncer. R-1116 does the same "in every pool", keeps the marker behind a pooler with the caches on only, and covers `check_schema = "off"` with step 14's own query. The false sentence is gone. `expect_confirm` carries the failure's `<n>`. T-0319's settings add `statement_cache = 0` and `shape_cache = 0` with a row B3 case. T-0318 asserts the start read in both modes, caches on and off. Carrying the failure's `<n>` into a later attempt is what lets ET1 through |
| ES3 | **Fixed as recommended.** R-1271's bullet, R-1272's fourth bullet, its "What you give up" and its advice now say: once per `stale_cooldown`, or once per `stale_pending_expiry` when that's shorter. Raising `stale_pending_expiry` alone helps only up to `stale_cooldown`. `proofs.md` notes that the `0A000` bullet's "about once per `stale_cooldown` each" holds only with `stale_pending_expiry` at least `stale_cooldown`. I found no other statement of the old rate. The model confirms the figures |
| ES4 | **Fixed as recommended.** R-1283 is retitled. It names every `[database]` field and says what a written `name`, `user`, `password` or `sslmode` does. "What you give up" says a weaker written `sslmode` drops the TLS the URL asked for, naming R-1278's notice. "How to proceed" gives the `MORM_DB_<FIELD>` rule and `[database.migration]`'s two variables. Each matches R-0060's shipped-file rule and R-0066 to R-0079: `MORM_DB_NAME`, `MORM_DB_USER`, `MORM_DB_SSLMODE`, `MORM_MIGRATION_USER` and `MORM_MIGRATION_PASSWORD` all exist. R-1282 and R-1280 name the new title, and nothing on the branch keeps the old one. T-0325 adds `sslmode = "disable"` against `?sslmode=require`: the file wins, and one warning names both modes. The notice keeps all four parts |
| ES5 | **Fixed.** The "Elsewhere" line on R-1269 is marked "superseded by EQ2" with what R-1269 now measures. The "don't change" sentence is scoped to the consolidated rewrite and notes R-1116's later changes by ER1 and ES2 |

### Loud notices and "use this when" lines

- R-1283 is loud with all four parts, now for every `[database]` field.
- R-1272 keeps its four parts with the corrected rates and advice.
- R-1284 stays about `host` and `port`, as choice 71 says. Writing another field's value isn't a new opt-in setting: `sslmode = "disable"` already has R-1279.
- No new opt-in call or setting lacks a line.

### Still faithful

- **No regressions.** Nothing outside the fourteen IDs changed. R-0060's block and its copies are untouched.
- **Rows and bounds.** The rows A1 to D3, the gate's events E1 to E8 and R-1271's bounds read as before, apart from ET1 and ES3's corrected rate.
- **Cross-references and wording.** No step file still says "with the caches off no call reaches the gate unread" or "whatever `stale_pending_expiry` is". The checker is clean.

### PR body

These parts are accurate:

- the section "The fifteenth review, ES1 to ES5";
- choices 67 to 73;
- the per-round ID sentence;
- "Open questions: None";
- the end-to-end result: the checker line, both CI links, 18 of 18 jobs each, and the use-when list.

Choices 67 and 70 describe the `<n>` the requests carry, which ET1's fix changes, so they will need a line once ET1 is fixed.

## Findings

### ET1. Should fix. R-1270 `expect_confirm` and `request`, R-0982 (step 14), T-0319 (step 20): a request carrying the `<n>` an earlier failure read can pass a closed window with `<n>` unmoved

- **Where:**
  - R-1270's `session()`, row B3: `n = read_n(conn)` at the failure, then `discard(conn); if !sure: expect_confirm(call, Replace, n)`.
  - `on_error`'s row B2 with no entry made calls `expect_confirm(call, Replace, n)` with the `<n>` read at the failure, too.
  - `expect_confirm`, the no-entry branch: "mark the transaction's next attempt ... if it confirms, requests (action, n)".
  - The prose: "Either way the request carries the `<n>` the failure read: kept with the attempt's mark".
  - `request`: `if window_open() or (n is not Unread and n != window.n): take(max(action, due), n)`.
  - T-0319's new case: B3's next attempt "requests *replace* under the `<n>` its failure read, with no read of its own".
  - R-0982: "a request takes an action only when the window is open or `<n>` moved since the last one, so two actions less than `stale_cooldown` apart have a migration between them"; and R-1271's I1.
- **What's wrong:**
  - **The gate tests "differs", not "newer".** The mark keeps the failure's `<n>` across `.retry(n)`'s backoff, so by the next attempt it can be older than the window's `<n>`, and it still passes.
  - **The trace,** straight to Postgres, with the gate open and the window's `<n>` = 1:
    - t = 0: a B3 `22P02` on an old connection reads `<n>` = 1, unmoved, and marks the next attempt with (*replace*, 1).
    - t = 0.05 s: a migration raises `<n>` to 2.
    - t = 0.06 s: another call's `42804` on an old connection reads 2. That read calls `on_moved`, and its `request(Replace, 2)` takes the migration's one action, so the window's `<n>` becomes 2.
    - t = 0.10 s: B3's next attempt confirms and calls `request(Replace, 1)`. The window is closed, but 1 isn't Unread and 1 ≠ 2, so the request takes a **second** *replace*, and the window's `<n>` goes back to 1.
    - t = 0.20 s: any Unread request reads 2, which isn't moved against the last-read `<n>` of 2, but 2 ≠ 1, so it takes a **third** action.

    That's three pool-wide actions in 0.2 s with one migration. It breaks I1 and R-0982's stated bound, and every connection is replaced twice for nothing. My model of the written procedure gives exactly this.
  - **The input is the client's.** B3's `22P02` can come from stored or client text, so a client can repeat the trace once per migration.
  - **The test contradicts the procedure.** T-0319's simulation generates migrations, calls and `.retry(n)` attempts in any order and checks I1 after every event. Against the procedure as written it should find this sequence and fail, and step 20's builder would have no text saying which side is right.
  - **The cause is older than this round.** Row A2's `request(Replace, n)` carries a `<n>` read before `replace(conn, call)` opens a connection and runs the call. That gap is short, inside one call, but another call's action in it slips through the same way.
- **Why the entry path is safe:** the *replace* on a B2 entry uses the pool's last-read `<n>`. That value is always the latest read, so it's never older than the window's `<n>`.
- **Evidence:** the lines quoted above, R-0982's bound and the trace. A copy of the model with the fix below gives one action.
- **Fix:** no owner choice is needed. This restores R-0982's bound as decided (SF, EO1, EO2, MB).
  - **The rule.** In `request`, a known `<n>` counts as the pool's last-read `<n>`. Every read that finds `<n>` moved has already called `on_moved`, so a `<n>` a call carries can only equal or lag the last-read one. For example, begin `request` with `if n is not Unread: n = last_read_n`, with a comment saying why.
  - **The mark.** `expect_confirm`'s mark then needs no `<n>` of its own. Its request, like the entry's, uses the pool's last-read `<n>` and makes no read. Choice 70's "no read of its own" stands.
  - **The prose.** Replace "Either way the request carries the `<n>` the failure read ..." with: "the request uses the pool's last-read `<n>`, which equals the failure's while nothing newer has been read, and is newer if something has been, so no request passes the gate with an older `<n>`". Align `proofs.md`'s ES2 bullet and PR choices 67 and 70.
  - **R-0982.** After "a request takes an action only when the window is open or `<n>` moved since the last one", add: "the `<n>` a request is judged by is the pool's last-read one, never an older one a call carried".
  - **The tests.**
    - **T-0319's unit test of rows B and D,** straight to Postgres: B3's `22P02` on an old connection under `<n>` = 1; a migration raising `<n>` to 2; another call's `42804` on an old connection taking an action under 2; then B3's next attempt confirms. Its *replace* is due, not taken, and the window's `<n>` stays 2.
    - **The same with row B2 and no entry,** with entries off.
    - **T-0318:** a request carrying a `<n>` older than the pool's last-read one, inside a window, is due, not taken.

## Summary

| Severity | Findings |
|---|---|
| Blocks approval | None |
| Should fix | ET1 |
| Minor | None |

What's written in:

- **ES1 to ES5** are each fixed as the fifteenth review recommended. The R-1277 manifest correction is accurate and has no twin.
- **The checks agree.** The changed IDs match INDEX's "Fifteenth review fixed" entry ID for ID. The checker reports 1287 requirements, 328 test obligations and 0 problems. CI is green on `7ed642b`, 36 of 36. The PR body is accurate.
- **The loud notices and "use this when" lines** are complete.

**PR #3 isn't ready for the owner yet.** One finding remains, ET1, and its fix needs no owner choice. It's a gap in the stale-session procedure that this round's ES2 fix opened: a request carrying an older `<n>` passes the gate, and a step-20 test as specified would catch it. Once ET1 is fixed and a short check confirms the edit, the checker and CI, PR #3 is ready for the owner.
