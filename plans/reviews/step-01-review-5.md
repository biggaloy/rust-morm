# Step 01, fifth review: changes since 3d4cddc (head 57b3253)

## 1. Verdict

**Findings remain: 6 minors, no blockers.** Every fourth-review finding is fixed, all three checker forms pass, and CI is green on `57b3253`. The owner's composite rule can be built and tested at steps 57 and 37. F2 is the one to fix first: it's a silent data loss path past the new guard. F1 is the order check the brief asked for: the new 56-on-28 edge left 15 "Can run beside" pairs false.

Any edit to an R or T row below must also go in `coverage.md`, `tools/manifest.json` and a Decisions entry.

## 2. Findings

### F1. Minor: the 56-on-28 edge made 15 "Can run beside" pairs false

- **Rule:** INDEX "Order": listed steps have "no dependency path between them". The fourth review confirmed this held, and `57b3253` broke it.
- **Evidence:** a transitive closure over all 59 step files. Step 14 now reaches 28 through 56, and 06 through 28. That gives 28 with each of 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 27 and 47, plus 06 with 14. All are listed both ways.
- **Failure:** step 28's file says it can run beside 14, but 14 can't start until 28 is done. No wrong run can happen, because the coordinator picks by dependency, so this is minor. The lines are still false, and a coordinator planning from them plans a parallel slot that can't exist.
- **Change:**
  - `28-manifest.md` line 5: `10, 11, 12`.
  - Step 06 line 5: `10, 11, 12`.
  - Step 14 line 5: delete `06` and `28`.
  - Line 5 of steps 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 27 and 47: delete `28`.

### F2. Minor: `rollback` drops an added attribute past the new guard, and data is lost silently

- **Requirement:** R-1059, T-0185.
- **Evidence:** R-1059 and T-0185 put the live check on `migrate` preflight only. R-0608 (step 33) doesn't say `rollback` runs op preflight checks, and neither does step 34. The fingerprint gate doesn't help: R-0600 covers only objects the chain manages. The inverse of "add at the end" is `DROP ATTRIBUTE`, which Postgres doesn't guard.
- **Reproduced on 18.6 and 17.11:** add attribute `b`, create an unmanaged table with a column of the type, insert `(1,'keep')`, run the inverse. It succeeds and the row reads `(1)`.
- **Failure:** the column is one no model declares, the case R-1059 exists for. `migrate` refuses that change, and `rollback` makes it. The other inverses are safe: Postgres refuses `DROP TYPE` and `ALTER ATTRIBUTE TYPE` here, and `ADD ATTRIBUTE` loses nothing.
- **Change:**
  - R-1059: replace "`migrate` preflight repeats these checks on the live database, because scratch can't see what no model declares." with "`migrate` preflight repeats these checks on the live database, because scratch can't see what no model declares, and `rollback` repeats them before an inverse that drops an attribute."
  - T-0185: after "for a column no model declares", add ", and `rollback` of an added attribute refuses the same way".
  - R-0597 says rollback works from the ledger when files are gone, so the check must read the live catalog and not rely on `meta.json`.

### F3. Minor: carried domains, composites and functions lose their owner and comment

- **Requirement:** R-1060, T-0186, against R-1059 and R-1014.
- **Evidence:** R-1059 keeps "owner, comment and grants" on the type it recreates, and R-1014 and R-1045 say why: they belong to the OID. R-1060 recreates carried objects "with their grants applied again", and T-0186 checks only "every function's grants". I ran the outside-in carry in SQL on 18.6: a domain, a composite and a function owned by `owner_a`, each with a comment, all came back owned by the migrating role with no comment, and no error.
- **Failure:** a step 37 session builds exactly this and T-0186 passes. A carried type then silently changes owner, where the same type recreated directly would not.
- **Change:**
  - R-1060: replace "with their grants applied again" with "keeping their owner, comment and grants".
  - T-0186: replace "every function's grants intact" with "every carried object's owner, comment and grants intact".

### F4. Minor: step 37 doesn't say where preflight gets the carried list

- **Requirement:** R-1060, T-0186 (a dependent no model declares exits `2`).
- **Evidence:** preflight must let carried dependents through and refuse undeclared ones, which needs the expected list at `migrate` time. `migrate` doesn't need the models (step 33 line 27), so it can't read the manifest. Steps 51, 53 and 58 quote the `op_data` and preflight hooks in Context, and step 37 doesn't. R-0989 is in step 32.
- **Failure:** "Everything needed for this step is in those files and this one", so the session has to stop and ask, or compare against the wrong source.
- **Change (Context only, no ID edit):** in `37-managed-deps.md`, add before "Update the earlier tests": "> `op_data` in `meta.json` and op preflight checks come from steps 32 and 33. `makemigrations` stores the carried list there, and `migrate` preflight compares the live walk with it, as step 53 does for enums."

### F5. Minor: the PR body misses three things the owner is asked to approve

- **Requirement:** `rules.md`, PRs. Fourth-review code findings 3a, 3b and 3c are fixed; these are other gaps.
- **a. Range:** the "Changing a domain or composite" bullet says "directly or through an array, a domain or another composite". R-1059 and INDEX say "an array, a range, a domain or another composite". The range guard is the point of `57b3253`.
  - Add "a range,", and append: "A range counts because Postgres loses that data with no error. A recreate keeps the type's owner, comment and grants."
- **b. New edge:** "Step 56 depends on 28" isn't in the body. The bullet listing the new edges has no entry for 56.
  - Add to that list: "56 on 28, which defines the manifest it writes labels into".
- **c. "Ops can extend the engine":** INDEX says "The owner approves this through PR #1", but no bullet describes it. R-0987 to R-0989 appear only inside the range "R-0975 to R-1008". This is the same defect as finding 3c.
  - Add: "**Ops can extend the engine:** an op can end its step and add its own decisions, `lint` findings, proof rows, `op_data` and preflight checks, so the enum steps change nothing in steps 30 to 33. R-0587, R-0608, R-0987, R-0988 and R-0989. The owner approves this through this PR."

### F6. Minor, needs owner choice O1: a composite recreate leaves compiled plpgsql stale

- **Requirement:** R-1059, R-1060, against R-0994 and R-1038 to R-1040.
- **Evidence:** a type used only in a plpgsql body has no `pg_depend` row, so "a recreate when anything depends on the type" isn't triggered and R-1060's walk doesn't see it. R-1038 to R-1040 cover only a swap, a rename or a removal.
- **Reproduced on 18.6:** a held session compiled `DECLARE v c[]`. After a recreate with 0 `pg_depend` rows, that session failed on every call with `cache lookup failed for type`. `DISCARD ALL` didn't clear it, and new sessions worked. Variables of plain `c` kept working in the bodies I tried, so only some bodies are hit.
- **Failure:** R-0981 recovers a morm app's own connection, but other services keep failing until they reconnect, and server sessions behind PgBouncer are the open question in proof 16. For enums this exact case is a refresh or a decision.
- **Change:** see O1.

## 3. Checked and good

- **Fourth review:** code 1, the R-0957 tag gone, 7 tags remain; code 2 kept, body wording changed; code 3a, 3b, 3c; plan B1, B2, M1, M2, M3, M4 and the M5 Decisions entry, each in the right place and text. No step still says an add's rollback is refused.
- **Composite rule:** the R-0993 pattern holds: 57 refuses, 37 replaces, T-0185 names the rewrite, and step 37 has "Update the earlier tests". No contradiction with step 36 or step 53's walk (R-1036 and R-1060 agree). Step 33's hook and every kind T-0185 and T-0186 need exist by those steps.
- **Additions justified, run on Postgres:**
  - Range: `DROP ATTRIBUTE` lost data through a range and a multirange column on 18.6 and 17.11. `pg_depend` links the multirange to the range, so a walk reaches its column.
  - Materialized view: lost data the same way on 18.6.
  - Attribute dependents: Postgres refuses without `CASCADE`, for a `BEGIN ATOMIC` function and a view, and `pg_depend` has the attribute row. A function with the type only in its signature doesn't block `ALTER TYPE`.
  - Carry: the full T-0186 kind list dropped and recreated in one transaction on 18.6, in plain SQL.
- **Graph:** depends-on lines match the table, no cycle, waves match levels exactly. Step 56's line is right: of the steps with no path to it, 10, 11, 12 and 29 are listed, and 07, 08 and 09 share `morm-macros`.
- **Checker:** all three forms exit 0 at 1060 requirements, 186 test obligations, 0 problems. The plain and `--step 01` forms also pass on a trial merge with `main`, which is clean and keeps 59 rows.
- **Traceability:** 7 IDs changed since `3d4cddc`: R-0991, R-1027, R-1055, R-1059, R-1060, T-0185, T-0186. All are named in "Fourth review corrections" or "Composite dependents split".
- **Frozen plan:** unchanged.
- **CI on `57b3253`:** runs 42195866414 and 42195876045 passed, 18 jobs each, 36 checks, merge state `CLEAN`. In the two jobs I opened, no step was skipped. The `integration (pg 18, tcp)` log shows 27, 9 and 1 tests passing, and the `coverage` log shows the checker's result.
- **PR body:** apart from F5, everything I checked is current, including the exact-lists wording.

## 4. Couldn't check

- Postgres 15 and 16: probes ran on 18.6 and, for the data loss cases, 17.11. The stale plpgsql and carry probes ran on 18.6 only.
- PgBouncer behaviour in F6: not run.
- `actionlint`, and `spikes.yml` on GitHub.
- Several commands were refused as too complex, and I reran them as plain separate commands, so nothing was skipped. `git merge-tree` left an unreferenced tree object in the shared object store; no ref changed.

## 5. Owner choices

**O1. Composite recreates and code Postgres doesn't track (F6).**
- **A, extend step 59:** in R-1038, add "a composite type recreate" to the list, in R-1039 change "the swap's transaction" to "that transaction", and add a composite case to T-0168. Managed functions are refreshed, and unmanaged ones become R-1040's decision. It reuses what 59 already builds. **I recommend this.**
- **B, accept it:** record that a composite recreate can leave other services needing to reconnect. No plan work, but it differs from the enum rule, and a non-morm client gets no warning.
