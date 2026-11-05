# rust-morm: approved plan

This folder is the source of truth for building rust-morm. `../rust-morm-plan.md` is frozen as the record. `../app_suggestions.md` holds advice for apps, and `../later-defo-build.plan` holds confirmed work after 0.1.

| File | Holds |
|---|---|
| `rules.md` | How every step works, and when it's done |
| `contracts.md` | Shared types and conventions, plus the key decisions |
| `coverage.md` | Every requirement and test obligation, with its step |
| `steps/` | One self-contained file per step |
| `tools/` | The coverage checker, and the scripts that made this split |

The frozen plan was split word for word into requirements (`R-nnnn`) and test obligations (`T-nnnn`), each in exactly one place. A few long blocks were cut at a sentence so each part sits in the step that can build it, and five of those parts were reworded to stand alone. `tools/check_coverage.py` fails if an item isn't in exactly one place or its text has changed, and with `--code` it fails if a finished step has an untagged one.

## Steps

| Step | Title | Phase | Depends on | Status |
|---|---|---|---|---|
| 01 | [Repo bootstrap and CI](steps/01-bootstrap.md) | Foundation | none | todo |
| 02 | [Proofs of the risky parts](steps/02-proofs.md) | Foundation | 01 | todo |
| 03 | [Freeze the contracts](steps/03-contracts-freeze.md) | Foundation | 02 | todo |
| 04 | [`#[model]`, names and types](steps/04-model-core.md) | Models | 03 | todo |
| 05 | [Bases, indexes and constraint names](steps/05-bases-indexes.md) | Models | 04 | todo |
| 06 | [Views, input, patch and secret](steps/06-views.md) | Models | 05 | todo |
| 07 | [Group declarations and masks](steps/07-groups.md) | Models | 06 | todo |
| 08 | [Declarative rules and `clean`](steps/08-validation.md) | Models | 04 | todo |
| 09 | [Config, TLS and connecting](steps/09-config.md) | Runtime | 03 | todo |
| 10 | [`morm::Pool`](steps/10-pool.md) | Runtime | 09 | todo |
| 11 | [`db.hold()`, jobs and PgBouncer](steps/11-hold-pooler.md) | Runtime | 10 | todo |
| 12 | [Numbers and special types](steps/12-types.md) | Runtime | 04 | todo |
| 13 | [Statement caches and fragments](steps/13-statements.md) | Runtime | 10, 12 | todo |
| 14 | [Reads](steps/14-reads.md) | Runtime | 13, 06 | todo |
| 15 | [Inserts](steps/15-inserts.md) | Runtime | 13, 06 | todo |
| 16 | [Updates and deletes](steps/16-updates.md) | Runtime | 14, 15 | todo |
| 17 | [Upsert](steps/17-upsert.md) | Runtime | 16 | todo |
| 18 | [Transactions](steps/18-transactions.md) | Runtime | 16 | todo |
| 19 | [Pool deadlocks](steps/19-pool-deadlocks.md) | Runtime | 18, 11 | todo |
| 20 | [Locks, timeouts and cancellation](steps/20-locks-timeouts.md) | Runtime | 19, 17 | todo |
| 21 | [JSON output and schemas](steps/21-json.md) | Runtime | 14 | todo |
| 22 | [Hooks and write ordering](steps/22-hooks.md) | Runtime | 20, 08 | todo |
| 23 | [Groups on queries](steps/23-group-enforcement.md) | Runtime | 21, 22, 07 | todo |
| 24 | [Transactional outbox](steps/24-outbox.md) | Runtime | 22 | todo |
| 25 | [Registration and `morm-mgr`](steps/25-manifest.md) | Migrations | 05 | todo |
| 26 | [Schema IR, scratch and typed ops](steps/26-schema-diff.md) | Migrations | 25, 09 | todo |
| 27 | [`lint`](steps/27-lint.md) | Migrations | 26 | todo |
| 28 | [Proof pipeline, files and decisions](steps/28-proof-files.md) | Migrations | 27 | todo |
| 29 | [Migration execution and recovery](steps/29-execution.md) | Migrations | 28 | todo |
| 30 | [Managed objects: declarations](steps/30-managed-decl.md) | Migrations | 29, 16 | todo |
| 31 | [Managed objects: dependencies](steps/31-managed-deps.md) | Migrations | 30 | todo |
| 32 | [Retire, purge and down with data](steps/32-retire.md) | Migrations | 31 | todo |
| 33 | [Safe forms](steps/33-safe-forms.md) | Migrations | 32 | todo |
| 34 | [Production mode, revert and squash](steps/34-live-modes.md) | Migrations | 33 | todo |
| 35 | [Rolling deploys](steps/35-rolling.md) | Migrations | 34, 14 | todo |
| 36 | [`morm-admin` core](steps/36-admin-core.md) | Tools | 32, 24, 15, 14 | todo |
| 37 | [`morm-admin import`](steps/37-import.md) | Tools | 36 | todo |
| 38 | [`morm-admin size`](steps/38-size.md) | Tools | 10, 36 | todo |
| 39 | [`sql!`, `check-sql`, test support and the loop detector](steps/39-query-tooling.md) | Tools | 36 | todo |
| 40 | [Telemetry](steps/40-telemetry.md) | Tools | 13, 22 | todo |
| 41 | [Benchmarks](steps/41-benchmarks.md) | Finish | 23, 24, 40, 39 | todo |
| 42 | [Lock-order detector and hardening](steps/42-hardening.md) | Finish | 23, 24, 35, 39, 40 | todo |
| 43 | [README and 0.1.0](steps/43-readme-release.md) | Finish | 41, 42, 37, 38 | todo |

Status is one of `todo`, `in progress`, `in review` or `done`. Only the coordinator changes it.

## Order

Each wave below needs only earlier waves. Each step file's "Can run beside" line says which steps it can run with: no dependency path between them, and no shared area of the code. Steps that aren't listed there run one after another, even in the same wave.

1. 01
2. 02
3. 03
4. 04, 09
5. 05, 08, 10, 12
6. 06, 11, 13, 25
7. 07, 14, 15, 26
8. 16, 21, 27
9. 17, 18, 28
10. 19, 29
11. 20, 30
12. 22, 31
13. 23, 24, 32, 40
14. 33, 36
15. 34, 37, 38, 39
16. 35, 41
17. 42
18. 43

Steps run in parallel only when `contracts.md` covers what they share. Each gets its own worktree and branch. When a parallel step needs something shared that isn't in `contracts.md`, it stops, and the contract is updated first.

## The coordinating session

The coordinator stays thin: it reads this file and never reads code.

1. Pick steps whose dependencies are all `done`, and mark them `in progress`.
2. Start a fresh agent for each one, with this prompt: read `plans/plan-approved/rules.md`, `contracts.md` and `steps/NN-name.md`, then do the step.
3. When it opens a PR, mark it `in review` and start a fresh review agent as `rules.md` describes.
4. Relay findings until the review is clean, then merge or ask the owner, as `rules.md` says.
5. Mark the step `done`. Take any design question straight to the owner.

## Decisions since the split

- **`morm-core` added:** a sixth crate for config loading, the name function and manifest types, because `morm-migrate` and `morm-admin` can't depend on `morm` but need all three. The owner should confirm this.
- **Doctor, `lint` and `import` rules gathered:** sentences about them moved from their topic into those tools' own steps, so no earlier step depends on a later tool.

## Record from the frozen plan

Superseded milestones, later work and build decisions, kept for traceability. Nothing here is a step requirement.

### Milestones

- **R-0882** **Repo bootstrap:** workspace in `biggaloy/rust-morm`, CI workflows, README skeleton.
- **R-0883** **Core models:** macro, `schema`, every integer type with range CHECKs, bases including `#[morm::base]`, index and name rules, views, global and model-scoped groups, declarative rules and `clean`, inference, `F`, compile-time checks.
- **R-0884** **Runtime:** `morm.toml` and environment variables, `morm::Pool` on the `deadpool` core (start with a quick proof-of-concept), `[server]`, dynamic size, named pools, `max_connections` adjustment, `db.hold()`, bounded `statement_cache` and `shape_cache`, insert, `get`/`update`/`delete` forms, `patch` views and `.apply()`, query builder, `filter_cols`, `db.sql()` and `#[derive(morm::Row)]`, `keyset`, `.in_batches`, `order_by_allowed`, `.custom_plan()`, `get_for_update`, failover handling, `Decimal` and `Numeric`, loop detector, typed errors, transactions, deadlock measures, all list and JSON paths, calibrated per-connection memory figures for `size`. → **First publish** (`0.1.0-alpha`).
- **R-0885** **Migration foundation:** `Schema` IR, catalog reader, diff, typed ops with inverses, scratch, proof pipeline.
- **R-0886** **Migration execution:** ledger, steps, fingerprint gates, advisory lock, timeouts, preflight, crash recovery, fault-injection suite.
- **R-0887** **Migration safety:** safe forms, batched backfills, retire and purge, production mode, `revert`, `squash`, decisions, `lint`, `rehearse`, expand/contract, `check_schema()`.
- **R-0888** **Managed objects:** views, materialized views, functions, procedures, triggers, policies, grants, enums and domains, with dependency recreation.
- **R-0889** **Hooks and outbox:** `Changes`, `Target`, all hooks, `validate`, `validate_changes`, `after_commit`, automatic transactions, `pg_outbox!`, `emit`, the worker and `morm-admin outbox`.
- **R-0890** **`morm-admin`:** `init`, `doctor`, `import`, `scratch`, `size`, `morm-mgr` packaging, Axum template.
- **R-0891** **Query safety:** `sql!`, `check-sql`, upsert, UNNEST and COPY, `#[morm::test]`.
- **R-0892** **Remaining features:** UUIDv7, `tls`, schemas and `utoipa`, optimistic locking, tracing, `metrics`, `[telemetry]`, `.tag()`, typed FKs, settings, pipelined transactions.
- **R-0893** **Benchmarks, lock-order detector, hardening, full README, 0.1.0 release.**

### Risks, open items and later work / Risks

- **R-0894** **Managed objects:** functions, triggers and policies with exact inverses are the hardest engine work.
- **R-0897** **Scope:** migrations alone outweigh the runtime.

### Risks, open items and later work / Later

**R-0899** Shared reads: a small opt-in lane that pipelines primary-key reads on a few connections, added only if a benchmark shows real gains with no worse p99 under mixed load. Also partitioning, a `dynamic` registry for by-name admin CRUD, sensitive-column prompts in `import`, exclusion constraints on models, and `morm-admin size --live`, which compares the estimate with `pg_stat_activity` and live pool figures to spot oversized pools.

### Risks, open items and later work / Confirmed for after 0.1

**R-0900** Features we will build are in `later-defo-build.plan`: automatic read routing and online key type changes across foreign keys.

### Risks, open items and later work / Not carried over from python-morm

- **R-0904** **`save`:** replaced by explicit `insert` and `update`.
- **R-0905** **`exclude_values_up/down`:** rarely useful, and it would make insert SQL vary per row. `Option` `None` already defers to the DB default, and `skip_json_if` covers output. It can come back if a real need appears.
- **R-0906** **`validator` and `modifier`:** replaced by the type system, native decoding, and declarative rules.
- **R-0907** **`pre_save` and `post_save`:** removed with `save`.

### How this gets built

**R-0908** After the final review, this plan is split into `plans/plan-approved/`:

- **R-0909** `INDEX.md`: order, dependencies, what can run in parallel, and status.
- **R-0910** `rules.md`: how every step works and when it counts as done.
- **R-0911** `contracts.md`: shared types and signatures that parallel steps rely on.
- **R-0912** `coverage.md`: every requirement, by ID, mapped to its step and tests. A script fails if a requirement has no step or no test.
- **R-0913** `steps/`: one self-contained file per step, about 26 in total, small enough for one fresh session.

**R-0914** The first splitting step assigns the requirement IDs, because this file has none.

**R-0915** **Decisions:**

- **R-0916** `plan-approved/` becomes the working source of truth once it's built. This file is then frozen as the record, so two live copies can't drift apart.
- **R-0917** Steps and reviews run as subagents, each with fresh context, and each reads only `rules.md`, `contracts.md` and its own step file. A thin coordinating session tracks `INDEX.md`.
- **R-0918** The owner approves the Foundation PRs. Later PRs merge once CI and an independent review both pass.

**R-0919** **Rules for every step:**

- **R-0920** The session that writes a step doesn't approve it. A separate one checks the work against the step file.
- **R-0921** Done means every test passes on Postgres 15–18, with nothing deferred or stubbed.
- **R-0922** If a step finds the plan is wrong, it stops and asks, and it doesn't improvise.
- **R-0923** Parallel steps use separate worktrees and branches, and run in parallel only when `contracts.md` covers what they share.
- **R-0924** The early proofs step comes before anything depends on the risky parts in section 16.
