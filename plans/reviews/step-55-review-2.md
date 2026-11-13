# Step 55 independent review, second round

PR #3 (`step-55-enum-proofs`), head `1e8e9f8`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or the fix. This round covers the fixes for the first review (`reviews/step-55-review.md`, EE1 to EE10, plus EE11 from `reviews/step-55-open-items.md`) and the owner's answers EA and SA to SH: `git diff 7a7285b origin/step-55-enum-proofs`.

## What I read

- **On the PR branch:** `rules.md`, `contracts.md`, `proofs.md` (all of step 55's section), `steps/55-enum-proofs.md`, and every step file the fix touches: 11, 12, 14, 15, 16, 20, 22, 33, 37, 38, 41, 44, 45, 50, 52, 56, 58 and 59. In the steps the fix leans on, I also read the text around each change: R-0118, R-0677, R-1062, R-1083, R-1117, R-1234 and T-0188.
- **On `origin/main`:**
  - INDEX's "Decisions since the split", especially "Step 55, EA and the stale-statement design" and "Stale-statement design, SA to SH";
  - `reviews/step-55-review.md`, `reviews/stale-statement-design.md` and `reviews/step-55-open-items.md`;
  - the PR body.
- **A change on `main` during the review:** `main` moved to `cf41636`, which records the owner's approval of the `22P02` narrowing. The narrowing is now an owner decision, not one of the fix's own choices, and EF1 below is framed that way.

## What I checked and ran

- **CI on the head.** `1e8e9f8` is green: `ci` passed on push (run 43467341466) and on the pull request (run 43467354108). Every job passed, including the `pooler` job and integration on 15 to 18 over TCP and Unix sockets.
- **Coverage checker.** I ran it on an export of the head.
  - `check_coverage.py --step 55 --code crates tests benches fuzz .github`: **1254 requirements, 310 test obligations, 0 problems**.
  - The plain form gives the same totals.
- **Manifest and INDEX.** I ran `git diff origin/main...origin/step-55-enum-proofs -- plans/plan-approved/tools/manifest.json` and compared the two manifests ID by ID.
  - 33 IDs changed and 24 are new; none was removed.
  - Changed: R-0697, R-0718, R-0975, R-0977, R-0978, R-0981 to R-0984, R-0994, R-0995, R-0997, R-1001, R-1023, R-1039, R-1040, R-1049, R-1050, R-1080, R-1084, R-1093, R-1116, R-1238, T-0104, T-0148, T-0150 to T-0153, T-0168, T-0171, T-0172 and T-0175.
  - New: R-1241 to R-1254 and T-0301 to T-0310.
  - Every one of the 57 is in the "Stale-statement design, SA to SH" entry's "written in at `1e8e9f8`" list. That list names nothing the manifest doesn't change, and the PR body's table by step matches both.
- **The first review's findings, one by one.** See "EE1 to EE11" below.
- **Postgres spot checks.** These ran on throwaway servers under `.claude/scratch/` in my worktree, with `TMPDIR` set there:
  - Postgres 18.6 (the system build);
  - Postgres 17.7, built from ftp.postgresql.org's tarball, SHA-256 `ef9e3433…e9b5`, which matches the published `.sha256`;
  - PgBouncer 1.22.1, built from pgbouncer.org's tarball, SHA-256 `2b018aa6…ba215`, which matches its `.sha256`;
  - `plpgsql_check` 2.10.16, built from the `v2.10.16` tag's tarball, SHA-256 `b4734ffa…88e3`, the same hash the design recorded.

  The results:

  | Claim | Where I ran it | Result |
  |---|---|---|
  | SA's table with `PREPARE b_lt(text,text)`, `b_const`, a free `($1::st)::text`, a `DECLARE v st` function, a constant-only function and a `DECLARE` procedure, under no relabel and no reset, relabel only, reset only, and both | 18.6 and 17.7, straight | Same as the design's table on both versions. With no relabel and no reset, everything is silently `true`. With the relabel only, `22P02` naming `_morm_retired_m2_st`, but `b_const` and the constant-only function stay silently `true`. With the reset only, `false` and right, but `DECLARE` gives `42883 operator does not exist: _morm_retired_m2_st < st`. With both, every statement is right and `DECLARE` gives `22P02` naming the retired type |
  | The reset behind PgBouncer | 18.6 through 1.22.1, `pool_mode = transaction`, `max_prepared_statements = 200`, one server connection, with protocol-level named statements from psql 18's `\parse` and `\bind_named` | Without the reset, the cached `((($1::text)::st) < (($2::text)::st))::text` and `('a'::st < 'c'::st)::text` stay silently `true`. With the relabel and the reset, both give `false`. One side note confirms that R-1220's carriers are load-bearing: with `($1::st)` and no carrier, the parameter's type is the old enum, fixed at Parse, so the reset can't help and Bind fails with `22P02` |
  | A no-op `ALTER FUNCTION ... COST 100` and `ALTER PROCEDURE ... SECURITY INVOKER` on an `INVOKER` procedure, run by a non-superuser owner after a relabel-and-reset swap that the same role ran | 18.6 and 17.7 straight, and 18.6 through PgBouncer 1.22.1 | `ALTER PROCEDURE ... COST 100` fails with `invalid attribute in procedure definition`. `SECURITY INVOKER` writes a new row: on 17.7, `xmin` went from 799 to 801 and `ctid` from `(43,33)` to `(43,34)`. After it, the failing session's procedure and function both return the right answer. So the fix's procedure choice holds |
  | "A domain over a built-in type needs no cast" (R-1241) | 18.6 and 17.7, `\gdesc` | Holds for a scalar: a domain over `text` comes back as `text`, over `int` as `integer`, and over an enum as the enum. **It doesn't hold for an array of a domain**: `posint[]`, `dtext[]` and `denum[]` come back as their own array types, with OIDs of 16384 or more (EF2) |
  | R-1093's word search for `performance_warnings`, and R-1080's for `disable:check` | 18.6 with `plpgsql_check` 2.10.16 | Escaped string literals hide the narrowing and miss both searches (EF3) |

## EE1 to EE11: are they fixed?

| Finding | Fixed by | Verdict and evidence |
|---|---|---|
| EE1, silent stale code on the kept type | SA | **Fixed.** The reset is R-1248 and T-0308. The relabel is R-0995, R-1049, R-1050, R-1023 and T-0309. T-0151, T-0168 and T-0175 gain the order-sensitive and constant-only cases. Proof 14's table no longer calls a shared label "right". I reproduced the design's table on 18.6 and 17.7, and the reset through PgBouncer. The narrowing in EF1 doesn't reopen the silent case: every case it touches is a loud `22P02` |
| EE2, the driver's lookup behind PgBouncer | EA | **Fixed, with two gaps.** EA is R-1241 to R-1243 and T-0301 to T-0304. R-1238 and T-0104 require pools of at least three. The gaps are a list of a domain over a built-in type (EF2), and T-0304's 1.22 and 1.26.1 legs, which don't run the failing configuration (EF4) |
| EE3, EB's framing | SC | **Fixed.** R-1245's table lists the three `42804` routines. R-0983 and R-0984 no longer read message text. Proof 14 corrects the `lc_messages` claim, and proof 15's table gives the trigger's routine |
| EE4, the per-pool marker | SD, SE | **Fixed, with one gap.** R-1246 puts the shared marker in the `_morm` comment, and R-0718 handles prepare-time errors. But a pool with `check_schema = "off"` no longer reads the marker at start (EF5) |
| EE5, the kept type is the default path | SB | **Text changed** in R-1040, R-1250 and R-1249. The approved `22P02` narrowing leaves that text promising more than the rule delivers (EF1) |
| EE6, the pragma that hides a narrowing | SH | **Fixed as designed**, in R-1093, R-1080, T-0310 and R-1084, but the word search can be bypassed (EF3) |
| EE7, `DISCARD PLANS` | R-0994 | **Fixed.** R-0994 names the shapes `DISCARD PLANS` clears and the ones it doesn't |
| EE8, statements cached before `ADD VALUE` | proof 13, T-0148 | **Fixed** |
| EE9, the composite decoder | R-1241, T-0301, `contracts.md` | **Fixed.** The decoder is now the text form, with NULL fields, bounds checks, and errors rather than panics |
| EE10, the end-to-end check | proofs.md | **Met**, provided the owner approves the fix's remaining choices. See EF9(c) on where the evidence ran |
| EE11a, a standby ends a transaction | SG | **Fixed** in R-1247, R-0697, R-1244, T-0305, T-0307, T-0172 and R-1252 |
| EE11b, stale errors at prepare time | SD | **Fixed** in R-0718 and R-1246 |

The stale text the task named is gone:

- **The driver's type lookups.** No step file describes a decode that relies on the driver's own lookup any more. The new text only explains why it must be avoided.
- **"Right" for a shared label on the kept type.** The proof 14 table now says the old type's answer comes back silently.
- **Per-pool generation counters.** R-0975 keeps the connection counter. The statement marker is the shared one from R-1246.
- **Matching translated text.** R-0981, R-0983 and R-0984 now use R-1245's signals.

## The fix's own choices

1. **EA covers types morm doesn't know, such as `citext` and `ltree`.** This is sound. The same lookup applies to them, and T5's rule for OIDs of 16384 or more is right. The claim that "a domain over a built-in type needs no cast" is only half right (EF2).
2. **An uncast enum or composite in raw SQL returns `SchemaMismatch`.** This is sound, and it has R-1254's notice.
3. **A `22P02` counts as stale only once its retry succeeds.** The owner has now approved this, at `cf41636`. It meets its goal for the generation: a client can't clear the caches. It doesn't reopen EE1's silent case. But it isn't complete:
   - a real stale session's retry lands on another pooled connection, which is often stale too;
   - a client can still make morm throw away one connection per request.

   EF1 covers both.
4. **`ALTER PROCEDURE ... SECURITY <current>` for procedures.** Confirmed on 18.6 and 17.7, and through PgBouncer.
5. **The `standby` job moves to step 12.** This is consistent. R-1083 should name the job (EF9).
6. **`check_schema` warns on a `_morm_` label, and `import` stops on one.** The warning is fine. `import` stopping on a retired type's labels is a problem (EF7).
7. **T-0151 and T-0152 run in German, and a unit test covers the third `42804` routine.** This is sound.

## Findings

### EF1. Blocks approval, needs an owner decision. R-0981, R-0984, R-1245, R-1040, R-1249, R-1250, T-0152 and T-0153: under the approved narrowing, a stale `22P02` session doesn't recover by itself, and a client can still churn connections

- **Where:**
  - R-0981 (step 14): "raises the pool's connection generation: at once for the first two, and for the `22P02` only once the retry has succeeded".
  - R-0982 and R-1245's `22P02` row.
  - R-0984 (step 20).
  - R-1040 and T-0168 (step 59).
  - R-1249 and R-1250 (step 50).
  - T-0152 and T-0153 (step 14).
  - Choice 3 in the PR body.
- **What's wrong:**
  - **The retry rarely confirms anything.**
    - The `22P02` comes from a plpgsql variable compiled in the session, so it's a stale *session*, not a stale statement.
    - Straight to Postgres, R-0981 retries it "on another" connection, taken from the pool.
    - Under the narrowing nothing raises the connection generation before that retry, so the checkout doesn't replace a connection that's behind.
    - Any connection that called the function before the cutover has it compiled too. So in a pool where the function is in use, the retry usually lands on another stale connection and fails the same way. The error then reaches the app, and nothing is raised.
    - Recovery becomes one failed call per stale connection, each discarded only when it fails itself.
    - Under SB as designed, the first `22P02` raised the generation, so the retry checked out a replacement, it succeeded, and no error reached the app.
  - **Inside a transaction, it can never be confirmed.**
    - R-0984 returns `SchemaChanged` and replaces that one connection, but there is no retry, so the generation never rises.
    - `.retry(n)`'s next attempt checks out another connection, which is probably stale.
    - So `.retry(3)` can run out while four or more connections still hold the function.
  - **The text still promises automatic recovery:**
    - R-1040: "morm apps connected straight to Postgres recover by themselves from ... a retired type's labels";
    - R-1249: "morm apps recover from it by themselves";
    - R-1250: "recover by themselves, by replacing the connection";
    - T-0152: "the same holds" for the `DECLARE` case, including "other connections in the pool are replaced at checkout". That only passes if the test's retry happens to land on a fresh connection.
  - **The narrowing doesn't stop churn.**
    - R-0981 still discards the failing connection before the retry.
    - So a client that sends a label holding `_morm_retired_` makes morm throw away and reopen one connection per request.
    - T-0153 asserts only that no generation is raised and nothing is replaced at checkout.
    - The PR body's rationale, "A true stale statement's retry succeeds, so nothing real is lost", mixes up a stale statement, which is retried on a fresh statement and does succeed, with a stale session, which is retried on another pooled connection and often doesn't.
- **Evidence:**
  - The design's own SB evidence: a `DECLARE v st` function compiled before the swap "kept failing on every call ... until something recompiled it", in every session that had compiled it.
  - My runs on 18.6 and 17.7: every session that called `f_decl` or `p_decl` before a relabel-and-reset swap got `22P02 invalid input value for enum _morm_retired_m2_st: "a"` until the no-op `ALTER`.
  - The rest is the text of R-0981, R-0975 (a checkout replaces only a connection behind the generation), R-0984 and T-0153.
  - No morm runtime exists yet, so this is shown from the rules and from Postgres's behaviour, not from a pool.
- **Options.** Each keeps the owner's goal that a client can't clear the caches.
  - **(a) Recommended. Look at the parameters morm sent before retrying.**
    - morm encodes every parameter itself, and `PgEnum` can't carry a `_morm_` label (R-0977). So when no bound parameter's text holds `_morm_retired_`, the client didn't supply that text.
    - In that case, retry once on a newly opened connection rather than a pooled one. A success there really confirms a stale session, as the owner's rule asks. Then raise the generation, and the rest of the pool is replaced at checkout.
    - When a bound parameter does hold `_morm_retired_`, return the error at once, with no retry and no discard. That's a client's bad label, and it costs nothing.
    - Inside a transaction, the same test on the parameters decides whether R-0984 raises the generation after the rollback, so `.retry(n)` gets fresh connections.
    - One case is left: text a function reads from a table that holds `_morm_retired_`. It costs one extra connection per call, and only on a schema that converts stored text to the enum. The README can say so.
  - **(b) Keep the rule exactly as approved, but retry on a newly opened connection, and don't discard the first connection until the retry succeeds.** This is simpler. A client's bad label then costs one new connection per request.
  - **(c) Keep the rule and correct the text.** R-1040, R-1249, R-1250, T-0152 and the PR body would say that recovery costs a failed call per stale connection, and that `.retry(n)` can run out inside a transaction. T-0153 would say a client can make morm replace one connection per request.
- **Whichever is chosen:**
  - T-0152 runs the `DECLARE` case with at least three pool connections that compiled the function, and asserts how many errors reach the caller.
  - T-0153 asserts what a client's `_morm_retired_` label costs in connections.

### EF2. Should fix. R-1241, R-1243, `contracts.md` (`READ_AS_TEXT`), T-0301 and T-0303: a list of a domain over a built-in type still makes the driver look its type up

- **Where:**
  - R-1241 says `READ_AS_TEXT` is "false for every other type", and "a domain over a built-in type needs no cast".
  - R-1243 says "morm's own column lists never give one".
  - T-0303 expects no finding for "a column of a domain over `integer`" or for "any query a model's builders write".
  - The same claim is in `proofs.md` (EA's paragraph), the `contracts.md` comment, and choice 1 in the PR body.
- **What's wrong:**
  - Postgres sends a scalar domain's base type in the row description, but not for an array of a domain.
  - So a `Vec<T>` field, where `T` is a `PgDomain` over `integer` on a `posint[]` column (R-1233 gives `PgDomain` lists), comes back as the array-of-domain type, with an OID of 16384 or more.
  - `tokio-postgres` then runs its named lookup statements, which is EE2's failure behind PgBouncer with no prepared statements.
  - T5 would also report a model's own query, against R-1243's text.
  - This is in `contracts.md`, which step 03 freezes.
- **Evidence:** `\gdesc` on 18.6 and 17.7, for a table with columns `dtext`, `posint`, `denum`, `posint[]`, `dtext[]` and `denum[]`:
  - the scalar columns come back as `text`, `integer` and `de`;
  - the array columns come back as `posint[]` (OID 16532 on 18.6), `dtext[]` and `denum[]`.

  `SELECT d::int4[], e::text[] FROM dt` returns `{1,2}` and `{u}`.
- **Fix:**
  - Read a list of a `PgDomain` over a built-in type as its base type's array, as in `"col"::int4[]`. That keeps the binary decoder, and Postgres accepts the cast, as the run above shows.
  - Alternatively, set `READ_AS_TEXT` for it.
  - Either way, correct R-1241, R-1243, `contracts.md` and `proofs.md`.
  - Add the column to T-0301, T-0303 and T-0304.

### EF3. Should fix, needs a design choice. R-1093, R-1080, T-0310 and R-1084: an escaped string literal hides a narrowing from SH's word search

- **Where:**
  - R-1093 searches `prosrc` for `performance_warnings`.
  - R-1080 searches for `disable:check`.
- **What's wrong:**
  - `plpgsql_check` reads a pragma from a string constant after Postgres's lexer has decoded it (`plch_apply_inline_pragmas` in `src/check_expr.c` reads the `A_Const`).
  - An `E'...'` or `U&'...'` literal can spell the pragma with escapes that `prosrc` doesn't hold as plain text.
  - The pragma takes effect, and neither search sees it.
  - A concatenation doesn't work: only constant arguments are read, so `sh_concat` was still reported.
- **Evidence:** 18.6 with `plpgsql_check` 2.10.16, on a function with `v int` reading a `bigint` column, checked with `all_warnings => true`. The two columns on the right show whether each search hits:

  | Line in the function | R-1080's finding | `prosrc ~* 'performance_warnings'` | `prosrc ~* 'disable:check'` |
  |---|---|---|---|
  | none | `performance 42804 cast "bigint" value to "integer" type` | no | no |
  | `PERFORM 'PRAGMA:DISABLE:PERFORMANCE_WARNINGS';` | gone | **yes** | no |
  | `PERFORM E'PRAGMA:DISABLE:PERFORMANCE\x5fWARNINGS';` | **gone** | no | no |
  | `PERFORM U&'PRAGMA:DISABLE:PERFORMANCE\005FWARNINGS';` | **gone** | no | no |
  | `PERFORM plpgsql_check_pragma(E'disable:performance\x5fwarnings');` | **gone** | no | no |
  | `PERFORM plpgsql_check_pragma('disable:performance' \|\| '_warnings');` | reported | no | no |
  | `PERFORM E'PRAGMA:DISABLE:\x43HECK';` in a block around the read | **gone** | no | no |

  The escaped functions run normally and return 5.
- **Options:**
  - **(a) Recommended.** Before searching, morm decodes every `E'...'` and `U&'...'` literal in `prosrc` by Postgres's rules (`\x`, octal, `\u` and `\U`, `U&`'s `\XXXX`, `\+XXXXXX` and `UESCAPE`), using the `morm-core` scanner that already finds strings, comments and dollar quotes. It then searches the decoded text as well as the raw text. T-0310 gains the four escaped forms above.
  - **(b) Simpler.** In a function R-1076 checks, refuse an `E'...'` literal with any escape outside a short allow-list (`\n`, `\t`, `\r`, `\b`, `\f`, `\\` and `\'`), and refuse any `U&'...'` literal. This stops `makemigrations` with exit code `2`, naming the function, the way R-1093 does.
  - **(c)** List the escaped forms in R-1084's README notice only.

### EF4. Should fix. T-0304 (step 15) with R-1238: the 1.22 and 1.26.1 legs don't run the failing configuration

- **Where:** T-0304 runs reads "through 1.20.1, and through 1.22 and 1.26.1 with `statement_cache = 0` and `shape_cache = 0`".
- **What's wrong:**
  - R-1238 fixes every 1.22 and 1.26.1 instance at `max_prepared_statements = 200`.
  - On those instances PgBouncer keeps the driver's own named lookup statements, so the lookup works with or without EA's cast. Review 1's table shows this: 1.22.0 with 200 gave 120 right out of 120.
  - morm's caches being off doesn't change that.
  - So those legs would pass with EA missing. Only the 1.20.1 leg tests the failure.
  - The case EE2 named, PgBouncer 1.21 or later with `max_prepared_statements = 0`, which DL3's caches-off mode supports, has no instance anywhere in the job.
- **Fix:**
  - Add an instance with `max_prepared_statements = 0` to R-1238, on 1.26.1 or 1.22, and run T-0304's second leg through it.
  - Or drop the 1.22 and 1.26.1 legs from T-0304 and say that 1.20.1 is the failing case.

  I recommend the first, so the supported configuration is covered.

### EF5. Should fix. R-1246, R-1116 and R-1117: a pool with `check_schema = "off"` doesn't read the marker at start

- **Where:**
  - R-1246: "A pool reads `<n>` when it starts, with one catalog query on its first connection, until step 41 folds that read into its startup statement".
  - R-1116: "in place of step 14's own catalog query".
- **What's wrong:**
  - Once step 41 lands, the read lives only in the startup statement.
  - R-1117 says that with `check_schema = "off"` the startup check doesn't run.
  - So such a pool never reads `<n>` at start. It prepares under whatever it defaults to, and shares stale server statements with other processes, which costs one failed call per statement before the retry moves it on.
  - The approved design, SD (a), said the pool reads `<n>` "in the startup statement once step 41 exists and in one catalog query before that or with `check_schema = "off"`". The write-in drops that last case.
- **Fix:**
  - R-1246 and R-1116 say that a pool whose startup check is off reads `<n>` with its own catalog query.
  - T-0306 starts one pool with `check_schema = "off"` after the swap, and asserts it prepares under the new marker with no stale error.

### EF6. Should fix. R-1248 and step 50: no README notice for the `_morm` ownership refusal, or for the reset's database-wide re-planning

- **Where:** R-1248 (step 33). There is no matching notice in step 50.
- **What's wrong:**
  - R-1248 adds a refusal. `migrate`, `rollback`, `rehearse` and `purge` stop with exit code `2` unless the migrating role owns `_morm`, or is a member of its owner, and has `CREATE` on it.
  - A deployment where `_morm` was created by another role, such as an admin role that set up the database, will be refused on its first migration after upgrading.
  - The reset is also a surprise outside morm: every `tx` step marks every cached plan in every session of the database invalid, other applications' sessions included. So each step costs every connected session one re-plan per cached statement.
  - `rules.md`'s README rule says every refusal and surprise gets a loud notice. Step 50 has neither. I searched it for `OWNER TO`, `_morm` and "reset".
- **Fix:** add a loud notice beside the migration commands. It names the refusal and `ALTER SCHEMA _morm OWNER TO <role>`, and says that each migration `tx` step makes every session on the database plan its cached statements again, once.

### EF7. Minor. R-1001 (step 44): `import` stops on a retired type's labels and advises renaming them

- **Where:** R-1001: "An enum label starting `_morm_` ... stops `import` with exit code `2`, naming the type and the label and saying to rename it with `ALTER TYPE ... RENAME VALUE`".
- **What's wrong:**
  - Between an online swap's cutover and `purge`, a morm-managed database holds a retired type whose labels all start `_morm_retired_`.
  - Step 44 doesn't say `import` skips retired types or columns. I searched it for "retired" and `_morm`.
  - So `import` on such a database stops and advises renaming those labels back. That would undo SA's relabel and make stale code silent again.
- **Fix:**
  - `import` leaves out types named `_morm_retired_*` and retired columns, as `verify` does under R-0586 and R-0995.
  - Or it stops naming `purge` as the fix.
  - It never advises renaming a retired type's labels.

### EF8. Minor. R-1062, T-0188, R-1023 and R-0995: what a rollback of an online enum swap does with a retired copy of a materialized view

- **Where:**
  - T-0188: "`rollback` restores [the retired copy] exactly".
  - R-1023: down "never renames the labels back".
  - R-0995: only "restoring a retired type to live, as a rollback of a retire does" renames them back.
- **What's wrong:**
  - When an online enum swap retires a copy of a materialized view that reads the enum column, the copy's column has the retired type.
  - If the swap's rollback restores the copy as it was and doesn't restore the type to live, the copy then:
    - shows `_morm_retired_<label>`;
    - fails `PgEnum` decoding with `UnknownEnumValue`;
    - has a column type `check_schema` won't match.
  - No text says which way the rollback goes, and neither T-0175 nor T-0188 rolls back an enum swap that has a materialized view above it.
- **Fix:**
  - State the rollback's path in R-1023 or R-1062: restore the type to live under R-0995, or rebuild the copy on the restored type.
  - Add that rollback to T-0175.

### EF9. Minor. Wording, references and evidence

- **(a) EE11's source.**
  - `proofs.md` says "the independent review's EE1 to EE11 (`reviews/step-55-review.md`)", and so does the PR body.
  - That file ends at EE10. EE11 is defined only in `reviews/step-55-open-items.md`.
  - Fix: cite both files.
- **(b) R-1083 still names "T-0172's job".**
  - That job is now step 12's `standby` job (R-1244).
  - R-1083's YAML reads `job.services.postgres.id`, and R-1244 doesn't say the primary is a service.
  - Fix: name the `standby` job, and say how the extension reaches its primary.
- **(c) Where the evidence ran.**
  - The step's end-to-end check expects results from spikes run in `spikes.yml`, with linked logs.
  - The evidence for SA to SH ran locally in the design session, on source builds, with no CI log.
  - `proofs.md` says so plainly, but INDEX's list of the fix's choices for the owner doesn't include it.
  - Fix: add it to that list, or bring the spikes back to run the SA, SD and SH tables in `spikes.yml`.
  - I reproduced SA's table, the reset and `ALTER` behind PgBouncer, SB's `ALTER` and SH's pragma table, on 18.6 and 17.7 where they apply.
- **(d) PR body choices 1 and 3.** Their claims need correcting, per EF2 and EF1.

## What I couldn't run

- **Postgres 15 and 16:** I had no builds of either. My spot checks ran on 18.6 and 17.7 only.
- **PgBouncer 1.20.1 and 1.26.1:** I built only 1.22.1, and I ran it with one server connection, so each check was deterministic. I didn't repeat the design's runs with three connections.
- **A hot standby:** I didn't build one, so SG and the reset's replay are taken from the design and the first review.
- **German messages:** neither build has translations, so I didn't check SC's German rows.
- **`plpgsql_check` on 15 to 17:**
  - 2.10.16 doesn't compile against 17.7's headers: `pg_mblen_cstr` is undeclared in `src/format.c`. It needs a later minor release, as PGDG's 17.12 package presumably has.
  - So EF3's table is from 18.6 only. I loaded the extension from my scratch tree through `extension_control_path` and `dynamic_library_path`.
- **morm itself:** there is no runtime yet, so EF1 is shown from the rules and from Postgres's behaviour, not from a pool.

## Verdict

**Not approved yet.** The first review's findings are fixed in substance, and the owner's answers are written in faithfully. I checked:

- that every changed ID is in INDEX's Decisions entry;
- that the checker reports 0 problems;
- that CI is green;
- that SA's table, the reset behind PgBouncer and SB's no-op `ALTER`s reproduce on 18.6 and 17.7.

What remains:

- **EF1 blocks approval:** under the approved `22P02` narrowing, a stale session's retry rarely confirms, so the recovery that R-1040, R-1249, R-1250 and T-0152 promise doesn't happen by itself, and a client can still make morm replace one connection per request. It needs an owner decision.
- **Should fix:** EF2, a list of a domain over a built-in type isn't covered by EA; EF3, escaped pragmas bypass SH and need a design choice; EF4, T-0304's legs on 1.22 and 1.26.1 don't test the failure; EF5, the marker isn't read with `check_schema = "off"`; EF6, missing README notices.
- **Minor:** EF7 to EF9.
