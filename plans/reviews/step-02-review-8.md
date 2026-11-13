# Step 02 independent review, round 8

PR #2 (`step-02-proofs`), head `5754360`. This is the eighth review, after the fixes for the seventh review's DN1 to DN4. It's done as `rules.md` "Independent review" says, by an agent that didn't write the step.

I read `rules.md` and `proofs.md` on the PR branch. On `origin/main` I read:

- step 01's `tests/tests/repo_policy.rs`: the guard tests `nothing_switches_a_job_or_step_off` and `no_file_under_github_hides_a_failure`, their helper `hides_failure`, and the R-0792 pooler tests (`ci_runs_every_check_the_plan_names`, `integration_targets_each_version_and_transport`, `pooler_starts_pgbouncer_from_another_source_when_apt_fails`);
- INDEX's "Decisions since the split" entries "Step 02 sixth review, DM1 to DM5" and "Step 02 seventh review, DN1 to DN4", at `d1ebd80`;
- `reviews/step-02-review-7.md`.

The scope is the round's diff, `git diff 878b5f7 origin/step-02-proofs`, one commit, `5754360`. It touches only `plans/`:

- `proofs.md`, the DM line;
- step 12's R-1238;
- the manifest's copy of R-1238.

## What I checked and ran

- **CI on `5754360`:** `ci` passed on its push run (43392647007) and its pull request run (43392656584), both on head SHA `5754360a5140…`. That covers every job: coverage, fmt, clippy, unit, trybuild, doc, integration on Postgres 15 to 18 over TCP and Unix sockets, pooler, msrv, features and the `ci` gate. The commit changes only plan text.
- **Checker:** `check_coverage.py --step 02 --code crates tests benches fuzz .github` and the plain form, run on a `git archive` of the head. Both report **1240 requirements, 300 test obligations, 0 problems**, and exit 0. The checker also checks that each manifest block appears whole in its step file.
- **Manifest:** I compared manifests by ID with a script.
  - **Against `878b5f7`:** 1 ID changed, R-1238, in its `text` and `block` only. None was added or removed, and none changed its step, kind or line. `frame` and `master_sha256` are unchanged.
  - **INDEX's DN entry** says "Written in at `5754360`: R-1238". That's the same ID.
  - **Against `main`:** the whole PR changes 27 IDs and adds 59, the same as the sixth and seventh reviews found.
- **Stale text:** I searched `plan-approved/` on the head for `== 'ran'`, "allow lists" and "join their". There are no hits. `no_file_under_github_hides_a_failure` now appears only in R-1238 (and its manifest copy), which says it doesn't change, and in `proofs.md`'s DM line, which says the same.
- **The guard tests and the new lines.** I compiled step 01's `hides_failure` on its own and ran it on lines a script and workflow built as R-1238 says would have. These all pass it:
  - `if: steps.pgbouncer.outputs.pgbouncer_1_22 != 'skipped'`, bare, with a leading `- `, and in `${{ }}`;
  - `echo "pgbouncer_1_22=skipped" >> "$GITHUB_OUTPUT"`;
  - a `::warning::` line naming the version, the file, the found hash and the expected one;
  - `found=$(sha256sum "$file" | cut -d' ' -f1)` and `if [ "$found" != "$expected" ]; then`;
  - `if ! curl -fsSL -o "$file" "$url"; then`;
  - `docker run --rm --user 1001:1001 --network host …`.

  `exit 0` and `return 0` are still refused. So the script and workflow can pass `no_file_under_github_hides_a_failure` unchanged, as R-1238 now requires. `nothing_switches_a_job_or_step_off` checks each `if:` line, trimmed and with `- ` stripped, against its `(file, line)` `ALLOWED` list, so adding the per-version lines to that list is the only change it needs. Several steps reading the same output can share one entry, because the check is `contains`.
- **The rewritten R-0792 tests.** `ci_runs_every_check_the_plan_names`, `integration_targets_each_version_and_transport` and `pooler_starts_pgbouncer_from_another_source_when_apt_fails` pin step 01's one-PgBouncer job, and R-1238 rewrites all three. No other `repo_policy.rs` test looks at the pooler job's steps or `if:` lines.
- **The `!= 'skipped'` semantics (item 1).** From GitHub's own docs (the `github/docs` repository, fetched through the API):
  - `contexts.md`: "If you attempt to dereference a nonexistent property, it will evaluate to an empty string."
  - `expressions.md`: comparisons are loose. A null is cast to `0` and a non-numeric string to `NaN`, so a null or an empty string is never equal to `'skipped'`.
  - `expressions.md`: "A default status check of `success()` is applied unless you include one of these functions."

  Together with the seventh review's API evidence that a step whose `if:` is false is reported `skipped` while its job succeeds, this gives the three cases:
  - **A skipped version.** The script wrote `skipped`, so `'skipped' != 'skipped'` is false, and the step is skipped. The start step's `::warning::` is the annotation that says why.
  - **A missing or misspelled output, or a misspelled step id.** It reads as an empty string, so the comparison is true and the step runs. If the version really was skipped, nothing listens on its port, and step 01's test helper (`tests/src/lib.rs`, "A server that can't be reached fails the test. Nothing is skipped.") panics with "can't connect to the test server". The step fails, and so does the job. If the version did run, the step runs normally, which hides nothing.
  - **A failed start step.** Every later step's `if:` has no status function, so the implied `success()` is false and they don't run. The start step's own failure fails the job.

  I couldn't run GitHub's expression evaluator here: running the downloaded `@actions/expressions` package was refused in this session. The evidence is the documented rules above.
- **DN3's stubs.** R-1238's stub list now reads "among them for `sudo`, `apt-get`, `systemctl`, `pgbouncer`, `curl`, `docker`, `pg_isready`, `psql` … and the build". That covers every command step 01's test stubs (its comment at line 375), plus `curl` and `psql`.
- **The PR body (item 3)** matches the branch except for one line that went stale half a minute after it was written (DO1).
- **Cleanup.** The archive of the head, the `hides_failure` build, the docs files and the downloaded npm package are deleted.

## The seventh review's findings

| Finding | Fixed as recommended? | Evidence |
|---|---|---|
| DN1 | Yes, both parts, as recommended | R-1238: "Each version's test step carries one `if:` on that output, comparing with `!= 'skipped'`, as in `if: steps.pgbouncer.outputs.pgbouncer_1_22 != 'skipped'`, so GitHub shows the step as skipped when the script skipped that version, and a missing or misspelled output, which reads as an empty string, runs the step, whose tests then fail loudly with no PgBouncer to reach. A failed start step still stops the job, through the `success()` an `if:` implies." And: "the rewritten workflow test, tagged with this requirement, asserts that each per-version `if:` names the start step's `id` and reads an output name that the stubbed test sees the script write." The cross-check covers a wrong step id as well as a wrong output name, a little more than the recommendation asked. The three cases hold, as above. |
| DN2 | Yes | R-1238: "Of its guard tests, `nothing_switches_a_job_or_step_off`'s `ALLOWED` list gains exactly those per-version `if:` lines, and nothing else; `no_file_under_github_hides_a_failure` doesn't change, and the script and workflow must pass it as it is." `proofs.md`'s DM line and the body's DM1 section say the same, and both mark it "corrected by DN2". INDEX's DM entry on `main` was corrected too (`18ea3d9`). |
| DN3 | Yes | The wrong-SHA-256 path: "a tarball with the wrong SHA-256 is skipped the same way, its `::warning::` also naming the file, the expected hash and the one found". The stub list says "among them" and adds `sudo`, `systemctl` and `pgbouncer`. |
| DN4 | Yes, but the replacement line is now stale (DO1) | The INDEX line says "the DL entry its 5 and the DM entry its 3", and the DM1 section is corrected. The body has a DN table, R-1238 (DN1 to DN3), which matches the manifest. |

## Findings

### DO1. The PR body's INDEX line is stale again

- **Severity:** minor
- **IDs:** none
- **Files:** PR #2's description

**What's wrong.**

- The body says: "INDEX's Decisions name the IDs of every round through DM … The DN entry names none yet, so the coordinator may want to add R-1238 to it."
- INDEX's DN entry on `main` now says "Written in at `5754360`: R-1238".
- The body was last edited at 08:24:12 UTC, and `d1ebd80`, which recorded the ID, was committed 30 seconds later. So the line was true when written. This is the sixth round running in which this one line has needed fixing (DJ10, DK9, DL7, DM5, DN4).

**Evidence.** `gh pr view 2 --json body,updatedAt`; `git show origin/main:plans/plan-approved/INDEX.md`, line 201; `git log -1 --format=%cI d1ebd80`.

**Fix.** Say the DN entry names its 1 ID, R-1238. To stop this recurring, word the line so it doesn't depend on what the coordinator has recorded yet. For example: "Each round's IDs are in the tables below; INDEX's Decisions entry for each round names the same IDs." The reviewer checks that claim anyway.

The rest of the body matches the branch:

- the CI results and the commit list through `5754360`, green on push and pull request;
- the checker totals, 1240 and 300 with 0 problems;
- the DN table, 1 ID, matching the manifest;
- the DN1 to DN4 sections;
- the DN round's one choice, that the `if:` example's step id and output names are illustrative, which INDEX's DN entry says the owner approves through PR #2.

## Requirement by requirement, for the IDs changed since `878b5f7`

- **R-1238 (step 12, `code`).**
  - It has DN1 to DN3 as recommended.
  - It's buildable. A script and workflow written as it says pass `no_file_under_github_hides_a_failure` unchanged, and `nothing_switches_a_job_or_step_off` needs only its `ALLOWED` list updated. No other step 01 guard test is affected, and the three R-0792 tests it rewrites are named.
  - It hides no failure. A skipped version shows a skipped step and a `::warning::`. A missing or misspelled output, or a wrong step id, runs the step, which fails if no PgBouncer is there. A failed start step stops the job. A PgBouncer that doesn't answer or reports another version still fails the job. The workflow test pins the `if:` names against what the script writes.
  - Nothing from earlier rounds is weakened.

Beyond the review, the fix added only the illustrative-names choice, which INDEX approves through PR #2. Nothing was skipped.

## Summary

- **Blocks approval:** none.
- **Should fix:** none.
- **Minor:**
  - DO1: the PR body still says INDEX's DN entry names no IDs. It names R-1238. Reword the line so it doesn't go stale again.

DN1 to DN4 are fixed as the seventh review recommended. With `!= 'skipped'` and the implied `success()`, a skipped version's step is skipped with a warning, a missing or misspelled output runs the step and fails it, and a failed start step stops the job. R-1238 is buildable against step 01's guard tests with only `nothing_switches_a_job_or_step_off`'s `ALLOWED` list updated. The one changed ID, R-1238, matches INDEX's DN entry. CI on `5754360` is green, and the checker reports 1240 requirements, 300 test obligations and 0 problems.
