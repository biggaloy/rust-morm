# Step 55 open items

PR #3 (`step-55-enum-proofs`), head `7a7285b` when the step agent reported. Spikes ran green on Postgres 15 to 18 (last commit with them `d8bd613`), then were deleted. CI green; checker 1240 R / 300 T, 0 problems. Full options are in the PR body.

## From the step agent, waiting on the owner

- **EA (Q1). The driver's own type lookup.** tokio-postgres looks up an enum or composite result column's type itself, once per connection per type, with named statements of its own. Options: (a) accept it and test it in step 12 behind PgBouncer 1.19.1 and 1.20.1 (recommended); (b) morm reads enum and composite columns as `::text`.
- **EB (Q2). Translated server messages.** R-0981 and R-0718 match error message text, which breaks when the server's `lc_messages` isn't English (every message checked translates except `XX000`'s; a non-superuser can't change `lc_messages`). Options: (a) match on SQLSTATE plus the error's routine field, which is never translated (recommended); (b) require English messages.
- **EC (Q3). Stale statements behind PgBouncer (proof 16 contradicts R-0718 and R-0981).** With prepared statements on, re-preparing the same text gets the stale server statement back, and a new client reaches the same server session, so stale plpgsql stays too. Options: (a) retry unnamed and put the statement generation into the cached statements' text, and change the unmanaged-function decision text to say "run RECONNECT on each PgBouncer" (recommended); (b) turn the caches off behind PgBouncer; (c) the migration sends `RECONNECT`.
- **ED (Q4). plpgsql compiled before a swap that keeps the old type** gives `22P02`, which R-0981 doesn't act on. Options: (a) rely on the swap's `CREATE OR REPLACE` refresh, run in the swap's own transaction (recommended); (b) also treat a `22P02` naming a type morm retired as stale; (c) always drop the old type in the swap.

## Choices the step agent made

- The false shapes tested are R-1079's list; every swap is `--fast`'s one-transaction form.
- PgBouncer ran with `default_pool_size = 1` and `max_prepared_statements = 200`; 1.19 and 1.20 weren't run, because DL3 runs them with morm's caches off.
- The standby's `max_standby_streaming_delay` was 5 s.
- Proofs 14 to 17 only record; outputs were compared across versions by diffing logs.
- Proof 15 differs from the step's text: `DISCARD PLANS` and `DISCARD ALL` cure `%ROWTYPE` and the trigger, not `DECLARE` or `INTO`.

## From the independent review at `7122fd9`

The review (`reviews/step-55-review.md`) changes several of the questions above. They are not yet answered.

- **EA:** framed wrongly and its recommendation (a) is unsafe. Behind PgBouncer with no prepared statements, which morm supports, the driver's own type lookup failed 83 of 120 times on 1.20.1 (pool of 4) and 86 of 120 on 1.22.0 (`max_prepared_statements = 0`), with `26000`. Testing it in step 12 comes after the contracts freeze. The review recommends (b), reading enum and composite columns as `::text`.
- **EE1 (blocks):** while the old type is kept after an online swap (the default until `purge`), a stale cached statement or compiled function silently runs on the old type for labels both types share: after a reorder a cached comparison returned the old answer and a cached `INSERT ... WHERE` wrote a row the new order excludes, with no error. Recommended: rename the kept type's labels at cutover, so any stale use fails with `22P02`, which the plan already retries, while the retired column keeps its rows.
- **EB (EE3):** a superuser can fix `lc_messages` for the app role with `ALTER ROLE ... SET`, or let it choose with `GRANT SET ON PARAMETER` on 15 and later, so a startup check like the UTC one is a missing option. EB also leaves out R-0984's `22P02` test, which reads the enum name from the message, and R-0983's message match. The `42804` message comes from three routines, and routine names change between major versions.
- **EC (EE4):** R-0975's per-pool generation counter starts at the same value in every process, and PgBouncer shares server statements by text, so a new process still gets stale ones. These errors also arrive at prepare time, which R-0718 doesn't cover. Recommended marker: one every process shares, such as the latest applied migration id.
- **ED (EE5):** the kept old type is the default online state, not an edge case; option (c) conflicts with R-1050; option (a) leaves R-1040's "recover by themselves" text wrong.
- **EE6:** plpgsql_check's `disable:performance_warnings` pragma can likely hide R-1080's narrowing decision; untested.
- **EE11:** (a) replaying a swap ends a standby session idle in a transaction holding the table with FATAL `40001`, including a `db.hold()` connection; the plan has no rule. (b) R-0718 and R-0981 have no rule for a stale-statement error at prepare time behind PgBouncer.
- **EE7 to EE9 (minor):** R-0994's text vs proof 15's `DISCARD PLANS` result; proof 13 never ran a statement cached before `ADD VALUE` after it (the reviewer's run shows it works); the recorded composite decoder shape leaves out NULL fields and bounds checks.
- **EE10:** proofs 14 to 16 need an owner-approved change before the step can pass.
- **Answered:** EA (b), read enum and composite columns as `::text`. The rest goes to one design pass: `reviews/stale-statement-design.md`.
- **Answered:** the stale-statement design SA to SH, all as recommended. EE7 to EE9 fixed as the review recommends.

## From the second review at `e8505c7`

- **EF1, EF3, EF9:** answered as recommended (fresh-connection retry guarded against `_morm_retired_` values; decode escaped literals; re-run the evidence in spikes CI). EF2, EF4 to EF8 fixed as recommended.

## From the third review at `399cdb2`

- **EG2, EG5, EG6:** answered as recommended (cool-down per statement; decode plain strings when `standard_conforming_strings` is off; rebuild inside the rollback's transaction, rollback opens R-1063's decision, ledger records the retired objects, re-runs suffix names with the migration id). EG1, EG3, EG4, EG7, EG8 fixed as recommended.

## From the fourth review at `eb56a80`

- **EH1, EH3:** answered as recommended (later failures count as failed confirmations; the ledger maps retired names). EH2, EH4 to EH8 fixed as recommended.

## From the fifth review at `a9ee0d3`

- **EI1:** answered: keep the retry, since stale sessions must heal by themselves; first-failure records carry the statement generation and are capped at 64 per pool (option (a)). EI2 to EI9 fixed as recommended.

## From the sixth review at `1df3f2d`

- **EJ1:** answered: on `53300`, replace the stale connection in its slot. EJ5 must heal behind PgBouncer too. EJ2 to EJ4, EJ6 to EJ9 fixed as recommended.
