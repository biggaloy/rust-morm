## 1. Verdict
**Findings remain: 13 blockers and 11 minors.** Head is still `79d7c42`, confirmed before and after a fresh fetch. Nothing was committed or pushed. I only detached this worktree at the head and wrote scratch files.

The owner decisions are carried faithfully. The problems are in build order, contradictions and undefined behaviour. Any edit to an R or T row below must also go in `coverage.md` and `tools/manifest.json`. The next free IDs are R-1058 and T-0182.

## 2. Findings

### Blockers

**B1. Step 52 needs step 54, but doesn't depend on it.**
- Evidence: T-0177 (`52-enum-online.md:77`) requires `--fast` to handle each refused layout, and R-1052 (line 65) lists domain and composite columns. Step 54 builds that. 52 and 54 share wave 19 with no ordering between them, so 52 could run first and fail.
- INDEX's owner decision and your walk order both put 54 first.
- Fix:
  - Step 52 and the INDEX row: `Depends on: 39, 54`.
  - Step 40 and the INDEX row: `Depends on: 52`.
  - Waves become: `19. 46, 48, 54` / `20. 52` / `21. 40` / `22. 41` / `23. 49` / `24. 50`.

**B2. R-0981 and R-0983 contradict each other on retries.**
- Evidence: `14-statements.md:82` retries `XX000` from "any statement". Line 84 says a statement not in a cache is "never retried". T-0152 expects the write to succeed. A first use is never cached (R-0707), so the step can't satisfy both.
- Fix, R-0983 last sentence: "A `42883` or `42804` from a statement that isn't in a cache is never retried, `XX000` with that message is retried from any statement, nothing is retried once a row has reached the caller, and a retry that fails again returns its error."
- The clause about rows is my own reasoning. I didn't run it.

**B3. R-0981 silently moves a held connection.**
- Evidence: `14-statements.md:82` says "retries once on another". R-0690 says a hold keeps one connection, because session state lives there. No test would catch this.
- Fix, add to R-0981: "On a `db.hold()` connection nothing moves: morm returns `Error::SchemaChanged`, and the connection is replaced on release."
- Also change the contracts comment on `SchemaChanged` to: "inside a transaction or on a held connection".

**B4. Half of R-0764 can't be built at step 14.**
- Evidence: `14-statements.md:78` promises `Error::SchemaChanged` inside a transaction. Transactions arrive in step 20, where R-0984 owns that behaviour. `rules.md` wants a test for each rule, so step 14 stops.
- Fix, R-0764: "Re-prepared or replaced automatically outside a transaction. Inside one, step 20 returns `Error::SchemaChanged`."

**B5. The fallback variant's label is undefined (your open point 1, confirmed).**
- Evidence: these all count every variant as a value, so the fallback would become a real label:
  - R-1055 (`13-types.md:81`)
  - R-0964 (`35-managed-decl.md:89`)
  - R-0990 (`:106`)
  - R-0997 (`41-rolling.md:45`), which makes `check_schema` fail on any enum that has a fallback
- Fix:
  - Add to R-0979 and R-1056: "This variant has no label: it isn't in the CHECK list, the managed type or `check_schema`, and it can't take `value`, `was` or `deprecated`."
  - Add to R-0964: "except an `unknown` variant".
  - Change R-0997 to "Every labelled variant's label".
  - Add the conflicting attribute cases to T-0150.

**B6. Nothing creates a `TextEnum` CHECK (your open point 2, confirmed).**
- Evidence: later changes are covered by generic constraint handling (R-0158, R-0326, R-0559), but only if the CHECK reaches the metadata, and no step puts it there.
  - The range CHECK has that path: step 05 metadata, proved on a database by T-0074 at step 30. `TextEnum` has neither.
  - It has no constraint name.
  - T-0180 needs a real constraint at step 13, which has no migrations.
  - With-data rows (R-0593) would break the CHECK, because they're keyed by `text`.
- Fix:
  - Add to R-1055: "`#[model]` emits the CHECK in metadata like a range CHECK, named `__CHK_{table}_{column}_enum__`, reading labels through a trait constant on the type."
  - T-0180 ends: "and the CHECK in `ModelMeta` lists the same spellings."
  - New step 30 row: `| T-0183 | integration | **TextEnum CHECK:** the migration creates the named CHECK, adding or removing a variant replaces it through the constraint diff, and a direct write of another value is rejected. |`
  - Add to R-0593: "A `TextEnum` column's rows use its labels."

**B7. Step 52 uses a scanner its crate can't reach.**
- Evidence: R-1051 (`52-enum-online.md:63`) uses step 14's scanner, which sits in `morm`. Step 52 is in `morm-migrate`, and `contracts.md` says that crate doesn't depend on `morm`.
- Fix:
  - Add to R-0433: "The scanner lives in `morm-core`."
  - Add `morm-core` to step 14's crates.
  - Add a contracts row: `| The SQL text scanner | morm-core | Statements, online swap |`.
  - T-0178 needs double-quoted identifiers replaced, so add to R-1051: "Identifiers in double quotes are compared, not skipped."

**B8. The online form breaks on nullable columns.**
- Evidence: `52-enum-online.md:53` adds `CHECK (shadow IS NOT NULL)` unconditionally, so any NULL row fails `VALIDATE`. `Option<E>` columns are common, and R-1048 relies on this check to catch missed rows.
- Fix, step 5: "For a `NOT NULL` column add `CHECK (shadow IS NOT NULL) NOT VALID`. For a nullable one add `CHECK (<col> IS NULL OR shadow IS NOT NULL) NOT VALID`, dropped at cutover. Then `VALIDATE`."
- R-1049: "sets `NOT NULL` from the validated check, for a column that had it".

**B9. The online form drops what R-1031 says Postgres rebuilds.**
- Evidence: R-1031's list is only true with `ALTER COLUMN TYPE`, which the online form doesn't use. R-1047 and R-1049 don't cover:
  - unique constraints
  - exclusion constraints, which can't be built concurrently
  - extended statistics
  - CHECKs with no literal
  - column comments and column grants
- A literal cast also keeps the old type name while the new type has its temporary name (R-1051).
- Fix:
  - R-1047 step 5: "Any CHECK naming the column goes the same way."
  - R-1049: add "re-attaches unique constraints with `ADD CONSTRAINT ... USING INDEX`, and copies the column's comment and grants".
  - R-1051: add "and replaces a cast to the old type's name with the temporary name".
  - R-1052: add "exclusion constraints and extended statistics on the column".
  - T-0177 covers each of these.

**B10. `down_map` is undefined.**
- Evidence: it appears only at `51-enum-swap.md:17` and `:80`. `rollback` has no `--decide` (R-0951), and an add opens no decision today.
- Fix, add to R-1015: "An add takes an optional `down_map`, given to `makemigrations` as a decision answer and stored in `op_data`, and it never leaves a decision open. On down, rows holding the added value move to that label. With none, down refuses with row counts."
- Add this case to T-0160.

**B11. Changes to domains and composites have no rule.**
- Evidence: step 36's deliverable promises a diff with an inverse for each kind, but no requirement says what a changed `check`, base type or attribute does. Synthetic rows can also fail a domain's `check` in the with-data proof.
- Fix: see owner choice O3.

**B12. Two proofs have no CI route.**
- Evidence: T-0164 needs `wal_level=logical`. T-0172 needs a standby. `ci.yml`'s Postgres is a default service container with neither, and steps 51 and 53 list no `.github` work. `rules.md` bans ignored tests.
- Fix:
  - T-0164 append: "Add a `ci.yml` job for this on Postgres 15–18 that starts Postgres with `wal_level=logical`."
  - T-0172 append: "Add a `ci.yml` job on Postgres 15–18 with a streaming standby."
  - Both tests need their own Cargo feature with `required-features`, as `pooler` is, and step 51 and step 53 need `.github` added to their crates line.

**B13. Proofs 13 to 17 have nowhere to run. This is step 02, which is next.**
- Evidence: `02-proofs.md:47` needs four Postgres versions, PgBouncer and a hot standby. CI doesn't run `spikes/`. This machine has no Docker, no Podman, no PgBouncer and one `pg_ctl`.
- Fix, append: "Run these in a temporary `.github/workflows/spikes.yml` on the step's PR, with Postgres 15–18, PgBouncer and a standby. Link the logs from `proofs.md`, and delete the workflow before merge."

### Minors

| ID | Evidence | Change |
|---|---|---|
| M1 | Open point 3 confirmed: R-1056 (`13-types.md:84`) assumes the error, and R-0979 covers `PgEnum` only | Add to R-1056: "Without it, an unknown stored value returns `Error::UnknownEnumValue`, with the Rust type's name and the value." |
| M2 | Open point 4 confirmed: no `trybuild` row exists | New step 13 row: `| T-0182 | trybuild | a TextEnum with two #[text_enum(unknown)] variants, or a variant that isn't a unit variant |` |
| M3 | T-0155 (`36-managed-ops.md:55`) expects an add to exit `2`, but step 51 replaces that refusal. Step 52 also changes the default form, which breaks tests in 51, 53 and 54 | T-0155: "...until step 51, which rewrites this case". Add to step 51 and step 52 context: "Update the earlier tests this changes; for 52, pass `--fast` in the tests of 51, 53 and 54." |
| M4 | R-0992 says `was` answers the decision, but R-1000 (`41-rolling.md:48`) still offers a path | R-1000: "`was` answers only whether it's a rename. The path stays a decision with the instant rename as its default." |
| M5 | R-0998 (`:46`) covers cutovers only, not `--fast` | Add: "A `--fast` swap takes the phase its cutover would." |
| M6 | R-0999 needs the `deprecated` flag recorded, but R-0990 (`35-managed-decl.md:106`) carries only `was`, and step 41 doesn't list `morm-core` | R-0990: "with each `was` and, from step 41, each `deprecated`". Add `morm-core` to step 41's crates. |
| M7 | R-0980 and T-0150 conflict when T-0155 exchanges two labels with `was` | Add to R-0980: "A current label beats any `was`." |
| M8 | R-0961 never says what `Param::pg_type` returns for domains and composites, though step 54 replaces those types | Add: "`pg_type` returns `Type::UNKNOWN`, and composite parameters use text format." Add composites to proof 13. |
| M9 | `contracts.md:117` says "every counter the plan names", but step 03 can't see R-0975, and R-0741's metrics omit it | Contracts row: "...including connections replaced for a stale generation". Add it to R-0741. |
| M10 | Stale "Can run beside" lines: 12 lists 40 and 41, and 40 and 41 list 12, but 40 depends on 12 through 53. Harmless in practice | Remove them from all three files. |
| M11 | R-1008 (`50-readme-release.md:168`) says renaming's down isn't cheap, but R-0991 says it renames back. It also implies old binaries only fail on `PgEnum`, which R-1057 disproves. `#[derive(TextEnum)]`, the preferred path, has no README row | Change to: "Adding a value is cheap, but its down is not." Remove "old binaries can't read values they don't know". Add row R-1058: `| #[derive(TextEnum)] | A column holds one of a fixed set of values; prefer this to a Postgres enum |` |

Two small additions fit under M9 and B6:
- T-0175 should run the cutover as the non-superuser role. T-0163 proves owner and grant copies only for the one-transaction form.
- `contracts.md` should name the trait that gives a column its SQL type and labels.

## 3. Step sizing

| Step | Size | Verdict |
|---|---|---|
| 02 | 17 proofs | **Split before it runs.** Proofs 1–12 stay. Proofs 13–17 become new step 55, between 02 and 03, keeping their numbers. They need a separate database harness, and proof 14 alone is about 96 cases. Step 03 then depends on 55, and `rules.md` adds 55 to the steps the owner approves. |
| 13 | 26 reqs, 9 tests | **Split.** New step 56 takes R-0187–0189, R-0961, R-0976–0980, R-1055, R-1056 and T-0141, 0142, 0148–0150, 0180–0182. Step 14 and step 31 depend on both. |
| 35 | 11 reqs, 3 tests | Keep: many forms, but repetitive. |
| 36 | 9 reqs, 2 tests | **Split.** The count understates it: 11 object kinds, and R-0916 calls some of these the hardest engine work. New step 57 takes enums, domains, composites and sequences (R-0966, 0967, 0991–0993, T-0145, T-0155). Step 37 depends on both. |
| 51 | 20 reqs, 7 tests | **Split.** 51 keeps diff, add, reorder, swap mechanics, recovery, limits and T-0161–0165. New step 58 takes removal, maps, inverses and with-data proof (R-1015, 1016, 1018, 1020–1023, 1027, T-0159, T-0160). R-1019 stays in 51. Step 53 depends on 58. |
| 52 | 9 reqs, 5 tests | Keep, but it's tight once B8 and B9 are in. |
| 53 | 12 reqs, 7 tests | **Split.** 53 keeps R-1029–1037 and T-0166, 0167, 0169, 0170. New step 59 takes R-1038–1040 and T-0168, 0171, 0172, plus T-0169's unmanaged function case. Step 59 takes the dependency on 12. Step 52 then depends on 39, 54 and 59. |
| 54 | 5 reqs, 2 tests | Keep. |

## 4. Checked and good
- All three checker forms pass with exit 0: 1057 requirements, 181 test obligations, 0 problems.
- The frozen plan is unchanged, and its hash in `manifest.json` still matches.
- `coverage.md` and `manifest.json` agree.
- All seven edited existing requirements (R-0188, R-0587, R-0593, R-0608, R-0718, R-0764, R-0860) were approved or necessary.
- Depends-on lines match the steps table, there's no cycle, and waves match dependency levels.
- 39/51 and 52/54 are correctly serial. 51 shares `morm-core` with 43 and 46, which is low risk.
- Step 36 and step 51 agree on the refusal handover, apart from M3.
- Every listed proof has an owner and a stop-and-ask, except that T-0152 relies on proof 14's and T-0151's.
- T-0134 to T-0140 and R-0960 match the current workflows.
- Main's INDEX hunks don't overlap this branch's by inspection.

**Checker judgement:** the text comparison still catches accidental drift between step files and the manifest. It no longer protects approved text, because the same PR edits both, so it only proves two files agree. It also doesn't check "Can run beside" lines, which is how M10 got through.

Exact change to `rules.md`, in "Independent review": "Run `git diff origin/main -- plans/plan-approved/tools/manifest.json`, and confirm each changed ID is covered by an entry in INDEX's Decisions."

## 5. Couldn't check
- No Postgres or PgBouncer behaviour was run. The technical claims in B8, B9 and B12 come from reading the plan and from my knowledge of Postgres.
- The design tests behind the enum steps aren't in the repo. I can't confirm that R-0975, R-0981–0985 or `deprecated`, which INDEX doesn't name, match what the owner approved.
- No test merge of the branch into main was run. I only read the hunks.
- A permission refusal blocked some compound commands. I reran them as plain separate commands, and nothing was skipped.

## 6. Owner choices
- **O1. Fallback variant (B5).** Options: no label, or a real label that can't be written. **Recommend no label**: a label nothing can write serves no purpose.
- **O2. `import` for a text column with a CHECK.**
  - Option A: write `String` with `checks(...)`, keeping the name.
  - Option B: add `#[text_enum(value = "...")]` so it can write a `TextEnum`.
  - **Recommend A.** The catalog can't show that a CHECK is an enum, and R-0118 says nothing is "improved". Add to R-1001: "A `text` column with a CHECK stays `String` with `checks(...)`, and `import` never writes `TextEnum`." The cost is that an existing column with other labels can't use `TextEnum`. B can come later on real need.
- **O3. Changed domains and composites (B11).** **Recommend** new R-1059 in step 36:
  - A changed domain `check` drops and adds the constraint `NOT VALID`, then runs `VALIDATE`.
  - A composite attribute is added or dropped with `ALTER TYPE` when no column uses the type.
  - A changed base type, or a change to a composite a column uses, is refused with exit code `2`.
  - The with-data proof leaves out domain columns whose `check` its rows fail.
  - The alternative is to refuse all such changes, which is simpler but makes these declarations create-only.
- **O4. Further splits (section 3).** The owner set four enum steps, so 58 and 59 change that decision, and 55, 56 and 57 add new steps. **Recommend 55 now**, because step 02 is next, and the others while the plan is being edited anyway.
- **O5. Stale-state retry on held connections (B3).** This adds a behaviour the owner hasn't seen. **Recommend the text in B3**, because moving the connection would lose session state.
