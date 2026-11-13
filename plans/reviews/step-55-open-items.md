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
