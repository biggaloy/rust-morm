# Step 01: Repo bootstrap and CI

- **Phase:** Foundation
- **Depends on:** nothing
- **Can run beside:** nothing
- **Crates:** workspace, all five crates as empty shells
- **PR approval:** the owner approves the PR

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Create the workspace so every later step has a place to land, and CI that runs the same checks for every PR from day one.

## Deliverables

- Cargo workspace with `crates/morm`, `crates/morm-core`, `crates/morm-macros`, `crates/morm-migrate`, `crates/morm-admin`, `examples/app`, `tests/`, edition 2024, MSRV 1.99.1
- `ci.yml`, `nightly.yml`, `bench.yml` and `release.yml`, including Postgres 15–18 over TCP and Unix socket and a PgBouncer service. `nightly.yml` also has `workflow_dispatch`, so it can run on a branch
- `ci.yml` runs `plans/plan-approved/tools/check_coverage.py --code crates tests benches fuzz .github`
- A test helper that starts or connects to each Postgres version, and a `trybuild` harness
- README skeleton, included as crate docs, plus `SECURITY.md` and Dependabot config
- Check that all six crate names, including `morm-mgr`, are still free on crates.io

## End-to-end check

A PR containing one trivial integration test tagged `covers: R-nnnn` passes `ci.yml` on every Postgres version and on PgBouncer. Removing the tag makes `--step` fail for that step.

## Out of scope

- Any morm feature
- Publishing crates

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools

**R-0038**

```
rust-morm/
├── crates/
│   ├── morm/            # runtime: pool, queries, hooks, JSON
│   ├── morm-macros/     # #[model], #[view], pg_* macros
│   ├── morm-migrate/    # migration engine
│   └── morm-admin/      # the admin binary
├── examples/
│   └── app/             # Axum + morm example
├── tests/               # integration tests against real Postgres
├── .github/workflows/
└── README.md
```

| ID | Name | Kind | Job |
|---|---|---|---|
| R-0039 | `morm` | crate | Runtime |
| R-0040 | `morm-macros` | crate | Procedural macros |
| R-0041 | `morm-migrate` | crate | Migration engine, used by `morm-admin` |
| R-0042 | `morm-admin` | installed binary | All admin work |

**R-0047** Cargo features of `morm`:

### Performance and memory / Query path

- **R-0727** Minimal dependencies and features; no CLI framework in `morm-admin`.

### Testing and CI

**R-0789** Tests are written with the code: **a milestone isn't done until its tests pass in CI.**

**R-0790** Migration tests:

**R-0791** Outbox tests:

### Testing and CI / GitHub Actions (`.github/workflows/`)

| ID | Workflow | When | Runs |
|---|---|---|---|
| R-0792 | `ci.yml` | Every push and PR | `fmt`, `clippy -D warnings`, unit, `trybuild`, doc tests, integration on Postgres 15–18 (TCP and Unix socket), `pooler` tests against a PgBouncer service, MSRV build, feature combinations, migration suite including fault injection |
| R-0793 | `nightly.yml` | Daily | Long property tests, fuzzing, full chain, `cargo audit` and `cargo deny` |
| R-0794 | `bench.yml` | Weekly and on demand | Benchmarks, compared with the last run; tracks binary size and RSS |
| R-0795 | `release.yml` | On tag | Tests, publish crates, build `morm-admin` binaries |

**R-0796** Every merge to `main` requires `ci.yml` to pass.

**R-0797** The repo is public, so Actions is free on standard runners, and the full Postgres matrix runs on every push and PR.

- **R-0957** **Workflows in a public repo.** Every workflow sets `permissions: contents: read` and raises it only per job where needed. Third-party actions are pinned to a full commit SHA. `pull_request_target` is never used, and PRs from forks get no secrets and run no release or publish job. `release.yml` runs only on a tag, in a protected environment holding the crates.io token.
- **R-0958** **Repo files for a public project.** `SECURITY.md` says how to report a vulnerability privately, and Dependabot config covers Cargo and Actions.

## Test obligations

None beyond one tagged test per requirement.
