# Step 55 independent review, fifteenth round

PR #3 (`step-55-enum-proofs`), head `80a671e`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes.

This round covers `git diff 8abcd9d origin/step-55-enum-proofs`, six commits:

- `7f8d9b2`: the fix for the fourteenth review's ER1 to ER7, with the owner's ER2 decision;
- `f0ad481`, `a373c7a` and `2e62d43`: the answers "`0A000` requests a repair once" and "`morm-admin` always carries TLS";
- `4431bf3`: "`morm-migrate` always carries TLS; `morm-mgr` turns on `tls`";
- `80a671e`: "`morm-mgr` is a member of the app's workspace".

I also checked that nothing earlier regressed.

## What I read

- **On the PR branch:** `rules.md`, `contracts.md` (the crate table and R-0019), and `proofs.md` (step 55's ER bullets, the three later decision bullets, "TLS over a Unix socket: evidence" and the new "PgBouncer's admin console without client TLS"). I read every changed and new ID in full:
  - **Changed:** R-0019, R-0062, R-0102, R-0107, R-0718, R-0981, R-0982, R-1116, R-1245, R-1258, R-1270 to R-1273, R-1277, R-1278, T-0312, T-0316, T-0318 to T-0320 and T-0323.
  - **New:** R-1280 to R-1287 and T-0324 to T-0328.
  - **For coherence:** R-0048, R-0058, R-0060's block, R-0061, R-0104 to R-0108, R-0111, R-1246, R-1269, and steps 29, 33 and 42's new deliverable lines.
- **On `origin/main`:**
  - `reviews/step-55-review-14.md`;
  - INDEX's Decisions entries "`init` writes a found socket loudly (ER2)", "`morm-admin` always carries TLS", "`0A000` requests a repair once", "`morm-migrate` always carries TLS; `morm-mgr` turns on `tls`" and "`morm-mgr` is a member of the app's workspace", each with its "written in at" list and the fix's own choices 43 to 66;
  - the entries they build on: "`sslmode` defaults to `require` (EQ1)", "Per-transaction `search_path` (EQ2)", "The settings file is a real `morm.toml`" and the stale-session machine's MA to ME.
- **The PR body.**

## What I checked and ran

- **CI on the head.** `80a671e` is green: `ci` on push (run 44313524358) and on the pull request (run 44313538816), 36 of 36 checks passing. The diff touches only `plans/plan-approved/`: `contracts.md`, `coverage.md`, `proofs.md`, `manifest.json` and steps 10, 14, 20, 29, 33, 41, 42, 50 and 59.
- **Coverage checker,** run on a `git archive` of the head, `.github` included. `check_coverage.py --step 55 --code crates tests benches fuzz .github` reports **1287 requirements, 328 test obligations, 0 problems**, and the plain form reports the same. That's the expected total.
- **Manifest against INDEX.** I compared `git diff 8abcd9d origin/step-55-enum-proofs -- plans/plan-approved/tools/manifest.json` ID by ID:
  - **Changed:** R-0019, R-0062, R-0102, R-0107, R-0718, R-0981, R-0982, R-1116, R-1245, R-1258, R-1270, R-1271, R-1272, R-1273, R-1277, R-1278, T-0312, T-0316, T-0318, T-0319, T-0320 and T-0323.
  - **New:** R-1280 and R-1281 (`code`, step 42), R-1282 (`code`, step 10), R-1283 and R-1284 (`doc`, step 50), R-1285 (`code`, step 33), R-1286 (`code`, step 29), R-1287 (`code`, step 42), T-0324 (`suite`, 42), T-0325 (`suite`, 10), T-0326 (`integration`, 33), T-0327 (`integration`, 29) and T-0328 (`suite`, 42).
  - **Nothing else moved:** no ID was removed, and `frame` and `master_sha256` are unchanged.

  That's exactly the union of the four "written in at" lists: `7f8d9b2`, `2e62d43`, `4431bf3` and `80a671e`, whose R-1287 and T-0328 are new in this range anyway.
- **PgBouncer's admin console without client TLS** (`proofs.md`'s new evidence, the basis of R-1258's and T-0316's new text):
  - **Setup.** I used PgBouncer 1.26.1, built from pgbouncer.org's `pgbouncer-1.26.1.tar.gz`. Its SHA-256, `00886a2994b4844cd37110970df777772cfc42a74c400522c82fba509a3f42d9`, matched the published `.sha256`. It ran in transaction mode with `client_tls_sslmode` unset, which `SHOW CONFIG` reported as `disable`, in front of a throwaway Postgres 18.6 with `ssl = off`. That Postgres is the Manjaro package; no Postgres 17 binary is installed here, and I built none. Both lived under `.claude/scratch/` in my worktree, with `TMPDIR` set there, their sockets under `~/.claude/scratch/` and their ports on `127.0.0.1` only.
  - **The probe** was built offline with `tokio-postgres` 0.7.18 and `NoTls`. Over TCP and over the Unix socket alike:
    - the admin console with `disable` or `prefer` connects, and `RECONNECT` completes;
    - with `require` it fails with `error performing TLS handshake: server does not support TLS`;
    - a pooled database with `disable` and `prefer` returns `SELECT version()`, 18.6, and with `require` fails the same way.
  - **`psql` 18.6** with `sslmode=require` over TCP fails with `server does not support SSL, but SSL was required`, and over the socket sends `RECONNECT`.
  - **Result.** It all matches `proofs.md`'s table. The servers, the build and the probe were deleted.
- **Cargo feature unification** (the evidence behind R-1287, R-1278 and T-0328). I used a throwaway workspace under `.claude/scratch/`, Cargo 1.99.1, offline, with resolver 3. A stand-in `morm` had a `tls` feature the app's binary prints, and `morm-mgr` depended on it with `features = ["tls"]`. The result matches `proofs.md`:

  | Layout | `-p app` | `--workspace` | Plain build at the root | Plain build in the app's folder |
  |---|---|---|---|---|
  | Virtual root | off | on | on | off |
  | App's package at the root | off | on | off | (same as the root) |

  Each layout had one `Cargo.lock`, at its root. The probe was deleted.
- **Not run:** Postgres 15 to 17; PgBouncer 1.20.1 and 1.22; a PgBouncer with client TLS on; a real rustls handshake. T-0316, T-0323, T-0326 and T-0327 run these where they're built.

## Each ER, as decided or recommended

| Finding | Verdict |
|---|---|
| ER1 | **Fixed as recommended.** R-1270's state table has the last-read `<n>`, read at pool start through R-1116's statement (or step 14's own query with `check_schema = "off"`). `request` reads, calls `on_moved` when its read finds `<n>` moved, and returns that flag, and the in-transaction branch keeps it, so only that call skips its entry. I traced the review's case: after a migration, `s1`'s `0A000` with the window closed reads the new `<n>`, takes the action and makes no entry; `s2`'s reads the same `<n>`, is due and makes its entry. T-0318, T-0319's unit test of rows B and D, and T-0312's new migration case assert it. One gap: the start read is limited to pools with the caches on, and the text then claims something the procedure doesn't hold (ES2) |
| ER2 (a) | **Written in as decided.** R-1280: the shipped file byte for byte, or only the non-default `host` or `port` active, each under a `# WARNING:` line saying it overrides `DATABASE_URL` and how to undo it, a non-default port in the default directory included. R-1282: the startup warning, never showing a password. R-1283 is the loud notice, with what, when, what you give up and how to proceed, and R-1284 its "use this when" line. R-0062 settles the precedence, and R-0107's alternative carries `sslmode` with its warning. T-0324 covers every detection case, and T-0325 the warning and precedence. Every part has an ID and a test. The warning covers every `[database]` field, but the notice it names only explains `host` and `port` (ES4) |
| ER3 | **Fixed, then refined by the `0A000` decision.** R-1272's advice and R-1271's bullet now give the rates. One clause is wrong when `stale_pending_expiry` is shorter than `stale_cooldown` (ES3) |
| ER4 | **Fixed.** In `session()`, `sure` requests *replace* once. A2 asks for no second request when `sure`, and B3 calls `expect_confirm` only when `!sure`. Rows A2 and B3 and R-1245 say the same, and T-0318 and T-0319 assert no action is due. The same double request is left in row B2's procedure (ES1) |
| ER5 | **Fixed as recommended.** R-1277 names every connection morm opens and how a URL-only connection takes `sslmode`. R-1258 adds the admin-URL rule, confirmed above. R-1281 covers `init`'s CI workflow, with its comment, and T-0324 tests it. T-0323 adds the mixed list without `tls` and the URL-only resolution, and T-0316 an admin URL with `sslmode` unset. R-1278's "How to proceed" gives the `Cargo.toml` line |
| ER6 | **Fixed.** R-1273 lists the R-0060 values that are examples. I checked that list against R-0060's block: every other key's value is its default, `sslmode = "require"` and `port = 5432` included. T-0320 fails on any drift, naming the key, and R-0102 gives `port`'s default, `5432` |
| ER7 | **Fixed** for the two lines named. A third history line of the same kind, and a sentence this round made stale, remain (ES5) |

### The owner's answers this round

- **ER2 (a): `init` writes a found socket loudly.** As above.
- **`morm-admin` always carries TLS:** R-1285 and T-0326 in step 33, with the release profile and `--no-default-features`, exit code `1` with `Error::Connect`, never the feature's `Error::Config`, and `rustls` in `cargo tree`. Step 33's deliverable line, `contracts.md`'s crate row and R-0019, R-1277 and R-1278 say the same.
- **`0A000` requests a repair once:** R-1245 ("So one failure never asks twice"), R-0718's rows B1 and D1, its entry bullet, the marked entry in R-1270's state table and `before_run`, and R-1271, R-1272, T-0312, T-0318 and T-0319. I recomputed T-0312's bounds from the procedure:
  - two statements at `"10s"`/`"1s"` fail five times each in five seconds, inside three to six;
  - one statement repeating its `id` fails once, with no action due;
  - one statement at `"1s"`/`"3s"` fails six times in six seconds, inside four to eight.

  That's what `proofs.md`'s model gives.
- **`morm-migrate` always carries TLS; `morm-mgr` turns on `tls`:** R-1286 and T-0327 in step 29, with T-0327 through the library entry point, R-1287 and T-0328 in step 42, the deliverable lines and `contracts.md`'s two rows.
- **`morm-mgr` is a member of the app's workspace:** R-1287 (members, no `[workspace]` of its own, no `exclude`, one `Cargo.lock`, and the comment on which builds turn `tls` on for the app). R-1278's notice says the same, `contracts.md`'s R-0019 and `morm-mgr` row agree, and T-0328 checks membership, the single lock file and both builds through `compiler-artifact` features. Choice 63 leaves the root layout to step 42, and both layouts are worded true, as the probe confirms.

### Loud notices and "use this when" lines

- R-1283 is new and loud, with all four parts, and R-1284 is its "use this when" line.
- R-1278 now says how to turn `tls` on and what a workspace build does and doesn't give the app.
- R-1281's generated `disable` names R-1278.
- R-1272's notice keeps its four parts with the corrected advice.
- No new opt-in call or setting lacks a line.

### Still faithful

- **The stale-session rows.** Rows A1 to A4, B1 to B5, C1, C2 and D1 to D3, the gate, the epoch and the bounds K1 to K5 read as the design and the recorded decisions state them, apart from ES1, which was already in the procedure and which R-1245's new sentence now contradicts.
- **Earlier IDs.** Those this range didn't touch still read as they did. R-0060's block and its copies are unchanged.
- **Cross-references.** No step file or `contracts.md` line still says `morm-admin` or `morm-migrate` may lack TLS. The checker is clean.

### PR body

The body's sections for this range, its choices 43 to 66, "Open questions: None" and its end-to-end result are accurate: the checker line, CI links, 18 of 18 jobs per run, and the use-when list with R-1284. Two stale lines remain (ES5).

## Findings

### ES1. Minor. R-1270 `before_run` and `on_error`, R-0718 row B2, R-1245 (step 14): on an old connection, B2's confirming run requests *clear* and *replace* for one failure

- **Where:**
  - R-1270's `on_error`, inside a transaction, for an `XX000` from a cached statement straight to Postgres:
    - it makes the entry, `entry(call.stmt, call.bytes, marked = err == 0A000)`, unmarked for an `XX000`;
    - then, if the connection is old, `discard(conn); expect_confirm(call, Replace)`.
  - `before_run`: the entry's confirming run, the next attempt with the same bytes, "a confirmation requests pooled ? Salt : Clear, unless e.marked".
  - The prose: "`expect_confirm` marks the run whose confirmation requests *replace*".
  - R-0718 row B2: "The confirming run's confirmation requests *replace* if the connection was discarded, *clear* otherwise".
  - R-1245, new this round: "So one failure never asks twice".
- **What's wrong:**
  - **The procedure asks twice.** For B2 on an old connection, the next attempt is both the entry's confirming run and the run `expect_confirm` marked, so as written it requests *clear* and *replace*.
  - **The second request costs a pool-wide action.** The gate takes the first and makes the second due, so one `stale_cooldown` later the pool clears every cache or replaces every connection again, for nothing: a *replace* already cures what a *clear* would.
  - **The texts disagree.** Row B2 reads "one or the other", and R-1245 now says no failure asks twice, but the procedure the simulation drives does both. It's the pattern ER4 removed from `session()`.
  - **No test catches it.** T-0312's B2 case asserts that *replace* is requested, not that nothing is due afterwards. T-0319's unit test of rows B and D doesn't name it.
- **Evidence:** the lines quoted above. Before this round the procedure already had both requests. This round's R-1245 sentence and ER4's fix make the contradiction explicit.
- **Fix:**
  - In `before_run`, a run marked by `expect_confirm` requests *replace* in place of the entry's *clear*, not beside it; for example "requests `e.expect or (pooled ? Salt : Clear)`", with `expect_confirm` recording *replace* on the entry.
  - T-0319's unit test of rows B and D asserts that B2 on an old connection in `.retry(1).transaction(..)` requests *replace* once and leaves no action due, and that B2 on a new connection requests *clear* once.
  - No owner choice is needed: row B2 already says which one.

### ES2. Minor. R-1270's state table, R-1116, T-0319 (steps 14, 41, 20): with the statement caches off, the pool never reads `<n>` at start, and "with the caches off no call reaches the gate unread" doesn't hold

- **Where:**
  - R-1270's new row "Last-read `<n>`": "read when the pool starts, straight to Postgres as behind PgBouncer, **whenever the statement caches are on** ... with the caches off no call reaches the gate unread". The prose adds that `take` "never meets an unset one".
  - R-1116: "With the statement caches on, straight to Postgres as behind a pooler, the same statement also reads the statement generation".
  - `session()`: B3 with a `22P02` calls `discard(conn); if !sure: expect_confirm(call, Replace)`. The next attempt's run, if it confirms, requests *replace*. The procedure gives that request no `<n>`, so it's `Unread`, and with the gate open `take` sets `window.n` to "the pool's last-read `<n>` if Unread".
  - T-0319's settings: "the defaults, pending entries off, caps of 1, and `stale_cooldown` five times `stale_pending_expiry`". Caches off isn't among them.
  - R-1272's notice recommends `statement_cache = 0` and `shape_cache = 0` for a pool.
- **What's wrong:**
  - **The claim is false.** The session class, B3's `22P02` among them, arises with the caches off: plpgsql variables compiled against a kept enum. So with the caches off, B3's confirming run reaches the gate unread, and nothing read `<n>` at pool start.
  - **What happens then is undefined.** It depends on whether `session()`'s earlier read counts as "moved" against an unset last-read value, which the text doesn't say. If it doesn't, `window.n` is set from an unset value, and the next request in the window takes a second action with `<n>` unmoved, breaking I1. That's ER1's defect, in the configuration R-1272 recommends.
  - **No test catches it.** The simulation never runs with the caches off, so T-0319 can't catch it.
- **Evidence:** the rows and lines quoted. The start read costs nothing extra where R-1116's statement runs, and one catalog query where `check_schema = "off"`, the same as with the caches on.
- **Fix:**
  - Read `<n>` at pool start in every pool, caches on or off: drop "whenever the statement caches are on" from R-1270's row and "With the statement caches on" from R-1116's last sentences, keeping the marker behind a pooler with the caches on.
  - Delete "with the caches off no call reaches the gate unread".
  - Say that `expect_confirm`'s request carries the `<n>` its failure read.
  - Add caches off, straight and behind PgBouncer, to T-0319's settings.

### ES3. Minor. R-1271 (step 20) and R-1272 (step 50): "each such statement fails about once per `stale_cooldown`, whatever `stale_pending_expiry` is" is wrong when `stale_pending_expiry` is the shorter

- **Where:**
  - R-1271's `search_path` bullet: "with more than one such statement the action comes about once per `W`, and each such statement fails about once per `W`, whatever `E` is".
  - R-1272's notice: "each such statement fails about once per `stale_cooldown`, whatever `stale_pending_expiry` is". Its advice: "When several statements switch ... raising `stale_pending_expiry` alone changes nothing then".
- **What's wrong:** with `E` shorter than `W`, each statement's entry expires before the due action ends it, so it fails once per `E`, not once per `W`. For example, two statements at `stale_pending_expiry = "1s"` and `stale_cooldown = "3s"`, switching every 100 ms:
  - `s1` fails at 0 and the action is taken, then `s2` fails and the next action is due;
  - each entry expires at about 1 s, so each statement fails again at 1 s and at 2 s, with the action still due until 3 s;
  - that's about three failures per `W` per statement.

  Raising `stale_pending_expiry` up to `stale_cooldown` then does reduce failures, so "raising it alone changes nothing" is wrong in that range. The notice's other bullet, "keeping it at least `stale_cooldown`", points the right way, but the rate it states is false. T-0312 and T-0319 state upper bounds, so the tests are fine; only the stated rate and the advice are wrong.
- **Evidence:** R-1270's procedure. It's the same trace `proofs.md`'s model gives for one statement at `"1s"`/`"3s"`, applied to two.
- **Fix:**
  - In both places: with several such statements, each fails about once per `stale_cooldown`, or once per `stale_pending_expiry` when that's the shorter.
  - In the advice: raising `stale_pending_expiry` past `stale_cooldown` changes nothing then, so raise both together.

### ES4. Minor. R-1282 and R-1283 (steps 10 and 50): the warning covers every `[database]` field, but the notice it names covers only `host` and `port`

- **Where:**
  - R-1282 warns for "a `host`, `port`, `name`, `user`, `password`, `sslmode` or any other field a URL can carry", and names "the README notice on it, 'A `host` or `port` written in `morm.toml` overrides `DATABASE_URL`'". That's the INDEX decision's "whenever a `morm.toml` field overrides a value `DATABASE_URL` also sets".
  - R-1283's title, "What you give up" ("keeps the file's host and port") and "How to proceed" (put back `host = "${MORM_DB_HOST}"` and `port = "${MORM_DB_PORT}"`, or set `MORM_DB_HOST` and `MORM_DB_PORT`) cover only those two.
- **What's wrong:** a user warned about `user`, `name` or `sslmode` is sent to a notice that says nothing about how to proceed for that field.
  - **The worst case is `sslmode`.** A `sslmode = "disable"` written in the file for local work silently overrides a deployment URL's `?sslmode=require` or `verify-full`, and the notice's "What you give up" doesn't say that it turns TLS off for a server the URL asked to encrypt.
  - **The rule isn't met.** The owner's rule needs a loud notice to say what's lost and how to proceed for what it covers.
- **Evidence:** R-1282 and R-1283 as quoted. T-0325 tests `host`, `port`, `user` and `password`, not `sslmode`.
- **Fix:**
  - Retitle R-1283, for example "A `[database]` value written in `morm.toml` overrides `DATABASE_URL`", keeping `init`'s `host` and `port` as the main example, and make R-1282 name it.
  - "What you give up" adds that a written `sslmode` weaker than the URL's drops its TLS, naming R-1278.
  - "How to proceed" says to put back each field's `${VAR}` placeholder, as R-1273 lists them, or set its `MORM_DB_*` variable.
  - T-0325 adds `sslmode = "disable"` written in the file against a URL with `?sslmode=require`: the file wins, as R-0062 says, and the warning names both modes.

### ES5. Minor. PR body: one more unmarked history line about the horizon, and a sentence that says R-1116 doesn't change

- **Where:**
  - "The consolidated stale-session rewrite", the "Elsewhere" bullet: "R-1269 (step 48) measures the due-action and horizon checks".
  - The end of "Changed and new IDs, by step": "R-1116, R-1248, R-0697, R-0985, R-1259, R-1260, T-0151 and T-0153 don't change, as the design's rewrite plan says".
- **What's wrong:**
  - **The horizon line.** EQ2 removed the horizon, and R-1269 now measures the pending check, the due action and an unnamed run. This line is ER7's kind and isn't marked superseded.
  - **The R-1116 sentence.** ER1 changed R-1116 in this range, and the table above it lists R-1116 as changed in step 41, so the sentence reads as contradicting it.
- **Fix:** mark the first "Superseded by EQ2: there's no horizon; R-1269 measures the pending check and the unnamed run". Scope the second to the rewrite ("didn't change in the rewrite"), and note R-1116's change by ER1.

## Summary

| Severity | Findings |
|---|---|
| Blocks approval | None |
| Should fix | None |
| Minor | ES1, ES2, ES3, ES4, ES5 |

What's written in:

- **ER1 to ER7** are fixed as decided or recommended.
- **The five owner answers** are written in as decided: ER2 (a), `morm-admin` always carries TLS, a `0A000` requests once, `morm-migrate` carries TLS and `morm-mgr` turns on `tls`, and `morm-mgr` is a member of the app's workspace.
- **Every new requirement** has its test, and the loud notices and "use this when" lines are there.
- **The checks agree.** The manifest matches INDEX ID for ID, the checker reports 1287 requirements, 328 test obligations and 0 problems, and CI is green, 36 of 36.
- **The new evidence holds.** The PgBouncer admin-console claim and the Cargo unification claim reproduce, on PgBouncer 1.26.1 built from the checked tarball with Postgres 18.6, and on Cargo 1.99.1.

The five findings are each a contradiction or an inaccuracy in text, and none needs an owner design choice:

- **ES1:** row B2's double request;
- **ES2:** the caches-off start read;
- **ES3:** one stated rate;
- **ES4:** the notice's scope;
- **ES5:** two PR body lines.

PR #3 isn't quite ready for the owner. Once ES1 to ES5 are fixed and a short check confirms the edits, the checker and CI, it is ready. Nothing in it blocks approval in substance.
