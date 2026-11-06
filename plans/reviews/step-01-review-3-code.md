# Step 01, third review: code findings

Reviewed at `79d7c42`. All minor, no blockers. The owner's decisions are at the end.

1. **R-0794:** the `gh` stub ignores the flags in `fetch_last_bench.sh`. Dropping `--status success` or `--branch`, the wrong workflow, removing or inverting the `.expired` check, the wrong artifact name, and `|| true` on `gh api` all pass. Assert the full `gh run list` arguments, make the stub evaluate the jq filter on JSON with `expired: true`, and add an `api-fails` mode.
2. **R-0794:** if a download succeeds but `previous/metrics.json` is missing, `bench_compare.py` exits 0 with an all-`n/a` table. `fetch_last_bench.sh` must fail in that case.
3. **R-0794:** two results with the same name overwrite each other and `bench_metrics.py` exits 0. Fail on a duplicate name.
4. **R-0794:** only `ns` and `µs` are tested. Add `ms`, `s` and `ps` to the synthetic test.
5. **R-0792, R-0793, R-0795, R-0797:** a matrix `exclude:`, `shell: "true {0}"` and a changed `working-directory` all pass. Refuse `exclude:`, allow-list `shell:`, and cover `working-directory`.
6. **R-0958:** removing `- /` from Dependabot, or pointing cargo at the wrong directory, passes. Assert `/` for both ecosystems.
7. **R-0793:** `for target in $(cargo fuzz list)` hides a failed list. Capture it first and fail if the command fails. The no-`fuzz/` path stays as it is until step 49.
8. **PR body:** the `step-01-bench-check` wording is stale, "Other jobs use stable" is wrong for `fuzz`, and "Any other failure fails the step" overclaims until finding 2 is fixed.
9. **R-0957, R-0727:** `a_failure_on_the_left_of_a_pipe_fails_the_step` shouldn't carry the R-0957 tag, and `[build-dependencies]` pass `ALLOWED_DEPS`.

## Owner decisions

- Build dependencies count under `ALLOWED_DEPS`.
- Command lists in `repo_policy.rs` pin only what matters: exact lists for `ci`, `verify`, `publish` and the suite commands. Setup and tool-install lines are dropped. `|| true` and similar forms are refused everywhere, including inside `.github/scripts/`.
