# Step 01, sixth review: changes since 57b3253 (head 09338c2)

## 1. Verdict

**Findings remain: 2 minors, no blockers.** F1 to F6 are applied as the fifth review says, the "Can run beside" lines are correct, all three checker forms pass, CI is green on `09338c2`, and the PR body matches. Both findings are in the step 59 and T-0168 text the fix agent wrote. They can go in one small edit, and neither stops a step session.

## 2. Findings

### G1. Minor: the recreate's source search misses functions that name a carried type

- **Requirement:** R-1038, T-0168, step 59 Context.
- **Evidence:** step 59's Context says "the source search for it is the type's name alone". R-1060 also carries domains, other composites and views through the recreate, and each of those gets a new OID. A function that names only one of them has no `pg_depend` row, and a search for the composite's name doesn't find it.
- **Reproduced on 18.6 and 17.11**, in a held session, with 0 `pg_depend` type rows. After dropping and recreating `c` and the object carried with it:

| Function body | Result with no refresh, on every call | After a refresh |
|---|---|---|
| `DECLARE v c2[]`, `v := '{}'`, where `c2` holds `c` | `cache lookup failed for type` | works |
| `DECLARE v d[]`, `v := '{}'`, where `d` is a domain over `c` | `cache lookup failed for type` | works |
| `DECLARE v d`, `v := NULL` | `type with OID ... does not exist` | works |
| `DECLARE v vw[]`, `v := '{}'`, where `vw` is carried | `cache lookup failed for type` | works |

- **Failure:** a step 59 session builds exactly what Context says, and T-0168 passes. A managed function that names only a carried domain then fails in every open session, which step 59's Goal says can't happen. An unmanaged one gets no decision at all, so other services get no warning. Owner decision A exists to prevent exactly this.
- **Change, see O1 for the wording of R-1038:**
  - **R-1038:** replace "also runs for the type's name, for each label that changes" with "also runs for the type's name, for every domain, composite type and view replaced along with it, for each label that changes".
  - **Step 59 Context:** replace "the source search for it is the type's name alone." with "the source search for it is the type's name and the name of every domain, composite type and view carried with it, because each of those gets a new OID too."
  - **T-0168:** after "and works with one." in the composite sentence, add "So does one that names only a domain carried through that recreate."
  - **PR body, composite recreate bullet:** replace "the source search runs first," with "the source search runs first, for the type and for every domain, composite type and view replaced with it,".
  - Update `tools/manifest.json` for both IDs. The `coverage.md` rows are cut off before these edits, so they don't change. Both IDs are already in "Fifth review corrections", so no new Decisions entry is needed.

### G2. Minor: T-0168's composite case is false for a body that doesn't assign a value

- **Requirement:** T-0168.
- **Evidence:** the row says a session that compiled `DECLARE v the_composite[]` "fails on every call", with no body given. On 18.6 and 17.11, `v := NULL` keeps working after a recreate and `v := '{}'` fails. On 18.6, an element assignment and `SELECT ... INTO v` fail too. All of these work again with a refresh. Context's "only some bodies go stale" hints at this, but the row doesn't say it.
- **Failure:** a session picks a minimal body and sees no failure. T-0168 says a contradicting result stops and asks the owner, so the owner gets a question with no real design problem behind it.
- **Change:** in T-0168, replace "compiled `DECLARE v the_composite[]` in a function" with "compiled `DECLARE v the_composite[]` and `v := '{}'` in a function". Update `tools/manifest.json` in the same edit as G1.

## 3. Checked and good

- **F1:** 431 "Can run beside" pairs over all 59 step files: 0 with a dependency path, 0 asymmetric, 0 unknown, no duplicates, no cycle. The only change to each line is the deletion the review named, and those 16 step files changed nothing else.
- **F2 to F5:** the text is exact, in the R-1059, R-1060, T-0185 and T-0186 rows, step 37 Context, step 57 Context and the PR body.
- **F6, option A:** R-1038, R-1039, T-0168, Goal, Deliverables, End-to-end check, Out of scope and Context are all applied.
- **Stale `steps_data.py`:** not a problem. Only `split.py` and `build_docs.py` import it. `check_coverage.py` doesn't, and no rule, step or workflow runs those two. `rules.md` has plan changes edited by hand. Running either script would overwrite the hand-edited plan, but nothing tells anyone to.
- **Item 3, step 59 can be built:** apart from G1 and G2, a session reading only step 59 can build and test the composite case. A test can run the recreate by hand, as T-0156 does, and Context gives the scope, the error and the reason for a refresh.
- **Item 3, order:** 59 reaches 57, 37 and 38 through 53, so recreates, carrying, the refresh and the search all exist by then.
- **Refresh:** `CREATE OR REPLACE` inside the recreate's own transaction clears the stale session, on 18.6 and 17.11. Plain `c` variables keep working, as Context says.
- **Item 4, "as step 53 does for enums":** it doesn't mislead. The sentence is complete without it, it's true (R-1030), and R-0989 in step 32 already names step 53 the same way. No change needed.
- **Item 4, `rollback` in step 37:** no change needed. Step 57 owns that check and T-0185 keeps its rollback case, which isn't one of the cases step 37 rewrites. The ledger's down SQL has no carry in it, so the refusal should stay at rollback. If a session loosened it for a dependent, Postgres would still refuse `DROP ATTRIBUTE`, so no data is lost.
- **Item 5:** plain, `--code` and `--step 01` forms each exit 0 at 1060 requirements, 186 obligations, 0 problems. The plain form also passes on a clean trial merge with `main`, which keeps 59 rows and the Fifth entry.
- **Traceability:** exactly 7 IDs changed: R-1038, R-1039, R-1059, R-1060, T-0168, T-0185, T-0186. All are in "Fifth review corrections", and their `coverage.md` rows match.
- **CI on `09338c2`:** runs 42198258120 and 42198266249 passed, 18 jobs each, 36 checks, merge state `CLEAN`. I read job conclusions, not logs.
- **Item 6:** the F5 additions, the rollback and carried-object text, the composite recreate bullet and every listed step edge match the step files.
- **Not a new defect:** for a recreate, "affected schemas" can only mean the type's own schema, so a function elsewhere is missed. That's R-0572's existing limit, the same as for enums.

## 4. Couldn't check

- Postgres 15 and 16: not installed here. The probes ran on 18.6 and 17.11. T-0168 runs on all four and stops on a contradiction.
- `DISCARD ALL` in step 59's Context: not rerun, taken from the fifth review.
- PgBouncer: not run. `actionlint`: not run, as told.
- Several commands were refused as too complex, and I reran them as plain separate commands, so nothing was skipped. `git merge-tree` left an unreferenced tree object in the shared store; no ref changed. The throwaway servers ran from scratch on localhost, are stopped, and their files are deleted.

## 5. Owner choices

**O1. How wide G1's search goes.** The same gap exists for enums, and that path hasn't changed since `57b3253`. Step 54 replaces a domain or composite over a swapped enum, and a hand-run swap on 18.6 and 17.11 left a function naming only that domain's array failing on every call until a refresh.
- **A, cover both:** use G1's wording, "replaced along with it", which covers a swap and a recreate in one line. It costs one more search name per wrapper, and it adds no test beyond G1's unless the owner wants an enum case in T-0168. **I recommend this.**
- **B, recreates only:** in R-1038 use "every domain, composite type and view step 37 carries through a recreate". The enum gap stays and should be recorded as known.

Either way one small gap stays: a view carried by `ALTER TYPE` or by a column type change (R-0542) also gets a new row type, and R-1038 doesn't run there. It needs an array of a view's row type in a plpgsql body, so I'd accept it.
