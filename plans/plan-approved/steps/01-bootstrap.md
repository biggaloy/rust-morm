# Step 01: Repo bootstrap and CI

- **Phase:** Foundation
- **Depends on:** nothing
- **Can run beside:** nothing
- **Crates:** workspace, all four crates as empty shells
- **PR approval:** the owner approves the PR

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Create the workspace so every later step has a place to land, and CI that runs the same checks for every PR from day one.

## Deliverables

- Cargo workspace with `crates/morm`, `crates/morm-macros`, `crates/morm-migrate`, `crates/morm-admin`, `examples/app`, `tests/`, edition 2024, MSRV 1.99.1
- `ci.yml`, `nightly.yml`, `bench.yml` and `release.yml`, including Postgres 15–18 over TCP and Unix socket and a PgBouncer service
- `tools/check_coverage.py` copied into the repo and run by `ci.yml` in `--code` mode
- A test helper that starts or connects to each Postgres version, and a `trybuild` harness
- README skeleton, included as crate docs
- Check that the five crate names are still free on crates.io

## End-to-end check

A PR containing one trivial integration test tagged `covers: R-nnnn` passes `ci.yml` on every Postgres version and on PgBouncer. Removing the tag makes the coverage job fail.

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

**R-0046** Cargo features of `morm`:

### Performance and memory / Query path

- **R-0676** Minimal dependencies and features; no CLI framework in `morm-admin`.

### Testing and CI

**R-0738** Tests are written with the code: **a milestone isn't done until its tests pass in CI.**

**R-0739** Migration tests:

**R-0740** Outbox tests:

### Testing and CI / GitHub Actions (`.github/workflows/`)

| ID | Workflow | When | Runs |
|---|---|---|---|
| R-0741 | `ci.yml` | Every push and PR | `fmt`, `clippy -D warnings`, unit, `trybuild`, doc tests, integration on Postgres 15–18 (TCP and Unix socket), `pooler` tests against a PgBouncer service, MSRV build, feature combinations, migration suite including fault injection |
| R-0742 | `nightly.yml` | Daily | Long property tests, fuzzing, full chain, `cargo audit` and `cargo deny` |
| R-0743 | `bench.yml` | Weekly and on demand | Benchmarks, compared with the last run; tracks binary size and RSS |
| R-0744 | `release.yml` | On tag | Tests, publish crates, build `morm-admin` binaries |

**R-0745** Every merge to `main` requires `ci.yml` to pass. The repo is private, so Actions minutes are limited: the Postgres matrix runs fully on `main` and PRs, while pushes to other branches use one version.

### Risks, open items and later work / Open items

- **R-0868** `morm`, `morm-macros`, `morm-migrate`, `morm-admin` and `morm-mgr` were free on crates.io on 2026-11-05. Reserve them at the first publish.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0001 | suite | **Unit:** macro output, SQL generation, diff, naming. |
