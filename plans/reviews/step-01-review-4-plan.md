# Fourth plan review: PR #1 at 3d4cddc

## 1. Verdict

**Findings remain: 2 blockers and 5 minors.** All 13 blockers and 11 minors from the last review are fixed in the right place, and the order graph is sound. Both blockers are in new or moved text: one comes from the 51/58 split, and the other is in R-1059.

Nothing was committed, pushed or posted. Any edit to an R or T row below must also go in `coverage.md` and `tools/manifest.json`.

## 2. Findings

### Blockers

**B1. Step 51 can't write a migration that adds a value.**

- **Evidence:**
  - `51-enum-swap.md:41` moved the inverse of an add to step 58 and says rolling back an add "is refused with exit code `2`".
  - `30-schema-diff.md:60` (R-0553) says down exists by construction.
  - `32-proof-files.md:64,67` (R-0589, R-0591) write files only after `down` returns to the previous schema exactly.
  - So an add with no inverse fails the proof, no files are written, and step 51's end-to-end check and T-0163 and T-0164 can't run. Nothing in step 32 skips `down`.
- **The fix is cheap.** Step 51 already builds what's needed: a plain swap back to the old labels is the add's inverse. R-1012 already says a row on a removed label fails the cast and rolls back. I ran this on Postgres 18: with no row on the label, the swap restores `{a,b}`; with a row, the cast fails and the rollback keeps `{a,b,c}`.
- **Exact changes** (none touch a requirement row):
  - `51-enum-swap.md:17`: change "the reorder decision and its inverse" to "the reorder decision and its inverse, a plain swap as an add's inverse".
  - `51-enum-swap.md:40`: change "so this step's swap only reorders" to "so this step's swap only reorders or undoes an add".
  - `51-enum-swap.md:41`, replace the line with: "`down_map` and the row-count refusal on an add's inverse (step 58). Until it exists, an add's inverse is a swap with no map back to the old labels, the op keeps the proof's rows off the added label, and a row holding it fails the cast and rolls back".
  - `58-enum-remove.md:29`, replace the line with: "> Step 51's inverse of an add is a swap with no map, where a row holding the value fails the cast, and step 57 refuses to remove a value with exit code `2`. This step adds the row-count refusal and `down_map` to the first and replaces the second."

**B2. R-1059's composite rule can't be exact, and its guard can miss a column.** This needs owner choice O1.

- **Evidence:** `57-managed-types.md:53` and `:61`.
- **`ALTER TYPE` only appends.** On Postgres 18, `(a,b,c)` with `b` dropped and added back became `a,c,b`, and adding `b` to `(a,c)` gave `a,c,b`. R-0961 and R-0964 make attribute order part of the type, so dropping a middle attribute has no exact inverse and adding a middle field can't reach the model. R-0590 or R-0591 then fails with exit `1`, not the refusal that T-0185 and the rule's "each with its inverse" promise.
- **Other attribute changes have no rule:** a changed attribute type, a new order and a rename.
- **Postgres doesn't enforce the guard.** On Postgres 18, `ADD ATTRIBUTE` and `DROP ATTRIBUTE` both ran on a composite a column uses, directly and through another composite, and the dropped data was lost with no error. Only `ALTER ATTRIBUTE TYPE` refused, and that was the direct-use case. morm's check is therefore the only protection, and "a column uses" read as direct use loses data silently.
- **Also unsettled:** R-1059 doesn't say `VALIDATE` is a separate step, though R-0559 does for every other `NOT VALID` form.
- **Exact change, with option A from O1:**
  - R-1059: "**Changing a domain or composite type.** A changed domain `check` drops and adds the constraint `NOT VALID`, then runs `VALIDATE` in a separate step. When no column uses a composite type, directly or through an array, a domain or another composite, a new attribute can be added at the end, the last attribute can be dropped, and an attribute's type can change, each with `ALTER TYPE`. Any other attribute change, a changed base type, or a change to a composite a column uses, is refused with exit code `2`. The with-data proof leaves out domain columns whose `check` its rows fail."
  - T-0185, change the sentence on refusals to: "A changed base type, a new attribute that isn't last, a dropped attribute that isn't last, and a change to a composite a column uses, directly or through another composite, each exit with code `2` and change nothing, and the with-data proof leaves out a domain column whose `check` its rows fail."

### Minors

**M1. Step 55 doesn't say the workflow must pass `repo_policy.rs`.**
- **Evidence:** `55-enum-proofs.md:37`. The policy tests read every file in `.github/workflows`, so `spikes.yml` is checked on each PR run, and a failure turns `ci` red.
- **Result of running them on Rust 1.99.1 in a scratch copy:**
  - A temporary workflow does fit. A sample with Postgres 15–18, a standby and PgBouncer passed all 27 tests, with no change to `repo_policy.rs`.
  - A sample using the common forms failed 5: `a_failure_on_the_left_of_a_pipe_fails_the_step`, `every_workflow_starts_with_read_only_permissions`, `every_action_is_pinned_to_a_full_commit`, `nothing_switches_a_job_or_step_off` and `no_file_under_github_hides_a_failure`.
- **Exact change, append to line 37:** "`tests/tests/repo_policy.rs` reads every workflow, so this one follows the same rules and that file doesn't change: top-level `permissions` of `contents: read`, actions pinned to a full commit, `shell: bash` on any step with a pipe, `--manifest-path spikes/Cargo.toml` and no `working-directory`, and no `if:`, `continue-on-error`, `exclude:`, `secrets.` or `|| true`. Give `spikes/Cargo.toml` its own `[workspace]` table, because it sits under the root workspace."

**M2. Three later steps change earlier tests and don't say who updates them.**
- **Step 40:** R-0580 and R-0582 (`40-live-modes.md:51,53`) make `rollback` exit `2` on any unmarked database. That breaks every `rollback` test since step 34, including all the enum steps. It also breaks the `full-chain` test, which `ci.yml` doesn't run, so step 40's own PR wouldn't show it. Step 40 used to come before these enum steps' tests existed, so it has no line for them.
- **Step 59:** R-1038 opens a decision on a source hit, so T-0166's function cases can stop with exit `3`.
- **Step 53:** the preflight walk can refuse dependents that earlier enum tests create by hand.
- **Exact change:**
  - `40-live-modes.md`, add to Context: "> Update the earlier tests this changes: a test that runs `rollback` or `rehearse` marks its database with `mode development --confirm NAME` first, including the `full-chain` test `nightly.yml` runs, which CI doesn't."
  - `53-enum-deps.md` and `59-enum-stale.md`, add to Context: "> Update the earlier tests this changes."

**M3. `TextEnum` labels don't reach step 32.**
- **Evidence:** R-0593 (`32-proof-files.md:69`) says the rows use the labels, but R-1055 (`56-enum-types.md:56`) only puts them in the CHECK, and the manifest list at `contracts.md:128` has no labels. Step 32 would have to parse SQL or stop. It doesn't list `morm-core`, where the manifest types live.
- **Exact change:**
  - R-1055 append: "`ModelMeta` and the manifest also carry these labels on the column, so step 32's with-data proof can read them."
  - Add `morm-core` to step 56's crates.
  - `contracts.md:128`: add "enum labels" after "constraints".

**M4. Steps 57 and 51 have no synthetic rows for enum or composite columns.**
- **Evidence:** R-1027 (`58-enum-remove.md:67`) arrives in step 58, but managed enum columns reach the with-data proof from step 57. No step defines rows for a composite column. A `NOT NULL` column of either kind can't get a row until then.
- **Exact change:**
  - R-0991 append: "The with-data proof gives an enum column a row per label, and a composite column one row built from its attributes' rows."
  - R-1027: delete "Enum columns get a row per label."

**M5. Five changed IDs have no Decisions entry.** See section 2a. Proposed entry, for the coordinator to add: "**Ops can extend the engine:** an op can end its step and add its own decisions, `lint` findings, proof rows, `op_data` and preflight checks, so the enum steps change nothing in steps 30 to 33. R-0587, R-0608, R-0987, R-0988, R-0989. The owner approves this through PR #1."

### 2a. Traceability

`git diff origin/main -- plans/plan-approved/tools/manifest.json` changes 163 IDs: 152 new, 9 with new text and 2 moved with no text change (R-0187 and R-0189). The rule names no form, so I mapped each by topic.

| Decisions entry | IDs |
|---|---|
| Empty CI suites | T-0134 to T-0139 |
| Full chain | R-0960, T-0138, T-0140 |
| Managed type declarations | R-0188, R-0860, R-0961 to R-0974, R-0976 to R-0978, R-1002 to R-1004, T-0141 to T-0146, T-0154 |
| Managed enums get full management | R-0986, R-0990 to R-0996, R-1008 to R-1054, T-0155 to T-0157, T-0159 to T-0179 |
| Managed enums take new steps | R-0718, R-0975, R-0979 to R-0985, R-0997 to R-1000, R-1005 to R-1007, T-0147 to T-0153, T-0158 |
| Five large steps split | R-0187, R-0189, T-0184, and the wording changes in R-0993, R-0994, T-0155, T-0159, T-0163, T-0169 |
| TextEnum labels, and fallback variants | R-1055 to R-1057, T-0180, T-0181 |
| `import` never writes `TextEnum` | R-1001 |
| Changing a domain or composite | R-1059, T-0185 |
| Third review corrections | R-0433, R-0593, R-0741, R-0764, R-1058, T-0182, T-0183, and the IDs it lists |

**No entry covers:** R-0587, R-0608, R-0987, R-0988, R-0989. These add the hooks that let an op end its step and add its own decisions, findings, data and preflight checks. The closest entry only says the design "goes in new steps". The third review called R-0587 and R-0608 "approved or necessary", but no Decisions text records that.

## 3. Step sizing

Every step fits one fresh session, including 55 to 59, so none needs a split. The tightest, none of which needs a change:
- **36:** seven object kinds plus runtime calls.
- **51:** 12 requirements and 5 tests, including fault injection and a new CI job.
- **52:** 9 requirements and 5 proof-heavy tests.

If one runs out of room, the rule to stop at a clean commit covers it. Natural cut points are `db.call` and `db.refresh` in 36, and T-0164 in 51.

## 4. Checked and good

All three checker forms pass with exit 0 (1059 requirements, 185 test obligations, 0 problems), and so does the plain form on a trial merge with main. The merge is clean and keeps 59 rows. The frozen plan is unchanged. Each last-review blocker and minor (B1 to B13, M1 to M11) is in the right place, with `coverage.md` and the manifest agreeing. Every ID is in exactly one step. The splits in 02, 13, 36, 51 and 53 stand alone, apart from B1. The 51/58 split is otherwise clean for R-1012, R-1013, R-0993, T-0159, T-0160 and T-0163. Depends-on lines match the table, there's no cycle, and waves match dependency levels exactly. All the new edges are right; 59 on 12 and 31 on 56 are redundant but harmless. "Can run beside" lines have no dependency path and are all symmetric. Approval lines match `rules.md`. Steps 51 and 59 own their `.github` work. The `rules.md` additions and R-1001 are sound, and `checks(...)` exists (R-0158).

## 5. Couldn't check

- Postgres probes ran on 18.6 only. Postgres 15 to 17, PgBouncer and a standby weren't run.
- `spikes.yml` was tested only against `repo_policy.rs`, using my two samples. It never ran on GitHub. The `[workspace]` advice in M1 comes from my knowledge of Cargo, not a run.
- The design tests behind the enum steps aren't in the repo.
- Two commands were refused as too complex, and I reran them as plain separate commands. Nothing was skipped. One refused form set `TMPDIR`, so I passed it as a literal path on the `cargo` runs instead.

## 6. Owner choices

**O1. Composite attribute changes on a type no column uses (B2).**
- **A:** add at the end, drop the last attribute, change an attribute's type; refuse the rest with exit `2`. It's simple and every inverse is exact, but a middle field needs a hand-written `--empty` migration.
- **B:** drop and recreate the type in one transaction for any other shape. It handles every shape, but it must also carry functions and other types that use it, which adds work to step 57.
- **Recommend A.** A composite no column uses is rare, and A keeps the proof exact with the least new engine work.
