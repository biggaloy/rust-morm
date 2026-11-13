# Step 02 independent review, round 5

PR #2 (`step-02-proofs`), head `f4bdd82`. This is the fifth review, after the fixes for the fourth review's DK1 to DK9. It's done as `rules.md` "Independent review" says, by an agent that didn't write the step.

I read `rules.md`, `contracts.md`, `proofs.md` and `steps/02-proofs.md` on the PR branch. On `origin/main` I read:

- every INDEX "Decisions since the split" entry from "Step 02's proofs, DA to DE" through "Step 02 fourth review, DK5", with its DK1 and DK2 sentences and its list of IDs;
- `reviews/step-02-review-4.md` and `reviews/step-02-open-items.md`.

I read the round's whole diff, `git diff 47be1d6 origin/step-02-proofs`. It touches only `plans/`:

- `contracts.md`, `coverage.md`, `proofs.md` and the manifest;
- steps 10, 12, 16, 22, 41, 43 and 50.

To check the new text against what it changes, I also read, at the head:

- CI's `pooler` job, `start_pgbouncer.sh` and `pgbouncer.ini`;
- `tests/tests/repo_policy.rs`, step 01's tests of that job (R-0792);
- `README.md`, step 50's deliverables and R-0798;
- `contracts.md`'s `Conn`, and where `connect_to` and `db.pool(name)` come from (steps 10 and 11).

## What I checked and ran

- **CI on `f4bdd82`:** `ci` passed on its push run (43377193053) and its pull request run (43377200321). That covers every job: coverage, fmt, clippy, unit, trybuild, doc, integration on Postgres 15 to 18 over TCP and Unix sockets, pooler, msrv and features. The commit changes only plan text.
- **Checker:** `check_coverage.py --step 02 --code crates tests benches fuzz .github` and the plain form, run on a checkout of the head, both report **1239 requirements, 300 test obligations, 0 problems**.
- **Manifest:** I ran `git diff origin/main...origin/step-02-proofs -- plans/plan-approved/tools/manifest.json`, and compared the manifests by ID with a script, against `47be1d6` and against `main`. `frame` and `master_sha256` are unchanged.
  - **This round changed 12 IDs:** R-0670, R-0677, R-1205, R-1210, R-1223, R-1224, R-1234, R-1236, T-0104, T-0284, T-0297 and T-0299.
  - **It added 4:** R-1237, R-1238, R-1239 and T-0300.
  - Nothing was removed, and no ID changed its step or kind.
  - **INDEX's DK entry** lists exactly these 16 IDs, so every one is covered.
  - **Against `main`,** the whole PR changes 27 IDs and adds 58, R-1205 to R-1239 and T-0278 to T-0300. The 26 the fourth review counted plus R-0677, and the 54 it counted plus this round's 4. Every one is in a Decisions entry.
  - **`coverage.md`** has rows for the four new IDs, and its totals read 1239 and 300.
  - **Step 43's quote of R-0670** is byte for byte the same as step 12's.
- **Stale text (item 2):** I searched `plan-approved/` for:
  - "older versions";
  - "one startup parameter";
  - "only `TimeZone`";
  - `Params: Sync` alone;
  - "the pinned 1.26.1 image";
  - "PgBouncer 1.22 and on";
  - "No startup parameters";
  - `application_name` not being sent;
  - any support for PgBouncer before 1.20.

  None is still live. R-0677's "1.21+" is right: it's about prepared statements. Every mention of `application_name` agrees with DK5. `contracts.md`, `proofs.md`, steps 10, 12, 16, 22, 41, 43 and 50 all say 1.20 or later, `Send + Sync`, and the two parameters, in the same way.
- **PgBouncer, run (item 4).**
  - **Downloads.** I downloaded pgbouncer.org's tarballs for 1.19.1, 1.20.0, 1.20.1 and 1.26.1, with their `.sha256` files. Each checksum matched:
    - 1.20.1: `24992cf557d73426d7048698dffc7b019e6364d4d8757ae2cf5e2471286a2088`;
    - 1.20.0: `e70d5a7cb8b71dd7dbabfd3571d71a4b6b99f2e85d8d71af1e734f6d86635f0e`;
    - 1.19.1: `58c3eff9bb72c18133b28e1f034fd59356ea76281c65e127432ca101c208a394`.
  - **1.20's NEWS** has "Add support for the `options` startup parameter ... This only works for PostgreSQL parameters that PgBouncer tracks". 1.20.1's NEWS fixes a 1.20.0 regression with `options` in `ignore_startup_parameters` (see DL4).
  - **Setup.** I built 1.19.1 and 1.20.1 from source. They ran in transaction mode with `default_pool_size = 1`, in front of a throwaway Postgres 18.6 whose own zone was `Asia/Dhaka`. Clients logged in as a role with `timezone = 'Europe/Berlin'`, through `tokio-postgres` 0.7.18 with `application_name("morm_app")` and `options("-c TimeZone=UTC")`.
  - **1.19.1, `ignore_startup_parameters` empty:**
    - the connection failed with `DbError` severity `FATAL`, SQLSTATE `08P01`, message `unsupported startup parameter: options`;
    - `tokio_postgres::Error`'s `Display` is only `db error`, so morm has to quote the `DbError`'s message, which R-0670's wording allows;
    - a client sending no `options` connected.
  - **1.19.1 with `options` in the list:** the session reported `Europe/Berlin` and `morm_app`. So R-0670's workaround needs the role set to `UTC`, as it says, and `application_name` passes even on 1.19.
  - **1.20.1:**
    - the client reported `UTC` and `morm_app`, and `UTF8`;
    - a client sending nothing reported `Europe/Berlin`, and then ran `SET timezone = 'Asia/Tokyo'` on the same backend pid;
    - after that, the first client still reported `UTC`;
    - a new `-c TimeZone=UTC` client reported `UTC`, and `'2026-01-01 00:00:00+00'::timestamptz::timestamp` gave `2026-01-01 00:00:00`;
    - a new client sending nothing reported `Europe/Berlin`;
    - `-c lock_timeout=5s` failed with `unsupported startup parameter in options: lock_timeout`.
  - **1.20.1 with `TimeZone` in the list** kept `UTC`. That agrees with R-1238: only 1.26.1 or later can drop the parameter.
  - **What I didn't rerun:** I had only Postgres 18.6 running, so I didn't rerun the 17.11 pairs. I didn't rerun 1.22.1 and 1.26.1, which the fourth review ran.
- **`Params`, run (item 4).** I built a scratch crate on rustc 1.99.1 with `tokio-postgres` 0.7.18. It has `pub trait Params: Send + Sync` with `bind(&self) -> impl ExactSizeIterator<Item = (&(dyn ToSql + Sync), Type)> + Send`, implemented for flat tuples of 0 to 64 values, each returning an array iterator. It ran on the same 18.6 server. These all compiled and ran:
  - **A `Conn`-like trait.** `fn sql<P: Params>(&self, q: Sql<'_, P>) -> impl Future + Send` and the `&Sql` form, with each future yielding before it reads `q`.
  - **A hook-like `async fn`** generic over `db: &impl Conn`. It built a query with run-time temporaries `&(n * 2)` and `&m`, where `m` came from `black_box`, and ran it by reference and then by value. It was spawned through `tokio::spawn` behind a `Send` assertion, and gave 81 for `n = 20`, right.
  - **Inside `tokio::spawn`,** a query holding `&(x + 1)` ran by reference and by value, and gave 22, right.
  - **A flat 64-tuple** returned as `Sql<'a, impl Params + use<'a>>`: `bind().len()` was 64, and `query_typed_raw` summed the 64 values to 2080, right.
- **README notices and "use this when" lines (item 5):**
  - R-1237 is loud, beside `pooler = "transaction"`, and says what to do.
  - R-1236's new line says what to do at the 64-value limit.
  - R-1239 is the "use this when" line for `-> Sql<'a, impl morm::Params>`.
  - R-1238's CI skip has a loud notice, but only inside a `code` requirement, and the README has no contributor section yet (DL2).
- **The PR body (item 6)** matches the branch except for one line (DL7).
- **Cleanup.** The server, the PgBouncers, the tarballs and the scratch crate are stopped and deleted.

## The fourth review's findings

| Finding | Fixed as recommended or decided? | Evidence |
|---|---|---|
| DK1 | Yes, the owner's option A | R-0670 and its step 43 quote: "Only tracked startup parameters, on PgBouncer 1.20 or later", and the refusal becomes `Error::Config` from all four entry points, quoting PgBouncer and naming both fixes, whatever `pooler` says. R-0677: 1.20 is the oldest supported. R-1205, R-1210 and R-1234: "1.20 or later". New R-1237 is the loud notice, and new T-0300 tests it on a 1.19. `proofs.md` has DK1. My 1.19.1 and 1.20.1 runs agree with every claim. |
| DK2 | Yes, the owner's decision, with gaps in how it reaches step 01's files | New R-1238 in step 12 (`code`, tagged in the workflow): 1.20, 1.22 and 1.26.1 on 15 to 18 on every run; pinned sources; a second 1.26.1 that drops `TimeZone`; a 1.19 for T-0300; skip with a `::warning::` naming both versions; one that installs but doesn't answer fails; a loud contributor notice. T-0104 and T-0297 point at it. See DL1 to DL4. |
| DK3 | Yes | `contracts.md`: `pub trait Params: Send + Sync`, with the comment saying why. R-1223 says the same. T-0299 adds a function generic over `db: &impl Conn` running `db.sql(q)` and `db.sql(&q)` inside `tokio::spawn`. My scratch crate compiled and ran it. |
| DK4 | Yes | `proofs.md`'s DJ3 line says the `1_i64 + 1` was promoted and cites the run-time evidence. T-0299 asks for "a free value computed at run time, such as `n + 1` with `n` a variable". The stray space is gone from both matrix rows. The optional `trybuild` case was left out, with the reason in the PR body. T-0299's run-time value already guards the struct-literal expansion. |
| DK5 | Yes, the owner's option A | R-0670 and R-1205: `application_name` is its own startup parameter, and `TimeZone` is "the one setting sent in `options`". R-1237 tells users. T-0104 and T-0300 check `pg_stat_activity`. `proofs.md`'s list of startup fields includes `client_encoding`. |
| DK6 | Yes | `contracts.md`: flat tuples of 0 to 64 values, with `sql!` refusing a 65th. R-1223, and R-1224's new refusal naming the limit and three ways round it. T-0284 is now 19 cases; I counted 19 plus the `E0401` case. T-0299 runs a 64-value query. R-1236 has the README line (see DL6 on its wording). A 64-tuple compiled and ran. |
| DK7 | Yes | R-1239 is in step 50's table beside R-1218, and the PR body lists it. |
| DK8 | Yes | T-0297's last case runs "through the PgBouncer that drops the parameter", and its sessions report `Europe/Berlin`. |
| DK9 | Yes for its four lines | The INDEX line, `Params: Sync`, the 1.26.1-image line and "the one startup setting" are updated. One line went stale since (DL7). |

## Findings

### DL1. R-1238 contradicts step 01's tests of the `pooler` job, and doesn't say the apt-to-image fallback ends

- **Severity:** should fix
- **IDs:** R-1238, T-0300; R-0792; DK2
- **Files:** `steps/12-hold-pooler.md`; step 01's `tests/tests/repo_policy.rs` and `.github/scripts/start_pgbouncer.sh`; INDEX, the "pooler install" decision

**What's wrong.**

- R-1238 says step 12 "changes step 01's `ci.yml`, `start_pgbouncer.sh` and `pgbouncer.ini`". It doesn't name `tests/tests/repo_policy.rs`. Three of that file's R-0792 tests pin the job as it is now, and each breaks under R-1238:
  - `integration_targets_each_version_and_transport` asserts that the `pooler` job has `image: postgres:18` and `MORM_TEST_PG_MAJOR: 18`. R-1238 runs 15 to 18.
  - `pooler_starts_pgbouncer_from_another_source_when_apt_fails` asserts that when apt fails twice, PgBouncer starts from the 1.26.1 image. It also asserts that "every source failing fails the step". R-1238 says a version that can't be installed is skipped, with a warning, not failed.
  - `ci_runs_every_check_the_plan_names` pins the job's two commands.
- `start_pgbouncer.sh`'s header still says "the pooler tests never run without PgBouncer".
- The INDEX decision for the pooler install (apt, one retry, then the pinned image, and a failing step if that fails) still reads as standing. R-1238 never says that fallback goes.
- So step 12's session could keep the fallback. A failed apt install would then run the "1.22" cases on 1.26.1, which the owner's DK2 answer says to skip. Or the session would meet red tests in a `done` step's file that its own step file doesn't mention, and have to stop and ask.
- **T-0300's last clause can't be proved where it's placed.** It says "a version that can't be installed is skipped with a `::warning::` ..., and one that installs but doesn't answer fails the job". T-0300 is an `integration` obligation "in the `pooler` job", and a CI run can't make an install fail on demand. So, as written, R-1238's riskiest path, a skip that could go silent, is tagged only in the workflow and never runs. Step 01 already has the right pattern: stubs for `apt-get`, `docker` and `pg_isready` that drive every branch of the script.

**Evidence.**

- `repo_policy.rs` at `f4bdd82`, lines 342 to 372 and 374 onward. The `down` case asserts `ok` with a `docker run` of the image, and the `docker-fails` case asserts `!ok`, "every source failing fails the step".
- `start_pgbouncer.sh` at `f4bdd82`, lines 1 to 17 and the `else` branch running `$image`.

**Fix.** In R-1238:

- name `tests/tests/repo_policy.rs` among the files step 12 changes;
- say that the apt-to-image fallback ends. The 1.26.1 image is its own version now, so a 1.22 that can't be installed is skipped, never replaced. Record in the DK2 Decisions entry that this replaces the step 01 fallback decision.
- require the stubbed script test to be rewritten for each version's paths, tagged R-1238:
  - installs on the second try: runs;
  - fails twice: skipped, exit 0, and a `::warning::` naming the PgBouncer version and the Postgres version;
  - installs but doesn't answer: the job fails.

Then T-0300's last clause points at that test, or moves into R-1238's own wording, so that no integration test claims it.

### DL2. The contributor notice for a skipped PgBouncer has no home that lasts, and nothing checks it's written

- **Severity:** should fix, needs a small owner choice (where contributor docs live)
- **IDs:** R-1238, R-0798; DK2
- **Files:** `steps/12-hold-pooler.md`, `steps/50-readme-release.md`

**What's wrong.**

- The owner asked for the skip rule to be stated loudly in the contributor docs. R-1238's last bullet puts it in "`README.md`'s section for contributors". But:
  - **There's no such section.** The README at the head has only a title, a status line, Requirements and License.
  - **No step owns one.** Step 50 builds the "Full README". R-0798 calls it "one compact, complete file ... enough to use morm", included as crate docs. Nothing in step 50 mentions contributors, so step 50's rewrite can drop the line step 12 wrote.
  - **Nothing checks it.** Every other loud notice in this PR (R-1210, R-1211, R-1215 to R-1217, R-1236, R-1237) is its own `doc` ID in step 50, with the text drafted. This one is half of a `code` requirement whose tag sits in the workflow. The checker can't tell whether it was written, and step 50's review has no ID to look for.
- INDEX's DK2 entry says "the contributor docs". Putting them in README is the fix's own choice, and it isn't in the list of choices INDEX approves through PR #2.

**Options:**

- **A (recommended).** Keep it in the README, in a short "Contributing" section at the end, and give the notice a `doc` ID in step 50 (R-1240, the next free ID) with the text drafted, for example:

  > **A green `pooler` job can have skipped a PgBouncer version.** When a version can't be installed in a run, its cases are skipped, not failed, and the job writes a `::warning::` naming the PgBouncer and Postgres versions. Read the job's warnings before trusting a green run.

  R-1238 then says step 12 writes that section now, and step 50 keeps it.
- **B.** Put the same `doc` ID's text in `CONTRIBUTING.md`, which GitHub shows to contributors and which stays out of the crate docs. R-1238 says so.

### DL3. R-1238 doesn't say how `max_prepared_statements` is set per PgBouncer, and the default changed between 1.22 and 1.26.1

- **Severity:** minor
- **IDs:** R-1238, R-0677, T-0104
- **Files:** `steps/12-hold-pooler.md`, `steps/22-locks-timeouts.md`

**What's wrong.**

- R-0677 says prepared statements need 1.21 or later "with `max_prepared_statements` set", and caches off on 1.20. T-0104 checks 1.20 with caches off.
- PgBouncer 1.24.0 changed `max_prepared_statements`' default from 0 to 200 (1.26.1's NEWS: "Enable prepared statement support by default, `max_prepared_statements` is now set to 200 by default").
- So with one shared `pgbouncer.ini` that doesn't set it, as now:
  - 1.22 runs with prepared statements unsupported;
  - 1.26.1 runs with them on;
  - 1.19 and 1.20 don't know the key.
- The same tests behind 1.22 and 1.26.1 then exercise different R-0677 cases by accident. Steps 14 to 22, whose statement caches run in this job, aren't told which instance is which.
- R-1238 says step 12 changes `pgbouncer.ini`, but not what each instance's config sets.

**Fix.**

- R-1238 says each 1.22 and 1.26.1 instance sets `max_prepared_statements` explicitly, for example 200, so the two versions test the same thing.
- 1.19 and 1.20 get their own config without the key, and their runs set `statement_cache = 0` and `shape_cache = 0`.
- T-0104 then says that on 1.22 and 1.26.1 the statement caches are on.

### DL4. R-1238 leaves the exact versions, their check and their build inputs open

- **Severity:** minor
- **IDs:** R-1238, T-0297, T-0300
- **Files:** `steps/12-hold-pooler.md`

**What's wrong.**

1. **The exact versions.** R-1238 says "1.20" and "1.19" built from a tarball "checked by SHA-256", with no patch version. 1.20.0 has a known regression around `options` that 1.20.1 fixes (1.20.1's NEWS: "Fix regression where putting `options` inside `ignore_startup_parameters` would not ignore unknown parameters inside the `options` startup parameter anymore ... introduced in 1.20.0"). That's harmless for morm's `TimeZone`, but it's the version users are most likely to run as "1.20", and it's the worse one to test against.
2. **Checking the version.** Nothing checks that each instance reports the version it stands for. The "1.22 from the runner's apt sources" is only as fixed as `ubuntu-24.04`'s archive, and DL1's fallback could put a 1.26.1 in its place.
3. **Build inputs.** Building 1.19 and 1.20 needs `libevent`, OpenSSL headers and `pkg-config`. On the runner those come from the same Ubuntu archive whose outage caused the fallback. INDEX notes that `apt.postgresql.org` lacks `libevent`. So one mirror outage skips 1.19, 1.20 and 1.22 together, leaving only 1.26.1, under warnings. That's within the owner's rule, but R-1238 doesn't say where the build inputs come from.
4. **The second 1.26.1.** R-1238 doesn't say whether the second instance runs in front of each of Postgres 15 to 18. T-0297's refusal and alias cases run through it, and T-0297 is "on Postgres 15–18".

**Evidence.** The checksums above, from pgbouncer.org's `.sha256` files, and 1.20.1's `NEWS.md`.

**Fix.** In R-1238:

- name 1.20.1 and 1.19.1, with the SHA-256s above;
- have each instance's startup check `SHOW VERSION` on its admin console, or `pgbouncer --version`, and fail on a mismatch;
- say the second 1.26.1 runs in front of each of 15 to 18;
- name where the build inputs come from, for example building inside a container image pinned by digest, so an Ubuntu mirror outage skips only 1.22.

### DL5. T-0300 tests the old-PgBouncer error from two of the four entry points R-0670 names

- **Severity:** minor
- **IDs:** R-0670, T-0300
- **Files:** `steps/12-hold-pooler.md`

**What's wrong.**

- R-0670 says the `Error::Config` for `unsupported startup parameter: options` comes "from `Db::connect`, `connect_as`, `connect_to` and a pool `db.pool(name)` adds".
- T-0300 checks only `Db::connect` and `connect_as`.
- `connect_to` (step 10) and `db.pool(name)` (step 10's R-0115, step 11) both exist before step 12. A pool added later opens its connections on its own path, which is where a missing mapping would hide.

**Fix.** T-0300: "`Db::connect`, `connect_as`, `connect_to` and a pool added by `db.pool(name)` each return `Error::Config` ...".

### DL6. R-1236's new line says "Rust has no tuples of any length", which isn't so

- **Severity:** minor
- **IDs:** R-1236; DK6
- **Files:** `steps/50-readme-release.md`

**What's wrong.**

- The README text reads "Its values are a tuple, and Rust has no tuples of any length, so `sql!` refuses a 65th value".
- Rust has tuples of any length. What stable Rust lacks is a way to implement a trait for every length (no variadic generics), so morm implements `Params` for a fixed range.
- This is user-facing text that step 50 copies word for word.

**Fix.** For example: "Its values are a tuple, and Rust can't implement a trait for tuples of every length, so morm implements it for up to 64 values, and `sql!` refuses a 65th at compile time with an error naming the limit."

### DL7. The PR body has one stale line

- **Severity:** minor
- **IDs:** none
- **Files:** PR #2's description

**What's wrong.**

- "INDEX's Decisions name the IDs of every round through DJ; the DK entry names none yet, so the coordinator may want to add the DK IDs above to it."
- INDEX's DK entry on `main` now names all 16 DK IDs, the same set as the body's DK table.

**Fix.** Say the DK entry names them. With DL1 to DL6 fixed, add their IDs to the DK round's table.

The rest of the body matches the branch:

- the CI results and the commit list through `f4bdd82`;
- the checker totals, 1239 and 300 with 0 problems;
- the DK round's table, 16 IDs, matching the manifest;
- the DK1 to DK9 section;
- the DK round's choices;
- the "use this when" lines, R-1239 included.

## Requirement by requirement, for the IDs changed since `47be1d6`

- **Step 10.** R-1205 matches DJ2, DK1 and DK5: `-c TimeZone=UTC` in `options`, beside `application_name`, on 1.20 or later. Nothing is weakened.
- **Step 12.**
  - **R-0670** matches DK1 and DK5, and my 1.19.1 and 1.20.1 runs: the refusal text, SQLSTATE `08P01`, the workaround leaving the role's zone, and `application_name` passing.
  - **R-1238** is DK2 as decided, plus the fix's 1.19 (approved in INDEX). It doesn't reach step 01's tests or the fallback (DL1), the contributor notice (DL2), the per-version config (DL3), or the exact versions and their check (DL4).
  - **T-0300** covers the error, `pooler = "none"`, the workaround and both parameters on every pair. Its entry points are short (DL5), and its skip clause can't run where it's placed (DL1).
- **Step 16.**
  - **R-1223** matches DK3 and DK6.
  - **R-1224** adds the 65th-value refusal with the limit named and three ways round it.
  - **T-0284** is 19 cases, among them the 65th value.
  - **T-0299** adds the run-time value, the `&impl Conn` function inside `tokio::spawn` and a 64-value query.
  - My scratch crate ran each of these shapes.
- **Step 22.**
  - **R-0677** names 1.20 as the oldest supported.
  - **T-0104** checks `application_name`, every pair the job runs, and 1.20 with caches off. Which cases run with caches on is open (DL3).
- **Step 41.**
  - **R-1234** says 1.20 or later.
  - **T-0297** runs on every pair, its refusal through the second 1.26.1, and its last case through that instance with sessions reporting `Europe/Berlin`. Whether that instance runs on each of 15 to 18 is open (DL4).
- **Step 43.** The quote of R-0670 matches step 12's exactly.
- **Step 50.**
  - **R-1210** says 1.20 and later.
  - **R-1236** adds the 64-value line, with a wording slip (DL6).
  - **R-1237** is a loud notice that agrees with R-0670 and R-0677.
  - **R-1239** is the "use this when" line.

What the fix added beyond the review is the list INDEX approves through PR #2:

- the 1.19 instance;
- R-1238 in step 12;
- 1.20 from a tarball;
- the error whatever `pooler` says;
- 0 to 64 flat tuples.

The README contributor section is the one addition INDEX doesn't list (DL2). Nothing is skipped or weakened.

## Summary

- **Blocks approval:** none.
- **Should fix:**
  - DL1: R-1238 contradicts step 01's `repo_policy.rs` tests and leaves the apt-to-image fallback standing. Its skip path needs the stubbed script test, not an integration claim in T-0300.
  - DL2: the contributor notice needs a `doc` ID in step 50 and a home that lasts. This needs a small owner choice: README section (recommended) or `CONTRIBUTING.md`.
- **Minor:**
  - DL3: `max_prepared_statements` per PgBouncer version;
  - DL4: exact versions 1.20.1 and 1.19.1, a version check, build inputs, and the second 1.26.1 on 15 to 18;
  - DL5: T-0300's entry points;
  - DL6: R-1236's tuple wording;
  - DL7: one stale PR body line.

DK1 to DK9 are fixed as the fourth review recommended or as the owner decided. DK1, DK2 and DK5 are written in consistently across `contracts.md`, `proofs.md` and steps 10, 12, 16, 22, 41, 43 and 50, with no live stale text. Every changed manifest ID has a Decisions entry.

When I ran them:

- the 1.20 and 1.19 claims held: the tarballs and checksums are real, 1.19.1 refuses `options` with `unsupported startup parameter: options`, and 1.20.1 passes `TimeZone` and `application_name` per client and keeps `UTC` with `TimeZone` in the ignore list;
- `Params: Send + Sync` over 64-tuples held on stable 1.99.1.

CI on `f4bdd82` is green, and the checker reports 1239 requirements, 300 test obligations and 0 problems.
