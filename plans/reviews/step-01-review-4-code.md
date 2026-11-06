# Step 01, fourth review: code findings

Reviewed `3d4cddc`, PR #1, everything outside `plans/`.

## 1. Verdict

Findings remain: three minor, no blockers. All nine third-review findings are really fixed, and CI and coverage are green. `rules.md` makes finding 1 a failed review, so the step can't pass until the tag is corrected. The fix is one line.

## 2. Findings

### Finding 1: `nothing_switches_a_job_or_step_off` carries a wrong R-0957 tag

- **Requirement:** R-0957. **Severity:** minor.
- **Where:** `tests/tests/repo_policy.rs:1211`.
- **Evidence:** I broke the workflows 14 ways against R-0957's rules: permissions, pins, `pull_request_target`, tag-only trigger, environment, fork guard, secrets and publish paths. This test passed on 12 of them.
  - The two it failed were `if: always()` replacing the fork guard, and `continue-on-error` on `verify`. Correctly tagged tests catch both.
  - Deleting the fork guard outright passes this test. Only `only_a_tag_runs_release_and_publishing_uses_the_protected_environment` fails.
  - The comment above the test gives reasons for matrix entries and steps switching off, and none for R-0957.
  - The tag was already there at `79d7c42`. The third review removed the same wrong tag from `a_failure_on_the_left_of_a_pipe_fails_the_step` and missed this one.
- **Change:** drop R-0957 from the tag. Seven other tests stay tagged with it and each catches its own rule, so coverage still passes.

### Finding 2: exact lists still pin more than the owner's scope

- **Requirement:** owner decision on `repo_policy.rs` lists, R-0795, R-0793. **Severity:** minor.
- **Where:** `tests/tests/repo_policy.rs:850-864` and `:473-490`.
- **Evidence:** the decision limits exact lists to `ci`, `verify`, `publish` and the suite commands. Setup and tool-install lines are correctly dropped: bumping `cargo-audit` or `cargo-fuzz`, or changing `apt-get install`, passes. Three edits outside that scope still fail:
  - `tar -czf` to `tar -cJf` in `binaries`.
  - Adding `--generate-notes` to `gh release create` in `github-release`.
  - Rewording the `echo "::error::..."` text in `fuzz`.

  The `binaries` and `github-release` lists are unchanged from `79d7c42`. The PR body says the file "pins only what matters".
- **Change:** see owner choice 1 in section 5.

### Finding 3: the PR body is stale or incomplete in three places

- **Requirement:** `rules.md`, PRs. **Severity:** minor. The owner approves from this text.
- **a. `step-01-fetch-check` closing line:** "These ran on the commit before the final rebase onto a plan-only commit, so the workflow and scripts were the same." That was true for `01be05e`: I diffed the `be79240` scripts against it, and the only difference is the push trigger. It isn't true for this PR's head, because `57e2c20` rewrote `fetch_last_bench.sh` and changed `bench_metrics.py`. Say it was true at `01be05e`, and that the review fixes came later.
- **b. "Not exercised yet":** the bullet names `fetch_last_bench.sh` and the `fuzz` step, but `57e2c20` also added the duplicate-name check to `bench_metrics.py`. That check has only run in tests, on synthetic output and the real Criterion fixture, not on Actions. Add it to the bullet.
- **c. Third review corrections:** `INDEX.md` on `main` says "The owner approves all of this through PR #1", but the body has no matching bullet. Of the 163 IDs changed or added since the merge base, 161 are named in the body. R-0433 and R-0741 aren't, and they're absent from the "edited in place" list too. Add a bullet covering that INDEX entry and naming both IDs.

## 3. Checked and good

- **Nine findings:** all 44 mutations of the changed scripts, workflows and Dependabot config were caught. That covers the `gh` flags, jq filter, `.expired`, `|| true`, missing `metrics.json`, duplicate names, all six units, `exclude:`, `shell:`, `working-directory`, Dependabot directories and the `cargo fuzz list` handling. Also checked: PR wording fixed, and the R-0957 tag gone from the pipe test.
- **Owner decisions:** build dependencies count, including `cfg` targets and default-features, and dev-dependencies stay out. Setup lines are dropped, and failure forms are refused in every file under `.github/`, scripts included.
- **`fetch_last_bench.sh`:** read closely. First-run, expired and no-artifact paths exit 0, and failed `gh` calls and missing metrics fail under `set -e`. An `if` with no branch taken returns 0, so the removed `exit 0` lines lost nothing. Nothing goes silent.
- **`fuzz` step:** the no-`fuzz/` path passes with its message. `bash -e` applies inside the `else` branch, so a failed list, an empty list or a failed run fails the job.
- **Test scan today:** `no_file_under_github_hides_a_failure` reads every `.github` file and no current line is wrongly refused.
- **Tag removal:** with the `R-0789` tag removed, `--step 01` prints the claimed error and exits 1.
- **CI and coverage:** both `ci` runs on `3d4cddc` passed (42190617204, 42190626474), all 36 checks pass, and the merge state is `CLEAN`. `check_coverage.py --step 01 --code crates tests benches fuzz .github` reports 0 problems, and so does the same command without `--step`. Local `repo_policy` and `workspace` tests pass, 36 of 36.
- **Repo state:** no throwaway branches, tags or releases exist, only `main` and `step-01-bootstrap`.
- **Settings match the body:** `ci` is required, admins are exempt, there are no required reviews, and force-push and deletion are blocked. `crates-io` has a required reviewer, a `v*` tag policy and the secret, and there are no repo-level secrets.
- **Throwaway runs:** every cited run ID has the claimed branch, purpose and result, confirmed from logs where the body quotes them. That includes the 8 `integration` failures from the three skipped tests. `step-01-workflow-check` matches its description.
- **Other body claims:** toolchains, `info` requirements, step numbers for T-0134 to T-0139 and new IDs, dependencies, no public calls, and `cargo publish --workspace --dry-run`, which I reran and which passed. `cargo-fuzz` 0.13.2 exists. No private paths are in the tree.

## Item 3 answer: can `no_file_under_github_hides_a_failure` over-refuse or be bypassed?

Yes to both, by probing the function directly. Neither affects any current file, so I don't count them as defects.

- **Over-refuses, and fails loudly:** Python `return 0`, `x.split(":")` (quote-stripping turns the string into a shell `:`), `set +x`, `cmd || { echo ...; exit 1; }`, `|| exit $?`, `|| return 1`, `: "${VAR:?}"` and `${{ a || b }}`.
- **Passes without complaint:** `cmd |true`, `cmd &`, `sys.exit( 0 )`, `raise SystemExit(0)`, `env true`, `bash +e script.sh`, `trap 'exit 0' ERR`, `if cmd; then echo ok; fi`, `subprocess.run(cmd)`.
- **Limit of the risk:** in pinned jobs the exact lists catch these, so a bypass matters only in `.github/scripts/` and setup lines. A line scan can't tell `if cmd; then` from a legitimate check.

## 4. Couldn't check

- **Rewritten `fetch_last_bench.sh` and `fuzz` on real Actions:** dispatch is off limits here and neither workflow is on `main`. I read them closely and mutated them against the stubs. The body discloses this.
- **"Dispatch currently returns a 404":** not tested, because that would be a dispatch.
- **`actionlint`:** not run, as told.
- **`smoke` and `pooler` locally:** there's no Postgres here, so I relied on the green CI jobs.
- **A truly expired artifact from the real API:** the real run used a deleted artifact as a stand-in, as the body says. The stub covers `expired: true`.

## 5. Owner choices

1. **Finding 2:**
   - **Option A, trim the lists (matches the decision as written):** `binaries` keeps only `cargo build`, `github-release` loses its exact list, and `fuzz` pins only the `cargo` commands, since `fuzz_fails_on_a_failed_or_empty_list_and_skips_without_a_fuzz_folder` already proves the control flow by running it.
   - **Option B, keep them:** the owner confirms these count as suite commands, and the PR body wording changes to say so.
2. **`hides_failure` limits (optional):**
   - **Option A, accept as is:** I recommend this.
   - **Option B, tighten it:** split on `|` and `&`, and stop stripping quotes around `:`. It's a small change, but it doesn't close the `if` or Python gaps.
