# Step 55 independent review, sixth round

PR #3 (`step-55-enum-proofs`), head `14bcb6f`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes. This round covers the fix for the fifth review's EI1 to EI9, `git diff 0501425 origin/step-55-enum-proofs`, and checks that nothing earlier regressed. The owner has decided that stale sessions heal by themselves, so the automatic fresh-connection retry stays; this round checks that it heals in every path.

## What I read

- **On the PR branch:** `rules.md`, `contracts.md`, `proofs.md` (step 55's section, including the new "fifth review" bullets), and every changed requirement and test in full: R-0583, R-0618, R-0981, R-0984, R-0996, R-1040, R-1050, R-1093, R-1249, R-1250, T-0157, T-0175, T-0311, T-0312 and the new T-0315. For the retry rules I also re-read T-0314, R-1049, R-0622 (`squash`), R-0650 and R-0702 (the connection budget) and R-1249's whole notice.
- **On `origin/main`:** INDEX's Decisions entries from "Stale-statement design, SA to SH" through "Step 55 fifth review, EI1 to EI9", with its IDs and the fix's own choices, and `reviews/step-55-review-5.md`.
- **The PR body.**

## What I checked and ran

- **CI on the head.** `14bcb6f` is green: `ci` on push (run 43568001740) and on the pull request (run 43568006227), every job, including `coverage`, `pooler`, `msrv`, `features` and integration on 15 to 18 over TCP and Unix sockets. The diff since `0501425` touches only `plans/plan-approved/` (9 files). No code, test or workflow changed.
- **Coverage checker,** on the head:
  - `check_coverage.py --step 55 --code crates tests benches fuzz .github`: **1255 requirements, 315 test obligations, 0 problems**;
  - the plain form: the same.
- **Manifest and INDEX.** Comparing `0501425`'s manifest with the head's, ID by ID, gives 15 changed IDs, one new (T-0315) and none removed: R-0583, R-0618, R-0981, R-0984, R-0996, R-1040, R-1050, R-1093, R-1249, R-1250, T-0157, T-0175, T-0311, T-0312 and T-0315. That is exactly the INDEX EI entry's "written in at `14bcb6f`" list, and the PR body's list since `0501425` matches both. Against `origin/main`, all 71 changed IDs are covered by INDEX's step 55 entries. `frame` and `master_sha256` are unchanged. `coverage.md` gains T-0315 and the new total.
- **Postgres spot checks of EI2.** These ran on throwaway Postgres 17.12, built from ftp.postgresql.org's tarball checked against its published SHA-256, and the system's 18.6. The data was under `.claude/scratch/` in my worktree, with `TMPDIR` set there, and the sockets were in `~/.claude/scratch/`. I wrote my own token-level mapper from R-1050's words (strings, `E''`, `U&''`, quoted and `U&""` identifiers with `UESCAPE`, nested comments, dollar quotes, bare identifiers with `$` inside, ASCII folding), not a regular expression, and quoted, schema- and table-qualified files like the ones `makemigrations` writes. Both versions printed the same, byte for byte apart from the version line.

  | Claim | Result |
  |---|---|
  | A migration removing label `c` (rows `1:c 2:a 3:b`), mapping it to `b` and keeping old values, `up.sql` and `down.sql` stored with their SHA-256 first. Run 1 up, then rollback | `1:b 2:a 3:b`, then `1:c 2:a 3:b` |
  | Between runs, row 2 changed to `c` and row 4 inserted as `c`. Run 2 as written | `type "_morm_retired_m2_st" already exists` |
  | Run 2 mapped to `_7` | Runs, six lines naming suffixed names; `t` then has `_morm_retired_m2_s` and `_morm_retired_m2_s_7` |
  | Run 2's rollback as written, in a transaction rolled back after | No error, but `1:c 2:b 3:b 4:b`: rows 2 and 4 wrong, read from the first run's column |
  | Run 2's rollback mapped | `1:c 2:c 3:b 4:c`, every row right |
  | A file naming the retired column in a `--` comment, a `DO $body$` block and a string | Refused, naming each |
  | `_MORM_RETIRED_M2_S` (bare, upper case), `"_morm_retired_m2_s"`, `U&"\005fmorm_retired_m2_s"`, `U&"!005fmorm_retired_m2_s" UESCAPE '!'`, `"t"."_morm_retired_m2_s"`, `"public"."t"."_morm_retired_m2_s"`, and the type bare and schema-qualified | As written, all read the first run's objects (`_morm_retired_a` for row 2, type `_morm_retired_m2_st`); mapped, all read the second run's (`_morm_retired_c`, `_morm_retired_m2_st_7`) |
  | `"_morm_retired_m2_s_x"`, `_morm_retired_m2_sx`, `x$_morm_retired_m2_s$`, `"_MORM_RETIRED_M2_S"` | Left alone, correctly: each is a different identifier to Postgres. `x$_morm_retired_m2_s$` is one identifier, not a dollar quote |
  | A quoted name left followed by its old `UESCAPE '!'` | `syntax error at or near "'!'"` (EJ8) |
  | The stored files' checksums | Still match |

  So `proofs.md`'s EI2 paragraph holds on both versions, with a mapper that follows the rule as now written.

## EI1 to EI9: are they fixed?

| Finding | Verdict and evidence |
|---|---|
| EI1 | **Fixed as the owner chose (option a), with two gaps.** R-0984's record now stores the statement generation from the `_morm` schema's comment; a failure on a newer connection with the generation moved is a new first failure, the record replaced and the connection discarded; records are capped at 64, oldest dropped, and cleared when the connection generation moves (the fix's own choice). For the review-5 scenario (confirmed through `.retry(2)`, then a second swap) and for a second migration after a no-`.retry(n)` heal, the text now heals. **But** the record's time and its generation are taken at different moments, and nothing says whether a failure on an older connection overwrites the record; one reading churns a connection per call (EJ2). T-0312's two-swap case goes through the clearing, not through the generation comparison, so it doesn't prove the fix (EJ3) |
| EI2 | **Fixed, as recommended.** R-1050 maps identifier tokens, bare or quoted, `"..."` and `U&"..."`, on the decoded name, bare names folded, written back quoted; it never touches strings, comments or dollar quotes; `makemigrations` won't write a file naming a retired object inside one and every command that runs migration text refuses one with exit code `2`. T-0315 covers the forms review 5 listed. My run above confirms it on 17.12 and 18.6. Two lexer cases are untested (EJ8), and the refusal has no README notice (EJ4) |
| EI3 | **Fixed, as recommended.** R-1050 lists `migrate` and `--resume`, `rollback`, each pass of `rehearse` and `revert`'s forward migration, mapping to the applied run, or the run being rolled back or resumed; R-0583, R-0996 and R-0618 say so where those commands live; T-0157 adds `revert` after a re-run and `rehearse` over an online swap. `squash` refuses while retired objects from those migrations remain (R-0622), so it can't strand a name |
| EI4 | **Fixed, as recommended.** R-0981 and R-0984 now use one rule: a retry fails only on the same `22P02` from `enum_in` holding `_morm_retired_`; one that gets past the conversion confirms whatever happens next and returns its own result; a failed retry returns its `22P02`. T-0311 adds the `23505` case. Errors that can't show whether the conversion was reached are still unclassified (EJ6) |
| EI5 | **Text fixed; but the path it describes doesn't always heal.** R-0981, R-1249, R-1250 and R-1040 now say a refused open can delay a really stale session by up to 60 seconds, and T-0311 asserts the call confirms once the cool-down ends with the limit lifted. While the refusal lasts, outside a transaction, the stale connections are never replaced (EJ1) |
| EI6 | **Fixed.** R-0981 leaves an emptied slot to the pool, which reopens on demand with its backoff; T-0311 states the one case left a connection short; T-0312 asserts size only where the attempt takes the slot, and `max` plus four concurrent checkouts elsewhere |
| EI7 | **Fixed.** R-0984: the attempt opens through the pool's ordinary on-demand open, as part of that attempt; if it can't within the pool's timeout, `.retry(n)` returns that error and the statement enters the memory. T-0312 adds the `CONNECTION LIMIT` case |
| EI8 | **Fixed.** T-0175 asserts `check_schema` and the walk after the first rollback |
| EI9 | **Fixed for what it named.** R-1093 names its choices, R-0981 names EH5's, the PR body adds choices 14 and 15 and marks EG3 "refined by EH5". The EI fix's own three choices are not named the same way (EJ9) |

Nothing earlier regressed outside the points below. Only `plan-approved/` changed, and the checker, CI and the ID list agree. SA to SH, EF1, the `22P02` narrowing, SF, EG1 to EG3 and EH1 to EH8 still read consistently with the new text.

## The retry rule set, path by path

| Path | Heals? |
|---|---|
| Outside a transaction, straight, room in the budget, open accepted | Yes, on the first call: confirmed, generation raised, pool replaced at checkout |
| Outside, full budget | Yes: first closed, new one in its slot, confirmed |
| Outside, open refused once | Yes, after up to 60 seconds (EI5, documented) |
| Outside, open refused while the refusal lasts (role `CONNECTION LIMIT` at the pool's size, or the server full from other processes), room in morm's budget | **No.** The first connection keeps its slot, nothing is discarded or raised, and every minute one more refused attempt (EJ1) |
| Inside, `.retry(n)` | Yes: discard, attempt on a new connection in the freed slot, confirmed |
| Inside, no `.retry(n)` | Yes: each stale pooled connection fails one transaction and is discarded; a second cutover heals the same way through the generation comparison, apart from EJ2's window |
| Inside, attempt's open refused | Yes, after up to 60 seconds; the discard freed a server slot first, so a persistent limit doesn't block it |
| Stored or client text | Bounded: one connection per statement per minute outside, a round of discards per migration inside (EJ7), apart from EJ2's overwrite reading |
| `db.hold()` | Yes, on release |
| Behind PgBouncer | Not by morm, by owner decision SB (`RECONNECT`); but R-0984's age test misreads a stale server session as stored text (EJ5) |

No path grows the pool past `max` or the process past its budget. The 64-entry caps on the memory and the records are owner-approved choices (EG2, EI1), so their thrash beyond 64 distinct statements isn't raised again here.

## Findings

### EJ1. Blocks approval. R-0981, T-0311, R-1249: outside a transaction, while Postgres keeps refusing the confirming connection, a real stale session never heals

- **Where:** R-0981: "If the new connection can't be opened, Postgres refusing it with too many clients included, the original `22P02` is returned at once, and in the full-budget branch the emptied slot is left to the pool ... A failed retry or a failed open confirms nothing and raises nothing ... After that the statement is tried again on a new connection, and a stale session then confirms". In the branch with room in the budget, a failed open closes nothing: "when it fails, the new connection is closed and the first keeps its slot".
- **What's wrong:**
  - morm's budget comes from the server's `max_connections` (R-0650's formula); it knows nothing of a role's `CONNECTION LIMIT` or of connections other services hold. So a pool can sit at `max` with "room in the budget" while every new connection is refused. T-0311 itself builds this case with the role's `CONNECTION LIMIT` reached.
  - In that state every confirming open is refused. The first, stale connection keeps its slot, the memory fills, nothing is raised, and after 60 seconds the next attempt is refused again. The pool's other stale connections are never replaced either: `max_lifetime` is off by default (R-0637) and a busy pool never idles. So that statement returns its `22P02` for as long as the limit holds, which with a role limit equal to the pool's size is for good.
  - This contradicts the owner's decision for this round, that stale sessions heal by themselves, and R-0981's and R-1249's "before morm tries again ... a stale session then confirms", which assume the next open is accepted. T-0311 asserts healing only "with the limit lifted".
  - Inside a transaction the same refusal heals, because the discard frees a server slot before the attempt opens. Only the out-of-transaction branch with budget room is stuck.
- **Evidence:** the text of R-0981 and T-0311; R-0650's budget formula; R-0702 (processes sized together against `max_connections` only).
- **Fix, a design choice for the owner:**
  - **(a) Recommended.** When the confirming open is refused (SQLSTATE `53300`, too many connections, for the server or the role), fall back to the full-budget branch at once: close the first connection, open the new one in its slot, and retry. If that open fails too, the slot is left to the pool and the statement enters the memory, as today. A stale session then confirms whenever one server slot can be had, which closing the first one makes likely. The cost for stored text is one closed and reopened connection per statement per minute, still capped by the memory.
  - **(b)** On a refused open, discard the first connection without retrying (slot left to the pool), return the original `22P02` and fill the memory. Simpler, but it raises nothing, so the memory then shields the other stale connections and the pool heals one connection per statement per minute.
  - **(c)** Keep the rule and say in R-0981 and R-1249 that it heals only once Postgres accepts a new connection, naming a role's connection limit at the pool's size. That doesn't meet the owner's decision.
  - With (a) or (b), T-0311's refused-open row asserts that, with the limit still in place, the stale connection is replaced and (for (a)) the generation is raised.

### EJ2. Should fix. R-0984: the record's time and generation are taken at different moments, and nothing says whether a failure on an older connection overwrites the record

- **Where:** R-0984: "records ... the statement's hash, the time of this first failure and the database's statement generation ... If the same statement later fails the same way in a transaction on a connection opened after the recorded time, morm reads the statement generation on that connection after the rollback ... If it has moved since the record ... the record is replaced ... Only with the generation unmoved does it count as a failed confirmation".
- **What's wrong:**
  1. **Which time.** The record's time is the failure's; its generation can only be read after the rollback. A migration `tx` step that commits between the two is folded into the record. A connection opened in that window that compiled the function before that commit is stale from it, yet it was "opened after the recorded time" with the generation "unmoved", so it is a failed confirmation every time: the error returns raw, nothing is discarded, and with no connection-generation move the record is never cleared. It never heals. The window is a round trip, but it falls exactly where the pool is busiest: right after a cutover, while failed connections are being discarded and reopened on demand, and while `migrate` commits its next `tx` steps (review 5's two-swaps-in-one-run case).
  2. **Overwrite.** The rule says when a record is replaced (generation moved) but not what a failure on a connection opened *before* the recorded time does to an existing record. It falls under "Otherwise ... records", which reads as overwriting it with a later time. Under that reading, with stored text and no `.retry(n)`: each failure on an older connection discards it and moves the time forward, so the connections the pool just reopened now count as older too, and each of their failures discards again. Round-robin checkout never reaches a connection opened after the latest record, so it costs a connection per call with no end. T-0312's "discards at most the four connections open at its first failure and then none" only holds if the record is kept, so the test and the rule's most literal reading disagree.
- **Fix:**
  - Take the record's time after the generation is read (the time of the read, not of the failure), and say the comparison is "opened after the generation was read".
  - Say that a failure on a connection opened before the recorded time keeps the record as it is when the generation is unmoved, and replaces it (new time and generation) only when the generation has moved.
  - T-0312 adds the overwrite case: four pooled connections, stored text, calls one at a time over more than eight calls, and asserts discards stop at four.

### EJ3. Should fix. T-0312: the two-swap case passes without EI1's generation comparison

- **Where:** T-0312: "across two hand-run swaps of the type the function reads, the second after the first was confirmed through `.retry(2)`, each connection the second swap left stale fails one transaction, is discarded and isn't taken for stored text".
- **What's wrong:**
  - The first swap is confirmed through `.retry(2)`, which raises the connection generation, and the EI1 fix clears every record when that moves. So the second swap meets no record at all, and the comparison of statement generations is never reached. An implementation that clears records but never compares generations passes the test.
  - The case the comparison exists for is a record that survives: no `.retry(n)`, so the connection generation never moves, or a statement fed stored text before the second swap. Nothing tests it.
  - The swaps are "hand-run". A hand-run swap doesn't raise the `_morm` schema's statement generation (only a morm `tx` step does), so a test that did reach the comparison would need the swap to raise it.
- **Fix:** T-0312 adds: in transactions with no `.retry(n)`, a first swap healed by discards; then a second swap run as a morm migration, or one that raises the `_morm` comment as a `tx` step does; then each connection the second swap left stale, including those opened after the first swap's first failure, returns `SchemaChanged` and is discarded, and none puts the statement in the memory. And the converse: the same second failure with the generation unmoved is a failed confirmation (the five-call case already covers this).

### EJ4. Should fix. R-1249 (README), R-1050: the new refusal of a migration naming a retired object in a string, comment or dollar quote, and the re-run's rewriting of names, have no README notice

- **Where:** R-1050: "a migration file never names a retired object inside one: `makemigrations` refuses to write one, and every command that runs migration text refuses such a file with exit code `2`, naming the file and the name"; and "a second run ... names its own retired objects with a suffix ... the text sent to Postgres maps each unsuffixed name to the actual one, never the stored file".
- **What's wrong:**
  - This is a new refusal with exit code `2` on `migrate`, `rollback`, `rehearse` and `revert`, and in `makemigrations`. Someone who hand-edits a migration to read the retired column in a `DO` block, or mentions it in a comment, is stopped. R-1249's "What you can't do" paragraph lists the other refusals of this feature (`_morm_` labels, `import`, `check_schema`) but not this one, and no other README requirement mentions it (`grep -i "dollar\|suffix\|mapp" steps/50-readme-release.md` finds nothing for it).
  - The rewriting is also a surprise worth one line: what Postgres runs, and what a log or `pg_stat_activity` shows, differs from the checksummed file on a re-run, and retired objects get `_<run>` suffixes that `purge` later drops.
- **Fix:** R-1249's "What you can't do" adds: "A migration file can't name a retired object (`_morm_retired_<migration>_<name>`) inside a string, a comment or a dollar quote such as a `DO` block: morm rewrites those names when a migration runs a second time before `purge`, and can't see them there, so `makemigrations` won't write one and `migrate`, `rollback`, `rehearse` and `revert` stop on one with exit code `2`, naming it." And one sentence under "Rolling back the swap": "If you run the migration again before `purge`, its retired objects get a `_<run>` suffix, and morm rewrites their names in the text it sends; the files and their checksums stay as written."

### EJ5. Minor. R-0984: behind a pooler, a client connection's age says nothing about the server session, so a stale server session is taken for stored text

- **Where:** R-0984: "If the same statement later fails the same way in a transaction on a connection opened after the recorded time ... Only with the generation unmoved does it count as a failed confirmation, because a fresh connection compiled the function since the last migration ... So a real stale session after any cutover, a second one included, is never taken for stored or client text". Behind a pooler "it discards nothing", but the record and the failed-confirmation rule aren't limited to straight connections.
- **What's wrong:** behind PgBouncer, a client connection opened after the record can reach a stale server session. With the generation unmoved, that failure counts as a failed confirmation: the error returns raw instead of `SchemaChanged`, so `.retry(n)` stops, and the statement enters the memory, so for 60 seconds every such failure is raw and `.retry(n)` never gets to land on a fresh server session. This repeats every minute until `RECONNECT`. The record exists to stop discards, and behind a pooler nothing is discarded, so it buys nothing there. The quoted sentence is false behind a pooler.
- **Fix:** say that behind a pooler no first-failure record is kept and no failure counts as a failed confirmation: a `22P02` holding `_morm_retired_` is `SchemaChanged` unless the parameter test or step 14's memory says otherwise. T-0312's pooler case asserts a stale server session reached through a newly opened client connection still returns `SchemaChanged`.

### EJ6. Minor. R-0981, R-0984: a retry ending in an error that doesn't show whether it got past the conversion isn't classified

- **Where:** R-0981: "It fails only when it gets the same `22P02` from `enum_in` holding `_morm_retired_`; a retry that gets past that conversion confirms a stale session, whatever happens next". R-0984 uses the same words.
- **What's wrong:** "fails only on the same `22P02`" and "gets past the conversion" aren't complements. A retry can end in a lost connection, `57014` (statement timeout or cancel), `55P03` (lock timeout), `40P01` (deadlock) or `57P01` (admin shutdown) before the function reaches the conversion. By the first clause that isn't a failure, so it confirms: the connection generation is raised and the whole pool replaced, and in the lost-connection case a dead connection "takes the first connection's slot". By the second clause nothing shows it got past. SF says the generation rises only for a confirmed stale statement.
- **Fix:** say that a retry confirms only when it returns rows or a server error other than the same `22P02` and other than classes `08` (connection), `57` (operator intervention) and `55P03`, `40P01`; any of those confirms nothing, raises nothing and fills the memory, as a failed open does, the safe direction (a real stale session waits up to 60 seconds). A lost new connection is closed, not given the slot. T-0311 adds a retry cancelled by `statement_timeout` before the conversion: no generation raised, the statement remembered.

### EJ7. Minor. R-0984, R-1249, R-1040, R-1250: stored text in transactions costs a round of discards after every migration, not only once

- **Where:** R-0984: "that text stops costing connections after at most the pool's connections that were open when it first failed". R-1249, R-1040 and R-1250: "or inside transactions at most the pool's connections that were open when it first failed".
- **What's wrong:** under EI1, every migration `tx` step moves the statement generation, and the first failure after it on a newer connection is a new first failure, so a statement fed stored text costs up to the pool's connections again after each migration, not once for the life of the process. The README understates it.
- **Fix:** add "and again after each migration" to R-0984's sentence and to the three notices.

### EJ8. Minor. T-0315: the `UESCAPE` clause and `$` inside a bare identifier aren't covered

- **Where:** T-0315 lists "a `U&"..."` name" and "a dollar quote"; R-1050 says the mapping reads `U&"..."` "as Postgres reads them".
- **What's wrong:** two lexer cases a mapper can get wrong aren't in the unit test:
  - `U&"!005f..." UESCAPE '!'` must be replaced together with its `UESCAPE` clause. Writing back only the quoted name leaves `"..." UESCAPE '!'`, which on 17.12 and 18.6 fails with `syntax error at or near "'!'"`.
  - `x$_morm_retired_m2_s$` is one identifier to Postgres (both versions), not a name followed by a dollar quote. A mapper that opens a dollar quote at `$_morm_retired_m2_s$` swallows the rest of the file and then refuses or skips names after it.
- **Fix:** T-0315 adds a `U&"..." UESCAPE '!'` name, rewritten without its clause, and an identifier holding `$` around a retired name, left alone with the names after it still mapped.

### EJ9. Minor. Attribution of the EI fix's own choices

- **What's wrong:** INDEX's EI entry lists three "fix's own choices the owner approves through PR #3": records also cleared when the connection generation moves; a refused open still fills the cool-down, with the text saying a stale session can wait up to 60 seconds; and the mapping of bare and quoted tokens with the exit-`2` refusal. But:
  - R-0984 ends "Owner decisions DH2, choice I, SC, SD, EF1, EG1, EH1, EH2 and EI1", with no "this fix's choice" for the clearing;
  - R-0981's list of choices doesn't include the refused open filling the cool-down;
  - R-1050's names only "the `<run>` suffix", not the mapping and refusal choice, and its owner decisions don't name EI2 or EI3;
  - the PR body's "Choices of this fix" list has no items for these three;
  - the PR body's EH5 bullet still says "the pool's refill", which EI6 replaced.
- **Fix:** add each choice to R-0984, R-0981 and R-1050 as "with this fix's choice the owner approves through PR #3: ...", add three items to the PR body's list, and mark the EH5 bullet "(refined by EI6)".

## What I couldn't run

- **morm itself.** There is no runtime yet. EJ1, EJ2, EJ3 and EJ5 to EJ7 are shown from the rules' text.
- **Postgres 15 and 16, PgBouncer and a standby.** I had 17.12 and 18.6 only, so EJ5 rests on how PgBouncer assigns server sessions, not on a run.
- **morm's own mapper.** My spot check used a mapper I wrote from R-1050's words. It shows the rule maps generated SQL correctly and that the Postgres behaviour `proofs.md` records holds, not that morm's future mapper will.

## Verdict

**Not approved yet.** The fix is faithful in most places:

- every changed ID is in INDEX's EI entry, and the PR body matches;
- the checker reports 0 problems, and CI is green on the head;
- EI2 to EI4 and EI6 to EI8 are fixed, and EI2's Postgres claims hold on 17.12 and 18.6 with a token-level mapper;
- EI1 heals the review-5 scenario and a second migration after a no-`.retry(n)` heal.

What remains:

- **Blocks approval:** EJ1. Outside a transaction, while Postgres refuses new connections (as with a role's connection limit at the pool's size), a stale session never heals. This needs a design choice; (a) is recommended.
- **Should fix:**
  - EJ2, the record's time and its overwrite rule;
  - EJ3, a test that reaches EI1's generation comparison;
  - EJ4, a README notice for the new refusal and the re-run's rewriting.
- **Minor:** EJ5 to EJ9.
