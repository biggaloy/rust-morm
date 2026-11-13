# Step 02 independent review, round 6

PR #2 (`step-02-proofs`), head `05f47e2`. This is the sixth review, after the fixes for the fifth review's DL1 to DL7. It's done as `rules.md` "Independent review" says, by an agent that didn't write the step.

I read `rules.md`, `contracts.md` and `proofs.md` on the PR branch. On `origin/main` I read INDEX's "Decisions since the split" entry "Step 02 fifth review, DL1 to DL7", and `reviews/step-02-review-5.md`.

The scope is the round's diff, `git diff f4bdd82 origin/step-02-proofs`. It touches only `plans/`:

- `coverage.md`, `proofs.md` and the manifest;
- steps 12, 22 and 50.

To check the new text against what it changes, I also read, at the head:

- CI's `pooler` job, `start_pgbouncer.sh` and `pgbouncer.ini`;
- every test in `tests/tests/repo_policy.rs` that reads `.github/`, not only the three R-1238 names;
- `README.md`, its symlink `crates/morm/README.md` and the `include_str!` in `crates/morm/src/lib.rs`;
- step 12's Deliverables, R-0670 and T-0300, and step 50's R-0798 and the notices beside R-1240.

## What I checked and ran

- **CI on `05f47e2`:** `ci` passed on its push run (43381725146) and its pull request run (43381733982). That covers every job: coverage, fmt, clippy, unit, trybuild, doc, integration on Postgres 15 to 18 over TCP and Unix sockets, pooler, msrv and features. The commit changes only plan text. In the push run's `pooler` log, apt installed `pgbouncer (1.22.0-1build4)` from `azure.archive.ubuntu.com`, so "1.22 from the runner's apt sources" is Ubuntu's 1.22.0, as INDEX says.
- **Checker:** `check_coverage.py --step 02 --code crates tests benches fuzz .github` and the plain form, on a `git archive` of the head, both report **1240 requirements, 300 test obligations, 0 problems**. The checker also checks that each manifest block appears whole in its step file, so the step files and the manifest agree.
- **Manifest:** I compared the manifests by ID with a script, against `f4bdd82` and against `main`.
  - **This round changed 4 IDs:** R-1236, R-1238, T-0104 and T-0300.
  - **It added 1:** R-1240.
  - Nothing was removed, no ID changed its step or kind, and `frame` and `master_sha256` are unchanged.
  - **INDEX's DL entry** lists exactly these 5 IDs: "R-1236, R-1238, R-1240, T-0104, T-0300".
  - **Against `main`,** the whole PR changes 27 IDs and adds 59, R-1205 to R-1240 and T-0278 to T-0300: the fifth review's 27 and 58, plus R-1240.
  - **`coverage.md`** has R-1240's row, and its totals read 1240 and 300.
  - **The two copies of the notice,** R-1238's quote in step 12 and R-1240's in step 50, are byte for byte the same.
- **Stale text:** I searched `plan-approved/` for "1.19" or "1.20" without a patch number, "fallback", "pinned image", "never run without", and "contributor". The only hits are `proofs.md`'s DK2 line, which is the record of that round and is followed by the DL line, and INDEX's step 01 "pooler install" entry, which the DL entry says R-1238 replaces. Nothing is still live.
- **PgBouncer, downloaded and built (item 3).**
  - **Checksums.** I downloaded pgbouncer.org's tarballs and `.sha256` files for 1.19.1, 1.20.0, 1.20.1, 1.24.0 and 1.26.1, and the 1.22.0 tarball alone for its sources. Every `.sha256` file matched its tarball:
    - 1.20.1: `24992cf557d73426d7048698dffc7b019e6364d4d8757ae2cf5e2471286a2088`, as R-1238 says;
    - 1.19.1: `58c3eff9bb72c18133b28e1f034fd59356ea76281c65e127432ca101c208a394`, as R-1238 says.
  - **The 1.20.0 regression.** 1.26.1's `NEWS.md`, under "2023-08-09 - PgBouncer 1.20.1": "Fix regression where putting `options` inside `ignore_startup_parameters` would not ignore unknown parameters inside the `options` startup parameter anymore. ([#908]) (regression was introduced in 1.20.0)". R-1238's reason for 1.20.1 holds.
  - **`max_prepared_statements`.** `src/main.c` gives its default as `"0"` in 1.22.0 and `"200"` in 1.24.0 and 1.26.1. 1.26.1's `NEWS.md` has "Enable prepared statement support by default, `max_prepared_statements` is now set to 200 by default" in the 1.24.0 section. 1.19.1 and 1.20.1 don't know the key. R-1238 and T-0104 are right.
  - **Building.** 1.19.1 and 1.20.1 built with `./configure && make` on gcc 16.2.1, OpenSSL 3.6.4 and libevent 2.1.13. That's stricter than Debian trixie's gcc 14.2 and newer than its OpenSSL 3.5.7 and libevent 2.1.12, which sources.debian.org lists. deb.debian.org answered. So a Debian container build is feasible, with two details R-1238 doesn't give (DM2): `make` is needed too, and PgBouncer refuses to run as root.
  - **`SHOW VERSION`.** On both builds, `psql -d pgbouncer -c 'SHOW VERSION'` on the admin console printed `PgBouncer 1.19.1` and `PgBouncer 1.20.1`. Each `configure` sets `PACKAGE_STRING='PgBouncer <version>'`, 1.22.0's included, and `admin_show_version` returns that string (DM2).
  - **Changing the config at run time.** On 1.19.1, `SET ignore_startup_parameters = 'options'` on the admin console worked, and `SHOW CONFIG` listed it as changeable (DM3).
  - **What I couldn't check.**
    - There's no Docker here, so I didn't build inside a Debian container or run a container with `--network host`.
    - I didn't run 1.22 or 1.26.1, or anything in front of Postgres, this round. The fifth review ran 1.19.1 and 1.20.1 against Postgres 18.6, and the fourth ran 1.22.1 and 1.26.1. This round's claims don't depend on those runs.
- **Contributing notice (item 4).** R-1240 and R-1238's copy are in R-1008's loud form, a bold first line in a quote. They say:
  - what happens: a version that can't be installed has its cases skipped, with a `::warning::`, and the notice gives an example;
  - what it means for you: a green run can have skipped a version, so read the annotations;
  - how to go on: rerun, and if it's skipped again, say so in the PR and hold a pooler change.

  The round adds two failures, a wrong version and a PgBouncer that doesn't answer. Each fails the job with an `::error::`, which is loud enough. It adds no other skip, and no refusal a user meets.
- **The PR body (item 5)** matches the branch except for one stale line (DM5).
- **Cleanup.** The PgBouncers are stopped, and the tarballs, builds and scratch files are deleted.

## The fifth review's findings

| Finding | Fixed as recommended or decided? | Evidence |
|---|---|---|
| DL1 | Yes, as recommended, but two step 01 tests are still missed (DM1) | R-1238 names `tests/tests/repo_policy.rs`. It says the apt-to-image fallback ends: "a 1.22 that can't be installed is skipped, never run on another version", and the script exits 0. It also says `start_pgbouncer.sh`'s header changes. The three named tests are rewritten for R-1238. The stubbed script test, tagged R-1238, drives four paths: run on the second try; skip after two failures, with exit 0 and the warning; fail when PgBouncer doesn't answer; fail on a wrong version. T-0300's skip clause now points at that test. INDEX's DL entry records that this replaces step 01's decision. |
| DL2 | Yes, the owner's option A | New R-1240 (`doc`, step 50) has the drafted notice and puts it in a "Contributing" section at the end of the README. R-1238 has step 12 write that section with the same text, and says step 50 keeps it. Step 12's Deliverables name it, and `coverage.md` has the row. |
| DL3 | Yes | 1.22 and 1.26.1 set `max_prepared_statements = 200`, with 1.24.0's change of default as the reason, which I confirmed in the sources. 1.19.1 and 1.20.1 leave the key out, and their pools set both caches to 0. T-0104 says the caches are on behind 1.22 and 1.26.1. |
| DL4 | Yes, with two wording problems (DM2, DM4) | R-1238 makes these changes: <ul><li>1.20.1 and 1.19.1, each with its SHA-256, both of which I confirmed;</li><li>a `SHOW VERSION` check;</li><li>builds in a Debian container pinned by digest, with inputs from Debian's archive;</li><li>the second 1.26.1 in front of each of 15 to 18.</li></ul> |
| DL5 | Yes | T-0300 names `Db::connect`, `connect_as`, `connect_to` and a pool added by `db.pool(name)`, as R-0670 does. |
| DL6 | Yes | R-1236 now says "stable Rust can't implement a trait for tuples of every length, so morm implements it for up to 64 values". That's accurate, and it agrees with `contracts.md`'s 0 to 64 flat tuples. |
| DL7 | Yes | The body now says INDEX's DK entry names its 16 IDs, and the DL table lists this round's 5. A new stale line has appeared since (DM5). |

## Findings

### DM1. R-1238's skip breaks two more of step 01's guard tests, which it doesn't name

- **Severity:** should fix. It needs a small design choice about how a skip shows in the workflow.
- **IDs:** R-1238; R-0792 to R-0797, which tag the guard tests
- **Files:** `steps/12-hold-pooler.md`; step 01's `tests/tests/repo_policy.rs`

**What's wrong.**

- R-1238 says a version that can't be installed has "its cases ... skipped, not failed: the script exits 0 for it". It names three `repo_policy.rs` tests to rewrite. Two other step 01 tests guard `.github/` against exactly this, and R-1238 doesn't mention them:
  - **`no_file_under_github_hides_a_failure`** (line 1544, `// covers: R-0792, R-0793, R-0794, R-0795`). It refuses any line under `.github/` that is `exit 0`, `return 0`, `|| true`, `|| echo skipped`, `set +e` or a leading `!`. Its own table asserts that `"exit 0"` and `"cargo test || exit 0"` are refused. A skip branch written the way R-1238 words it, ending in `exit 0`, turns this test red.
  - **`nothing_switches_a_job_or_step_off`** (line 1510, `// covers: R-0792 ... R-0797`). It allows only two `if:` lines in any workflow: `if: always()` in `ci.yml`, and `release.yml`'s repository check. The natural way to skip a version's cases is a test step with `if:`, keyed on the script's result, and this test refuses it.
- So step 12's session meets the same problem DL1 fixed for the other three tests: red tests in a `done` step's file, which its own step file doesn't mention. It must then stop and ask, or work around the guards. One workaround is a skip with no `if:` and no `exit 0`, such as tests that read a marker file and return early. That leaves the guards blind to the one skip the owner approved, and it looks much like the ignored tests `rules.md` forbids.

**Evidence.** `repo_policy.rs` at `05f47e2`, lines 1508 to 1540 and 1542 to 1600. R-1238's sixth and seventh bullets.

**Options.**

- **A (recommended): a skip you can see.**
  - `start_pgbouncer.sh` writes each version's result (`ran` or `skipped`) to `$GITHUB_OUTPUT` and writes the `::warning::`. It ends with status 0 by finishing normally, with no `exit 0` line.
  - Each version's test step has one `if:` on that output, so GitHub shows the step as skipped.
  - R-1238 names both guard tests among those step 12 changes. `nothing_switches_a_job_or_step_off`'s `ALLOWED` list gains exactly those `if:` lines, and `no_file_under_github_hides_a_failure` stays as it is.
  - The stubbed script test asserts the output it writes.
- **B: the tests skip themselves.** The script writes a skip list, and the pooler tests return early for a version on it. The workflow keeps no `if:`, and the guards need no change. But the skip doesn't appear in the run's step list, and a test that returns early passes, which is close to an ignored test. Not recommended.

Either way, R-1238 should replace "the script exits 0 for it" with the mechanism chosen, and name the two tests.

### DM2. R-1238's version strings never match what `SHOW VERSION` prints, and the container build is missing two details

- **Severity:** minor
- **IDs:** R-1238
- **Files:** `steps/12-hold-pooler.md`

**What's wrong.**

1. **The version strings.** R-1238 says the job fails "unless it reports the version it stands for: `1.20.1`, `1.19.1` and `1.26.1` exactly, and a version starting `1.22.` for apt's". `SHOW VERSION` returns `PACKAGE_STRING`, which is `PgBouncer 1.20.1`. So an exact comparison, done as written, fails every run, and a value "starting `1.22.`" never matches.
2. **The stubs.** The stubbed script test is "with stubs for `apt-get`, `docker`, `pg_isready` and the build". Its "reports another version" path needs a stub for whatever reads `SHOW VERSION`, normally `psql`, and none is listed.
3. **Running as root.** PgBouncer refuses to run as root: 1.20.1's `src/main.c` line 998 has `if (getuid() == 0) die("PgBouncer should not run as root")`, and 1.19.1 has the same at line 993. A Debian container runs as root unless told otherwise. The build would then fail loudly, as "doesn't answer", but R-1238 says 1.19.1 and 1.20.1 are "run inside a container" without saying as whom.
4. **`make`.** The list of build inputs, "`libevent`, OpenSSL's headers, `pkg-config` and a C compiler", leaves out `make`, which the build needs and slim Debian images don't have.

**Evidence.**

- I ran `SHOW VERSION` on my 1.19.1 and 1.20.1 builds, and each `configure` sets `PACKAGE_STRING`, as described above. 1.22.0's `configure` has `PACKAGE_STRING='PgBouncer 1.22.0'`.
- The `main.c` lines quoted in point 3.

**Fix.** In R-1238:

- "reports `PgBouncer 1.20.1`, `PgBouncer 1.19.1` and `PgBouncer 1.26.1` exactly, and a value starting `PgBouncer 1.22.` for apt's";
- add `psql`, or whatever runs `SHOW VERSION`, to the stubbed test's stubs;
- say the container runs PgBouncer as a non-root user, for example with `docker run --user` or `user =` in its config;
- add `make` to the build inputs, or say `build-essential`.

### DM3. T-0300's second 1.19.1 case needs a config R-1238 doesn't give

- **Severity:** minor
- **IDs:** R-1238, T-0300
- **Files:** `steps/12-hold-pooler.md`

**What's wrong.**

- T-0300 has two 1.19.1 cases:
  - with `ignore_startup_parameters` empty, every entry point gets the refusal;
  - "through 1.19.1 with `options` in `ignore_startup_parameters`", the pool starts.
- R-1238 gives 1.19.1 one instance, which "runs for the refusal of `options` only". It also says "Each instance has its own config", and that the 1.19.1 config leaves out `max_prepared_statements`. It never says how the second case gets `options` into the list.
- The second case can be run two ways:
  - a second 1.19.1 instance;
  - a test that changes the one instance at run time. On my 1.19.1 build, `SET ignore_startup_parameters = 'options'` on the admin console worked, but the change then affects every other test using that instance until it's set back.
- Choosing between them is a decision about the job's layout that the step file leaves open. A later review could also read "the refusal of `options` only" as excluding the workaround case altogether.

**Fix.** In R-1238, say either:

- that a second 1.19.1 instance runs with `ignore_startup_parameters = options`, for T-0300's workaround (recommended, because each instance keeps one fixed config); or
- that T-0300 sets it on the one instance through the admin console, and puts it back, with no other test using that instance at the same time.

### DM4. R-1238 doesn't say whether a SHA-256 mismatch is a skip or a failure

- **Severity:** minor. It needs a small owner choice.
- **IDs:** R-1238
- **Files:** `steps/12-hold-pooler.md`

**What's wrong.**

- R-1238 says each tarball's SHA-256 "must be" the one given. It also says a version that "can't be installed" after one retry is skipped, with a warning, and that a wrong `SHOW VERSION` fails the job. It doesn't say which of these a checksum mismatch is.
- Read as "can't be installed", a mismatch is skipped. But a mismatch isn't a passing outage. It means a tarball replaced upstream, a tampered download or a wrong pin, and it repeats on every run. 1.20.1, the oldest version morm supports, would then go untested with nothing but a warning.
- The owner already approved, through PR #2, that "a wrong version fails the job rather than being skipped". A mismatch is the same kind of thing.
- The stubbed script test doesn't drive this path, whichever it is.

**Options.**

- **A (recommended).** A mismatch fails the job with an `::error::` naming the file, the expected hash and the one found. A download or apt failure is still a skip. The stubbed test drives the mismatch.
- **B.** A mismatch is a skip, like a failed download, and the stubbed test drives that.

### DM5. The PR body has one stale line again

- **Severity:** minor
- **IDs:** none
- **Files:** PR #2's description

**What's wrong.** The body says: "INDEX's Decisions name the IDs of every round through DK ...; the DL entry names none yet, so the coordinator may want to add the DL IDs below to it." But INDEX's DL entry on `main` (`c0c91d6`) now says "Written in at `05f47e2`: R-1236, R-1238, R-1240, T-0104, T-0300". Those are the same 5 IDs as the body's DL table and the manifest diff.

**Fix.** Say the DL entry names its 5 IDs. When DM1 to DM4 are fixed, add their IDs to a DM row.

The rest of the body matches the branch:

- the CI results and the commit list through `05f47e2`;
- the checker totals, 1240 and 300 with 0 problems;
- the DL table, 5 IDs, matching the manifest;
- the DL1 to DL7 section;
- the DL round's choices, which are the five INDEX approves.

## Requirement by requirement, for the IDs changed since `f4bdd82`

- **R-1238 (step 12, `code`).**
  - It's DK2 as decided, with DL1, DL3 and DL4 written in, and DL2's section.
  - It's tagged in the workflow and the stubbed script test, and the stubbed test proves the skip, retry and failure paths without a real outage.
  - Nothing from DK2 is weakened. "Not as a fallback" left the first bullet, but the sixth bullet now ends the fallback outright, which is stronger.
  - It misses two of step 01's guard tests (DM1), prints versions in a form `SHOW VERSION` doesn't use (DM2), doesn't give T-0300's second 1.19.1 case a config (DM3), and leaves a checksum mismatch undecided (DM4).
- **T-0300 (step 12).** It covers all four entry points (DL5) and both parameters on every pair, through 1.20.1, 1.22 and 1.26.1. The skip, no-answer and wrong-version paths moved to the stubbed test, which an integration test couldn't drive. Its workaround case depends on DM3.
- **T-0104 (step 22).**
  - It names 1.20.1.
  - On 1.22 and 1.26.1 at `max_prepared_statements = 200`, it adds the caches running through PgBouncer's prepared statement support. That's testable through `pg_prepared_statements` or `SHOW SERVERS`.
  - On 1.20.1, with both caches 0, everything goes through `query_typed`.
  - It agrees with R-0677 and R-1237, and nothing is weakened.
- **R-1236 (step 50, `doc`).** DL6's wording, accurate.
- **R-1240 (step 50, `doc`).** DL2's notice, loud, the same text as R-1238's quote. The section's place, the end of the README, which is also the crate docs through the symlink, is the owner's option A.

What the fix added beyond the review is the five choices INDEX approves through PR #2:

- the Debian container;
- `max_prepared_statements = 200`;
- `1.22.x` from apt;
- a wrong version failing the job;
- the notice's next steps.

Nothing else was added, and nothing was skipped.

## Summary

- **Blocks approval:** none.
- **Should fix:**
  - DM1: R-1238's skip breaks step 01's `no_file_under_github_hides_a_failure` (no `exit 0`) and `nothing_switches_a_job_or_step_off` (no `if:`), which it doesn't name. This needs a small design choice: a visible step-level skip with those `if:` lines added to the allow list (recommended), or tests that skip themselves.
- **Minor:**
  - DM2: the `SHOW VERSION` strings (`PgBouncer 1.20.1`, not `1.20.1`), a `psql` stub, a non-root user in the container, and `make`;
  - DM3: a config for T-0300's 1.19.1 case with `options` ignored;
  - DM4: whether a SHA-256 mismatch is a skip or a failure. This needs a small owner choice; failing is recommended;
  - DM5: one stale PR body line.

DL1 to DL7 are fixed as the fifth review recommended or as the owner decided. The changed IDs match INDEX's DL entry exactly.

The new facts held when I checked them:

- both SHA-256s against pgbouncer.org's `.sha256` files and the tarballs;
- the 1.20.0 regression in `NEWS.md`;
- `max_prepared_statements` at 0 in 1.22.0 and 200 from 1.24.0, in the sources;
- 1.19.1 and 1.20.1 built on a newer toolchain than Debian trixie's.

I couldn't run a Debian container here. CI on `05f47e2` is green, and the checker reports 1240 requirements, 300 test obligations and 0 problems.
