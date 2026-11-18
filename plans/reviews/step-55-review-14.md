# Step 55 independent review, fourteenth round

PR #3 (`step-55-enum-proofs`), head `8abcd9d`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes. This round covers the fix for the thirteenth review's EQ1 to EQ12 and three owner decisions: the real `morm.toml`, `sslmode = "require"`, and EQ2 option (a). That's `git diff 564723e origin/step-55-enum-proofs`, commits `a918585`, `74dcf6d`, `01b447c`, `f194d00` and `8abcd9d`. I also checked that nothing earlier regressed.

The source of truth for the stale-session rules is `reviews/stale-session-state-machine.md` on `main` (the design), with the owner's answers MA to ME, EQ2 (a) and the fix choices INDEX lists.

## What I read

- **On the PR branch:**
  - `rules.md`, `contracts.md` (the `SchemaChanged` comment, `PoolStats`, R-0019), and `proofs.md` (step 55's new EQ bullets and "TLS over a Unix socket: evidence").
  - Every changed and new ID in full: R-0060 and its copies, R-0111, R-0718, R-0982, R-0984, R-1245, R-1253, R-1261 to R-1263, R-1268 to R-1279, T-0312 and T-0318 to T-0323.
  - For coherence: R-0048, R-0061 to R-0063, R-0102, R-0109, R-0113, R-0981, R-1205, R-1235, R-1246, R-1256, T-0306, T-0317, step 41's R-1116, step 42's deliverables, R-0058 and R-0104 to R-0108, and the setting lines in steps 39, 51, 52, 59 and 60.
- **On `origin/main`:**
  - the design, sections 1 to 3 and 6 in full;
  - `reviews/step-55-review-13.md`;
  - INDEX's entries "The settings file is a real `morm.toml`, not a template", "`sslmode` defaults to `require` (EQ1)" and "Per-transaction `search_path` (EQ2)", with its "written in at `8abcd9d`" list and the fix's own choices.
- **The PR body.**

## What I checked and ran

- **CI on the head.** `8abcd9d` is green: `ci` on push (run 44269992533) and on the pull request (run 44270002338), 36 of 36 checks passing. The diff touches only `plans/plan-approved/`: `contracts.md`, `coverage.md`, `proofs.md`, `manifest.json` and steps 10, 11, 12, 14, 20, 27, 39, 42, 47, 48, 50, 51, 52, 59 and 60.
- **Coverage checker,** run on a `git archive` of the head, `.github` included:
  - `check_coverage.py --step 55 --code crates tests benches fuzz .github`: **1279 requirements, 323 test obligations, 0 problems**;
  - the plain form gives the same.
- **Manifest against INDEX.** `git diff 564723e origin/step-55-enum-proofs -- plans/plan-approved/tools/manifest.json`, compared ID by ID:
  - changed: R-0060, R-0111, R-0718, R-0982, R-0984, R-1245, R-1253, R-1261, R-1262, R-1263, R-1268 to R-1276, T-0312, T-0318 to T-0322;
  - new: R-1277 (`code`, step 10), R-1278 and R-1279 (`doc`, step 50), T-0323 (`integration`, step 10);
  - nothing removed; `frame` and `master_sha256` unchanged.

  That's exactly INDEX's "written in at `8abcd9d`" list. Step 42's new deliverable line has no ID, so the manifest can't show it (ER2).
- **R-0060's copies.** Steps 11, 12, 27 and 47 quote step 10's block exactly: 72 non-blank lines each, the same hash once the quote marks are stripped.
- **Leftovers.** No step file and no line of `contracts.md` names a pending entry's horizon, a per-statement prepare sequence, `morm.example.toml` or "settings template". `proofs.md` keeps the template bullet, marked superseded. The PR body keeps two stale lines (ER7).
- **The driver, on a throwaway Postgres 18.6.** The Manjaro package; no Postgres 17 binary is installed here, and I built none. The cluster lived under `.claude/scratch/` in my worktree, with `TMPDIR` set there, its socket under `~/.claude/scratch/` and its port on `127.0.0.1` only. It had `ssl = on` and a self-signed certificate. The probe was built offline against `tokio-postgres` 0.7.18, the version in the workspace's `Cargo.lock`, using `NoTls`:

  | Connection | `SslMode` | Result |
  |---|---|---|
  | Unix socket | `Disable` | connects, `pg_stat_ssl.ssl` false |
  | Unix socket | `Prefer` | connects, false |
  | Unix socket | `Require` | **fails:** `error performing TLS handshake: server does not support TLS` |
  | TCP | `Prefer` | connects, false |
  | TCP | `Require` | fails: `no TLS implementation configured` |
  | One config, hosts `[socket, 127.0.0.1]` | `Require` | fails: the socket fails as above, then TCP fails as above |
  | One config, hosts `[socket, 127.0.0.1]` | `Disable` | connects through the socket, false |

  - `psql` 18.6 with `sslmode=require` connects over the socket with `ssl` false, and over TCP with `ssl` true.
  - The source agrees. `connect_tls.rs` sends the TLS request for `Prefer` and `Require` on any stream and fails `Require` when the server answers `N`. `connect.rs` calls `make_tls_connect(hostname.unwrap_or(""))` for a Unix host before it checks the mode.
  - So `proofs.md`'s table and R-1277's reason hold, and so does the need for one driver config per host in a mixed list. The server, the probe and the socket directory were deleted.

## Each EQ, as decided or recommended

| Finding | Verdict |
|---|---|
| EQ1 | **Fixed for the shipped file.** Every field a URL can carry is a `${VAR}` placeholder naming its own variable, with its default and R-0060's example in the comment. Settings with no default are placeholders too. The profiles are commented out, and `[server]` is active with placeholders. T-0320 loads the file with no variable set, with each variable alone, and with `DATABASE_URL` and `MORM_MIGRATION_URL` pointing elsewhere with `?sslmode=disable`, and gets the URLs' parts. `name`, `user`, `password` and the certificate paths are said to have no default, and `sslmode` has R-1277's. `morm-admin init` puts the override back for some projects (ER2) |
| EQ2 (a) | **Fixed.** In R-0718, R-1270's state table and procedure, R-1271's new bullet, cost and memory, R-0984, R-1253, R-1268, R-1269 and R-1272, a pending statement runs unnamed in both modes. The horizon and the prepare sequence are gone. T-0312 has the ten per-transaction switches in both modes, T-0318 has unnamed runs in both modes, and T-0319's I8 has the `search_path` clause. The procedure's straight-to-Postgres `<n>` handling can stop entries from being made after a migration (ER1). The notice's tuning advice doesn't hold in the common case (ER3) |
| EQ3 | **Fixed.** `contracts.md` and R-0984 say the same thing: `SchemaChanged` is returned "when the next attempt may succeed", with the repairs, D2's first time, the transient `XX000` and D1 with entries off; otherwise `Error::Postgres` |
| EQ4 | **Fixed.** Outside a transaction, C1 reads `<n>` and, if it moved, calls `on_moved` instead of `request`. `on_moved` sets the window's `<n>` and starts a window behind PgBouncer. E1 says so, and T-0318 asserts one adoption and at most one salt per window |
| EQ5 | **Fixed.** I4 and R-1271's churn read `2 × max + 2` per `W` plus `2 × max` per migration, with the `+ 2` explained. I9 is split as R-1271's silent wrong answers are |
| EQ6 | **Fixed.** T-0319 has a `#[cfg(test)]` fault switch for each of the design's seven broken machines, each mapped to the invariant section 6 names: no gate to I1; no epoch to I4 or I7; PR #3's ungated `42804`/`XX000` to I1; an epoch that never ends to I8 or I7; no replacement to I8 or I7; a dropped deferral to I2; no fresh run in the window to I7. Each must be caught within 256 cases, and a fault that isn't caught fails CI. Each fault breaks the property its invariant states, and the model has the events that expose it (abusive stored text, migrations, refused opens, requests inside a window), so the switches would catch all seven. The same simulation, built from the procedure as written, would also fail I1 and I8 through ER1 |
| EQ7 | **Fixed.** The text says straight to Postgres `<n>` is read only for the old-connection test, a session-class failure or B2's `XX000`, and by a request while the window is closed, and that a read finding it moved makes no entry. The procedure has A4's branch, and R-1245 counts any session-class `XX000` as certain. The new `moved()` test is where ER1 lives |
| EQ8 | **Fixed.** `sslmode`'s `# WARNING:` names R-1278, and T-0320 and T-0321 check it |
| EQ9 | **Fixed for ranges and variables.** The loader keeps one table of settings, and T-0320 compares each comment's range and variable with it. Defaults are compared only with that same table (ER6) |
| EQ10 | **Fixed.** R-1263 names the window, the epoch and the cool-down |
| EQ11 | **Fixed.** T-0322 tests `morm-migrate`'s library load. R-1275 is the one place that scopes the removed-setting message, and R-1261 and R-1262 point to it. T-0322 tests `[database]`, `[pool]`, a profile and `[migrations]` |
| EQ12 | **Fixed.** The sentence is struck through and the current reason given |

### Still faithful to the design

Rows A1 to A4, B1 to B5, C1, C2, D1 to D3, E1 to E8, the gate, the epoch, the state bounds, H1 to H4, K1 to K5 and both silent wrong answers read as the design states them. They differ only where an owner decision or an approved fix choice changes them, each recorded in INDEX:

- EQ2: unnamed runs straight to Postgres, which replace H1's per-connection re-prepare;
- EQ4: adoption starts a window;
- EQ3: the contract;
- EQ5: I4 and I9.

Nothing kept in the design's ledger is contradicted. SA to SH and EF1 through EP8 read as before. The two defects below (ER1, ER4) are in the procedure. ER4 was already in the design's own pseudocode. ER1 comes from the new `moved()` and `on_moved` lines, which the design doesn't have.

### The real `morm.toml`

- Every setting is present at its default with its comment.
- Placeholders cover every field a URL can carry, so a copied file never overrides `DATABASE_URL`.
- The header and the four `# WARNING:` lines are as the decision asks.
- One table of settings feeds parsing, validation, the unknown-key check and the tests.
- Steps 39, 51, 52, 59 and 60 add a setting's row and line in the same PR.

T-0320 fails on a missing or extra key, a changed range or variable, or a removed key left in the file, and it checks that the file loads as no file does. Two gaps remain: a default that differs from the plan's isn't caught (ER6), and `init`'s variant of the file isn't tested at all (ER2).

### `sslmode = "require"`

R-1277, R-0111, R-0048, T-0323, R-1278 and R-1279 state the decision:

- TCP only;
- socket hosts given `SslMode::Disable` and no connector;
- one driver config per host in a mixed list;
- `Error::Connect` for a TCP server without TLS, and `Error::Config` without the `tls` feature;
- morm's own suites set `disable`.

The notice is loud and has its "What you give up". The gaps are the connections and generated files the requirement doesn't name, and the notice not saying how to turn the feature on (ER5).

### PR body

It's accurate for this round: the changed and new IDs, choices 34 to 42, no open questions, and the CI links. Two history lines still describe the superseded straight-to-Postgres mechanism (ER7).

## Findings

### ER1. Should fix. R-1270 (step 14), state table and decision procedure; R-0718 row B1: straight to Postgres, "the pool's last-read `<n>`" is never defined or set, and the gate's own read never updates it. After a migration, in-transaction `0A000`s make no pending entry, and EQ2's per-switch failures come back

- **Where:**
  - R-1270 defines `moved(n)` as "a read found `<n>` other than the pool's last-read one".
  - `take` sets `window = (now, n, or the pool's last-read <n> if Unread)`.
  - Only `on_moved` assigns `last_read_n = n`.
  - `request`'s read, `if !window_open() and n is Unread: n = read_n(conn)`, compares only with `window.n` and never calls `on_moved`.
  - The in-transaction branch ends `elif err == 0A000: n = request(...)` and then `if !moved(n): entry(...)`.
  - The state table's only holder of a last-read `<n>` is the marker, which is "behind PgBouncer with the caches on".
  - Straight to Postgres no pool reads `<n>` at start: R-1246 and R-1116 read it only behind a pooler with the caches on.
- **What's wrong:**
  - **The state is missing.** Straight to Postgres, the pool's last-read `<n>` isn't in the state table, which says "each item is stated here once", and nothing sets it before the first `on_moved`. So `take(Clear, Unread)`, the B1 path with the window open, sets `window.n` from an undefined value.
  - **I1 can break.** The next request inside that window reads `<n>`, finds it differs from `window.n`, and takes a second action less than `W` after the first with no migration between.
  - **The entry is lost after a migration.** Say a migration raises `<n>`, and the first in-transaction `0A000` after it finds the window closed. `request` reads the new `<n>`, takes the migration's one action, and sets `window.n`, but `last_read_n` keeps the old value. So that call makes no entry, as intended. But every later in-transaction `0A000` whose request reads `<n>` also finds `moved(n)` true, and makes no entry either. That lasts until a session-class failure or a B2 `XX000` happens to run `on_moved`.
  - **The cost goes back to per switch.** Under a per-transaction `search_path`, each switch then fails one transaction again, which is exactly what EQ2 (a) removed. R-1272's stated cost and T-0319's new I8 clause fail.
  - **No test catches it.** T-0318 asserts that "a moved `<n>` takes a request through a closed window", but not what the next signal does. T-0312 has no migration in its `search_path` case.
- **Evidence:** the procedure and state table as quoted. A trace: straight to Postgres, statements `s1` and `s2` under a per-transaction `search_path`, and a migration at `t0`:
  - at `t0 + 1`, `s1`'s `0A000` with the window closed: the read finds `<n>` moved, the action is taken, and no entry is made, as intended;
  - at `t0 + 2`, `s2`'s `0A000`: the read gives the same new `<n>`, equal to `window.n`, so the action is due; `moved(n)` is true against the stale `last_read_n`, so again no entry;
  - every switch of `s1` or `s2` from then on fails one transaction.
- **Fix:**
  - Straight to Postgres, give the pool a last-read `<n>` in the state table. Either set it at pool start, through R-1116's startup statement with no extra round trip, as behind a pooler, or set it at the first read, which then doesn't count as moved.
  - Every read that finds `<n>` moved calls `on_moved`, the gate's read in `request` included. So "a call that reads it and finds it moved makes no pending entry" holds for that one call only.
  - `take` uses that value for `Unread`.
  - T-0318 asserts: two `0A000`s of different statements inside one window with `<n>` unmoved take one action, straight to Postgres; and after the gate's read finds `<n>` moved, the next in-transaction `0A000` in that window makes an entry.
  - T-0312's `search_path` case adds a migration between two switches: one more `SchemaChanged`, then unnamed runs.

### ER2. Should fix, a design choice for the owner. Step 42's new deliverable: `morm-admin init` writing a detected non-default socket's `host` and `port` as active lines brings back EQ1's silent override of `DATABASE_URL`. It isn't loud, it has no ID or test, and it misses a non-default port in the default directory

- **Where:**
  - Step 42's new deliverable: "when the socket is somewhere other than the default `host`, `init` writes the found directory and port in place of the `host` and `port` placeholders, with a comment beside them saying that a value written here overrides `DATABASE_URL`'s host and port".
  - INDEX lists this among the fix's choices "the owner approves through PR #3", and the PR body's choice 37 says the same.
  - R-0104 to R-0107 (detection, where R-0105's socket file name also gives the port), R-0058 and R-0060's comment "Separate fields override URL parts". R-0062: "defaults, then `morm.toml`, then environment variables".
- **What's wrong:**
  - **It's the case EQ1 blocked.** The owner's decision, and EQ1's fix, rest on "a copied file never moves an app off the host ... its `DATABASE_URL` names" (R-1273). A project made by `init` is that copied file. The socket is somewhere other than `/var/run/postgresql` on Arch and Manjaro (`/run/postgresql`) and on a Homebrew Mac (`/tmp`), so those projects get `host = "/run/postgresql"` or `host = "/tmp"` as an active line.
  - **Deployment then breaks quietly.** When such an app is deployed with `DATABASE_URL` naming a managed database, it connects to the local socket instead: silently, if the server has a local Postgres there, or with a connection error naming a socket path the operator never set. It's the same hazard; only the path into the file differs.
  - **It isn't loud.** The only warning is an ordinary comment. There's no `# WARNING:` line, no README notice, nothing at startup, and nothing in T-0320 or T-0321. That falls short of the owner's rule for loud notices, and short of R-1273's own warnings, which every other line that changes what the app does without saying so has.
  - **No ID, no test.** It's a Deliverables bullet, so the checker and the manifest can't see it. Nothing tests that `init` writes the shipped file byte for byte, that only those two lines differ, or that the comment is there.
  - **The default directory with another port is missed.** "Somewhere other than the default `host`" excludes `/var/run/postgresql/.s.PGSQL.5433`. For that socket the placeholders stay, the app tries port 5432, and R-0104's "writes it into `morm.toml`" fails.
  - **R-0107 isn't reconciled.** When no socket is found, it writes "the TCP settings as a commented-out alternative". The bullet doesn't say how that fits "byte for byte". Under the new default, a local TCP server without TLS also needs `sslmode = "disable"`, which that alternative doesn't say.
  - **The plan's precedence is unsettled.** R-0062 puts environment variables, `DATABASE_URL` among them, above `morm.toml`. R-0060's comment says the file's separate fields override URL parts. The bullet, and EQ1's fix, assume the second.
- **Options:**
  - **(a) Recommended. Keep the active lines and make them loud.**
    - A `# WARNING:` line beside them names a new README notice in step 50, with a new ID: "A `host` or `port` written in `morm.toml` overrides `DATABASE_URL`", with what's lost and how to proceed (set `MORM_DB_HOST` and `MORM_DB_PORT`, or put the placeholders back before deploying).
    - morm warns at startup, through the warning sink, when a `[database]` field from `morm.toml` overrides a different value in `DATABASE_URL`, naming both and never the password.
    - `init` also writes a non-default port found in the default directory.
    - R-0107's alternative carries `sslmode`'s line and its warning.
    - The bullet becomes a `code` requirement with a test obligation in step 42. With the default socket, `init`'s file equals the shipped file byte for byte. With another socket, only `host`, `port` and the warning lines differ. With `DATABASE_URL` naming another host, the app starts with the warning.
  - **(b) Keep the placeholders.** `init` writes the found values only into `.env.example` and prints the `MORM_DB_HOST` and `MORM_DB_PORT` lines to export. No override is possible, but R-0104's "most projects never set it by hand" no longer holds on those systems.
  - **(c) Settle the precedence the other way.** An environment `DATABASE_URL` outranks `morm.toml`'s separate fields, as R-0062 reads, so an active file `host` can't override it. This changes EQ1's premise, and it needs a rule for a file `url = "${DATABASE_URL}"`, whose value then counts as the environment's.

  Whichever is chosen, give it IDs and tests, and handle the port.

### ER3. Should fix. R-1272 (step 50): "Tune `stale_pending_expiry`, longer for fewer failures" doesn't hold when a pending statement's values recur or more than one statement switches. The switches request a pool-wide action every window themselves, and each one ends every entry

- **Where:**
  - R-1272's mitigations: "**Tune `stale_pending_expiry`,** longer for fewer failures and more unnamed runs".
  - Its cost: "about one failed transaction per statement per `stale_pending_expiry` ... plus one after each pool-wide repair".
  - R-0718: a confirming run's confirmation requests *clear* or *salt*. R-0982 and E5: taking an action removes every pending entry, and a request inside the window is due at its end.
- **What's wrong:**
  - **Recurring values.** Under a per-transaction `search_path`, statement `s` fails with `0A000`, which takes *clear* (or *salt*) and makes `s`'s entry with that call's bytes. Any later run with the same bytes, such as the same `id`, runs unnamed, succeeds and is a confirmation, so it requests *clear*. The window is closed, so the request is due and taken at the window's end, and that removes `s`'s entry. The next switch fails again.
  - **More than one statement.** Two such statements do the same to each other with no repeated values: `s2`'s `0A000` inside `s1`'s window is due, and its action at the window's end ends both entries.
  - **So the window sets the rate.** In the common case the failures come about once per `stale_cooldown` per statement, and the pool clears every statement cache (or adds a salt) every `stale_cooldown`, whatever `stale_pending_expiry` is. A user who follows the advice and raises it to `"10m"` sees no change.
  - The stated cost is still true as a formula, because "plus one after each pool-wide repair" covers it. Only the advice misleads.
  - T-0312's `search_path` case uses "different values" and one statement, so it never sees this.
- **Evidence:** the rules quoted above; the trace follows from R-0718's confirming run and E4/E5 with no other input.
- **Fix:**
  - The notice says the switches themselves request a repair at most once per `stale_cooldown`, which ends every pending entry. So with recurring values, or more than one such statement, failures come about once per `stale_cooldown`. It changes the tuning advice to raise `stale_pending_expiry` and `stale_cooldown` together, and says what a longer `stale_cooldown` costs (R-1268).
  - R-1271's bullet says the same rate.
  - T-0312 adds a case with two statements, or one with recurring values, at `stale_pending_expiry = "10s"` and `stale_cooldown = "1s"`: about one `SchemaChanged` per second, set by `stale_cooldown`.

  This keeps the owner's EQ2 mitigations and only makes them accurate.

### ER4. Minor. R-1270's `session()` and R-0981's rows A2 and B3 (step 14): a certain signal on an old connection requests *replace* twice, so every connection is replaced a second time one window later, for nothing

- **Where:**
  - The procedure: `if err in {42804, XX000}: request(Replace, n)`. Then, outside a transaction, `r = replace(conn, call); if confirms(r, err): request(Replace, n)`. Inside one, `expect_confirm(call, Replace)`, whose confirming run requests *replace* again.
  - Row B3: "`42804` and `XX000` request *replace*; so does the next attempt's run ... if it confirms".
  - R-0982: a request inside the window is due and taken at the first checkout after it.
- **What's wrong:**
  - The first request takes *replace* and starts the window. The second, from the same failure's confirmation, finds the window closed with `<n>` unmoved, so it's due.
  - `W` later every connection is replaced again, though every one was opened after the change and is already cured. That's `max` needless opens per incident.
  - It stays inside K3's bound, so no invariant catches it. T-0312's B3 case uses a `22P02` signal, which requests only once.
  - The design's pseudocode has the same lines. Its row A2 says "A confirmation, or `42804` or `XX000`, requests *replace*", which reads as one request, and R-1245 already counts those two signals as confirmed on their own.
- **Fix:** request *replace* once per failure. For `42804` and a session-class `XX000`, the confirmation requests nothing more. In B3, `expect_confirm` applies only to a signal that isn't certain on its own. T-0318 asserts that a `42804` on an old connection, outside a transaction and inside one with `.retry(1)`, leaves no due action. This needs no owner choice, because K1 never counted two actions for one failure.

### ER5. Minor. R-1277, T-0323, R-1278 (steps 10 and 50): `require`'s reach beyond pools isn't stated, `init`'s generated CI workflow will fail on it, a mixed host list without `tls` isn't tested, and the notice doesn't say how to turn `tls` on

- **Where:**
  - R-1277: "That holds for every pool and profile, `[database.migration]` and PgBouncer over TCP included".
  - R-1256 (step 59): `pgbouncer_admin_urls` connections to PgBouncer's admin console. R-0080: `scratch_url`.
  - R-0058 (step 42): `init` creates "a CI workflow". R-0107: `init`'s commented TCP alternative.
  - `contracts.md` R-0019: `tls` is "not compiled in unless enabled".
  - R-1278's "How to proceed".
- **What's wrong:**
  - **Admin console and scratch connections.** Whether the default applies to admin-console connections and to `scratch_url` is unsaid. If it applies, a PgBouncer without `client_tls_sslmode` refuses every `RECONNECT` send, and `migrate` exits `1` after each migration. If it doesn't, the admin password crosses TCP in clear by default. Either way R-1258 should say what to set.
  - **`init`'s CI workflow.** A typical CI service container is TCP without TLS, so a workflow generated by `init` fails on its first run with `Error::Config` (no `tls`) or `Error::Connect`. Only "morm's own test suites" set `disable`.
  - **Mixed host lists.** A list holding a socket and a TCP host without the `tls` feature refuses to start, by R-0111's per-host rule. T-0323 doesn't test it.
  - **The notice.** `tls` is off by default, so every TCP user meets the `Error::Config` first. R-1278 says morm stops "without the `tls` feature", but its "How to proceed" never says to turn the feature on.
- **Fix:**
  - R-1277 names every connection morm opens, pools, `[database.migration]`, `scratch_url`, `pgbouncer_admin_urls` and `morm-admin`'s own, and says each gets the default. R-1258 says an admin URL to a PgBouncer without client TLS needs `?sslmode=disable`.
  - Step 42's CI workflow sets `MORM_DB_SSLMODE=disable` for its service container, with a comment naming R-1278. R-0107's alternative carries the `sslmode` line.
  - T-0323 adds the mixed list without `tls`, and an admin-URL case goes in step 59's tests.
  - R-1278's "How to proceed" adds "enable morm's `tls` feature", in its `Cargo.toml` form.

### ER6. Minor. T-0320, R-1273, R-0102 (step 10): the default check compares the file with the loader's own table, so a default that drifts from the plan passes; and R-0102 gives no default `port`

- **Where:** T-0320: "an active setting with a default holding R-0060's value, or, where R-0060 shows another value than the default, giving R-0060's in its comment". The test learns "the default" from the loader's table. R-1273: "`host` and `port` have R-0102's defaults". R-0102 states only the default `host`.
- **What's wrong:**
  - If step 10 puts a wrong default in the table, say `wait_timeout = "10s"`, the file holds `"10s"` and its comment shows R-0060's `"5s"` as an example. T-0320 passes, and so does "loads as no file does".
  - Only settings with their own default tests, such as T-0317's stale-session four, would catch it. The owner's rule is that the file is "kept in step with the plan by a test".
  - R-1273 now says exactly which R-0060 values aren't defaults: `name`, `user`, the placeholders, `[server]`'s `ram` and `cores`, and the commented-out profiles. So the test can be strict.
- **Fix:**
  - T-0320 asserts that, for every other key of R-0060's block, the table's default equals R-0060's value, and fails naming the key otherwise.
  - R-0102, or R-1273, states that the default `port` is `5432`.

### ER7. Minor. PR body: two history lines still describe the superseded straight-to-Postgres pending mechanism, unmarked

- **Where:**
  - "The consolidated stale-session rewrite": R-0718's pending entries are described as having "one per statement text, horizon, confirming bytes, expiry".
  - "The owner's answers", EN1: "**Straight to Postgres, refined by the rewrite:** a pending entry makes each connection prepare its older copy again (R-0718)".
- **What's wrong:** EQ2 (a) removed the horizon and the re-prepare. Choice 25 and the "Fix choices that change" paragraph are marked superseded, but these two lines aren't, so the body contradicts itself.
- **Fix:** mark both superseded by EQ2: a pending statement runs unnamed in both modes.

## Summary

| Severity | Findings |
|---|---|
| Blocks approval | None |
| Should fix | ER1, ER2 (a design choice), ER3 |
| Minor | ER4, ER5, ER6, ER7 |

EQ1 to EQ12 and the three owner decisions are written in as decided or recommended:

- the real `morm.toml` and its placeholders, so the shipped file never overrides `DATABASE_URL`;
- `sslmode = "require"` over TCP only, with the driver claim confirmed on Postgres 18.6;
- pending statements unnamed in both modes;
- the contract, adoption, invariants, fault switches and the smaller fixes.

The manifest matches INDEX ID for ID, the checker reports 0 problems, and CI is green.

What remains:

- **ER1:** one missing piece of state in the procedure's straight-to-Postgres `<n>` handling, which after a migration undoes EQ2's fix.
- **ER2:** `init`'s active `host` and `port`, which bring EQ1's override back for projects made on non-Debian systems, without a loud notice or a test. It needs an owner choice.
- **ER3:** one piece of tuning advice in the `search_path` notice that doesn't hold.
- **Minor:** ER4 to ER7.

PR #3 isn't ready for the owner until ER1 to ER3 are fixed and ER2's choice is made. The minor findings can be fixed in the same round.
