# Step 43: `morm-admin doctor`

- **Phase:** Tools
- **Depends on:** 42
- **Can run beside:** 08, 23, 26, 39, 40, 41, 44, 45, 47, 48, 49, 51, 52, 53, 54, 58, 59, 60
- **Crates:** `morm-admin`, `morm-core`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

One command that reports everything that's wrong with a setup, gathered here from every rule in the plan.

## Deliverables

- Every `doctor` check listed below, using the budget function in `morm-core`

## End-to-end check

`doctor` reports each check correctly when its condition is forced, and passes on a healthy project.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> ```
> morm-admin init [path] [--axum]       # create a project
> morm-admin doctor                     # check Postgres, roles, scratch, versions
> morm-admin size [options]             # settings to resources, or resources to settings (section 11)
> morm-admin import URL [--out DIR]     # generate models from a database
> morm-admin scratch                    # check or create the scratch database
> morm-admin outbox [list | retry ID]   # inspect failed events, deliver one again
> morm-admin mgr <command>              # migrations and schema work (section 10)
> ```
>
> - **Code Postgres doesn't track.** A `plpgsql` function body has no recorded dependency on the columns it names, so a retire or rename can break inserts through a trigger like `NEW.note := 'x'`. Before either, morm searches `pg_proc.prosrc` in every schema for the name, whole words only, in any case, because a body in another schema can name the object too. Common names like `status` will give false hits, which is fine, because a hit is only a decision. A hit on a managed function is a decision with evidence, and `makemigrations` proposes the rewritten function. `migrate` preflight repeats the search on the live database, which is the only way to see unmanaged functions, and a hit not yet accepted is an open decision, which stops `migrate` with exit code `3` until `--decide` answers it. `rollback` preflight does the same before an inverse that retires or renames.
>
> Pools open in one process share the budget, so `db.pool("jobs")` beside the default pool can't double it. If `min` or `max` exceed the budget, including explicit values, morm lowers them and emits a warning, through `tracing` when enabled and stderr otherwise.
>
> - **No startup parameters.** PgBouncer rejects unknown ones by default. Falling back to session `SET` isn't safe here, because in transaction mode that setting stays on a server connection other clients will share. So set `lock_timeout`, `idle_in_transaction_session_timeout`, `client_connection_check_interval` and `search_path` on the role with `ALTER ROLE`, and `timezone` to `UTC`, because morm's sessions always run in UTC: a pool whose sessions report another zone refuses to start, with an error naming the zone and this fix.
>

## Out of scope

- `size` (step 46)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools / Configuration: `morm.toml`

**R-0103** `morm-admin doctor` reports the exact socket path it tries, and warns if `host` no longer matches a live socket.

**R-0112** `morm-admin doctor` checks that credentials work, warns if `morm.toml` contains a literal password, and warns if Postgres can log query values: `log_parameter_max_length` and `auto_explain.log_parameter_max_length` default to full values, which puts anything sent as a parameter, including `secret` columns, in the server log.

### Migrations / Triggers, functions and other objects

- **R-0541** `doctor` checks that the real roles exist.

### Migrations / Source of truth

**R-0550** `doctor` compares the Postgres major version and installed extensions of scratch and the target.

### Migrations / Down that can restore data

- **R-0573** `doctor` runs the same scan.

### Migrations / Runtime guarantees

- **R-0604** `doctor` reports the server version.

### Performance and memory / `max_connections`

**R-0651** `morm-admin doctor` runs the same check.

### Performance and memory / Behind a pooler

- **R-0671** `doctor` checks that they're there.
- **R-0679** `doctor` warns if `MORM_MIGRATION_URL` doesn't point at Postgres directly.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0786 | Query values, such as password hashes, reaching the Postgres log | `doctor` warns when statements are logged and `log_parameter_max_length` isn't `0` |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0110 | integration | `doctor` warnings on parameter logging |
