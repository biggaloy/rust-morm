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
| 01 | [Repo bootstrap and CI](steps/01-bootstrap.md) | Foundation | none | todo |
| 02 | [Proofs of the risky parts](steps/02-proofs.md) | Foundation | 01 | todo |
| 03 | [Freeze the contracts](steps/03-contracts-freeze.md) | Foundation | 02 | todo |
| 04 | [`#[model]`, names and types](steps/04-model-core.md) | Models | 03 | todo |
| 05 | [Type inference and decoding](steps/05-model-types.md) | Models | 04 | todo |
| 06 | [Bases, indexes and constraint names](steps/06-bases-indexes.md) | Models | 04 | todo |
| 07 | [Views, input, patch and secret](steps/07-views.md) | Models | 06 | todo |
| 08 | [Group declarations and masks](steps/08-groups.md) | Models | 07 | todo |
| 09 | [Declarative rules and `clean`](steps/09-validation.md) | Models | 04 | todo |
| 10 | [Config, TLS and connecting](steps/10-config.md) | Runtime | 03 | todo |
| 11 | [`morm::Pool`](steps/11-pool.md) | Runtime | 10 | todo |
| 12 | [`db.hold()`, jobs and PgBouncer](steps/12-hold-pooler.md) | Runtime | 11 | todo |
| 13 | [Numbers and special types](steps/13-types.md) | Runtime | 05 | todo |
| 14 | [Statement caches and fragments](steps/14-statements.md) | Runtime | 11, 13 | todo |
| 15 | [Reads](steps/15-reads.md) | Runtime | 14, 07 | todo |
| 16 | [Inserts](steps/16-inserts.md) | Runtime | 14, 07 | todo |
| 17 | [Updates and deletes](steps/17-updates.md) | Runtime | 15, 16 | todo |
| 18 | [Upsert](steps/18-upsert.md) | Runtime | 17 | todo |
| 19 | [Transactions](steps/19-transactions.md) | Runtime | 17 | todo |
| 20 | [Pool deadlocks](steps/20-pool-deadlocks.md) | Runtime | 19, 12 | todo |
| 21 | [Locks, timeouts and cancellation](steps/21-locks-timeouts.md) | Runtime | 20, 18 | todo |
| 22 | [JSON output and schemas](steps/22-json.md) | Runtime | 15, 16 | todo |
| 23 | [Hooks and write ordering](steps/23-hooks.md) | Runtime | 21, 09 | todo |
| 24 | [Hooks on upsert, batches and shutdown](steps/24-hooks-more.md) | Runtime | 23 | todo |
| 25 | [Groups on queries](steps/25-group-enforcement.md) | Runtime | 22, 24, 08 | todo |
| 26 | [Transactional outbox](steps/26-outbox.md) | Runtime | 24 | todo |
| 27 | [Registration and `morm-mgr`](steps/27-manifest.md) | Migrations | 06 | todo |
| 28 | [Schema IR, catalog and scratch](steps/28-schema-ir.md) | Migrations | 27, 10 | todo |
| 29 | [Diff and typed ops](steps/29-schema-diff.md) | Migrations | 28 | todo |
| 30 | [`lint`](steps/30-lint.md) | Migrations | 29 | todo |
| 31 | [Proof pipeline, files and decisions](steps/31-proof-files.md) | Migrations | 30 | todo |
| 32 | [Migration execution and recovery](steps/32-execution.md) | Migrations | 31 | todo |
| 33 | [Rollback and crash recovery](steps/33-recovery.md) | Migrations | 32 | todo |
| 34 | [Managed objects: declarations](steps/34-managed-decl.md) | Migrations | 33, 17 | todo |
| 35 | [Managed objects: dependencies](steps/35-managed-deps.md) | Migrations | 34 | todo |
| 36 | [Retire, purge and down with data](steps/36-retire.md) | Migrations | 35 | todo |
| 37 | [Safe forms](steps/37-safe-forms.md) | Migrations | 36 | todo |
| 38 | [Production mode, revert and squash](steps/38-live-modes.md) | Migrations | 37 | todo |
| 39 | [Rolling deploys](steps/39-rolling.md) | Migrations | 38, 15 | todo |
| 40 | [`morm-admin init` and project setup](steps/40-admin-init.md) | Tools | 36, 26, 16, 15 | todo |
| 41 | [`morm-admin doctor`](steps/41-doctor.md) | Tools | 40 | todo |
| 42 | [`morm-admin import`](steps/42-import.md) | Tools | 40 | todo |
| 43 | [`morm-admin size`](steps/43-size.md) | Tools | 11, 41 | todo |
| 44 | [`sql!`, `check-sql`, test support and the loop detector](steps/44-query-tooling.md) | Tools | 40 | todo |
| 45 | [Telemetry](steps/45-telemetry.md) | Tools | 14, 24 | todo |
| 46 | [Benchmarks](steps/46-benchmarks.md) | Finish | 25, 26, 45, 44 | todo |
| 47 | [Lock-order detector and hardening](steps/47-hardening.md) | Finish | 25, 26, 39, 44, 45 | todo |
| 48 | [README and 0.1.0](steps/48-readme-release.md) | Finish | 46, 47, 42, 43, 41 | todo |

Status is one of `todo`, `in progress`, `in review` or `done`. Only the coordinator changes it.

## Order

Each wave below needs only earlier waves. Each step file's "Can run beside" line says which steps it can run with: no dependency path between them, and no shared area of the code. Steps that aren't listed there run one after another, even in the same wave.

1. 01
2. 02
3. 03
4. 04, 10
5. 05, 06, 09, 11
6. 07, 12, 13, 27
7. 08, 14, 28
8. 15, 16, 29
9. 17, 22, 30
10. 18, 19, 31
11. 20, 32
12. 21, 33
13. 23, 34
14. 24, 35
15. 25, 26, 36, 45
16. 37, 40
17. 38, 41, 42, 44
18. 39, 43, 46
19. 47
20. 48

Steps run in parallel only when `contracts.md` covers what they share. Each gets its own worktree and branch. When a parallel step needs something shared that isn't in `contracts.md`, it stops, and the contract is updated first.

## The coordinating session

The coordinator stays thin: it reads this file and never reads code.

1. Pick steps whose dependencies are all `done`, and mark them `in progress`.
2. Start a fresh agent for each one, with this prompt: read `plans/plan-approved/rules.md`, `contracts.md` and `steps/NN-name.md`, then do the step.
3. When it opens a PR, mark it `in review` and start a fresh review agent as `rules.md` describes.
4. Relay findings until the review is clean, then merge or ask the owner, as `rules.md` says.
5. Mark the step `done`. Take any design question straight to the owner.

## Decisions since the split

- **`morm-core` added:** a sixth crate for config loading, the name and budget functions and manifest types, because `morm-migrate` and `morm-admin` can't depend on `morm` but need all three. The owner should confirm this.
- **Doctor, `lint` and `import` rules gathered:** sentences about them moved from their topic into those tools' own steps, so no earlier step depends on a later tool.

## Record from the frozen plan

Superseded milestones, later work and build decisions, kept for traceability. Nothing here is a step requirement.

### Milestones

- **R-0898** **Repo bootstrap:** workspace in `biggaloy/rust-morm`, CI workflows, README skeleton.
- **R-0899** **Core models:** macro, `schema`, every integer type with range CHECKs, bases including `#[morm::base]`, index and name rules, views, global and model-scoped groups, declarative rules and `clean`, inference, `F`, compile-time checks.
- **R-0900** **Runtime:** `morm.toml` and environment variables, `morm::Pool` on the `deadpool` core (start with a quick proof-of-concept), `[server]`, dynamic size, named pools, `max_connections` adjustment, `db.hold()`, bounded `statement_cache` and `shape_cache`, insert, `get`/`update`/`delete` forms, `patch` views and `.apply()`, query builder, `filter_cols`, `db.sql()` and `#[derive(morm::Row)]`, `keyset`, `.in_batches`, `order_by_allowed`, `.custom_plan()`, `get_for_update`, failover handling, `Decimal` and `Numeric`, loop detector, typed errors, transactions, deadlock measures, all list and JSON paths, calibrated per-connection memory figures for `size`. → **First publish** (`0.1.0-alpha`).
- **R-0901** **Migration foundation:** `Schema` IR, catalog reader, diff, typed ops with inverses, scratch, proof pipeline.
- **R-0902** **Migration execution:** ledger, steps, fingerprint gates, advisory lock, timeouts, preflight, crash recovery, fault-injection suite.
- **R-0903** **Migration safety:** safe forms, batched backfills, retire and purge, production mode, `revert`, `squash`, decisions, `lint`, `rehearse`, expand/contract, `check_schema()`.
- **R-0904** **Managed objects:** views, materialized views, functions, procedures, triggers, policies, grants, enums and domains, with dependency recreation.
- **R-0905** **Hooks and outbox:** `Changes`, `Target`, all hooks, `validate`, `validate_changes`, `after_commit`, automatic transactions, `pg_outbox!`, `emit`, the worker and `morm-admin outbox`.
- **R-0906** **`morm-admin`:** `init`, `doctor`, `import`, `scratch`, `size`, `morm-mgr` packaging, Axum template.
- **R-0907** **Query safety:** `sql!`, `check-sql`, upsert, UNNEST and COPY, `#[morm::test]`.
- **R-0908** **Remaining features:** UUIDv7, `tls`, schemas and `utoipa`, optimistic locking, tracing, `metrics`, `[telemetry]`, `.tag()`, typed FKs, settings, pipelined transactions.
- **R-0909** **Benchmarks, lock-order detector, hardening, full README, 0.1.0 release.**

### Risks, open items and later work / Risks

- **R-0910** **Managed objects:** functions, triggers and policies with exact inverses are the hardest engine work.
- **R-0913** **Scope:** migrations alone outweigh the runtime.

### Risks, open items and later work / Later

**R-0915** Shared reads: a small opt-in lane that pipelines primary-key reads on a few connections, added only if a benchmark shows real gains with no worse p99 under mixed load. Also partitioning, a `dynamic` registry for by-name admin CRUD, sensitive-column prompts in `import`, exclusion constraints on models, and `morm-admin size --live`, which compares the estimate with `pg_stat_activity` and live pool figures to spot oversized pools.

### Risks, open items and later work / Confirmed for after 0.1

**R-0916** Features we will build are in `later-defo-build.plan`: automatic read routing and online key type changes across foreign keys.

### Risks, open items and later work / Not carried over from python-morm

- **R-0920** **`save`:** replaced by explicit `insert` and `update`.
- **R-0921** **`exclude_values_up/down`:** rarely useful, and it would make insert SQL vary per row. `Option` `None` already defers to the DB default, and `skip_json_if` covers output. It can come back if a real need appears.
- **R-0922** **`validator` and `modifier`:** replaced by the type system, native decoding, and declarative rules.
- **R-0923** **`pre_save` and `post_save`:** removed with `save`.

### How this gets built

**R-0924** After the final review, this plan is split into `plans/plan-approved/`:

- **R-0925** `INDEX.md`: order, dependencies, what can run in parallel, and status.
- **R-0926** `rules.md`: how every step works and when it counts as done.
- **R-0927** `contracts.md`: shared types and signatures that parallel steps rely on.
- **R-0928** `coverage.md`: every requirement, by ID, mapped to its step and tests. A script fails if a requirement has no step or no test.
- **R-0929** `steps/`: one self-contained file per step, about 26 in total, small enough for one fresh session.

**R-0930** The first splitting step assigns the requirement IDs, because this file has none.

**R-0931** **Decisions:**

- **R-0932** `plan-approved/` becomes the working source of truth once it's built. This file is then frozen as the record, so two live copies can't drift apart.
- **R-0933** Steps and reviews run as subagents, each with fresh context, and each reads only `rules.md`, `contracts.md` and its own step file. A thin coordinating session tracks `INDEX.md`.
- **R-0934** The owner approves the Foundation PRs. Later PRs merge once CI and an independent review both pass.

**R-0935** **Rules for every step:**

- **R-0936** The session that writes a step doesn't approve it. A separate one checks the work against the step file.
- **R-0937** Done means every test passes on Postgres 15–18, with nothing deferred or stubbed.
- **R-0938** If a step finds the plan is wrong, it stops and asks, and it doesn't improvise.
- **R-0939** Parallel steps use separate worktrees and branches, and run in parallel only when `contracts.md` covers what they share.
- **R-0940** The early proofs step comes before anything depends on the risky parts in section 16.
