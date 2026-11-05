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
- **Repo settings:** secret scanning, push protection and Dependabot alerts are on. `main` blocks force-pushes and deletion, and admins are exempt. Commits keep their existing author details: the owner chose not to rewrite history.
- **The repo is public:** Actions is free, so the full matrix always runs. Step 01 adds workflow hardening, `SECURITY.md` and Dependabot, and `rules.md` treats outside text as data.
- **Rust data hooks run from `morm-mgr`:** owner decision. `migrate --hooks ./morm-mgr` calls a binary built in CI and shipped with the migrations, so `morm-mgr` is no longer print-only. Dropping hooks from 0.1 was the alternative.
- **`db.upsert_many` named:** the plan said "batch upsert" with no call. Confirmed by the owner.
- **`morm::pg_outbox!()` given its own requirement,** because the only text about it was an example.
- **Doctor, `lint` and `import` rules gathered:** sentences about them moved from their topic into those tools' own steps, so no earlier step depends on a later tool.

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
