# rust-morm: approved plan

This folder is the source of truth for building rust-morm. `../rust-morm-plan.md` is frozen as the record. `../app_suggestions.md` holds advice for apps, and `../later-defo-build.plan` holds confirmed work after 0.1.

| File | Holds |
|---|---|
| `rules.md` | How every step works, and when it's done |
| `contracts.md` | Shared types and conventions, plus the key decisions |
| `coverage.md` | Every requirement and test obligation, with its step |
| `steps/` | One self-contained file per step |
| `tools/` | The coverage checker, and the scripts that made this split |

The frozen plan was split word for word into requirements (`R-nnnn`) and test obligations (`T-nnnn`), each in exactly one place. A few long blocks were cut at a sentence so each part sits in the step that can build it, and a few of those parts were reworded to stand alone. `tools/check_coverage.py` fails if an item isn't in exactly one place, its text has changed, or the frozen plan has changed, and with `--code` it fails if a finished step has an untagged one.

## Steps

| Step | Title | Phase | Depends on | Status |
|---|---|---|---|---|
| 01 | [Repo bootstrap and CI](steps/01-bootstrap.md) | Foundation | none | in review |
| 02 | [Proofs of the risky parts](steps/02-proofs.md) | Foundation | 01 | todo |
| 03 | [Freeze the contracts](steps/03-contracts-freeze.md) | Foundation | 02 | todo |
| 04 | [`#[model]`, names and types](steps/04-model-core.md) | Models | 03 | todo |
| 05 | [Type inference and decoding](steps/05-model-types.md) | Models | 04 | todo |
| 06 | [Bases, indexes and constraint names](steps/06-bases-indexes.md) | Models | 04 | todo |
| 07 | [Views, input, patch and secret](steps/07-views.md) | Models | 06, 05 | todo |
| 08 | [Group declarations and masks](steps/08-groups.md) | Models | 07 | todo |
| 09 | [Declarative rules and `clean`](steps/09-validation.md) | Models | 04 | todo |
| 10 | [Config, TLS and connecting](steps/10-config.md) | Runtime | 03 | todo |
| 11 | [`morm::Pool`](steps/11-pool.md) | Runtime | 10 | todo |
| 12 | [`db.hold()`, jobs and PgBouncer](steps/12-hold-pooler.md) | Runtime | 11 | todo |
| 13 | [Numbers and special types](steps/13-types.md) | Runtime | 05 | todo |
| 14 | [Statement caches and fragments](steps/14-statements.md) | Runtime | 11, 13 | todo |
| 15 | [Reads](steps/15-reads.md) | Runtime | 14, 07 | todo |
| 16 | [Raw SQL, related rows and keyset pages](steps/16-reads-more.md) | Runtime | 15 | todo |
| 17 | [Inserts](steps/17-inserts.md) | Runtime | 14, 07 | todo |
| 18 | [Updates and deletes](steps/18-updates.md) | Runtime | 16, 17 | todo |
| 19 | [Upsert](steps/19-upsert.md) | Runtime | 18 | todo |
| 20 | [Transactions](steps/20-transactions.md) | Runtime | 18 | todo |
| 21 | [Pool deadlocks](steps/21-pool-deadlocks.md) | Runtime | 20, 12 | todo |
| 22 | [Locks, timeouts and cancellation](steps/22-locks-timeouts.md) | Runtime | 21, 19 | todo |
| 23 | [JSON output and schemas](steps/23-json.md) | Runtime | 16, 17 | todo |
| 24 | [Hooks and write ordering](steps/24-hooks.md) | Runtime | 22, 09 | todo |
| 25 | [Hooks on upsert, batches and shutdown](steps/25-hooks-more.md) | Runtime | 24 | todo |
| 26 | [Groups on queries](steps/26-group-enforcement.md) | Runtime | 23, 25, 08 | todo |
| 27 | [Transactional outbox](steps/27-outbox.md) | Runtime | 25 | todo |
| 28 | [Registration and `morm-mgr`](steps/28-manifest.md) | Migrations | 06, 05 | todo |
| 29 | [Schema IR, catalog and scratch](steps/29-schema-ir.md) | Migrations | 28, 10 | todo |
| 30 | [Diff and typed ops](steps/30-schema-diff.md) | Migrations | 29 | todo |
| 31 | [`lint`](steps/31-lint.md) | Migrations | 30, 07, 13 | todo |
| 32 | [Proof pipeline, files and decisions](steps/32-proof-files.md) | Migrations | 31 | todo |
| 33 | [Migration execution and recovery](steps/33-execution.md) | Migrations | 32 | todo |
| 34 | [Rollback and crash recovery](steps/34-recovery.md) | Migrations | 33 | todo |
| 35 | [Managed objects: declarations](steps/35-managed-decl.md) | Migrations | 34 | todo |
| 36 | [Managed objects: catalog and ops](steps/36-managed-ops.md) | Migrations | 35, 18 | todo |
| 37 | [Managed objects: dependencies](steps/37-managed-deps.md) | Migrations | 36 | todo |
| 38 | [Retire, purge and down with data](steps/38-retire.md) | Migrations | 37 | todo |
| 39 | [Safe forms](steps/39-safe-forms.md) | Migrations | 38 | todo |
| 40 | [Production mode, revert and squash](steps/40-live-modes.md) | Migrations | 39 | todo |
| 41 | [Rolling deploys](steps/41-rolling.md) | Migrations | 40, 15 | todo |
| 42 | [`morm-admin init` and project setup](steps/42-admin-init.md) | Tools | 38, 27, 17, 16 | todo |
| 43 | [`morm-admin doctor`](steps/43-doctor.md) | Tools | 42 | todo |
| 44 | [`morm-admin import`](steps/44-import.md) | Tools | 42 | todo |
| 45 | [`sql!`, `check-sql`, test support and the loop detector](steps/45-query-tooling.md) | Tools | 42 | todo |
| 46 | [`morm-admin size`](steps/46-size.md) | Tools | 11, 43, 45 | todo |
| 47 | [Telemetry](steps/47-telemetry.md) | Tools | 14, 25 | todo |
| 48 | [Benchmarks](steps/48-benchmarks.md) | Finish | 26, 27, 47, 45 | todo |
| 49 | [Lock-order detector and hardening](steps/49-hardening.md) | Finish | 26, 27, 41, 45, 47 | todo |
| 50 | [README and 0.1.0](steps/50-readme-release.md) | Finish | 48, 49, 44, 46, 43 | todo |

Status is one of `todo`, `in progress`, `in review` or `done`. Only the coordinator changes it.

## Order

Each wave below needs only earlier waves. Each step file's "Can run beside" line says which steps it can run with: no dependency path between them, and no shared area of the code. Steps that aren't listed there run one after another, even in the same wave.

1. 01
2. 02
3. 03
4. 04, 10
5. 05, 06, 09, 11
6. 07, 12, 13, 28
7. 08, 14, 29
8. 15, 17, 30
9. 16, 31
10. 18, 23, 32
11. 19, 20, 33
12. 21, 34
13. 22, 35
14. 24, 36
15. 25, 37
16. 26, 27, 38, 47
17. 39, 42
18. 40, 43, 44, 45
19. 41, 46, 48
20. 49
21. 50

Steps run in parallel only when `contracts.md` covers what they share. Each gets its own worktree and branch. When a parallel step needs something shared that isn't in `contracts.md`, it stops, and the contract is updated first.

## The coordinating session

The coordinator stays thin: it reads this file and never reads code.

1. Pick steps whose dependencies are all `done`, and mark them `in progress`.
2. Start a fresh agent for each one, with this prompt: read `plans/plan-approved/rules.md`, `contracts.md`, `proofs.md` and `steps/NN-name.md`, then do the step.
3. When it opens a PR, mark it `in review` and start a fresh review agent as `rules.md` describes.
4. Relay findings until the review is clean, then merge or ask the owner, as `rules.md` says.
5. Mark the step `done`. Take any design question straight to the owner.

## Decisions since the split

- **`morm-core` added:** a sixth crate for config loading, the name and budget functions and manifest types, because `morm-migrate` and `morm-admin` can't depend on `morm` but need all three. Confirmed by the owner.
- **Crate names reserved:** all six are published on crates.io at `0.0.0`, as placeholders.
- **Repo settings:** secret scanning, push protection and Dependabot alerts are on. `main` blocks force-pushes and deletion, and admins are exempt. Commits keep their existing author details: the owner chose not to rewrite history. `ci` is a required check on `main`, from GitHub Actions only, and PR branches needn't be up to date. The `crates-io` environment deploys only from `v*` tags, needs biggodev's approval, and holds `CARGO_REGISTRY_TOKEN`. `SECURITY.md` gives `biggodev@gmail.com`, confirmed by the owner. A tag ruleset lets only admins create, move or delete `v*` tags, and admins can't skip the `crates-io` approval. `main` needs no review: biggodev can't approve its own PRs, and the admin override would skip `ci` too.
- **The repo is public:** Actions is free, so the full matrix always runs. Step 01 adds workflow hardening, `SECURITY.md` and Dependabot, and `rules.md` treats outside text as data.
- **Rust data hooks run from `morm-mgr`:** owner decision. `migrate --hooks ./morm-mgr` calls a binary built in CI and shipped with the migrations, so `morm-mgr` is no longer print-only. Dropping hooks from 0.1 was the alternative.
- **`db.upsert_many` named:** the plan said "batch upsert" with no call. Confirmed by the owner.
- **`morm::pg_outbox!()` given its own requirement,** because the only text about it was an example.
- **Doctor, `lint` and `import` rules gathered:** sentences about them moved from their topic into those tools' own steps, so no earlier step depends on a later tool.
- **Empty CI suites get test obligations:** a job with nothing to run passes empty, and its tag already counts as covered. So each step that introduces a migration, property, full-chain or fuzz suite, or fault injection, carries a test obligation proving its CI job runs real tests. Owner decision.
- **"Full chain" is defined in step 34:** no single step builds the chain, and 34 is the first that can run all up, all down, all up. Nightly keeps CI's migration command until then, and step 49 widens the job to the whole chain, including managed objects. Owner decision.
- **`github-release` stays:** R-0795 asks only for built binaries, but they need a download location that doesn't expire. `contents: write` stays limited to that job, which runs no repo code. Owner decision.
- **Release tags are limited:** `crates-io` deploys only from `v*` tags and needs its reviewer, and the release workflow checks that the commit is on `main` and the tag matches the workspace version. Owner decision.
- **Managed type declarations:** no step could declare domains, composite types or sequences, and enums never reached the manifest, though step 36 and the full chain rely on all four. `#[pg_domain]`, `#[pg_composite]` and `#[pg_enum]` sit on the Rust type, and without one the derive only binds to an existing type. `morm::pg_sequence!` covers sequences, which apply with types, before tables, so there's no `OWNED BY`. A bare derive binds to the snake_case of the Rust name, and `#[pg_type(name = "...")]` overrides that without turning on management. `check` is optional on `#[pg_domain]`, and dropping a sequence is `lossy`. Owner decision.
- **Managed enums get full management:** including removing and reordering values, which Postgres has no command for, so morm must swap the type. `TextEnum` stays the preferred path, and the README says so loudly. Removing or reordering runs online by default, with `--fast` for one transaction. Dropping management to keep only a bind-only `PgEnum` was the alternative, and the owner chose to keep full management. The design was tested on Postgres 18 and goes in new steps, and its open proofs must stop and ask the owner if a result contradicts it. Owner decision.
- **Managed enums take new steps:** `--fast` refuses a rewrite above `max_rewrite_mb`, 1024 by default. Enum rules also bring wider stale-connection recovery with a pool generation counter, `Error::SchemaChanged`, and `#[pg_enum(deprecated)]`, `was` and `value`. A held connection never moves on a stale error: it returns `Error::SchemaChanged` and is replaced on release. Owner decisions.
- **Five large steps split:** 02 gives proofs 13 to 17 to new step 55, 13 gives its enum work to 56, 36 gives enums, domains, composites and sequences to 57, 51 gives removal and inverses to 58, and 53 gives stale functions and standbys to 59. The owner approves step 55's PR. Owner decision.
- **Fallback variants have no label:** they exist only in Rust, so they aren't in a CHECK, the type or `check_schema`. Owner decision.
- **`import` never writes `TextEnum`:** a `text` column with a CHECK stays `String` with `checks(...)`, because the catalog can't show that a CHECK is an enum. Owner decision.
- **Changing a domain or composite:** a domain's `check` is replaced `NOT VALID`, then validated in a separate step. When no column uses a composite, directly or through an array, a range, a domain or another composite, adding at the end, dropping the last attribute and changing an attribute's type use `ALTER TYPE`, and any other shape drops and recreates the type in one transaction, carrying what depends on it. A changed base type, or a composite a column uses, is refused with exit `2`. Owner decision.
- **Reviews trace plan changes:** the checker only proves step files and `manifest.json` agree, so a review confirms each changed ID has a Decisions entry here. Owner decision.
- **Third review corrections:** these make owner-decided rules buildable and add no new behaviour. Retries: R-0764, R-0981, R-0983, T-0152. Scanner in `morm-core`: R-0433, R-1051. Online swap: R-1047, R-1049, R-1051, R-1052, T-0175, T-0177. `down_map`: R-1015, T-0160. `TextEnum` CHECK: R-0593, R-1055, T-0180, T-0183. Labels and rolling: R-0961, R-0964, R-0979, R-0980, R-0990, R-0997, R-0998, R-1000, R-1056, T-0150, T-0182. CI for proofs: T-0164, T-0172. Counters: R-0741. README: R-1008, R-1058. Step edges this needed: 14 depends on 12 for the held-connection rule, and 30 depends on 56 for the CHECK test. The owner approves all of this through PR #1.
- **Ops can extend the engine:** an op can end its step and add its own decisions, `lint` findings, proof rows, `op_data` and preflight checks, so the enum steps change nothing in steps 30 to 33. R-0587, R-0608, R-0987, R-0988, R-0989. The owner approves this through PR #1.
- **Fourth review corrections:** an add's inverse in step 51 is a plain swap back to the old labels, with `down_map` and row counts arriving in step 58. `TextEnum` labels reach the manifest, and the with-data proof gets rows for enum and composite columns. R-0991, R-1027, R-1055, R-1059, T-0185, step 55's workflow rules, and notes on who updates earlier tests in steps 40, 53 and 59. The owner approves these through PR #1.
- **Composite dependents split:** step 57 refuses a composite change that needs a dependent dropped, and `migrate` preflight repeats the checks on the live database, because scratch can't see an undeclared column. Step 37 carries managed dependents, because it owns recreation. A column using a composite through a range is refused too, because Postgres loses that data with no error. Step 56 depends on 28, which defines the manifest it writes labels into. R-1059, R-1060, T-0185, T-0186. The owner approves this through PR #1.
- **Fifth review corrections:** `rollback` repeats the composite checks before an inverse that drops an attribute, because Postgres loses that data with no error. Carried objects keep their owner, comment and grants. A composite recreate refreshes stale plpgsql the way an enum swap does, and unmanaged functions become a decision, in step 59. R-1038, R-1039, R-1059, R-1060, T-0168, T-0185, T-0186. Composite recreate: owner decision; the rest: the owner approves through PR #1.
- **Stale plpgsql, one general rule:** the source search, refresh and decision run on any op that changes a column's type, or drops and recreates a type or view, in place of listed cases, because tests on Postgres 17 and 18 found stale functions in each. The search also covers every domain, composite type and view replaced along with it. R-1038, R-1039, R-1040, T-0168. Owner decision.
- **Online column swap carries views:** a managed view above the column is dropped and recreated on the new column in the short cutover transaction, locking the view before the table, and a view morm doesn't manage is refused with the full list. A materialized view can't be rebuilt under that lock, so a new one is built beside the old on the new column, as late as possible, and swapped by rename at cutover. Its rebuild opens a decision saying the contents come from today's table, and the old view is kept as a retired object until `purge`, because a view can hold rows the table has lost. One created `WITH NO DATA` stays unpopulated. The enum online form follows the same rules. R-0566, R-0571, R-1049, R-1061, T-0187. Owner decision.
- **Stale connections recover in morm apps:** R-0981 also replaces a connection on "type of parameter ... does not match that when preparing the plan". R-0981, R-1040. Owner decision: no gaps left open.
- **The source search covers every schema:** a body in another schema that names the changed object is found too. R-0572, R-1038. Owner decision: no gaps left open.
- **Managed functions are proven by a real call:** a managed function or procedure whose source names a changed object is called on scratch after the change, and a failure stops `makemigrations` with exit `2`, so a `RETURNS TABLE` shape that no longer fits never ships. One with arguments declares its call with `proof`, and one that needs a call and has none also stops with exit `2`. Trigger functions are proven by a write. Unmanaged functions can't be proven, so they stay a migration decision naming the function. Owner decision: no gaps left open.
- **Cutover and view rewrites:** a cutover takes each view's lock with its own `DROP VIEW`, outside in, then locks the table and walks `pg_depend` again, because `LOCK TABLE` on a view deadlocked. A view's query is rewritten by renaming the column on scratch and taking `pg_get_viewdef`, and a `USING` view counts as matching if the scratch proof shows the same meaning. The real-call rule also covers R-0572's retires and renames. `integer out of range` from a `%TYPE` variable stays in the migration decision's text, which says apps must reconnect, because `22003` can't be matched safely. R-0572, R-0593, R-0981, R-0983, R-1040, R-1047, R-1049, R-1050, R-1051, R-1052, R-1061 to R-1069, T-0122, T-0152, T-0175, T-0187 to T-0193. Owner decisions.
- **Function paths are proven statically where it matters:** a real call only proves the path it takes, and plpgsql is checked only as each statement first runs. So `plpgsql_check` is required on scratch when a managed plpgsql function names a changed object. Without it, `makemigrations` stops and says to install it, or to add `proof` lines and accept the risk through a decision. A function can have several `proof` lines. A body using `EXECUTE` can't be checked statically, so it must have a `proof`, and the README says what that leaves unproven. Owner decision.
- **How `plpgsql_check` runs:** it runs in its own scratch database, so the proof role keeps its limits. Any error stops `makemigrations`. A false one is silenced with `PERFORM 'PRAGMA:...'` in the function, or accepted through a decision naming the function, and the README says so loudly, because silencing a check can hide a real break. A narrowing such as an `int` variable reading a `bigint` column is a decision naming the function. `EXECUTE` is found by a word search that may ask for a `proof` it didn't need, and morm refuses to run with `plpgsql_check.mode` disabled. R-0572, R-1038, R-1065 to R-1068, R-1076 to R-1091, T-0191, T-0192, T-0198 to T-0205, and step 55's proof 18. Owner decisions.
- **`plpgsql_check` closing rules:** a superuser creates one `morm_plpgsql_check` template per scratch server, and morm clones it with `CREATEDB`. Any change to a column's type, length or precision, when a managed plpgsql function names that column, opens a decision naming the function, because the tool can't see those limits. A trigger function with no trigger must have a `proof`, as with `EXECUTE`. A narrowing is a cast pair that isn't implicit in `pg_cast`. If proof 18 shows `@plpgsql_check_options` can hide an error, morm refuses functions containing it. Owner decisions.
- **Exit codes:** every open decision exits `3`, and `2` is kept for hard refusals, as R-0588 says. A run with both exits `2`, because answering the decision wouldn't let it through. A recorded `refuse` answer that `migrate` enforces stays `2`. `migrate` and `rollback` take `--decide` in the same form as `makemigrations`, recorded in the ledger, for decisions that only appear against the live database. R-0572, R-0588, R-0593, R-0608, R-1015, R-1040, R-1076, R-1078 to R-1080, T-0168, T-0184, T-0198 to T-0202. Owner decisions.
- **Functions that reach a column indirectly:** the source search repeats on the name of each function it finds until nothing new turns up, so call chains are covered. Any column type, length or precision change also opens a decision for every managed function using `EXECUTE`, because no tool can see what a built query reads. R-1040's reconnect text names both `integer out of range` and `numeric field overflow`. R-1038, R-1040, R-1068, R-1085, R-1092 to R-1094, T-0206, T-0207. Owner decisions.
- **Answering decisions at `migrate` and `rollback`:** an answer stands only for the same migration id, checksum, direction and decision. A live decision's ID is built from the migration id, kind and function signature, plus a hash of the function's source. Answers go in a new `_morm.decisions` table with a history row, and the ledger keeps `meta.json` too, so `rollback` can search when the files are gone. Owner decisions.
- **Stale function search, wider:** any column type change also opens one decision listing every unmanaged `EXECUTE` function on the live database. `pg_depend` is followed to views, defaults and checks, so they get the same decision. R-0572's search ignores case. The reconnect text says "such as" and names `integer out of range`, `numeric field overflow` and `value too long`. R-0572, R-0608, R-1040, R-1081, T-0184, T-0208, T-0209. Owner decisions.
- **One general walk:** after a column's type, length or precision changes, morm walks everything that reaches it, of any kind, and repeats until nothing new turns up. It follows `pg_depend` to every dependent object, and the source search on the name of everything found, including operator and aggregate symbols. Everything found gets the same decision, as one list per migration. This replaces the lists of kinds. `rehearse` takes `--decide` too. A `BEGIN ATOMIC` body whose hash changes after a rename asks again on `--resume`. R-0597, R-0615, R-0950, R-0951, R-1069, R-1095 to R-1100, T-0122, T-0210 to T-0216. Owner decisions.
- **What the walk lists, and its limit:** only objects that carry code or an expression: functions, procedures, views, triggers, policies, rules, expression and partial indexes, and non-constant defaults and checks, because a plain index or `NOT NULL` can't hold a stale type. It counts any such object on the changed table, not just on that column, so a whole-row read is caught. A value can flow further than any tool can follow, through arguments, other tables and application code, so every widening's decision carries a fixed line saying values can exceed the old limit anywhere that reads them, including code morm can't see, and the README says so. This is the owner-accepted limit of the walk, stated rather than hidden. A `rehearse` rerun carries on from where it stopped. Other ops use the same walk, each hit with its own decision. Preflight skips objects already accepted with the same hash, and `--resume` re-asks for deparsed objects whose text changes after a rename. R-0952, R-1094, R-1101, T-0206, T-0217. Owner decisions.
- **Walk details:** on other columns of the changed table, a default or check that only calls built-ins isn't listed, because it can't depend on the changed column. A widening with nothing to list still prints the fixed line in `makemigrations` output and as a comment in the migration, without stopping. `check_schema`'s accepted type pairs, including length and precision, are listed in the plan. Built-in functions are never listed. A view's rewrite rule counts as code. Expressions in exclusion constraints, statistics objects, partition keys and publication filters are listed. A pair morm can't classify gets the fixed line. R-0612, R-1102, T-0218, T-0219. Owner decisions.
- **Cutover defaults and pushing through:** 50 ms lock timeout, 250 ms pause, 600 attempts, about 3 minutes, then it stops and `migrate --resume` carries on. `--terminate-blockers` on `migrate` and `rollback` ends what's blocking the cutover, opt-in, because waiting longer didn't help. The transient `BEGIN ATOMIC` error is retried on the same connection. R-0981, R-0983, R-1049, R-1061, R-1072 to R-1075, T-0152, T-0196, T-0197. Owner decisions.
- **A cutover gives way to live transactions:** it uses a short lock timeout and retries with a pause, and if it can't get in, it stops with nothing changed and can be run again. A migration never kills a live app transaction by default. An opt-in flag lets someone push it through. Owner decision.
- **Same-meaning checks and `USING` views:** two view definitions match if their stored query trees match once position and alias details are removed. A `USING` view binds columns by position, so preflight refuses the online swap on a table where an unmanaged column could return wrong rows, and `--fast` handles it. A transient `could not open relation with OID` during a cutover is retried by morm apps, and the README says other clients can see one such error. R-0981, R-0983, R-1070, R-1071, T-0178, T-0184, T-0194, T-0195. The online swap no longer uses the scanner. Owner decisions.
- **Finishing step 01's review:** after six rounds, the last text edit gets a short check of that edit, the checker and CI, then goes to the owner. Later steps' reviews cover the design as it's built. Owner decision.
- **Repo tooling moved to later work:** the stronger failure-hiding check and proving `actionlint`, a real release and an expired artifact are in `later-defo-build.plan`, because they're project tooling, not morm itself. Owner decision.
- **Pinned commands and the failure-hiding check:** the exact lists also keep `binaries`, `github-release` and the `fuzz` step's text, as suite commands. The line-scanning check stays as it is, with its known limits. Owner decisions.
- **`TextEnum` labels keep the Rust spelling:** `Pending` is stored as `Pending`, unlike `PgEnum`'s snake_case. It gets the same opt-in `unknown` fallback as `PgEnum`. Owner decision.

## Record from the frozen plan

Superseded milestones, later work and build decisions, kept for traceability. Nothing here is a step requirement.

### Milestones

- **R-0904** **Repo bootstrap:** workspace in `biggaloy/rust-morm`, CI workflows, README skeleton.
- **R-0905** **Core models:** macro, `schema`, every integer type with range CHECKs, bases including `#[morm::base]`, index and name rules, views, global and model-scoped groups, declarative rules and `clean`, inference, `F`, compile-time checks.
- **R-0906** **Runtime:** `morm.toml` and environment variables, `morm::Pool` on the `deadpool` core (start with a quick proof-of-concept), `[server]`, dynamic size, named pools, `max_connections` adjustment, `db.hold()`, bounded `statement_cache` and `shape_cache`, insert, `get`/`update`/`delete` forms, `patch` views and `.apply()`, query builder, `filter_cols`, `db.sql()` and `#[derive(morm::Row)]`, `keyset`, `.in_batches`, `order_by_allowed`, `.custom_plan()`, `get_for_update`, failover handling, `Decimal` and `Numeric`, loop detector, typed errors, transactions, deadlock measures, all list and JSON paths, calibrated per-connection memory figures for `size`. → **First publish** (`0.1.0-alpha`).
- **R-0907** **Migration foundation:** `Schema` IR, catalog reader, diff, typed ops with inverses, scratch, proof pipeline.
- **R-0908** **Migration execution:** ledger, steps, fingerprint gates, advisory lock, timeouts, preflight, crash recovery, fault-injection suite.
- **R-0909** **Migration safety:** safe forms, batched backfills, retire and purge, production mode, `revert`, `squash`, decisions, `lint`, `rehearse`, expand/contract, `check_schema()`.
- **R-0910** **Managed objects:** views, materialized views, functions, procedures, triggers, policies, grants, enums and domains, with dependency recreation.
- **R-0911** **Hooks and outbox:** `Changes`, `Target`, all hooks, `validate`, `validate_changes`, `after_commit`, automatic transactions, `pg_outbox!`, `emit`, the worker and `morm-admin outbox`.
- **R-0912** **`morm-admin`:** `init`, `doctor`, `import`, `scratch`, `size`, `morm-mgr` packaging, Axum template.
- **R-0913** **Query safety:** `sql!`, `check-sql`, upsert, UNNEST and COPY, `#[morm::test]`.
- **R-0914** **Remaining features:** UUIDv7, `tls`, schemas and `utoipa`, optimistic locking, tracing, `metrics`, `[telemetry]`, `.tag()`, typed FKs, settings, pipelined transactions.
- **R-0915** **Benchmarks, lock-order detector, hardening, full README, 0.1.0 release.**

### Risks, open items and later work / Risks

- **R-0916** **Managed objects:** functions, triggers and policies with exact inverses are the hardest engine work.
- **R-0919** **Scope:** migrations alone outweigh the runtime.

### Risks, open items and later work / Later

**R-0921** Shared reads: a small opt-in lane that pipelines primary-key reads on a few connections, added only if a benchmark shows real gains with no worse p99 under mixed load. Also partitioning, a `dynamic` registry for by-name admin CRUD, sensitive-column prompts in `import`, exclusion constraints on models, and `morm-admin size --live`, which compares the estimate with `pg_stat_activity` and live pool figures to spot oversized pools.

### Risks, open items and later work / Confirmed for after 0.1

**R-0922** Features we will build are in `later-defo-build.plan`: automatic read routing and online key type changes across foreign keys.

### Risks, open items and later work / Not carried over from python-morm

- **R-0926** **`save`:** replaced by explicit `insert` and `update`.
- **R-0927** **`exclude_values_up/down`:** rarely useful, and it would make insert SQL vary per row. `Option` `None` already defers to the DB default, and `skip_json_if` covers output. It can come back if a real need appears.
- **R-0928** **`validator` and `modifier`:** replaced by the type system, native decoding, and declarative rules.
- **R-0929** **`pre_save` and `post_save`:** removed with `save`.

### How this gets built

**R-0930** After the final review, this plan is split into `plans/plan-approved/`:

- **R-0931** `INDEX.md`: order, dependencies, what can run in parallel, and status.
- **R-0932** `rules.md`: how every step works and when it counts as done.
- **R-0933** `contracts.md`: shared types and signatures that parallel steps rely on.
- **R-0934** `coverage.md`: every requirement, by ID, mapped to its step and tests. A script fails if a requirement has no step or no test.
- **R-0935** `steps/`: one self-contained file per step, about 26 in total, small enough for one fresh session.

**R-0936** The first splitting step assigns the requirement IDs, because this file has none.

**R-0937** **Decisions:**

- **R-0938** `plan-approved/` becomes the working source of truth once it's built. This file is then frozen as the record, so two live copies can't drift apart.
- **R-0939** Steps and reviews run as subagents, each with fresh context, and each reads only `rules.md`, `contracts.md` and its own step file. A thin coordinating session tracks `INDEX.md`.
- **R-0940** The owner approves the Foundation PRs. Later PRs merge once CI and an independent review both pass.

**R-0941** **Rules for every step:**

- **R-0942** The session that writes a step doesn't approve it. A separate one checks the work against the step file.
- **R-0943** Done means every test passes on Postgres 15–18, with nothing deferred or stubbed.
- **R-0944** If a step finds the plan is wrong, it stops and asks, and it doesn't improvise.
- **R-0945** Parallel steps use separate worktrees and branches, and run in parallel only when `contracts.md` covers what they share.
- **R-0946** The early proofs step comes before anything depends on the risky parts in section 16.
