# Step 55 independent review, thirteenth round

PR #3 (`step-55-enum-proofs`), head `564723e`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes. This round covers the consolidated stale-session rewrite and the settings template, `git diff 0f26ba7 origin/step-55-enum-proofs` (commits `7a3db77` and `564723e`), and checks that nothing earlier regressed.

The source of truth for the stale-session rules is `reviews/stale-session-state-machine.md` on `main` (the design), with the owner's answers MA to ME. After this head, `main` gained one more owner decision (`882f335`, INDEX "The settings file is a real `morm.toml`, not a template"): the template becomes a real `morm.toml` with every setting at its default, and the next fix round writes that in. I reviewed the template as the head has it. EQ1, EQ8 and EQ9 apply to the real file as much as to the template, and each says how.

## What I read

- **On the PR branch:** `rules.md`, `contracts.md`, `proofs.md` (step 55's section, the new MA-to-ME bullets and "The stale-session state machine: evidence"). I read every changed, retired and new ID in full: R-0060 and its copies, R-0718, R-0975, R-0981 to R-0984, R-1040, R-1238, R-1245, R-1246, R-1249, R-1250, R-1253, R-1256, R-1258, R-1261 to R-1276, T-0152, T-0306, T-0311, T-0312, T-0314, T-0316 to T-0322. For coherence I also read R-0061, R-0063's table, R-0102, R-0111, R-0113, R-1235 and R-0901, and searched every step file for settings, removed mechanisms and the stale-session terms.
- **On `origin/main`:** the design, in full; INDEX's Decisions entries "Step 55 twelfth review, and one consolidated stale-session design", "Stale-session state machine, MA to ME, and a commented settings template", "Unknown `morm.toml` keys are refused" and the new "The settings file is a real `morm.toml`"; `reviews/step-55-review-12.md`.
- **The PR body.**

## What I checked and ran

- **CI on the head.** `564723e` is green: `ci` on push (run 44255977121) and on the pull request (run 44255984935), 36 of 36 checks passing. The diff touches only `plans/plan-approved/`: `contracts.md`, `coverage.md`, `proofs.md`, `manifest.json` and steps 10, 11, 12, 14, 20, 27, 39, 47, 48, 50, 51, 52, 59 and 60.
- **Coverage checker,** run on a `git archive` of the head:
  - `check_coverage.py --step 55 --code crates tests benches fuzz .github`: **1276 requirements, 322 test obligations, 0 problems**;
  - the plain form gives the same.
- **Manifest against INDEX,** ID by ID:
  - `0f26ba7` to `7a3db77`: changed R-0060, R-0718, R-0975, R-0981 to R-0984, R-1040, R-1238, R-1245, R-1246, R-1249, R-1250, R-1253, R-1256, R-1258, R-1261 to R-1269, T-0152, T-0306, T-0311, T-0312, T-0314, T-0316, T-0317; new R-1270 to R-1274, T-0318 to T-0321.
  - `7a3db77` to `564723e`: changed R-1261, R-1273, T-0320; new R-1275, R-1276, T-0322.
  - None removed; `frame` and `master_sha256` unchanged. R-1261 and R-1267 are now `info`.

  That's exactly the two INDEX entries' "Written in at" lists, with R-1261 and R-1267 under "retired as `info`".
- **R-0060's copies.** Steps 11, 12, 27 and 47 quote step 10's block byte for byte (79 lines each, quote marks stripped).
- **Removed mechanisms.** No step file outside step 10's R-1261, step 14's R-1262 and T-0317, and step 50's R-1267 and R-1276 names `stale_first_failure_max`, its variable, first-failure records, "step 14's memory", a "pending confirmation" or the one-clear window. The `proofs.md` and PR-body history keeps them, marked superseded.
- **Spot check on throwaway Postgres 18.6** (the Manjaro package's `initdb`), a cluster under `.claude/scratch/` in my worktree with `TMPDIR` set there and its socket under `~/.claude/scratch/`, through `psql`. Schemas `t1` and `t2` each hold `items`, `t2`'s with one more column. With `search_path = t1`, `PREPARE s`, then:

  | Step, standing in for morm straight to Postgres | Result |
  |---|---|
  | In a transaction, `set_config('search_path', 't2', true)`, `EXECUTE s(1)` (the copy prepared before the entry) | `0A000` `cached plan must not change result type`, `RevalidateCachedQuery` |
  | `DEALLOCATE s` (B1's evict); in a transaction under `t2`, `PREPARE s` again and `EXECUTE` (the attempt's re-prepare, newer than the entry's horizon) | Right, three columns |
  | Next transaction under `t1`, `EXECUTE s(1)` (that newer copy) | `0A000` again |
  | `DEALLOCATE`, re-prepare under `t1`, then a transaction under `t2` | `0A000` again |

  So a copy prepared after a pending entry's horizon fails on the next switch. See EQ2. The server was stopped and deleted. PgBouncer wasn't run: no claim in this round needed it beyond the design's own X3 and X7.

## Faithful to the design

I compared section by section.

| Design section | In the plan | Verdict |
|---|---|---|
| 1. Signals, confirmation, no-confirm list | R-1245's table and paragraph | Faithful. The design's event rows (refused open, migration, hand-made change, `RECONNECT`) sit in R-0981's *replace* and R-0982's E1 to E3 instead, which is fine. One wording narrows a certain signal (EQ7) |
| 2. State, bounds, clearing | R-1270's table | Faithful, and more complete: it adds that a no-confirm outcome and a pooled session-class signal remove the entry, as the design's pseudocode and "before any row" do |
| 3. Procedures, rows A1 to A4, B1 to B5, C1, C2, D1 to D3, entries off, E1 to E8, pseudocode | R-0718 (A1, B1, B2, C1, D1, entries off), R-0981 (A2 to A4, B3, B4, parameter test, transient `XX000`), R-1246 (C2, D2), R-0984 (B5, D3), R-0982 (gate, E1 to E8), R-1270 (pseudocode, verbatim) | Every row is there with the design's outcome and cost. The pseudocode carries the design's own mismatches with the rows (EQ4, EQ7) |
| 4. Guarantees H1 to H4, K1 to K5, (c), (d), (e) | R-1271, with K5's entry cost in R-0718 | Faithful, both silent wrong answers stated exactly. MD's stated cost isn't true straight to Postgres (EQ2) |
| 5. EP1 to EP8, N1 to N5 | Below | Closed, apart from EP7's use-when line (EQ10) |
| 6. Simulation | T-0319, T-0318 | The model, events, settings and the prototype's missing pieces are all in T-0319. Two invariants disagree with R-1271 (EQ5), and nothing shows the shipped invariants catch a broken machine (EQ6) |
| 7. Rewrite plan | Every row's ID changed as listed; R-0985, R-1116, R-1248, R-0697, R-1259, R-1260 unchanged; `contracts.md` and `proofs.md` as listed | Faithful. The contract's new `SchemaChanged` comment, copied from the design, contradicts rows D1 and D2 (EQ3). The fix's own deviations (D1's test in T-0312, X10's case in T-0316 on a new R-1238 instance) are in INDEX and the PR body |
| Ledger of owner decisions | Each "kept" decision reads as kept | Nothing kept is contradicted |

### EP1 to EP8

| Finding | Verdict |
|---|---|
| EP1 | **Closed.** *replace* passes R-0982's gate (R-0975 says so in step 11); the epoch decides replacements (R-0981); T-0312's ten-window cycle asserts at most four discards and one *replace* per window |
| EP2 | **Closed for the gate** (0A000 is gated, MB). MD's notice states a cost the machine doesn't give straight to Postgres (EQ2) |
| EP3 | **Closed.** R-0982: "The window never changes a call's own outcome"; a new salt replaces the old (R-0982, R-1246) |
| EP4 | **Closed.** The in-window plpgsql path is gone; T-0306's second hand-made drop inside the window is answered right |
| EP5 | **Closed.** T-0306 asserts the deferred salt at the first checkout after a `"1s"` window; T-0318 and T-0319 (I2) assert deferral |
| EP6 | **Closed.** R-1262 refuses `0` and `"0"`; T-0317 tests both and accepts `"off"` and `"0s"` |
| EP7 | **Closed, apart from one line.** R-1262 and R-1268 say "each holds per pool"; R-0982 names the only `<n>` reads straight to Postgres; R-0060's comment names the window, the epoch and the cool-down. R-1263 doesn't (EQ10) |
| EP8 | **Closed.** R-1262 and R-1268: a small `stale_cooldown_max` costs a retry or a `SchemaChanged`, "never a connection" |

### Coherence

Apart from the findings below, the rewritten requirements agree with each other, with `contracts.md`, and with steps 11, 12, 33, 41, 48 and 59. R-1261 and R-1267 are clean `info` lines. Nothing earlier regressed: SA to SH, EF1, EG1 to EG3, EH1 to EH8, EI1 to EI9, EJ1 to EJ9, EK1 to EK8, EL1 to EL7, EM1 to EM6, EN1 to EN5 and EO1 to EO10 read as the design's ledger marks them, kept or superseded.

### The settings template and unknown keys

The template's header, `# WARNING:` lines, R-0060 order and drift tests are as R-1273 says. Steps 39, 51 and 59 add their keys to the template in the same PR, and steps 52 and 60 say any key they add goes in too. I found no other step that adds a key outside R-0060's block, as the PR body says. R-1275's refusal covers every table, an unknown table, a key in the wrong table and the two named refusals, and T-0322 tests each table. The README notice R-1276 is loud and has its "What you give up". The findings are EQ1, EQ8, EQ9 and EQ11.

### PR body

Accurate for this round's changes, its fix choices and the open question's answer. One leftover sentence is stale (EQ12).

## Findings

### EQ1. Blocks approval. R-1273, T-0320, R-0060 (step 10): the template must hold R-0060's values as active lines *and* load as no file would, but R-0060's `name`, `user` and `sslmode` aren't defaults, and active `[database]` fields override `DATABASE_URL`'s parts

- **Where:**
  - R-1273: "Every key of R-0060's block appears in the template, in the same table, with R-0060's value." Only `[server]`, `[pools.replica]`, `[pools.replica.server]`, `[pools.jobs]`, and settings with no default that R-0060 doesn't show, are commented out, "so loading the template unchanged, with no environment variable set, gives the same config as no file at all".
  - T-0320: every key of R-0060's block with "the same value"; every key the config structs read "with the default the loader uses"; and "loading the template with no environment variable set gives the same config as loading no file".
  - R-0060's `[database]`: `name = "app"`, `user = "app"`, `sslmode = "disable"`, `host = "/var/run/postgresql"`, `port = 5432`, and its comment "Separate fields override URL parts". R-0111: `MORM_DB_*` values override the matching parts of `DATABASE_URL`.
- **What's wrong:**
  - No requirement makes `"app"` the default database or user, or names `sslmode`'s default. R-0060 is an example. So T-0320 passes only if step 10 invents `"app"` as the default database name and user, and picks an `sslmode` default, which nobody decided. Otherwise it fails by construction.
  - Even where R-0060's value is a default (`host`, by R-0102), an active line isn't the same as an unset field. A user who copies the template and sets `DATABASE_URL` to a remote host gets `host`, `port`, `name` and `user` from the file, because separate fields override URL parts. Their `DATABASE_URL` is then silently overridden: the app connects to the local socket as `app`. T-0320 can't see this, because it loads with no environment variable set.
  - The owner's new decision (`882f335`) keeps the same two rules: "every setting present and set to its default", and "loading it unchanged gives exactly the defaults". So the fix round meets the same problem.
- **Evidence:** the R-1273, T-0320, R-0060, R-0102 and R-0111 text above. There's no default for `name`, `user` or `sslmode` in steps 10, 11, 12, 42 or 43.
- **Fix:**
  - In the file, write every `[database]` and `[database.migration]` connection field that `DATABASE_URL` or `MORM_MIGRATION_URL` can also carry (`url`, `host`, `port`, `name`, `user`, `password`, `sslmode`, and the certificate paths) as a `${VAR}` placeholder naming its own variable, such as `host = "${MORM_DB_HOST}"`, and put the value or default in the comment. An unset variable leaves the field unset (R-0061), so the URL's parts win and the file loads as no file does. This fits the new decision's "Settings with no default ... are written with a `${VAR}` placeholder".
  - R-1273 and T-0320 compare R-0060's value with the comment, not the active line, for those fields. T-0320 adds a case: with `DATABASE_URL` set to another host and database, loading the file gives the URL's host, port, name and user.
  - Step 10 says which `[database]` fields have defaults. `host` and `port` do, by R-0102. If `sslmode` is meant to have one, it's an owner choice, because it's a security default. Until then it's written as a placeholder too.

### EQ2. Should fix, a design choice for the owner. R-1272, R-0718, R-1271 (MD): straight to Postgres, a per-transaction `search_path` fails *every* switch inside a transaction, not once per `stale_pending_expiry`, and behind PgBouncer each pool-wide action restarts it

- **Where:**
  - R-1272: "Inside one you get `Error::SchemaChanged` once per `stale_pending_expiry` ... per statement and process, and then the statement runs fresh while morm keeps it pending", and "without `.retry(n)`, one failed transaction per statement per `stale_pending_expiry`". The design's MD (a) says the same, for both modes.
  - R-0718: while an entry exists, straight to Postgres, "a copy cached on this connection before the entry's horizon is prepared again first". A later signal "moves its horizon". B1 evicts the failing copy.
  - R-0982 and E5: taking any pool-wide action "removes every pending entry".
- **What's wrong:**
  - **Straight to Postgres.** Connection C has `s` cached under `t1`. A transaction under `t2` gets `0A000`: B1 evicts and makes the entry, and the caller gets `SchemaChanged`. The next attempt prepares `s` again under `t2`, newer than the horizon, so that copy is trusted. The next transaction under `t1` runs it, gets `0A000`, and B1 moves the horizon and returns `SchemaChanged` again. The spot check shows each re-prepared copy failing at the next switch. So each switch on each connection fails one transaction without `.retry(n)`, at the rate tenants alternate, not once per 60 seconds. `.retry(n)` with `n` at least 1 still hides it, because the attempt re-prepares under the attempt's own path.
  - **Behind PgBouncer.** While the entry lasts every run is unnamed, which answers under either path (X2), so the notice holds for one entry. But flips of other statements keep requesting *salt*. Each salt the gate takes, at most one per `stale_cooldown`, removes every entry. So the statement's next switch fails again, and the true rate is about once per `min(stale_pending_expiry, stale_cooldown)` per statement and process.
  - No test covers the in-transaction cost of switches in either mode. T-0306 tests switches outside a transaction behind PgBouncer; T-0312 has no `search_path` case.
- **Evidence:** the spot check table above; R-0718, R-0982 and E5 as quoted.
- **Fix, a choice for the owner, because MD approved "the stated cost":**
  - **(a) Recommended. While a statement has an entry, run it unnamed straight to Postgres too**, through `query_typed`, as behind PgBouncer. Then R-1272's cost holds in both modes, apart from the reset by a pool-wide action. Pending entries become one rule in both modes, and the per-connection horizon compare is no longer needed for runs. The cost: K5's "prepared again once per connection" becomes "unnamed for every run" while an entry lasts, as behind PgBouncer. R-0718, R-1270's state table, K5's text in R-1271, R-1268's `stale_pending_expiry` line and R-1269's horizon benchmark change with it. This reverses the fix choice "straight to Postgres an entry now makes each connection prepare its older copy again", which the owner approved through PR #3.
  - **(b)** Keep the mechanism and state the cost as it is. Straight to Postgres, inside a transaction each switch fails one transaction per connection without `.retry(n)`. Behind PgBouncer it fails once per `stale_pending_expiry`, and again after each time morm moves its statements to fresh text. R-1272 and MD's text in R-1271 say so.

  Either way, R-1272 adds that a pool-wide action restarts the count, and T-0312 adds per-transaction switches in both modes. Ten alternations with no `.retry(n)` assert the stated number of `SchemaChanged`, and with `.retry(1)` none reaches the caller.

### EQ3. Should fix. `contracts.md` (`Error::SchemaChanged`), R-0984 (step 20): "returned only where morm repaired something for the next attempt" contradicts rows D1 and D2 and the transient `XX000`, which return `SchemaChanged` with nothing repaired

- **Where:**
  - `contracts.md`: `SchemaChanged` is "Returned only where morm repaired something for the next attempt: a statement evicted, a pending entry made, or a connection discarded or marked for replacement; where it repaired nothing, the server's error is returned as Error::Postgres, so .retry(n) stops". R-0984's first paragraph says the same.
  - R-0984's own list of `SchemaChanged` cases: "behind PgBouncer a session-class signal from a statement not in step 14's cool-down, D2; and the transient `XX000` ..., which moves nothing".
  - R-0718's "Pending entries off": behind PgBouncer "D1 returns `Error::SchemaChanged` on every call", with no entry made and nothing evicted.
- **What's wrong:** in all three cases nothing is repaired. D2 adds the statement to the cool-down, which makes the next attempt return the error as it is. The transient `XX000` repairs itself. So the contract and R-0984's lead each say one thing and R-0984's list another. A builder who codes the contract's rule turns D2's first failure into `Error::Postgres`, contradicting EK1 ("the cool-down follows the first `SchemaChanged`"). The design has the same contradiction between its `contracts.md` bullet and its rows.
- **Fix:** state the rule as the rows apply it: `SchemaChanged` is returned when the next attempt may succeed. That's when morm repaired something (a statement evicted, an entry made, a connection discarded or marked for replacement); behind PgBouncer, on a session-class failure's first time, whose next attempt may reach another server session; on the transient `XX000`, which doesn't repeat; and behind PgBouncer with pending entries off, on D1. Otherwise morm returns `Error::Postgres`. Write it the same way in `contracts.md` and R-0984.

### EQ4. Should fix. R-1270's decision procedure (step 14): outside a transaction behind PgBouncer, a moved `<n>` is salted instead of adopted, against row C1, E1 and T-0306; and whether adoption counts as the window's action is unsaid

- **Where:**
  - The procedure, outside a transaction: `if confirms(r, err) or err == 0A000: request(pooled ? Salt : Clear, read_n_if_needed())`, and `request(action, n): if window_open() or n != window.n: take(max(action, due))`.
  - Row C1: "A moved `<n>` is adopted, not gated; otherwise *salt* is requested". E1: adoption "drops the salt". T-0306: "the changed column's `0A000` answered by one unnamed run with the moved `<n>` adopted and no salt".
  - The procedure's in-transaction branch does adopt: `if pooled and n moved: adopt(n)`.
- **What's wrong:**
  - Built as written, the out-of-transaction branch passes a moved `<n>` to `request`, which takes *salt*. The marker becomes the old generation plus a salt, not `g<n>`. That fails T-0306, and K4 gets a salt per migration on top of the adoption. The design's pseudocode has the same slip, and R-1270 says the module is built from it.
  - Nothing says whether adopting sets the window's `<n>`, or starts a window. If it doesn't, the first *salt* request after a migration passes a closed window too, because `n != window.n`. Behind PgBouncer that's an adoption and a salt per migration, where R-1246 and R-1271 promise "at most one salt per `stale_cooldown`, plus one adoption per migration".
- **Fix:**
  - Write the out-of-transaction branch as the in-transaction one: `n = read_n_if_needed(); if pooled and n moved: adopt(n) elif confirms(r, err) or err == 0A000: request(...)`.
  - Say in R-0982's E1 that adopting a moved `<n>` sets the window's `<n>` to it, and whether it starts a window. Recommended: it sets `<n>` and starts a window, so one migration lets one pool-wide action through, as K1 counts it.
  - T-0318 asserts both.

### EQ5. Should fix. T-0319's I4 and I9 (step 20) disagree with R-1271, so a correct machine can fail them

- **Where:**
  - I4: stale-driven opens "within `2 × max + 2` per `stale_cooldown` plus `max` per migration".
  - R-1271's churn (the design's K3): "at most `2 × max` new connections from stale handling per `W`, plus `2 × max` per migration".
  - I9: "A wrong answer with no error comes only from a copy cached before a hand-made swap that kept the old type, and never later than `stale_cooldown` after the pool's first confirmation following the last change".
  - R-1271's silent wrong answers: "for plpgsql, when the session ends, straight to Postgres at the connection's replacement and behind PgBouncer at `RECONNECT` or `L`; if nothing in the pool ever errors, when the connection or server session is replaced".
- **What's wrong:**
  - **I4.** A migration starts a new epoch, up to `max` replacements (K2). It also lets one request through the closed window, and a *replace* replaces up to `max` more at checkout (K1). That's `2 × max` per migration, which K3 counts and I4 doesn't. A model whose migrations are followed by a session-class failure and a certain signal fails I4 with a correct machine.
  - **I9.** A confirmation requests *clear*, which re-prepares statements but doesn't recompile plpgsql. Behind PgBouncer nothing morm does reaches a server session. So plpgsql compiled before a kept-type swap keeps answering by the old order after `stale_cooldown`, exactly as R-1271 says it may. I9 would fail on the model's `DECLARE v st` function compared on shared labels.
  - The design has both mismatches too. Its prototype passed, so its model likely never produced these sequences, which T-0319's fuller model will.
- **Fix:**
  - I4: "within `2 × max + 2` per `stale_cooldown`, plus `2 × max` per migration; behind PgBouncer none", with the `+ 2` (two failed opens per suspended epoch, K2) said in R-1271's churn bullet too.
  - I9, split as R-1271 is: a cached statement's silent wrong answer ends within `stale_cooldown` of the pool's first confirmation or `0A000` after the change, or when its own stale error is repaired, or when its connection is replaced; plpgsql's ends when its session ends (straight, the connection's replacement; behind PgBouncer, `RECONNECT` or `server_lifetime`); behind PgBouncer a statement's can recur between entries before a salt.

### EQ6. Should fix. T-0319 (step 20): nothing shows the shipped simulation would catch a broken machine

- **Where:** T-0319 states the model, events, settings and I1 to I9. The design's section 6 showed, on the prototype, that seven broken machines are each caught by the invariant meant for them: no gate, no epoch, PR #3's ungated `42804`, an epoch that never ends, no replacement on an old connection, a dropped deferral, and no fresh run inside the window.
- **What's wrong:** a simulation can pass because its oracle or invariant checks are weaker than intended, for example an oracle that shares the machine's notion of "stale", or an I7 that never sees a visible error. The prototype's evidence doesn't carry over to the new code. T-0319 asks only that the invariants hold on the shipped module, so a vacuous checker passes it. That's "does each tagged test really prove it?" for R-1271.
- **Fix:** T-0319 adds a mutation check. The module has a `#[cfg(test)]` fault switch for each of the design's seven broken machines. A unit test per fault runs the simulation with that fault and asserts a failing sequence is found within the CI case count (256), naming the invariant the design expects to catch it. The PR body reports each one's shrunk sequence length.

### EQ7. Minor. R-1270's procedure and R-1245 (step 14): three smaller mismatches with the rows

- **Reading `<n>` and making an entry, straight to Postgres, inside a transaction.** The procedure reads `<n>` unconditionally (`n = read_n(conn)`) and makes the entry even when `<n>` moved. R-0982 says the epoch's read and a request with the window closed are "the only reads of `<n>` straight to Postgres". B1's cost has no catalog read. R-1270's state table makes entries "unless `<n>` moved". **Fix:** straight to Postgres, read `<n>` only for B2's old-connection test or a request with the window closed, and skip the entry when the read finds `<n>` moved, or say that B1 reads it.
- **Which `XX000` is certain on its own.** R-1245: "on an old connection that R-0981 replaces, `42804` and an `XX000` ... from a fresh run". But rows A2 and B3, and the procedure's `if err in {42804, XX000}: request(Replace, n)`, count any session-class `XX000`, including one from a statement that isn't cached, such as T-0152's trigger. **Fix:** R-1245 says "a session-class `XX000`", matching the rows.
- **Held connections.** R-1270 lists "on a held connection or not" in the call's context, but the procedure has no branch for row A4, and as written `ctx.out` sends a held connection to `replace`. **Fix:** add A4's branch: `SchemaChanged`, and an old connection is marked for replacement on release.

### EQ8. Minor. R-1273, T-0320, T-0321: `sslmode`'s `# WARNING:` must name "the README notice that explains it", and there's no such notice

- **Where:** R-1273's warning list includes "`sslmode` values that don't encrypt a TCP connection", each warning "naming the README notice that explains it". T-0320 requires a `# WARNING:` for `sslmode`. Step 50 has no loud notice about `sslmode`, only R-0903's "Security: TLS settings" section and R-0111's startup warning for `prefer` without `tls`.
- **What's wrong:** the warning has nothing to name, and the owner's rule is a loud README notice for anything that silences a check. The real file the owner now wants keeps the same warning.
- **Fix:** step 50 gains a loud notice beside the TLS settings, a new ID: `disable` and `prefer` over TCP may send queries and passwords unencrypted, what's lost, and when it's fine, such as a Unix socket. The file's warning names it, and T-0321 checks it like the others.

### EQ9. Minor. T-0320: the drift test checks that a range is written, not that it's right

- **Where:** T-0320 checks that "each setting has a comment giving its default and its allowed values or range". It compares defaults and environment variables with the loader, but not ranges.
- **What's wrong:** a step that changes a bound, such as R-1262's ceilings, leaves the file's comment stale with no test failing. R-1273 asks that "the stale-session settings carry step 14's ranges".
- **Fix:** the loader keeps one table per setting (table, key, default, variable, bounds) that its validation uses, and T-0320 compares each comment's range with it. The same applies to the real file.

### EQ10. Minor. R-1263 (step 50): `stale_cooldown`'s use-when line doesn't name the window, the epoch and the cool-down, as the design's EP7 closure says it does

- **Where:** the design's EP7 row: "R-0060's comment and the use-when line name the window, the epoch and the cool-down". R-1263: "You want stale-session handling, and the clearing of every connection's statement caches, to react sooner or later than once a minute". The design's own section 7, item 7, gives the shorter line, so the design is inconsistent with itself.
- **Fix:** R-1263 adds a clause, such as "it sets the one window for pool-wide repairs, the epoch straight to Postgres and the cool-down behind PgBouncer".

### EQ11. Minor. R-1275, R-1262, T-0322 (step 10): `morm-migrate` isn't tested, and the removed-key message's scope differs

- **`morm-migrate`.** R-1275: "`morm-admin` and `morm-migrate` refuse the same file the same way". T-0322 tests `Db::connect`, `connect_as` and `morm-admin`. **Fix:** T-0322 adds `morm-migrate`'s config load, through its library entry point.
- **Scope of the removed-key message.** R-1262 gives `stale_first_failure_max` its removed-setting message "in `[pool]` or in any `[pools.<name>]`". R-1275 and R-1261 say only that the key keeps its own message. **Fix:** say what `stale_first_failure_max` in `[database]` or another table gets. Recommended: the removed-setting message wherever it appears, as R-1235 does for `timezone`.

### EQ12. Minor. PR body, "The owner's settings" in the tenth-review section: one sentence still describes the old rules

- **Where:** "The three refused at `0` would let text holding a retired label open or discard a connection on every call".
- **What's wrong:** the table above it marks `stale_first_failure_max` superseded. But only `stale_cooldown_max` is still refused at `0`, as `stale_cooldown` is at `"0s"`, and neither now costs a connection (R-1262: "a retry or a `SchemaChanged` on every call").
- **Fix:** mark the sentence superseded by MA and give the current reason.

## Summary

| Severity | Findings |
|---|---|
| Blocks approval | EQ1 |
| Should fix | EQ2 (a design choice), EQ3, EQ4, EQ5, EQ6 |
| Minor | EQ7, EQ8, EQ9, EQ10, EQ11, EQ12 |

The state machine is written in faithfully. Every row, the state table, the gate, the epoch, the events, the guarantees and both silent wrong answers are there, and EP1 to EP8 are closed. What's left is where the design's own text disagrees with itself, carried in: the procedure against its rows (EQ4, EQ7), the contract against D1 and D2 (EQ3), two invariants against the guarantees (EQ5), and MD's stated cost straight to Postgres (EQ2). Separately, the simulation needs to show it can fail (EQ6), and the settings file needs its `[database]` fields written so they don't override `DATABASE_URL` and don't invent defaults (EQ1). That carries over to the owner's new real-`morm.toml` decision.
