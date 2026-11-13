# Step 02 independent review, round 7

PR #2 (`step-02-proofs`), head `878b5f7`. This is the seventh review, after the fixes for the sixth review's DM1 to DM5. It's done as `rules.md` "Independent review" says, by an agent that didn't write the step.

I read `rules.md` and `proofs.md` on the PR branch. On `origin/main` I read:

- step 01's `tests/tests/repo_policy.rs`: the guard tests `no_file_under_github_hides_a_failure` and `nothing_switches_a_job_or_step_off`, their helper `hides_failure`, and the R-0792 pooler tests;
- INDEX's "Decisions since the split" entry "Step 02 sixth review, DM1 to DM5";
- `reviews/step-02-review-6.md`.

The scope is the round's diff, `git diff 05f47e2 origin/step-02-proofs`, one commit, `878b5f7`. It touches only `plans/`:

- `proofs.md`, one new DM line;
- step 12's R-1238 and T-0300;
- step 50's R-1240;
- the manifest.

To check the new text against what it changes, I also read `start_pgbouncer.sh`, `pgbouncer.ini` and CI's `pooler` job at the head.

## What I checked and ran

- **CI on `878b5f7`:** `ci` passed on its push run (43390690775) and its pull request run (43390698763), both on head SHA `878b5f75ef93…`. That covers every job: coverage, fmt, clippy, unit, trybuild, doc, integration on Postgres 15 to 18 over TCP and Unix sockets, pooler, msrv, features and the `ci` gate. The commit changes only plan text.
- **Checker:** `check_coverage.py --step 02 --code crates tests benches fuzz .github` and the plain form, run on a `git archive` of the head. Both report **1240 requirements, 300 test obligations, 0 problems**. The checker also checks that each manifest block appears whole in its step file.
- **Manifest:** I compared manifests by ID with a script.
  - **Against `05f47e2`:** 3 IDs changed, R-1238, R-1240 and T-0300. None was added or removed, and none changed its step or kind. `frame` and `master_sha256` are unchanged.
  - **INDEX's DM entry** says "Written in at `878b5f7`: R-1238, R-1240, T-0300". Those are the same 3 IDs.
  - **Against `main`:** the whole PR changes 27 IDs and adds 59, the same as the sixth review found.
- **The two notice copies:** R-1238's quote in step 12 and R-1240's in step 50 are byte for byte the same once step 12's two-space list indent is stripped. The manifest's copies match too.
- **Stale text:** I searched `plan-approved/` for "exit 0" and "exits 0". In step 12, the only hit is R-1238's "with no `exit 0` line". Elsewhere, the only hits are `proofs.md`'s DM line and step 49's T-0139, which is about the fuzz job. No step still says the script exits 0 for a skip.
- **The guard tests and the new lines.**
  - I compiled step 01's `hides_failure` on its own and ran it on candidate lines. These all pass it:
    - `if:` lines comparing an output with `== 'ran'` and with `!= 'skipped'`, bare, with `${{ }}` and with a leading `- `;
    - `echo "…=skipped" >> "$GITHUB_OUTPUT"`;
    - `if ! install; then`;
    - a `[ "$found" != "$expected" ]` test;
    - `docker run --rm --user 1001:1001 …`.
  - `exit 0` and `return 0` are refused, as before. So a script built as R-1238 says passes `no_file_under_github_hides_a_failure` unchanged.
  - `nothing_switches_a_job_or_step_off` refuses any `if:` that isn't in its `(file, line)` `ALLOWED` list. That list is the one that needs the new lines.
  - No other `repo_policy.rs` test refuses `$GITHUB_OUTPUT`, `curl`, a step `id:` or `docker run --user`. I checked the matrix, pipe, pinning and permission tests.
- **Spot checks (item 3).**
  - **A step skipped by `if:` shows as skipped.** In `actions/setup-node`'s run 43388998092, the "Upload artifact" step has `if: ${{failure() && steps.diff.conclusion == 'failure'}}` in `actions/reusable-workflows`' `check-dist.yml`. The jobs API reports that step as `completed | skipped`, and its job as `success`. That's the API record behind the grey skipped icon in the run's step list, so R-1238's "GitHub shows the step as skipped" holds.
  - **`docker run --user` and PgBouncer's root refusal.** I downloaded pgbouncer.org's tarballs for 1.19.1, 1.20.1, 1.22.0 and 1.26.1. Each matched its `.sha256` file, and the 1.19.1 and 1.20.1 hashes are the ones R-1238 gives.
    - In every version's `src/main.c`, the only check is `if (getuid() == 0) die("PgBouncer should not run as root")`. In 1.20.1 it's at line 998, and it comes after the optional `change_user`.
    - So a process started with a non-zero uid, which is what `docker run --user` gives, passes it.
    - `pgbouncer.ini` writes its log and pid file to `/tmp`, which a Debian image leaves world-writable, and it has no auth file. So a non-root uid has nothing else to trip on.
    - Building needs root for `apt-get`, so the builder will build as root and run under `--user` in a separate run. R-1238 allows that.
  - **What `SHOW VERSION` prints.** In all four versions, `admin_show_version` writes `PACKAGE_STRING`, and each `configure` sets `PACKAGE_STRING='PgBouncer <version>'`. I also checked the two binaries CI will really run:
    - Ubuntu noble's `pgbouncer_1.22.0-1build4_amd64.deb`, the package the sixth review saw apt install. `strings` on its `/usr/sbin/pgbouncer` finds `PgBouncer 1.22.0`, which starts with `PgBouncer 1.22.`.
    - The 1.26.1 image `start_pgbouncer.sh` pins (`ghcr.io/cloudnative-pg/pgbouncer@sha256:7e21afe7…dd7c`), fetched from the registry without Docker. Its amd64 config builds from PGDG's `1.26.1-1.pgdg13+1`, runs as `User: pgbouncer`, and `strings` on its `/usr/sbin/pgbouncer` finds `PgBouncer 1.26.1`.
  - **What I couldn't check.**
    - There's no Docker or Podman here. I didn't run `docker run --user`, build inside a Debian container, or run the image.
    - I didn't run `SHOW VERSION` this round. The sixth review ran it on 1.19.1 and 1.20.1 builds, and this round's evidence for 1.22.0 and 1.26.1 is the sources and the strings in the shipped binaries.
    - I didn't check what GitHub's UI shows, only the API's `skipped` conclusion.
- **The PR body (item 4)** matches the branch except for one stale line, and it repeats DN2's wording (DN4).
- **Cleanup.** The tarballs, the `.deb`, the image layers, the `hides_failure` build and the archive of the head are deleted.

## The sixth review's findings

| Finding | Fixed as recommended or decided? | Evidence |
|---|---|---|
| DM1 | Yes, the owner's option A, with one wording slip (DN2) and one gap the owner's choice left open (DN1) | R-1238 drops "the script exits 0". `start_pgbouncer.sh` writes `ran` or `skipped` per version to `$GITHUB_OUTPUT`, writes the `::warning::`, and "ends with status 0 by finishing normally, with no `exit 0` line". "Each version's test step carries one `if:` on that output." Both guard tests are named, and the stubbed test "asserts what each writes to `$GITHUB_OUTPUT`". |
| DM2 | Yes | The version strings are now `PgBouncer 1.20.1`, `PgBouncer 1.19.1`, `PgBouncer 1.26.1`, and a value starting `PgBouncer 1.22.`, which matches the binaries above. `psql` (and `curl`) join the stubs. The container runs PgBouncer through `docker run --user`. `make` joins the build inputs, "as in `build-essential`". |
| DM3 | Yes, as recommended | 1.19.1 "runs two instances", one with `ignore_startup_parameters` empty and one with `= options`, and "Each instance keeps one fixed config". T-0300 names each instance for its case. |
| DM4 | Yes, the owner's option B | "A downloaded tarball whose SHA-256 isn't the one given is treated like a failed install: that version's cases are skipped, with a `::warning::` naming the PgBouncer version, the file, the expected hash and the one found." The skip sentence, the stubbed test ("or a tarball with the wrong SHA-256, it's skipped"), both notice copies and T-0300's pointer say so too. The stubbed test doesn't yet assert the warning's file and hashes (DN3). |
| DM5 | Yes for the line it named; one new stale line (DN4) | The body says the DL entry names its 5 IDs, and it has a DM table of 3 IDs, which matches the manifest. |

## Findings

### DN1. A misspelled output skips a version's tests with no warning

- **Severity:** should fix. It needs a small choice about the `if:` form, which DM1 left open.
- **IDs:** R-1238
- **Files:** `steps/12-hold-pooler.md`, and the manifest's copy

**What's wrong.**

- R-1238 says each version's test step "carries one `if:` on that output". It doesn't say how the step compares the output. The natural form is `if: steps.<id>.outputs.<version> == 'ran'`.
- In GitHub Actions, an output that wasn't written reads as an empty string. This happens when the start step's `id`, or a version's output name, differs between `ci.yml` and `start_pgbouncer.sh`. Then `'' == 'ran'` is false, and the step is skipped.
- The script wrote `ran`, so it wrote no `::warning::`. The run is green and has no annotation, and the Contributing notice tells contributors that annotations are where a skip shows. That's a failure hidden without a trace, which is what DM1 set out to prevent.
- Nothing R-1238 names would catch it:
  - the stubbed test pins the names the script writes;
  - `nothing_switches_a_job_or_step_off`'s allow list pins the `if:` lines;
  - no test checks that the two agree.

**Evidence.**

- R-1238's seventh and eighth bullets.
- GitHub's documented behaviour: an output that wasn't set is an empty string, and an `if:` with no status function implies `success() &&`.
- The setup-node run above shows that a step whose `if:` is false is reported `skipped` while its job succeeds.

**Fix.** Add to R-1238:

- The `if:` compares with `!= 'skipped'`, as in `if: steps.pgbouncer.outputs.pgbouncer_1_22 != 'skipped'`.
  - When the script did skip the version, the result is the same as before.
  - A missing or misspelled output now runs the step, and its tests fail loudly, because there is no PgBouncer to reach.
  - The start step failing still stops the job, through the implied `success()`.
- The rewritten workflow test, tagged R-1238, asserts that each per-version `if:` reads an output name that the stubbed test sees the script write.

I recommend both. The first is enough on its own. Option B, keeping `== 'ran'` and adding only the cross-check, also closes the gap, but it leaves the silent direction as the default.

### DN2. `no_file_under_github_hides_a_failure` has no allow list to join

- **Severity:** minor
- **IDs:** R-1238
- **Files:** `steps/12-hold-pooler.md` and the manifest; the same words are in `proofs.md`'s DM line, INDEX's DM entry on `main` and the PR body

**What's wrong.**

- R-1238 says the two guard tests "stay as they are, except that exactly those per-version `if:` lines join their allow lists".
- Only `nothing_switches_a_job_or_step_off` has an allow list, its `ALLOWED` `(file, line)` pairs.
- `no_file_under_github_hides_a_failure` has none. It runs `hides_failure` on every line under `.github/`, skipping only comment lines. An `if:` line never trips that check (see the run above), and neither does a script with no `exit 0` line.
- The sixth review's option A, which the owner chose, said "`no_file_under_github_hides_a_failure` stays as it is".
- As written, step 12's session must either stop and ask, or add an exemption mechanism to the failure-hiding guard. That mechanism has no use, and it weakens the guard.

**Evidence.** `repo_policy.rs` on `main`: `nothing_switches_a_job_or_step_off`'s `ALLOWED`, and `no_file_under_github_hides_a_failure`, which has no list. `hides_failure` returned `false` for each `if:` form tried.

**Fix.** In R-1238 and its manifest copy, write: "`nothing_switches_a_job_or_step_off`'s `ALLOWED` list gains exactly those per-version `if:` lines, and nothing else. `no_file_under_github_hides_a_failure` doesn't change, and the script must pass it as it is." Correct `proofs.md`'s DM line and the PR body the same way. The coordinator should correct INDEX's DM entry on `main`.

### DN3. The stubbed test doesn't pin the checksum warning, and its stub list leaves out the 1.22 path's commands

- **Severity:** minor
- **IDs:** R-1238
- **Files:** `steps/12-hold-pooler.md`, and the manifest's copy

**What's wrong.**

1. **The checksum warning.** R-1238's third bullet says a checksum skip's `::warning::` names "the PgBouncer version, the file, the expected hash and the one found". The stubbed test's wrong-SHA-256 path asserts only "a `::warning::` naming the PgBouncer and Postgres versions". The PR approves the file and both hashes as one of the round's own choices, but no test proves them.
2. **The stub list.** The test has "stubs for `apt-get`, `curl`, `docker`, `pg_isready`, `psql` (which runs `SHOW VERSION`) and the build". The 1.22 path also runs `sudo`, `systemctl stop pgbouncer` and `pgbouncer -d`. Step 01's test stubs all three (its comment at line 375 lists `sudo`, `apt-get`, `systemctl`, `pgbouncer`, `docker` and `pg_isready`). Without them, the test either starts a real PgBouncer or asks for `sudo`. A builder would add them, but the list reads as the whole set.

**Evidence.** R-1238's third and last bullets. `repo_policy.rs` on `main`, lines 375 to 421.

**Fix.** In the last bullet:

- the wrong-SHA-256 path asserts a `::warning::` naming the PgBouncer and Postgres versions, the file, the expected hash and the one found;
- the stub list includes `sudo`, `systemctl` and `pgbouncer`, or says "among them".

### DN4. The PR body has one stale line again

- **Severity:** minor
- **IDs:** none
- **Files:** PR #2's description

**What's wrong.**

- The body says: "INDEX's Decisions name the IDs of every round through DL, … the DM entry names none yet, so the coordinator may want to add the DM IDs below to it."
- INDEX's DM entry on `main` (`86d6d14`) now says "Written in at `878b5f7`: R-1238, R-1240, T-0300". Those are the same 3 IDs as the body's DM table and the manifest diff.
- The body's DM1 section also repeats DN2's "join the allow lists of … `no_file_under_github_hides_a_failure` and `nothing_switches_a_job_or_step_off`".

**Fix.** Say the DM entry names its 3 IDs, and correct the DM1 section as in DN2. Add a DN row once DN1 to DN3 are fixed.

The rest of the body matches the branch:

- the CI results and the commit list through `878b5f7`, green on push and pull request;
- the checker totals, 1240 and 300 with 0 problems;
- the DM table, 3 IDs, matching the manifest;
- the DM2 to DM5 sections;
- the DM round's three choices, which INDEX approves through PR #2: the checksum warning's contents, `docker run --user`, and the `curl` stub.

## Requirement by requirement, for the IDs changed since `05f47e2`

- **R-1238 (step 12, `code`).**
  - It has DM1 to DM4 as decided or recommended.
  - Its new facts hold where I could check them: the `SHOW VERSION` strings on 1.22.0's and 1.26.1's real binaries, the root refusal in all four versions' sources, and a step skipped by `if:` showing as skipped.
  - It's buildable against step 01's guard tests once `ALLOWED` gains the `if:` lines. Nothing from earlier rounds is weakened: a wrong version and a PgBouncer that doesn't answer still fail the job.
  - The `if:` form can hide a skip without a warning (DN1). It names an allow list that doesn't exist (DN2). Its stubbed test misses the checksum warning's contents and three stubs (DN3).
- **T-0300 (step 12).** It names 1.19.1's two instances exactly as R-1238 sets them up: the refusal through the one with the list empty, and the workaround through `ignore_startup_parameters = options`. Its pointer to the stubbed test adds the wrong-SHA-256 path. It agrees with R-1238.
- **R-1240 (step 50, `doc`).** It adds "or its download doesn't match its pinned SHA-256" as a skip cause, which is DM4's decision. It's still loud, and it's the same text as R-1238's copy. The example warning, an install failure on 1.22, is still accurate, because 1.22 has no tarball.

Beyond the review, the fix added only the three choices INDEX approves through PR #2. Nothing was skipped.

## Summary

- **Blocks approval:** none.
- **Should fix:**
  - DN1: a per-version `if:` written as `== 'ran'` skips a version's tests with no warning when the output name is misspelled. Compare with `!= 'skipped'` (recommended), and check that the `if:` names match the script's output names.
- **Minor:**
  - DN2: `no_file_under_github_hides_a_failure` has no allow list. Only `nothing_switches_a_job_or_step_off`'s `ALLOWED` gains the `if:` lines. This also needs correcting in `proofs.md`, INDEX's DM entry and the PR body.
  - DN3: the stubbed test should assert the checksum warning's file and both hashes, and stub `sudo`, `systemctl` and `pgbouncer` too.
  - DN4: one stale PR body line about INDEX's DM entry.

DM1 to DM5 are fixed as the sixth review recommended or as the owner decided: DM1 as option A and DM4 as option B, a skip with a warning. The changed IDs match INDEX's DM entry exactly. CI on `878b5f7` is green, and the checker reports 1240 requirements, 300 test obligations and 0 problems.
