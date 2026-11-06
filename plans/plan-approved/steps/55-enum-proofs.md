# Step 55: Proofs of the managed enum design

- **Phase:** Foundation
- **Depends on:** 02
- **Can run beside:** nothing
- **Crates:** throwaway `spikes/` directory and temporary `.github/workflows/spikes.yml`, both deleted when the step ends
- **PR approval:** the owner approves the PR

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Prove the managed enum design on Postgres 15–18, behind PgBouncer and on a hot standby, and the `plpgsql_check` rule on Postgres 15–18, before the contracts freeze. A failed proof stops the project and goes to the owner with options.

## Deliverables

- One small spike per item below, each compiled and run
- `plans/plan-approved/proofs.md`: result, the final shape of each API, and any design change the owner approved
- `contracts.md` updated with what was learned

## End-to-end check

Every spike runs in the temporary workflow, and `proofs.md` links its logs and records a pass or an owner-approved change for each item.

## Proof list

These continue the numbering of step 02.

| # | Proof | Pass means |
|---|---|---|
| 13 | Enum and composite binding with no OID, on Postgres 15–18 | A hand-written `ToSql` and `FromSql` bind and decode a Postgres enum through `query_typed` with `Type::UNKNOWN` and through a prepared statement, with no OID and no variant list, before and after `ALTER TYPE ... ADD VALUE` and a type swap. Arrays and composite values work as text-format parameters, and they decode |
| 14 | Stale statements after a type swap, on Postgres 15–18 | `proofs.md` records the SQLSTATE and message each version returns for a cached statement after a swap, with the old type dropped and with it kept: a statement returning the column, an enum parameter and an enum array, each in and out of a transaction, and on a second run. Postgres 18 gave `0A000`, `XX000` with `cache lookup failed for type`, `42883` and `42804`, the failures repeated until a re-prepare, and a re-prepare cured them. Also record the SQLSTATE of `type of parameter ... does not match that when preparing the plan` from a plpgsql function compiled before a column type change, which was `42804` on Postgres 17 and 18, confirm it on 15 and 16, and confirm that a new connection clears it. Record whether either message is ever translated by `lc_messages` |
| 15 | Compiled plpgsql after a swap, on Postgres 15–18 | `proofs.md` records what a session gets when it compiled a function before the swap: `DECLARE v the_enum`, `SELECT ... INTO`, `%ROWTYPE`, and a trigger function that never names the type. On Postgres 18 these failed on every call, with `XX000` when the old type was dropped, and `42804` for `%ROWTYPE` and the trigger when it was kept. `DISCARD PLANS` didn't help, a no-op `CREATE OR REPLACE` from another session cured them, and a fresh session was fine |
| 16 | Stale state behind PgBouncer in transaction mode | Proofs 14 and 15 repeated through PgBouncer. `proofs.md` records what the client sees, whether replacing the client connection clears the stale server session, and what does clear it |
| 17 | Stale state on a hot standby | A swap on a primary reaches a hot standby. `proofs.md` records what cached statements and compiled functions on the standby return, whether replay cancels queries there, and whether the recovery proved in 14 and 15 works on a `read_only` pool |
| 18 | `plpgsql_check` on Postgres 15–18 | Installed from PGDG into the running `postgres:NN` service with no restart, it reports `42703` for a column dropped from a branch a passing call never reaches, as a user with `CREATEDB` that isn't superuser, in a clone of a template a superuser made, while that user's `CREATE EXTENSION` fails with `42501` and a database made from `template0` holds no extension. `proofs.md` records the version, and for each Postgres version: the finding for each false shape and the `PERFORM 'PRAGMA:...'` line that clears it, the same line running with no extension, `PRAGMA:DISABLE:CHECK` hiding a real break up to the end of its block, a `PRAGMA:TABLE` line that no longer matches, the level, SQLSTATE and detail of a narrowing and a widening, a constant, a `format` and a built `EXECUTE`, `plpgsql_check.mode` set to `disabled` by `SET`, on a role, in a connection's `options` and on a function, and each trigger form, including no table and transition tables. Also record whether an in-comment `@plpgsql_check_options` line can hide an `error`, because a body could carry one |

Proofs 13 to 17 check the managed enum design, which was tested only on Postgres 18, and proof 18 checks step 59's `plpgsql_check` rule. Spikes are deleted, so each result stays in `proofs.md`, and steps 14 and 59 keep permanent tests of it. A result that contradicts the expected behaviour above isn't worked around: stop and ask the owner with the result and options. Run these in a temporary `.github/workflows/spikes.yml` on the step's PR, with Postgres 15–18, PgBouncer and a standby. Link the logs from `proofs.md`, and delete the workflow before merge. `tests/tests/repo_policy.rs` reads every workflow, so this one follows the same rules and that file doesn't change: top-level `permissions` of `contents: read`, actions pinned to a full commit, `shell: bash` on any step with a pipe, `--manifest-path spikes/Cargo.toml` and no `working-directory`, and no `if:`, `continue-on-error`, `exclude:`, `secrets.` or `|| true`. Give `spikes/Cargo.toml` its own `[workspace]` table, because it sits under the root workspace.

## Out of scope

- Production code: spikes are thrown away

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

None beyond the deliverables above.

## Test obligations

None beyond one tagged test per requirement.
