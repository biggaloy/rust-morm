# Step 55 independent review, fifth round

PR #3 (`step-55-enum-proofs`), head `0501425`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes. This round covers the fix for the fourth review's EH1 to EH8, `git diff baf82f0 origin/step-55-enum-proofs`, and checks that nothing earlier regressed.

## What I read

- **On the PR branch:** `rules.md`, `contracts.md`, `proofs.md` (step 55's section, including the new "fourth review" bullets), and every changed requirement and test in context: R-0586, R-0981, R-0984, R-0995, R-1040, R-1050, R-1062, R-1080, R-1084, R-1093, R-1249, R-1250, T-0175, T-0310, T-0311, T-0312 and T-0314. For the fix's claims I also read R-0120, R-0583, R-0618, R-0634 to R-0640, R-0650, R-0996, R-1014, R-1101, R-1221 and R-1223.
- **On `origin/main`:** INDEX's Decisions entries from "Stale-statement design, SA to SH" through "Step 55 fourth review, EH1 to EH8", with its IDs and the fix's own choices, and `reviews/step-55-review-4.md`.
- **The PR body.**

## What I checked and ran

- **CI on the head.** `0501425` is green: `ci` on push (run 43565219669) and on the pull request (run 43565225927), every job, including `coverage`, `pooler`, `msrv`, `features` and integration on 15 to 18 over TCP and Unix sockets. The diff since `baf82f0` touches only `plans/plan-approved/` (9 files). No code, test or workflow changed.
- **Coverage checker,** on the head:
  - `check_coverage.py --step 55 --code crates tests benches fuzz .github`: **1255 requirements, 314 test obligations, 0 problems**;
  - the plain form: the same.
- **Manifest and INDEX.** Comparing `baf82f0`'s manifest with the head's, ID by ID, gives 17 changed IDs, none new and none removed: R-0586, R-0981, R-0984, R-0995, R-1040, R-1050, R-1062, R-1080, R-1084, R-1093, R-1249, R-1250, T-0175, T-0310, T-0311, T-0312 and T-0314. That is exactly the INDEX EH entry's "written in at `0501425`" list, and the PR body's list since `baf82f0` matches both. Against `origin/main`, all 66 changed IDs are covered by INDEX's step 55 entries. `frame` and `master_sha256` are unchanged.
- **Postgres spot checks.** These ran on throwaway 17.11 and 18.6 servers. The data was under `.claude/scratch/` in my worktree, with `TMPDIR` set there, and the sockets were in `~/.claude/scratch/`. Both versions printed the same.

  | Claim | Result |
  |---|---|
  | EH3: a migration that removes label `c`, mapping it to `b` and keeping old values, with `up.sql` and `down.sql` held in a table and their SHA-256 taken first. It runs up and is rolled back. Then row 1 changes `c` to `a`, row 2 `a` to `c`, and row 4 is inserted as `c` | The first run and its rollback restore `c`, `a`, `c` |
  | EH3: the second run as written | `column "_morm_retired_m2_s" of relation "t" already exists` |
  | EH3: the second run with its names mapped to `_7` | Runs. `t` then holds both `_morm_retired_m2_s` (`_morm_retired_m2_st`) and `_morm_retired_m2_s_7` (`_morm_retired_m2_st_7`) |
  | EH3: the second rollback as written, in a transaction that was then rolled back | No error, but it leaves row 2 and row 4 as `b` where `c` was expected, because it read the first run's column (row 2 holds `_morm_retired_a` there, row 4 `NULL`) |
  | EH3: the second rollback with its names mapped | Restores `a`, `c`, `c`, `c`, each row as it was before the second run. Both stored files still match their checksums |
  | EH7: a function with no `SET` clause, under a role default of `off`, in a session that sets `standard_conforming_strings = on` first | Returns `PRAGMA:DISABLE:\x43HECK`, not decoded. The same body with `SET standard_conforming_strings = off` returns `PRAGMA:DISABLE:CHECK` |

  So `proofs.md`'s EH3 paragraph holds, as does EH7's premise. One caveat: my mapping function, like the fix's, used `regexp_replace` on whole words. That also rewrote a quoted `"_morm_retired_m2_st"` and the name inside a `--` comment. So the run shows that mapping works, but not that the rule as written maps the text morm generates (EI2).

## EH1 to EH8: are they fixed?

| Finding | Verdict and evidence |
|---|---|
| EH1 | **Fixed for one cutover, as the owner decided.** R-0984 records the statement's hash and the time of its first unconfirmed `22P02` in a transaction. A later failure on a connection opened after that time is a failed confirmation: the statement enters R-0981's memory, the error returns as it is, and nothing is discarded. Behind a pooler a transaction discards nothing. T-0312 adds the five-call case, the remembered statement and the pooler case, and R-1040, R-1249 and R-1250 state the cost. **But the record never expires,** so a later cutover's real stale session counts as a failed confirmation, and inside transactions it never recovers (EI1) |
| EH2 | **Fixed, as recommended.** R-0984 confirms only with the same text and the same parameter bytes, compared by a hash of both. Other parameters, or no run of the statement, confirm nothing, and the new connection stays as an ordinary pooled one. T-0312 adds the closure that binds another value and the closure that doesn't call the function. A failure on the fresh connection with other parameters counts as a failed confirmation, which is right, because a fresh connection compiled the function. The definition of a failure is still inconsistent between R-0981 and R-0984 (EI4) |
| EH3 | **Fixed in substance, as the owner decided.** The files name retired objects unsuffixed and stay as checksummed. The ledger records each run's actual names. `migrate` and `rollback` map them in the text they send, and T-0175 rolls back a second run with rows changed and inserted between the runs. My run confirms the Postgres behaviour. **Three gaps remain.** The rule names `morm-core`'s scanner, which skips quoted identifiers, yet generated SQL always quotes names (EI2). The mapping covers only that migration's own files in `migrate` and `rollback`, not `revert` or `rehearse` (EI3) |
| EH4 | **Fixed.** R-1050, R-0586 and R-1062 make a rollback's recorded objects retired for `verify`, `check_schema`, the walk, R-1062's refusal and `purge`. T-0175 asserts `verify` and the refusal, but not `check_schema` or the walk (EI8) |
| EH5 | **Mostly fixed.** `max` bounds the slots, and the confirming connection is a momentary extra within the budget. In the full-budget branch a failed open leaves the slot to the pool. A failed open fills the memory, and in a transaction the attempt's connection takes the discarded one's slot. T-0311 adds the refused open called twice. **But:** the claim that the memory only delays a statement fed such text is now false, and the README doesn't say so (EI5). The failed-retry branch lost its return value (EI4). T-0312's "in every case the pool's size is unchanged" contradicts its own discards and R-0638 (EI6). A failed open in a transaction is unspecified (EI7) |
| EH6 | **Fixed, as recommended.** T-0314 is in `morm-tests` and uses `morm`'s `#[doc(hidden)]` `test-hooks` feature, and `repo_policy.rs` checks that no other manifest enables it. R-0981 names the feature |
| EH7 | **Fixed, as recommended.** R-1093 decodes plain literals only under the function's own `SET`. R-1080 and R-1084 say "a function declared `SET standard_conforming_strings = off`". T-0310's role-default row now expects the narrowing reported, not refused. Confirmed on 17.11 and 18.6 above. Attribution: EI9 |
| EH8 | **Fixed.** R-1249 says the rollback's decision refuses by default and stops with exit code `3` until `rollback --decide ID=OPT`. It also says a statement fed such text at a cutover can see its `22P02` for up to 60 seconds. R-0981 and R-1050 name this fix's choices, and the PR body names `READ_AS_LIST` |

Nothing earlier regressed outside the points below. Only `plan-approved/` changed, and the checker, CI and the ID list agree. SA to SH, EF1, the `22P02` narrowing, SF and EG1 to EG3 still read consistently with the new text, apart from EI1, EI4 and EI5.

## Findings

### EI1. Blocks approval. R-0984, R-0981, T-0312, R-1249: the transaction's first-failure record never expires, so after a later cutover a real stale session counts as a failed confirmation and, inside transactions, never recovers

- **Where:** R-0984: "Otherwise morm returns `Error::SchemaChanged` and records, in step 14's pool, the statement's hash and the time of this first failure ... If the same statement later fails the same way in a transaction on a connection opened after that time ... it counts as a failed confirmation: the statement enters step 14's memory, the error returns as it is, and nothing is discarded or raised. A real stale session never fails on a connection opened after the cutover, so this changes nothing for it."
- **What's wrong:**
  - The record has no lifetime, no bound and no clearing rule. R-0981's memory has all three: 60 seconds, the connection generation, and 64 entries. R-0984 gives the record none of them, and nothing else in the plan does (`grep "first failure"` finds only R-0984, R-0981's pointer to it, and T-0312).
  - The safety argument compares a connection's age with the time of the **first** failure, not with the **latest** cutover. It holds only while those are the same cutover.
  - Take a process with a pool of four, and a function `f` with `DECLARE v st` called in `.retry(2).transaction(..)`:
    - A first swap cuts over at 10:00. `f` first fails at 10:00:01, which is recorded. The connection is discarded, and the next attempt confirms on a new connection, so the generation is raised and the pool is replaced at checkout. Every connection is now opened after 10:00:01.
    - A second swap cuts over at 10:05: another swap of `st`, or of another enum `f` reads, perhaps in the same `migrate` run. The pooled connections compiled `f` before 10:05, so they are really stale.
    - The next call fails on a connection opened at 10:00:02, after the record. So it is treated as a failed confirmation. The `22P02` is returned as it is, not as `SchemaChanged`, so `.retry(n)` doesn't run again. Nothing is discarded or raised, and the statement enters the memory for 60 seconds.
    - When the memory expires, the next failure is again on a connection opened after 10:00:01, so it is again a failed confirmation. The stale connections are never discarded. `max_lifetime` is off by default (R-0637), and a busy pool never idles (R-0634). So a statement used only in transactions fails until the process restarts.
  - That breaks EG1's owner decision that `.retry(n)` recovers a stale session, and SF's confirmation. It also makes R-1249's "morm apps recover from it by themselves" and R-0984's "this changes nothing for it" false.
  - The record set is also unbounded. Every distinct statement that ever fails this way stays in it for the life of the process.
- **Evidence:** the text of R-0984 and R-0981, and R-0637's default. No test covers two cutovers: T-0312 runs one hand-run swap.
- **Fix, a design choice for the owner:**
  - **(a) Recommended.** Record the statement generation with the time. Every migration `tx` step raises that generation (SA), and morm already reads it from the `_morm` schema's comment (R-1246). On a candidate failed confirmation, morm reads the current generation on that connection after the rollback, which is one catalog read on a rare path.
    - If the generation has moved since the record, a migration has run since, so the failure is treated as a new first failure: the record is replaced, the connection discarded, and `.retry(n)` goes on as for a first failure.
    - Only an unmoved generation counts as a failed confirmation.
    - Clear the record when the connection generation moves. Keep the records in a set of at most 64 per pool that drops the oldest, like the memory. Dropping a record only turns a later failure into a first failure, which costs one discard, the safe direction.
  - **(b)** Give the record the memory's lifetime: 60 seconds or until the connection generation moves, at most 64. This is simpler, and a later cutover recovers after at most 60 seconds. But after each expiry, stored text in transactions again costs up to the pool's open connections, not one per minute.
- **Either way,** T-0312 adds two hand-run swaps of the type `f` reads, the second after the first was confirmed through `.retry(2)`. It asserts that the second swap's stale connections are each discarded and the generation is raised again. R-0984's safety sentence then says what it compares against.

### EI2. Should fix. R-1050, R-0995: the mapping is specified through `morm-core`'s scanner, which skips quoted identifiers, but generated SQL always quotes names

- **Where:** R-1050: "`migrate` and `rollback` map each unsuffixed name to that run's actual name, as a whole identifier outside strings and comments, through `morm-core`'s scanner, in the text they send".
- **What's wrong:**
  - R-0120 says "Generated SQL always quotes every table and column name", and `contracts.md` line 23 says "Generated SQL always quotes names". So `up.sql` and `down.sql` name retired objects as `"_morm_retired_m2_s"` and `"_morm_retired_m2_st"`.
  - The scanner R-1050 names treats quoted identifiers like strings. R-1221 lists "quoted identifiers with `""`, and `U&".."`" among the things it steps over, and R-1223 says "Outside strings, quoted identifiers, comments and dollar quotes, which the `morm-core` scanner skips".
  - So an implementer who follows the text maps nothing in a generated file. Or they map some forms and not others, as in a hand-edited file that mixes quoted and bare names. A partial mapping is the dangerous case: an `up.sql` whose renames are mapped but whose `down.sql` reads a bare or quoted name the scanner skipped would read the first run's retired column with no error. That is the silent wrong restore EH3 closed.
  - The rule also doesn't say what happens to a name inside a dollar quote, such as a `DO` block like R-1014's. "Outside strings" suggests it is left alone, which would again read the first run's object with no error.
  - `proofs.md`'s EH3 evidence used `regexp_replace` on whole words. That maps quoted identifiers, comments and strings alike, so it doesn't model the rule as written.
- **Fix:**
  - Say that the mapping rewrites identifier tokens, bare or quoted (`"..."` and `U&"..."`), matched on the identifier's decoded name and written back quoted, and never touches strings or comments.
  - Say that a generated file never names a retired object inside a string, comment or dollar quote. `makemigrations` refuses to write one that does, and `migrate` and `rollback` refuse such a file with exit code `2`, naming it.
  - A unit test of the mapping covers a bare name, a quoted name, a schema-qualified name, a longer name sharing the prefix (`_morm_retired_m2_st` beside `_morm_retired_m2_s`), and the name in a string, a comment and a dollar quote. T-0175's files use the quoted form `makemigrations` writes.

### EI3. Should fix. R-1050, R-0583, R-0996, R-0618: only the migration's own files are mapped, and only in `migrate` and `rollback`, so `revert` after a re-run reads the first run's retired column, and `rehearse`'s third pass isn't named

- **What's wrong:**
  - **`revert`.** R-0583: `revert` "creates a new forward migration from that migration's down". R-0996: "Retired values come back with their rows". That forward migration is a new, static, checksummed file that names the reverted migration's retired column, unsuffixed.
    - R-1050 maps names only "to that run's actual name", meaning the run of the migration whose files are running. A different migration's file that names `m2`'s retired objects isn't mapped.
    - So on a database where `m2` ran twice, `revert m2`'s migration reads `"_morm_retired_m2_s"`, the first run's column. That column is still there after the first rollback (shown above, beside `_morm_retired_m2_s_7`). The values are restored from the wrong run with no error.
    - This can happen in production, where `rollback --production --reason` (R-0585) followed by a re-run gives two runs, and in development.
    - The chain also stays linear across databases while `<run>` is each database's own `_morm.history` id. So a file can't carry the suffix itself either.
  - **`rehearse`.** R-0618 runs "pending up → down → up". The third pass is exactly a re-run before `purge`, the commonest one, because the down doesn't bring the retired column back. R-1050 names only `migrate` and `rollback`.
  - **`migrate --resume`** continues a run and should map with that run's names. That is implied, but not said.
- **Fix:**
  - Say that every command that executes migration text maps it: `migrate` and its `--resume`, `rollback`, each pass of `rehearse`, and the migration `revert` writes.
  - Say that an unsuffixed `_morm_retired_<migration>_<name>` in any migration's text maps to the actual name from the currently applied run of `<migration>`, the one the ledger records. So `revert`'s file reads the applied run's retired objects.
  - T-0157 adds a `revert` of an enum swap that was run, rolled back and run again, with rows changed between the runs, and asserts that each row is restored from the second run's column. T-0217, or `rehearse`'s own test, runs `rehearse` over an online enum swap and asserts that the third pass uses the suffix.

### EI4. Should fix. R-0981, R-0984, T-0311: "a failed retry" counts any error outside a transaction but only the `22P02` inside one, and the failed-retry branch no longer says what the caller gets

- **Where:** R-0981: "on success it takes the first connection's slot ... if the retry fails too, the new connection is closed and the first keeps its slot ... A failed retry means the value came from somewhere other than stale compiled code ... So after a failed retry ... the pool remembers that statement". R-0984: the attempt confirms if it "gets no `22P02`".
- **What's wrong:**
  - Outside a transaction any failure of the retry counts as a failed retry. But a retry that fails with another error, such as a unique violation, a check constraint or a statement timeout, got past the enum conversion on a fresh connection. That is exactly the evidence that the first `22P02` came from stale compiled code.
  - As written, such a real stale session:
    - fills the memory, so the other stale pooled connections return their `22P02` at once for 60 seconds;
    - raises no generation, so they are not replaced;
    - keeps the stale first connection in its slot.
  - Inside a transaction, R-0984's "gets no `22P02`" confirms in the same case. So the two halves of one design disagree.
  - The fix for EH5 also dropped "and the error is returned" from the failed-retry branch. R-0981 now says what the caller gets after a failed open, the original `22P02`, but not after a failed retry, or after a retry that fails with another error.
- **Fix:**
  - In R-0981, a retry fails only when it gets the same `22P02` from `enum_in` holding `_morm_retired_`. A retry that gets past it confirms, whatever happens next: the generation is raised, the slot rules for success apply, and the retry's own result is returned, success or error. A failed retry returns its `22P02`.
  - R-0984 uses the same words.
  - T-0311 adds a function that, on a fresh connection, gets past the conversion and raises `23505`. The caller gets `23505`, the generation is raised and nothing is remembered.

### EI5. Should fix. R-0981, R-1249, R-1040, R-1250: a failed open now fills the memory, so a real stale session can wait 60 seconds, but the text says only text-fed statements can, and the README doesn't say so

- **Where:** R-0981: "after a failed retry or a failed open the pool remembers that statement ... A stale session always confirms on its first new connection, so the memory only delays recovery for a statement that was being fed such text when a cutover happened". R-1249: "So after a real cutover, a statement that was being fed such text just before can see its `22P02` for up to 60 seconds before it recovers."
- **What's wrong:**
  - Under EH5 a refused open adds the statement to the memory. The causes include `too many clients`, the role's `CONNECTION LIMIT`, a failover or an authentication hiccup.
  - A real stale session's confirming open can be refused, often just after a migration, when connections are under pressure. Then that statement returns its `22P02` at once for up to 60 seconds, with no retry, no discard and no generation raised. Its stale pooled connections keep their slots.
  - So "a stale session always confirms on its first new connection" and "only delays recovery for a statement that was being fed such text" are false. R-1249, R-1040 and R-1250 describe only the text-fed case.
  - An operator would see a minute of `22P02` after a migration with no explanation.
- **Fix:**
  - R-0981's sentence names both causes: a statement being fed such text at the cutover, and a statement whose confirming connection Postgres refused.
  - R-1249's notice adds: "If Postgres refuses the new connection, as with too many clients, that statement's `22P02` returns at once for up to 60 seconds before morm tries again."
  - T-0311's refused-open row also asserts that, once the cool-down ends with the limit lifted, the call confirms and the generation is raised.

### EI6. Should fix. T-0312, T-0311, R-0981: "the pool's size is unchanged in every case" contradicts the discards with no `.retry(n)`, and "the pool's own refill" names a mechanism the pool doesn't have

- **Where:** T-0312: "with no `.retry(n)` ... one connection is discarded ... the same function in transactions with no `.retry(n)`, called five times, discards at most the four connections ... and in every case the pool's size in `pool_stats()` is unchanged". T-0311: "its size in `pool_stats()` is what it was once any refill is done". R-0981: "the emptied slot is left to the pool's own refill".
- **What's wrong:**
  - R-0638: "`min` is not a floor ... Connections reopen on demand". Nothing refills a discarded connection eagerly.
  - With no `.retry(n)`, a discarded connection leaves the pool one short until something checks one out. In the five-call case, run one call at a time, calls 1 to 4 take and discard the four idle connections, and call 5 opens one on demand. The pool ends at one connection, not four.
  - Only the `.retry(n)` cases keep the size, because the attempt's new connection takes the slot.
  - So T-0312's final clause asserts what the rules don't produce, and the test can't be built as written. "Once any refill is done", in T-0311, has no defined end.
- **Fix:**
  - T-0312 asserts the size is unchanged only where the attempt's connection takes the slot. Where connections are discarded with no `.retry(n)`, it asserts the pool's slots never pass `max`, and that the pool is back to four after four concurrent checkouts.
  - R-0981 and T-0311 say "left to the pool, which reopens it on demand under R-0638, with R-0653's backoff" instead of "refill".

### EI7. Minor. R-0984, T-0312: what `.retry(n)`'s next attempt does when its new connection can't be opened isn't said

- **Where:** R-0984: "`.retry(n)`'s next attempt run on a newly opened connection, opened as step 14 opens one, which takes the slot the discarded connection left."
- **What's wrong:** step 14's failed-open branch returns "the original `22P02`" and fills the memory. In a transaction the original error was already returned as `SchemaChanged`, and the attempt hasn't started. R-0984 doesn't say:
  - whether the attempt waits for the pool's on-demand open with backoff, falls back to a pooled connection, or ends `.retry(n)` with a connection error;
  - whether it counts as one of the `n` attempts;
  - whether the statement enters the memory.

  T-0312 has no refused-open case, though T-0311 does.
- **Fix:**
  - Say that the attempt's connection is opened through the pool's ordinary on-demand open, with R-0653's backoff, counting as that attempt. If it can't be opened within the pool's timeout, `.retry(n)` returns that error, and the statement enters the memory as R-0981 says for a failed open.
  - T-0312 adds the `CONNECTION LIMIT` case.

### EI8. Minor. T-0175: EH4's `check_schema` and walk aren't asserted

- **Where:** R-1050 now says a rollback's recorded objects are retired for "`verify` and `check_schema` ... a later op's `pg_depend` walk passes over them ... a later change Postgres refuses under the retired copy is refused with exit code `2`".
- **What's wrong:** T-0175 asserts `verify` and the refusal only. Nothing asserts that a pool with `check_schema = "on"` started after the rollback raises no drift or `_morm_` label warning for the retired type and column. Nothing asserts that a later migration whose walk reaches the retired copy passes over it, rather than counting it as a live dependent.
- **Fix:** T-0175 adds both after the first rollback.

### EI9. Minor. Attribution and the PR body's choices

- **R-1093** ends "Owner decisions SH, EF3, EG5 and EH7". But INDEX's EH entry lists "plain literals are decoded only when the function itself declares `SET standard_conforming_strings = off`" among "the fix's own choices the owner approves through PR #3". Add "with this fix's choice the owner approves through PR #3", as R-0981 and R-1050 do.
- **R-0981** lists its choices as "the 60 seconds, the hash key, the 64 entries and the `test-hooks` feature". It leaves out EH5's choice that `max` bounds the slots and the confirming connection is a momentary extra within the budget, which INDEX lists. Add it.
- **The PR body's "Choices of this fix" list** has no item for EH7's plain-literal choice. It also has none for EG's "morm's check session sets `standard_conforming_strings = on` itself", which INDEX's EG entry lists as a fix's choice. Add both, so the owner's approval of the PR covers every listed choice.
- **The PR body's EG3 bullet** still says "the pool's size never changes", which EH5 replaced. A note "(refined by EH5)" would keep the record accurate.

## What I couldn't run

- **`plpgsql_check`.** It isn't installed here. EH7's change rests on Postgres's own decoding, which I confirmed, and on the fourth review's reading of the extension.
- **Postgres 15 and 16, PgBouncer and a standby.** I had 17.11 and 18.6 only. The pooler parts of EH1 and EI1 follow from the text of R-0981 and R-0984, not from a run.
- **morm itself.** There is no runtime yet. EI1, EI4 to EI7 are shown from the rules. EI2 and EI3 are shown from R-0120, R-1221, R-1223, R-0583 and R-0996, with the coexisting first-run and second-run retired names confirmed on 17.11 and 18.6.

## Verdict

**Not approved yet.** The fix is faithful in most places:

- every changed ID is in INDEX's EH entry, and the PR body matches;
- the checker reports 0 problems, and CI is green on the head;
- EH2, EH4, EH6, EH7 and EH8 are fixed;
- EH3's Postgres claims hold on 17.11 and 18.6.

What remains:

- **Blocks approval:** EI1. The transaction's first-failure record never expires, so a second cutover's stale session is taken for stored text and, in transactions, never recovers. This needs a design choice; (a) is recommended.
- **Should fix:**
  - EI2, the mapping must cover the quoted names generated SQL always writes;
  - EI3, `revert` and `rehearse` must map too;
  - EI4, a failed retry means the same `22P02`, and its return value;
  - EI5, a failed open's 60-second delay for a real stale session, with a README line;
  - EI6, T-0312's pool-size clause and "refill".
- **Minor:** EI7, EI8 and EI9.
