# rust-morm: approved plan

This folder is the source of truth for building rust-morm. `../rust-morm-plan.md` is frozen as the record. `../app_suggestions.md` holds advice for apps, and `../later-defo-build.plan` holds confirmed work after 0.1.

| File | Holds |
|---|---|
| `rules.md` | How every step works, and when it's done |
| `contracts.md` | Shared types and conventions, plus the key decisions |
| `coverage.md` | Every requirement and test obligation, with its step |
| `steps/` | One self-contained file per step |
| `tools/` | The coverage checker, and the scripts that made this split |

The frozen plan was split word for word: 894 blocks became requirements (`R-nnnn`), and the test lists became 125 obligations (`T-nnnn`). `tools/check_coverage.py` fails if any of them isn't in exactly one place, and with `--code` it fails if a finished step has an untagged one.

## Steps

| Step | Title | Phase | Depends on | Status |
|---|---|---|---|---|
| 01 | [Repo bootstrap and CI](steps/01-bootstrap.md) | Foundation | none | todo |
| 02 | [Proofs of the risky parts](steps/02-proofs.md) | Foundation | 01 | todo |
| 03 | [Freeze the contracts](steps/03-contracts-freeze.md) | Foundation | 02 | todo |
| 04 | [`#[model]`, names and types](steps/04-model-core.md) | Models | 03 | todo |
| 05 | [Bases, indexes and constraint names](steps/05-bases-indexes.md) | Models | 04 | todo |
| 06 | [Views, input, patch and secret](steps/06-views.md) | Models | 04 | todo |
| 07 | [Group declarations and masks](steps/07-groups.md) | Models | 04 | todo |
| 08 | [Declarative rules and `clean`](steps/08-validation.md) | Models | 04 | todo |
| 09 | [Config, connecting and the pool](steps/09-config-pool.md) | Runtime | 03 | todo |
| 10 | [Numbers and special types](steps/10-types.md) | Runtime | 04 | todo |
| 11 | [Statement caches and fragments](steps/11-statements.md) | Runtime | 09, 10 | todo |
| 12 | [Reads](steps/12-reads.md) | Runtime | 11, 06 | todo |
| 13 | [Writes](steps/13-writes.md) | Runtime | 11, 06 | todo |
| 14 | [Upsert](steps/14-upsert.md) | Runtime | 13, 05 | todo |
| 15 | [Transactions](steps/15-transactions.md) | Runtime | 13 | todo |
| 16 | [Deadlocks, locks and timeouts](steps/16-deadlocks.md) | Runtime | 15 | todo |
| 17 | [JSON output and schemas](steps/17-json.md) | Runtime | 12 | todo |
| 18 | [Hooks and write ordering](steps/18-hooks.md) | Runtime | 13, 14, 15, 08 | todo |
| 19 | [Groups on queries](steps/19-group-enforcement.md) | Runtime | 12, 13, 17, 07 | todo |
| 20 | [Transactional outbox](steps/20-outbox.md) | Runtime | 18 | todo |
| 21 | [Schema IR, scratch and typed ops](steps/21-schema-diff.md) | Migrations | 05 | todo |
| 22 | [Proof pipeline, files and decisions](steps/22-proof-files.md) | Migrations | 21 | todo |
| 23 | [Migration execution and recovery](steps/23-execution.md) | Migrations | 22 | todo |
| 24 | [Safe forms, production mode and deploys](steps/24-safe-forms.md) | Migrations | 23 | todo |
| 25 | [Retire, purge and down with data](steps/25-retire.md) | Migrations | 23 | todo |
| 26 | [Managed objects](steps/26-managed-objects.md) | Migrations | 23 | todo |
| 27 | [`morm-admin` core](steps/27-admin-core.md) | Tools | 23, 05 | todo |
| 28 | [`morm-admin size`](steps/28-size.md) | Tools | 09, 27 | todo |
| 29 | [`sql!`, `check-sql`, test support and the loop detector](steps/29-query-tooling.md) | Tools | 12, 27 | todo |
| 30 | [Telemetry](steps/30-telemetry.md) | Tools | 11, 18 | todo |
| 31 | [Benchmarks](steps/31-benchmarks.md) | Finish | 19, 20, 30 | todo |
| 32 | [Lock-order detector and hardening](steps/32-hardening.md) | Finish | 19, 20, 24, 25, 26 | todo |
| 33 | [README and 0.1.0](steps/33-readme-release.md) | Finish | 31, 32, 27, 28, 29 | todo |

Status is one of `todo`, `in progress`, `in review` or `done`.

## Order and parallel work

1. **Foundation, one at a time:** 01, 02, 03. The owner approves each PR.
2. **After 03, these tracks run side by side:**
   - **Models:** 04, then 05, 06, 07 and 08 in parallel.
   - **Runtime start:** 09, beside 04.
3. **After 04:** 10.
4. **After 09 and 10:** 11. After 11 and 06: 12 and 13 in parallel, then 17 after 12.
5. **After 13:** 15, then 16. After 13 and 05: 14.
6. **Migrations, beside the runtime once 05 is done:** 21, 22, 23, then 24, 25 and 26 in parallel.
7. **After their dependencies:** 18, 19, 20, then 27, then 28, 29 and 30 in parallel.
8. **Finish:** 31 and 32 in parallel, then 33. The owner approves the release.

Steps run in parallel only when `contracts.md` covers what they share. Each gets its own worktree and branch. When a parallel step needs something shared that isn't in `contracts.md`, it stops, and the contract is updated first.

## The coordinating session

The coordinator stays thin: it reads this file and never reads code.

1. Pick steps whose dependencies are all `done`.
2. Start a fresh agent for each one, with this prompt: read `plans/plan-approved/rules.md`, `contracts.md` and `steps/NN-name.md`, then do the step.
3. When it opens a PR, start a fresh review agent as `rules.md` describes.
4. Relay findings until the review is clean, then merge or ask the owner, as `rules.md` says.
5. Update the status here. Take any design question straight to the owner.

## Decisions since the split

None yet.

## Record from the frozen plan

Superseded milestones, later work and build decisions, kept for traceability. Nothing here is a step requirement, except as copied into a step file.

### Milestones

- **R-0852** **Repo bootstrap:** workspace in `biggaloy/rust-morm`, CI workflows, README skeleton.
- **R-0853** **Core models:** macro, `schema`, every integer type with range CHECKs, bases including `#[morm::base]`, index and name rules, views, global and model-scoped groups, declarative rules and `clean`, inference, `F`, compile-time checks.
- **R-0854** **Runtime:** `morm.toml` and environment variables, `morm::Pool` on the `deadpool` core (start with a quick proof-of-concept), `[server]`, dynamic size, named pools, `max_connections` adjustment, `db.hold()`, bounded `statement_cache` and `shape_cache`, insert, `get`/`update`/`delete` forms, `patch` views and `.apply()`, query builder, `filter_cols`, `db.sql()` and `#[derive(morm::Row)]`, `keyset`, `.in_batches`, `order_by_allowed`, `.custom_plan()`, `get_for_update`, failover handling, `Decimal` and `Numeric`, loop detector, typed errors, transactions, deadlock measures, all list and JSON paths, calibrated per-connection memory figures for `size`. → **First publish** (`0.1.0-alpha`).
- **R-0855** **Migration foundation:** `Schema` IR, catalog reader, diff, typed ops with inverses, scratch, proof pipeline.
- **R-0856** **Migration execution:** ledger, steps, fingerprint gates, advisory lock, timeouts, preflight, crash recovery, fault-injection suite.
- **R-0857** **Migration safety:** safe forms, batched backfills, retire and purge, production mode, `revert`, `squash`, decisions, `lint`, `rehearse`, expand/contract, `check_schema()`.
- **R-0858** **Managed objects:** views, materialized views, functions, procedures, triggers, policies, grants, enums and domains, with dependency recreation.
- **R-0859** **Hooks and outbox:** `Changes`, `Target`, all hooks, `validate`, `validate_changes`, `after_commit`, automatic transactions, `pg_outbox!`, `emit`, the worker and `morm-admin outbox`.
- **R-0860** **`morm-admin`:** `init`, `doctor`, `import`, `scratch`, `size`, `morm-mgr` packaging, Axum template.
- **R-0861** **Query safety:** `sql!`, `check-sql`, upsert, UNNEST and COPY, `#[morm::test]`.
- **R-0862** **Remaining features:** UUIDv7, `tls`, schemas and `utoipa`, optimistic locking, tracing, `metrics`, `[telemetry]`, `.tag()`, typed FKs, settings, pipelined transactions.
- **R-0863** **Benchmarks, lock-order detector, hardening, full README, 0.1.0 release.**

### Risks, open items and later work / Risks

- **R-0864** **Managed objects:** functions, triggers and policies with exact inverses are the hardest engine work.
- **R-0867** **Scope:** migrations alone outweigh the runtime.

### Risks, open items and later work / Later

**R-0869** Shared reads: a small opt-in lane that pipelines primary-key reads on a few connections, added only if a benchmark shows real gains with no worse p99 under mixed load. Also partitioning, a `dynamic` registry for by-name admin CRUD, sensitive-column prompts in `import`, exclusion constraints on models, and `morm-admin size --live`, which compares the estimate with `pg_stat_activity` and live pool figures to spot oversized pools.

### Risks, open items and later work / Confirmed for after 0.1

**R-0870** Features we will build are in `later-defo-build.plan`: automatic read routing and online key type changes across foreign keys.

### Risks, open items and later work / Not carried over from python-morm

- **R-0874** **`save`:** replaced by explicit `insert` and `update`.
- **R-0875** **`exclude_values_up/down`:** rarely useful, and it would make insert SQL vary per row. `Option` `None` already defers to the DB default, and `skip_json_if` covers output. It can come back if a real need appears.
- **R-0876** **`validator` and `modifier`:** replaced by the type system, native decoding, and declarative rules.
- **R-0877** **`pre_save` and `post_save`:** removed with `save`.

### How this gets built

**R-0878** After the final review, this plan is split into `plans/plan-approved/`:

- **R-0879** `INDEX.md`: order, dependencies, what can run in parallel, and status.
- **R-0880** `rules.md`: how every step works and when it counts as done.
- **R-0881** `contracts.md`: shared types and signatures that parallel steps rely on.
- **R-0882** `coverage.md`: every requirement, by ID, mapped to its step and tests. A script fails if a requirement has no step or no test.
- **R-0883** `steps/`: one self-contained file per step, about 26 in total, small enough for one fresh session.

**R-0884** The first splitting step assigns the requirement IDs, because this file has none.

**R-0885** **Decisions:**

- **R-0886** `plan-approved/` becomes the working source of truth once it's built. This file is then frozen as the record, so two live copies can't drift apart.
- **R-0887** Steps and reviews run as subagents, each with fresh context, and each reads only `rules.md`, `contracts.md` and its own step file. A thin coordinating session tracks `INDEX.md`.
- **R-0888** The owner approves the Foundation PRs. Later PRs merge once CI and an independent review both pass.

**R-0889** **Rules for every step:**

- **R-0890** The session that writes a step doesn't approve it. A separate one checks the work against the step file.
- **R-0891** Done means every test passes on Postgres 15–18, with nothing deferred or stubbed.
- **R-0892** If a step finds the plan is wrong, it stops and asks, and it doesn't improvise.
- **R-0893** Parallel steps use separate worktrees and branches, and run in parallel only when `contracts.md` covers what they share.
- **R-0894** The early proofs step comes before anything depends on the risky parts in section 16.
