# Step 55 independent review, third round

PR #3 (`step-55-enum-proofs`), head `9caf8a2`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or either fix. This round covers the fix for the second review's EF1 to EF9: `git diff 1e8e9f8 origin/step-55-enum-proofs`, plus the spikes as they stood at `b682836`.

## What I read

- **On the PR branch:** `rules.md`, `contracts.md` (`SqlType`, `Param`, the binding comment), `proofs.md` (step 55's section), and every changed requirement and test in context: steps 12, 14, 15, 20, 38, 41, 44, 45, 50, 52, 56 and 59. For the fix's claims I also read R-0571, R-0586, R-0650, R-0653, R-0975, R-0995, R-0996, R-1023, R-1032, R-1063, R-1191, R-1213, R-1233 and R-1248.
- **On `origin/main`:** INDEX's "Stale-statement design, SA to SH" and "Step 55 second review, EF1 to EF9" entries, `reviews/step-55-review-2.md` and `reviews/stale-statement-design.md`.
- **At `b682836`:** `spikes/src/bin/p19.rs`, the new `pragmas` section of `p18.rs`, `start_pgbouncers.sh` and `spikes.yml`.
- **The PR body.**

## What I checked and ran

- **CI on the head.** `9caf8a2` is green: `ci` on push (run 43474080814) and on the pull request (run 43474091778), every job, including `pooler` and integration on 15 to 18 over TCP and Unix sockets. The diff since `1e8e9f8` touches only `plans/plan-approved/` (16 files). No code, test or workflow changed, and `spikes/` and `spikes.yml` are gone again.
- **Coverage checker,** on an export of the head:
  - `check_coverage.py --step 55 --code crates tests benches fuzz .github`: **1255 requirements, 313 test obligations, 0 problems**;
  - the plain form: the same.
- **Manifest and INDEX.** Comparing `1e8e9f8`'s manifest with the head's ID by ID gives 26 changed IDs, 4 new and none removed:
  - changed: R-0981, R-0982, R-0984, R-1001, R-1040, R-1050, R-1062, R-1080, R-1083, R-1093, R-1116, R-1238, R-1241, R-1243, R-1244, R-1245, R-1246, R-1249, R-1250, T-0152, T-0153, T-0175, T-0301, T-0303, T-0304 and T-0310;
  - new: R-1255, T-0311, T-0312 and T-0313.

  That is exactly the INDEX entry's "written in at `9caf8a2`" list, and the PR body's list since `1e8e9f8` matches both.
- **The spikes' CI runs.**
  - Run 43472484941 (push at `4ae3a1e`) and run 43473420754 (push at `b682836`) are green on all four jobs.
  - I read every job log of 43473420754. Postgres was 15.20, 16.16, 17.12 and 18.7, and `plpgsql_check` was `postgresql-NN-plpgsql-check 2.10.16-1.pgdg13+2`.
  - Each log has "every check passed" for proofs 13 to 18 and for `p19 (direct)`, `p19 (bouncer)` twice (ports 6432 and 6433) and `p19 (standby)`.
  - The `22P02` rows name `<schema>._morm_retired_m2_st` with routine `enum_in`, and `ALTER PROCEDURE ... COST 100` gives `42P13 ... [compute_common_attribute]`.
- **What `p19` and proof 18's new rows prove.** `p19` checks every cell of `proofs.md`'s table, under all four variants:
  - a cell asserts its exact token, which for a `22P02` means the message holds `_morm_retired_`;
  - the routine is asserted for the `42804` row;
  - the retired column reads `_morm_retired_a` and `_morm_retired_c`;
  - the swap role isn't superuser;
  - each no-op `ALTER` writes a new `pg_proc` row (by `xmin` and `ctid`), and the old session then gets `false`.

  The PgBouncers have one server connection and `max_prepared_statements = 200`, as `proofs.md` says. The `pragmas` section asserts, for every row, that the narrowing is hidden or reported as the table says, that the function runs and returns `1`, and that a word search of `prosrc` misses each escaped row. So the claims in "Re-run in CI for the second review" hold.

  What `p19` doesn't show is anything about EF1's new mechanism, that a newly opened connection confirms a stale session while a client's text fails again. That's Postgres's ordinary behaviour, and `proofs.md` doesn't claim `p19` shows it. I checked the client cases myself (EG1, EG2).
- **Postgres spot checks.** These ran on a throwaway 18.6 server under `.claude/scratch/` in my worktree, with `TMPDIR` set there. I also read `plpgsql_check` 2.10.16's `src/check_function.c` at its `v2.10.16` tag.

  | Claim | Result |
  |---|---|
  | A `DECLARE v st` function given `lower($1)`, with the client sending `_MORM_RETIRED_X` | `22P02 invalid input value for enum st: "_morm_retired_x"`, routine `enum_in`. The parameter held no `_morm_retired_` (EG2) |
  | The same with `p->>'s'` from a `jsonb` holding `"_morm_retired_x"` | The same `22P02`, and the parameter's text holds `_`, not `_` (EG2) |
  | A function reading a stored `_morm_retired_x` from a table into `v st` | The same `22P02`, from a call with no parameter that holds it (EG1) |
  | `ALTER TABLE ... ALTER COLUMN s TYPE st2` while a materialized view reads `s` | `cannot alter type of a column used by a view or rule`, `rule _RETURN on materialized view mv depends on column "s"` (EG6) |
  | A function with `SET standard_conforming_strings = off` that returns `'PRAGMA:DISABLE:\x43HECK'` | Returns `PRAGMA:DISABLE:CHECK`, while `prosrc ~* 'disable:check'` is false. `plpgsql_check`'s `load_configuration()` applies the function's `proconfig` with `ProcessGUCArray` before it checks (EG5) |

## EF1 to EF9: are they fixed?

| Finding | Verdict and evidence |
|---|---|
| EF1, the stale `22P02` | **Fixed outside a transaction, as the owner decided.** R-0981 tests the parameters: with no `_morm_retired_` in them, morm retries on a newly opened connection, and a success discards the first connection and raises the generation; with it, morm returns at once. R-0982, R-1245, T-0152, T-0153 and the new T-0311 match. **Inside a transaction it isn't** (EG1). The parameter test can be got round (EG2), and what happens to the new connection is undefined (EG3) |
| EF2, a list of a domain over a built-in type | **Fixed in the text:** `READ_AS` with `"int4[]"` (R-1241, T-0301, T-0303, T-0304, `proofs.md`). But the contract can't express it for a generic `Vec<T>` (EG4) |
| EF3, escaped pragmas | **Fixed as the owner decided:** `E''` and `U&''` are decoded (R-1093, R-1080, T-0310, proof 18's rows, green in CI). One more decoding path is open (EG5) |
| EF4, T-0304's legs | **Fixed:** a third 1.26.1 instance with `max_prepared_statements = 0` (R-1238), which T-0304 runs through with 1.20.1. One sentence of R-1238 now contradicts it (EG8) |
| EF5, `check_schema = "off"` | **Fixed:** R-1246 and R-1116 keep the catalog query, and T-0313 tests both settings in the `pooler` job |
| EF6, README notices | **Fixed:** R-1255 names the refusal, `ALTER SCHEMA _morm OWNER TO`, and the database-wide re-planning, and matches R-1248's preflight list |
| EF7, `import` and retired types | **Fixed:** R-1001 leaves out `_morm_retired_` types and retired columns and never advises renaming their labels, and R-1249's notice says so |
| EF8, a retired copy of a materialized view on rollback | **Text written, with the owner.** The direction is right, but the mechanics, the contents decision and the ledger are missing or contradictory (EG6, EG7) |
| EF9 (a) | **Fixed.** Both review files are cited in `proofs.md` and the PR body |
| EF9 (b) | **Fixed.** R-1083 and R-1244 name the `standby` job and the primary as its `postgres` service. One wording slip remains (EG8) |
| EF9 (c) | **Fixed.** The evidence ran in `spikes.yml` on 15 to 18, with linked runs, and `b682836` is named as the last commit with the spikes |
| EF9 (d) | **Fixed.** Choices 1 and 3 in the PR body are corrected |

Nothing earlier regressed. Only `plans/plan-approved/` changed, and the checker, CI and the ID list agree.

## Findings

### EG1. Blocks approval. A small design choice. R-0984, T-0312, R-1245, R-1040, R-1249 and R-1250: inside a transaction, an unconfirmed `22P02` raises the connection generation, so a stored or transformed `_morm_retired_` text replaces the whole pool on every call

- **Where:** R-0984 (step 20) says that for a `22P02` from `enum_in` holding `_morm_retired_`, "when no bound parameter's text holds `_morm_retired_`, morm returns `Error::SchemaChanged`, and once the transaction has rolled back it discards that connection and, straight to Postgres, raises the connection generation".
- **What's wrong:**
  - Inside a transaction nothing is retried, so nothing is confirmed. Yet the generation is raised, and under R-0975 every connection in the pool is then replaced at checkout.
  - R-0981 names a case where the parameter test passes but the session isn't stale: "a function converts text it reads from a table that holds `_morm_retired_`". EG2 adds a client's text transformed inside the function.
  - Outside a transaction, R-0981 limits that case to "one new connection per call", and raises nothing. Inside one, the same call replaces the whole pool every time.
  - Anyone who can store a `_morm_retired_x` string in a column that such a function later converts can do this with an ordinary write.
- **What it contradicts:**
  - SF, an owner decision: the generation is raised only for a confirmed stale statement.
  - The owner's EF1 decision, "so a client can't churn the pool".
  - R-1040 and R-1250, which say that case "costs one new connection per call".
  - R-1249, which tells users "the call is retried once on a newly opened connection". Inside a transaction there is no retry: the app gets `Error::SchemaChanged`, and recovers only through `.retry(n)`.
- **Evidence:**
  - On 18.6, a function that reads a stored `_morm_retired_x` into `v st` gives `22P02 invalid input value for enum st: "_morm_retired_x"` from `enum_in`, from a call with no parameter holding it.
  - The rest is the text of R-0984, R-0975, R-0981 and R-1245.
  - T-0312 runs only the true stale case and the parameter case, not this one.
- **Options:**
  - **(a) Recommended.** Inside a transaction, after the rollback, discard only that connection and raise nothing. Mark the pool so that `.retry(n)`'s next attempt runs on a newly opened connection, as R-0981's retry does. If that attempt gets past the call that failed, raise the generation then. Without `.retry(n)`, nothing is raised, and the next call outside a transaction confirms through R-0981.
  - **(b)** Never raise the generation from inside a transaction for a `22P02`. Discard that connection only, and let R-0981 confirm later.
- **Either way:**
  - T-0312 adds the stored-text case inside `.retry(2).transaction(..)`, and asserts that at most one connection is discarded and no generation is raised.
  - R-1249's notice says that inside a transaction the app gets `Error::SchemaChanged`, which `.retry(n)` runs again on a fresh connection.

### EG2. Should fix. Needs a design choice. R-0981, R-1245, R-1249 and T-0153: the parameter test is literal, so a client's text transformed inside the function gets past it

- **Where:** R-0981: "when a bound parameter's text holds `_morm_retired_`, the client sent it".
- **What's wrong:**
  - The test reads the bytes morm sent, in one case, as the literal `_morm_retired_`. But the `22P02` signal is the server's message, which holds whatever value reached `enum_in`.
  - Some ordinary functions build that value from what the client sent: `v := lower(p)`, `(p->>'status')::st` on raw JSON, `trim`, `replace` and so on.
  - So a client sending `_MORM_RETIRED_X`, or a JSON body with `"_morm_retired_x"` passed as a `String` to a `jsonb` parameter (R-1213 lets `String` carry `jsonb`), triggers the signal while no parameter holds the literal text.
  - morm then opens a new connection on every such request, and inside a transaction it replaces the whole pool (EG1).
  - That is the client-driven churn the owner's EF1 decision set out to stop. R-1249's "A label you send that holds `_morm_retired_` gets its `22P02` at once" is true only for the literal form.
- **Evidence:** on 18.6, `f_low('_MORM_RETIRED_X')` and `f_js('{"s":"_morm_retired_x"}')` each gave `22P02 invalid input value for enum st: "_morm_retired_x"`, routine `enum_in`.
- **Options:**
  - **(a) Recommended. Cap the cost rather than chase every transformation.**
    - When a confirmation retry on a new connection fails, the pool remembers that statement's text.
    - Until the pool's connection generation next moves, or for a fixed time such as 60 seconds, a `_morm_retired_` `22P02` from that statement returns at once.
    - A real stale session always confirms on its first new connection, because that connection compiles the function fresh. So the memory only delays recovery for a statement that a client was abusing when a cutover happened.
    - R-1249 and R-1250 say so, and T-0311 asserts that a second such call opens no connection.
  - **(b)** Make the test match in any case, and decode `\u` escapes in `json` and `jsonb` text parameters. This narrows the gap but doesn't close it (`replace`, concatenation of two parameters).
  - **(c)** Keep the rule, and say in R-1249 and R-1250 that text the function transforms before the enum costs one new connection per call.

### EG3. Should fix. R-0981 and T-0311: what happens to the newly opened connection, and the pool's size, is undefined when the retry fails

- **Where:**
  - R-0981: "If that retry fails too ... the error is returned, the new connection joins the pool, and nothing is raised, so that case costs one new connection per call".
  - T-0311: "with no generation raised and the first connection kept".
- **What's wrong:**
  - If the first connection is kept and the new one joins the pool, the pool grows by one on every such call, past its `max` and past R-0650's shared `max_connections` budget, which morm otherwise enforces.
  - Even on success, the new connection is open beside the checked-out first one for a moment, so a pool at `max` holds one more than its budget allows.
  - The text doesn't say whether that open goes through the pool's accounting, or what happens when it fails with "too many clients". R-0653's backoff is for the pool's own opens.
- **Fix:**
  - Say that the new connection is opened outside the pool's slots, but counted against the process's budget.
  - On success it takes the first connection's slot, and the first is closed.
  - On failure it is closed, or it replaces the first; either way the pool's size doesn't change.
  - If it can't be opened, the original `22P02` is returned and nothing is raised.
  - T-0311 asserts that the pool's size, from `pool_stats()`, is the same before and after each branch.

### EG4. Should fix. `contracts.md` (`SqlType::READ_AS`), R-1241 and R-1213: a generic `Vec<T>` can't build its `READ_AS` from `T`'s

- **Where:** `contracts.md` replaces `const READ_AS_TEXT: bool` with `const READ_AS: &'static str`. It is `"text[]"` for a list of an enum or composite, `"int4[]"` for a list of a `PgDomain` over `integer`, and `""` for a scalar domain over a built-in type and for `Vec<i32>`.
- **What's wrong:**
  - R-1213 makes `Vec<T>` and `&[T]` take their constants from `T`. That's why `SqlType` has `CAST_LIST` and `LIST_CARRIER` beside `CAST` and `CARRIER`: stable Rust can't build a `&'static str` in a generic impl by joining `T::READ_AS` with `"[]"`. Neither `concat!` nor `const_format` works over a generic parameter's associated constant.
  - With the boolean, `Vec<T>` could copy `T::READ_AS_TEXT`. With a string, `Vec<PgDomain over integer>` needs `"int4[]"` while `T::READ_AS` is `""`, and `Vec<i32>` needs `""` while `T::READ_AS` is also `""`. So the list's value can't be derived from `T::READ_AS` at all.
  - Step 03 freezes `contracts.md`, so this has to be settled now.
- **Fix:**
  - Add `const READ_AS_LIST: &'static str;` beside `CAST_LIST`. It is `"text[]"` for `PgEnum`, `PgComposite` and a `PgDomain` over either, `T::CAST_LIST` (such as `"int4[]"`) for a `PgDomain` over a built-in `T`, `"text[]"` for a text-form field on a type morm doesn't know, and `""` for every other type.
  - `Vec<T>` and `&[T]` take `T::READ_AS_LIST` as their `READ_AS`.
  - Say so in R-1241 and R-1213, and in the comment in `contracts.md`.
  - T-0301 already checks the resulting column lists.

### EG5. Should fix. Needs a design choice. R-1093, R-1080 and T-0310: a plain literal is decoded too when `standard_conforming_strings` is off

- **Where:** R-1093 decodes "every `E'...'` and `U&'...'` literal in `prosrc`" before the searches for `@plpgsql_check_options`, `performance_warnings` and `disable:check`.
- **What's wrong:**
  - With `standard_conforming_strings = off`, Postgres processes backslash escapes in a plain `'...'` literal too.
  - That setting can come from the function's own `SET` clause. `plpgsql_check`'s `load_configuration()` applies the function's `proconfig` (`ProcessGUCArray`) before it checks, so the check parses the body with that setting.
  - It can also come from a database or role default in the session morm checks from.
  - So `PERFORM 'PRAGMA:DISABLE:\x43HECK';`, with no `E`, in a function declared `SET standard_conforming_strings = off`, reaches `plpgsql_check` as `PRAGMA:DISABLE:CHECK`, while neither search sees it.
  - This is the same kind of gap the owner chose to close in EF3.
- **Evidence:**
  - On 18.6, a function with that `SET` clause returning `'PRAGMA:DISABLE:\x43HECK'` returned `PRAGMA:DISABLE:CHECK`, and `prosrc ~* 'disable:check'` was false.
  - `check_function.c` at `v2.10.16`, `load_configuration()`, lines 1335 to 1371.
  - I couldn't build the extension here, so the hidden narrowing itself is inferred from that code, not run.
- **Options:**
  - **(a) Recommended.** morm's check session sets `standard_conforming_strings = on`. When a checked function's `proconfig` sets it `off`, R-1093's scanner decodes that function's plain literals as `E` strings too, and searches the result. T-0310 gains that case.
  - **(b)** Refuse, with exit code `2` and naming the function, a function R-1076 checks whose `proconfig` sets `standard_conforming_strings = off`. This is simpler, and the setting has been deprecated in practice since 9.1.
- **Either way:** the scanner reads the `E` and `U&` prefixes in any case (`e'...'`, `u&'...'`), and T-0310 adds one lowercase-prefix row.

### EG6. Should fix, with the owner's EF8 decision. R-1050, R-1062 and T-0175: the rebuild's mechanics contradict the one-transaction down, and the contents decision and ledger are missing

The direction is sound, and consistent with SA:

- The retired copy of the materialized view reads the *retired column*, not the live one.
- An online enum swap's down maps placeholders back in its own expression on the live column (R-1023), and never renames the retired column back.
- So moving the old copy back, as step 38 does for a column swap, would leave a live view bound to the retired column whatever happened to the type.
- Building again on the restored type renames no label back, so SA's rule holds.

What's wrong is the rest:

- **(a) The mechanics.**
  - R-1050 says the rollback "builds the copy again beside the new one ... on the type the rollback restores ... and swaps it in by rename in the inverse's short transaction, as the swap built its own".
  - But R-0996 says "`rollback` uses `down.sql`, which is always the one-transaction form", and step 52 repeats it.
  - The restored type, and the column on it, exist only inside that transaction, so nothing can be built beside the view on them beforehand.
  - Inside it, Postgres refuses the `ALTER COLUMN ... TYPE` while the live copy reads the column. On 18.6 it gave `cannot alter type of a column used by a view or rule`.
  - In the one-transaction form, R-1032 already drops a managed materialized view before the rewrite and recreates it after, with a refresh, under the lock.
  - **Fix.** R-1050 says the down's one-transaction form recreates the view on the restored type under R-1032, `WITH NO DATA` if the retired copy was unpopulated, and doesn't move the retired copy back. Drop "beside the new one" and "short transaction".
- **(b) The contents decision.**
  - R-1063 opens a decision on the way up, because a rebuilt copy's contents "will come from today's table" and a view "may be frozen on purpose".
  - The rollback's rebuild replaces a frozen copy's rows the same way, and opens nothing. The PR body admits it: "the copy the rollback leaves is freshly populated".
  - **Fix.** `rollback` preflight opens the same decision, naming the view and saying the retired copy keeps the old rows until `purge`. Step 50 gets a line in a loud notice. T-0175 asserts the decision, and that an unpopulated copy stays unpopulated.
- **(c) The ledger, `purge` and running the migration again.**
  - After the rollback, the retired copy stays "until `purge`". It reads the retired column, so the retired column and the retired type `_morm_retired_<migration>_<type>` must stay too: Postgres refuses `DROP COLUMN` under it, as step 39's evidence records.
  - All three are named after a migration that is now rolled back. Nothing says which ledger record `purge` then finds them through.
  - Nothing says what a second `migrate` of the same migration does when it tries to retire the type, the column and the view under the same names.
  - T-0189's "up, down and up" pattern exists for the column swap, but T-0175 runs only the rollback.
  - **Fix.** State where the ledger keeps these objects after a rollback, and how `purge` reaches them. Also state how a re-run names its retired objects, for example with a run suffix through the 63-byte rule, or by refusing until `purge`. T-0175 runs up, rollback and up again with a materialized view above the column, then `purge`.

### EG7. Minor. EF8's wording: R-1062's sentence, the stated reason, and its attribution

- **R-1062's sentence.** The inserted clause reads "... exactly as it was, apart from an online enum swap, whose retired copy holds a retired type and which step 52 rebuilds instead: the same rows, unpopulated if it was, and with no refresh". The colon now attaches "the same rows ... no refresh" to the enum rebuild, the opposite of R-1050. Move the exception after "and with no refresh", as a sentence of its own.
- **The reason in the PR body.** Choice 8 says restoring the type to live "would rename its labels back, which SA forbids while stale code can reach it". R-0995 says the opposite: "Restoring a retired type to live ... renames the type and its labels back, which is right, because it's live again with the same OID". The sound reason is the one in EG6: the copy reads the retired column, which the down doesn't bring back. Correct the reason before the owner decides.
- **The attribution.** R-1050 ends "Owner decisions SA and EF8", and `proofs.md` lists EF8's path among the fixes, while INDEX says the path "is with the owner". Until the owner approves it, say "a choice of this fix the owner approves through PR #3".

### EG8. Minor. Other wording, README notices and records

- **R-1238.** The new bullet adds a third 1.26.1 instance with `max_prepared_statements = 0`, but a later bullet still says "The 1.22 and 1.26.1 instances set `max_prepared_statements = 200` explicitly". Add "apart from the third 1.26.1 instance".
- **R-1083.** It installs the package "into its primary ... from which the standby replays it". Replay carries `CREATE EXTENSION`'s catalog rows, not the package's files. The standby needs no files unless a test calls the extension there, and none does. Say "into its primary, where `makemigrations` runs".
- **R-1243.** It says "an array of any domain ... is [reported], with the base array's cast as the fix, such as `::int4[]`". For a domain over an enum, the base array is an enum array, which T5 reports too. The fix there is `::text[]`, as R-1241 gives it.
- **README notices** (`rules.md`'s rule for every refusal and surprise):
  - R-1254 tells users to cast enum and composite columns in raw SQL, but not a column holding an array of a domain, which T5 now reports, with its `::int4[]` fix.
  - R-1084 says `performance_warnings` "anywhere in ... source" stops `makemigrations`. It should add that a spelling with escapes in an `E''` or `U&''` string is refused too, since that is new with EF3.
- **`proofs.md`.** The SD, SE and SF paragraph ends with two parentheticals in a row: "... (EF5: R-1246, R-1116, T-0313). (R-1246, R-0718, ...)". Merge them.
- **The PR body's README line.** It lists R-1249 to R-1254 and R-1084, but not R-1255, which the EF6 bullet names.

## What I couldn't run

- **`plpgsql_check`.** Building it here was refused, so EG5's hidden narrowing is inferred from the extension's source, which applies `proconfig` before checking, and from Postgres decoding the literal. It wasn't run end to end.
- **Postgres 15 to 17, PgBouncer and a standby.** I had none locally. My spot checks ran on 18.6 only. The rows on 15 to 18, through PgBouncer and on a standby, are taken from the CI logs I read.
- **morm itself.** There is no runtime yet, so EG1 to EG3 are shown from the rules and from Postgres's behaviour.

## Verdict

**Not approved yet.** The fix is faithful in most places:

- every changed ID is in INDEX's EF entry;
- the checker reports 0 problems;
- CI is green on the head, and the spikes' runs on 15 to 18 show what `proofs.md` says;
- EF2 to EF7 and EF9 are fixed;
- EF1 is fixed outside a transaction.

What remains:

- **Blocks approval:** EG1. Inside a transaction an unconfirmed `22P02` raises the connection generation, so stored or transformed `_morm_retired_` text replaces the whole pool on every call. This contradicts SF and EF1's goal, and it needs a small design choice.
- **Should fix:**
  - EG2, the literal parameter test can be got round (a design choice);
  - EG3, the new connection's slot and the pool's size;
  - EG4, `READ_AS` needs a list counterpart in `contracts.md`;
  - EG5, plain literals under `standard_conforming_strings = off` (a design choice);
  - EG6, EF8's mechanics, contents decision and ledger, for the owner's EF8 decision.
- **Minor:** EG7 and EG8.
