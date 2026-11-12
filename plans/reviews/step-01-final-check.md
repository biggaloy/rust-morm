# Step 01: final check

The last short check of PR #1 before it goes to the owner, as "Finishing step 01's review" in `plan-approved/INDEX.md` says. Scope: `git diff 09338c2 origin/step-01-bootstrap`, at branch head `28b0c68`, against the decisions in `INDEX.md` from "Silent narrowing, closed" onward, `reviews/fk-widening-design.md`, `reviews/step-60-families-design.md` and `reviews/step-01-open-items.md`, plus the CI changes.

## What was checked

- **Decisions written in.** Each decision from "Silent narrowing, closed" to "CF to CI" was traced to its requirements in steps 05, 10, 13, 14, 29, 30, 33, 34, 38, 39, 40, 41, 44, 50, 52 and 60 and `contracts.md`, with greps for stale text: `--fast` handling a child or a partition without every one declared, a refusal of `pk` on a partition, the old unbounded copy, and the old lock order. Nothing stale was found apart from CJ7. Two places contradict a decision or another requirement (CJ3, CJ4).
- **README notices and one-liners.** Every refusal added in scope has a loud notice in step 50 (R-1119, R-1129, R-1137 to R-1145, R-1155, R-1156, R-1163 to R-1166, R-1170, R-1171, R-1173, R-1188, R-1189), and the new opt-ins have rows (R-1112, R-1120, R-1146, R-1073, R-1074, R-1100). One notice's "What you give up" is wrong (CJ2).
- **Postgres claims,** checked on a throwaway Postgres 18.6. CF to CI held as written: `ATTACH PARTITION` took over `p3_other` with `conparentid` set; `RENAME CONSTRAINT p3_other TO p3_pkey` kept `conparentid` and the index's `pg_inherits` row; adding or dropping identity on a partition alone failed with the quoted messages; attaching a table with identity failed with `table "q2" being attached contains an identity column "id"`; identity added on the parent showed on the partition's `attidentity`, with no sequence of its own; `ADD PRIMARY KEY` on a partition holding its parent's key failed with `multiple primary keys`. R-1150's fit table did not hold as written (CJ1), and a partition's model with no identity can't insert under `GENERATED ALWAYS` (CJ2).
- **Coverage checker,** at `28b0c68`:
  - `check_coverage.py`: 1189 requirements, 267 test obligations, 0 problems, exit 0
  - `check_coverage.py --step 01 --code crates tests benches fuzz .github`: the same, exit 0
  - `check_coverage.py --code crates tests benches fuzz .github`: the same, exit 0
- **CI at `28b0c68`.** The push run 43166440847 and the pull_request run 43166448412 each passed all 18 jobs: fmt, clippy, unit, trybuild, doc, coverage, the 8 integration jobs, pooler, msrv, features and ci. PR #1 is `MERGEABLE` and `CLEAN`.
- **CI scripts.** `start_pgbouncer.sh` and `install_time.sh` fail the step when every source fails, print `::warning::` on the fallback and `::error::` on failure, cut off each apt command and download, and are tested through every path in `repo_policy.rs` with stubs. The pins are real: the image digest `sha256:7e21afe7…dd7c` is what `ghcr.io` serves for `cloudnative-pg/pgbouncer:1.26.1-202610260932-trixie`, and its amd64 image runs `/usr/bin/pgbouncer /etc/pgbouncer/pgbouncer.ini` as user `pgbouncer`, which is where the script mounts the ini. Both Debian URLs served a file whose SHA-256 is the pinned `10257c6f…5e23`. Run 43081296446 took the image path and passed, and bench run 43141161272 took the Debian path and passed. Only CJ8 is open here.
- **PR #1 body,** read against the branch and the runs it cites. Two problems (CJ5, CJ6).

## Findings

### CJ1. R-1150's fit predicates give false refusals, raw errors and a miss

- **Severity:** should fix
- **Where:** `steps/60-key-widening.md`, R-1150 (table rows for `numeric(p,s)`), R-1127 (the sequence check against `10^(p-s)`), T-0239
- **What's wrong:** as written, `10^(p-s)` is `double precision`, because `^` with integer operands resolves to `float8`, and `numeric >= float8` then compares as `float8`. Three things break:
  1. **False refusals at 16 digits or more.** `99999999999999999999` fits `numeric(20,0)`, but `abs(round(v, 0)) >= 10^(20-0)` is true. `999999999999999999::bigint` fits `numeric(18,0)`, but `abs(v) >= 10^18` is true. `numeric(20,0)` is the plan's own example of a key type, so a valid narrowing to it is refused.
  2. **Raw errors at preflight.** `abs(v)` on the integer types overflows on their minimum: `abs((-2147483648)::int)` gave `integer out of range`, and `bigint`'s minimum gave `bigint out of range`, while assigning it to `numeric(19,0)` worked. `10^(p-s)` overflows `float8` once `p - s` reaches 309, which `numeric` allows: `10^(400-0)` gave `value out of range: overflow`.
  3. **A miss with a negative scale from an integer type.** The integer row has no `round`. `9999960::int` into `numeric(5,-2)` failed assignment with `numeric field overflow` (`must round to an absolute value less than 10^7`), but `abs(v) >= 10^7` is false, so preflight passes it and the copy stops mid-run with a raw error. R-1150 says it agreed with the cast on "a negative scale", which must have come only from `numeric`.
- **Evidence:** Postgres 18.6: `pg_typeof(10^(20-0))` is `double precision`. Each case above was run against the assignment cast in a table. `abs(round(v, 0)) >= 10::numeric ^ (20-0)` gave the right answer for the 20-digit case.
- **Fix:** keep every bound and comparison in `numeric`. For both `numeric(p,s)` rows, use `abs(round(v::numeric, s)) >= 10::numeric ^ (p - s)`, written as a `numeric` literal such as `1e20` or `10::numeric ^ (p-s)`. This one form also covers the integer source types: the cast removes the overflow, and `round` handles a negative scale. Build the integer-range row with `numeric` constants too. Use the same `numeric` bound in R-1127's sequence-position check. Add to T-0239's pairs: 20 nines into `numeric(20,0)`, 18 nines from `bigint` into `numeric(18,0)`, the minimum of `integer` and `bigint` into `numeric(p,0)`, `9999960` from `integer` into `numeric(5,-2)`, and a value into `numeric(400,0)`.

### CJ2. A partition's model with no identity can't insert under `GENERATED ALWAYS`

- **Severity:** should fix. It needs a design choice.
- **Where:** `steps/50-readme-release.md` R-1189; `steps/33-execution.md` R-1186; `steps/41-rolling.md` R-1180; `steps/44-import.md` R-1181
- **What's wrong:** CH tells a partition's model to use `base = "bare"` with `pk` and no identity. R-1189 says this gives up "Nothing". But that model's key field is a plain field, so its inserts send the key:
  - When the parent's identity is `GENERATED ALWAYS`, every insert through the partition's model fails.
  - When it's `BY DEFAULT`, the app must supply every key, and supplied keys can collide later with the shared sequence.

  R-1180 lets such a build pass `check_schema`, and R-1181's `import` writes the same model. So the build starts, and then fails on every insert through that model, while the README says nothing is lost. BJ already refuses at pool start the mirror case, a build whose inserts leave out a column that has lost its identity.
- **Evidence:** Postgres 18.6, on a partitioned table `p` with `id int GENERATED ALWAYS AS IDENTITY` and partition `p1`:
  - `INSERT INTO p1 (id, k) VALUES (100, 1)` failed with `428C9` `cannot insert a non-DEFAULT value into column "id"`, `Column "id" is an identity column defined as GENERATED ALWAYS`.
  - `INSERT INTO p1 (k) VALUES (2)` took `1` from the parent's sequence.
- **Fix (recommended):**
  - The partition's model marks the held-identity key `db_set` (R-0875), so inserts leave it out and take the parent's sequence.
  - R-1186's message, R-1189's "How to go on" and R-1181's `import` output say so.
  - R-1180's `check_schema` refuses at pool start a partition's model whose inserts send a column holding `GENERATED ALWAYS` identity, naming the column and pointing to `db_set`, as BJ does for the mirror case.
  - R-1189's "What you give up" then says inserts through the partition's model can't choose the key.
  - The other choice is to have morm leave the column out of the partition model's inserts by itself. It can't see at compile time that the table is a partition, so `db_set` is simpler and explicit. Either one needs the owner's answer.

### CJ3. `rollback --fast` is defined to change nothing, but the plan relies on it changing the form

- **Severity:** should fix. It needs a design choice.
- **Where:** `steps/41-rolling.md` R-1110; `steps/39-safe-forms.md` R-1167 and T-0253; `steps/50-readme-release.md` R-1170
- **What's wrong:**
  - R-1110 says `rollback --phase contract` and `rollback --fast` each let a silent narrowing through and "change nothing else: `rollback` runs the same steps of `down.sql` either way".
  - R-1167 refuses, on Postgres 15 to 17, a foreign key that step 38's rollback of a retire would add back `NOT VALID` on a partitioned table, "saying `--fast` handles it". T-0253 then says "Under `--fast` each is added validated".
  - R-1170's README notice tells the user to "roll back with `rollback --fast`".

  Under R-1110, `rollback --fast` runs the same `NOT VALID` statement, so the refusal has no way through at rollback time, and the README's advice fails.
- **Fix (recommended):**
  - Give `rollback --fast` one meaning: it runs the inverse with apps stopped, and every inverse step that has a one-transaction form uses it. Here, that adds the foreign key validated in one statement, as R-1167 describes for `--fast`.
  - Narrow R-1110's sentence to say that, for a silent narrowing, the flag changes nothing else.
  - R-1167 names `rollback --fast` explicitly, and T-0253 tests it.
  - If the owner prefers `rollback --fast` to stay a pure gate, R-1167 and R-1170 must instead name the real way through on 15 to 17, such as a forward migration written with `makemigrations --fast`, and T-0253 must change to match.

### CJ4. The merge check runs in the swap's first step, not at preflight, when the new type is new

- **Severity:** should fix
- **Where:** `steps/39-safe-forms.md` R-1159; `steps/52-enum-online.md` R-1162 and T-0251; `steps/50-readme-release.md` R-1165
- **What's wrong:**
  - The owner answered BX "as recommended": a `GROUP BY` preflight refusal, which `INDEX.md` records as "refuses at preflight".
  - As written, when the migration creates the new type, as every enum swap does, the check runs in the swap's first step. A merge then stops the migration as `failed` with exit code `2`, not as a preflight refusal with nothing recorded.
  - This placement isn't among BV to BY's "edit's own choices the owner approves through PR #1", so it reached the plan without an owner decision.
- **Fix (recommended):** run it at preflight, in a transaction that creates the new type, runs the `GROUP BY` and rolls back. `CREATE TYPE` is transactional, and the check takes only `ACCESS SHARE` on the table, so nothing changes and no failed state is left. Otherwise, list the first-step placement in `INDEX.md` and the PR body as an edit choice for the owner to approve.

### CJ5. The PR body still says no third-party image is needed

- **Severity:** should fix
- **Where:** PR #1 body, "Choices the step file doesn't settle"
- **What's wrong:** "PgBouncer source: Ubuntu's package on the runner, so no third-party image is needed." Since `b38e532`, `start_pgbouncer.sh` falls back to CloudNativePG's image from `ghcr.io`, as the body's own BZ/CA item says.
- **Fix:** say that Ubuntu's package is tried first, with one retry, then CloudNativePG's PgBouncer 1.26.1 image pinned by digest, and that the step fails if that fails too.

### CJ6. The PR body leaves out most of the choices the owner approves through it

- **Severity:** should fix
- **Where:** PR #1 body, "Owner decisions made", the items from "Rollback loss" to "CF to CI"
- **What's wrong:** `INDEX.md` says the owner approves these through PR #1, and the body lists only BE to BR's:
  - `contracts.md` gaining `Error::SchemaMismatch`
  - the FK widening edit's choices: R-1126 carries choice I instead of R-1025, the scaled wait rounds down and under 1 ms keeps the exit-2 refusal, partitions are refused online, the cutover moves a replica identity index
  - BS to BU's: concurrent rebuild in place under `_morm_new_<name>`, the swap's indexes from scratch's `pg_get_indexdef` re-attached `USING INDEX`, "partitioned" including a partition and re-checked under the lock, the index check living in step 39
  - BV to BY's: the top parent's step where its tree's first step sits, refusals for a declared child without the same change and for two parents, constraint-owned index re-attached in a short transaction, an undeclared foreign key on a rebuilt index refused, BX skipped only among the integer types, BY refusing only the parent and a unique key missing a partition-key column even under `--fast`
  - BZ/CA's: the rolling rename on declared trees, batched backfills with `UPDATE ONLY`
  - CB to CD's: a CHECK a child both declares and inherits is the parent's while inherited
  - CF to CI's: the third case refusing a partition's own key removed in the same migration, a partition `pk` in a later migration after the parent's key is dropped, identity as R-1168's one exception, a partition's held identity never failing `check_schema`
- **Fix:** add a line to each item listing what the owner approves through this PR, copied from `INDEX.md`, plus CJ4's placement if it stays.

### CJ7. Step 60's quote of step 52 is out of date

- **Severity:** minor
- **Where:** `steps/60-key-widening.md`, "Context from other steps", the step 52 paragraph
- **What's wrong:** it says step 52 refuses "partitioned tables" and that "`--fast` handles a partitioned table only when the models declare every partition". Since BS and BV, step 52 also refuses a partition and old-style inheritance (R-1052), and `--fast` needs every child and partition declared with the same swap (R-1161). The rules are right in step 52 and in step 60's own R-1125 and R-1126. Only the quote is stale.
- **Fix:** quote R-1052's list, and say "only when every child and partition the change reaches is declared by a model with the same change".

### CJ8. The PgBouncer image fallback doesn't show why it failed

- **Severity:** minor
- **Where:** `.github/scripts/start_pgbouncer.sh`
- **What's wrong:** `docker run -d` succeeds as soon as the container starts. If PgBouncer then exits, say on a config error, `pg_isready -t 30` fails the step correctly, but the log doesn't show the container's output, so the cause isn't visible.
- **Fix:** on the image path, if `pg_isready` fails, run `docker logs morm-pgbouncer` before exiting `1`, and test that path in `repo_policy.rs`.

## Summary

Nothing blocks approval on its own. Six findings should be fixed before the owner approves:

- CJ1 is a wrong formula a later step would copy.
- CJ2 and CJ3 each need an owner choice, and a recommendation is given for each.
- CJ4 is a decision written in differently from the answer the owner gave.
- CJ5 and CJ6 are PR body accuracy.

CJ7 and CJ8 are minor. The coverage checker is clean in all three forms, CI is green on every job of both runs at `28b0c68`, and PR #1 is `MERGEABLE` and `CLEAN`.
