# Step 40: `morm-admin init` and project setup

- **Phase:** Tools
- **Depends on:** 36, 26, 16, 15
- **Can run beside:** 08, 22, 25, 37, 39, 45
- **Crates:** `morm-admin`, `morm-macros`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Create a working project from one command.

## Deliverables

- `init` with socket detection and the Axum template, `morm-mgr` packaging, `scratch`, `outbox`, `baseline`, the managed `morm::pg_outbox!` form

## End-to-end check

`morm-admin init --axum` creates a project that builds, migrates and serves a request, and `outbox list` and `retry` work on it.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> **How it fits together:** `morm-admin` holds the engine. When a command needs the models, it builds and runs `morm-mgr` to get the manifest. Commands that don't need models (`migrate`, `rollback`, `status`, `purge`) run from `morm-migrations/` alone, so a production host needs `morm-admin` and no Rust toolchain. The app binary carries no migration code or schema metadata, apart from the short per-model column list `check_schema` reads.
>
> **Registration:** models, objects and `sql!` queries register themselves through a distributed static list, with no central list to maintain. Their metadata is behind `cfg(morm_mgr)`, not a cargo feature, because `cargo build --workspace` unifies features and would pull it into the app. `morm-admin` builds `morm-mgr` in its own target directory with that `cfg` set. This needs proof early, including models in several crates, where a linker can drop a crate nothing references (section 16).
>

## Out of scope

- `doctor` (step 41)
- `import` (step 42)
- `size` (step 43)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools

| ID | Name | Kind | Job |
|---|---|---|---|
| R-0043 | `morm-mgr` | small package in each project | Links your models and prints their schema and queries as JSON. Nothing else |

### Crates and tools / `morm-admin`

**R-0057** `init` creates the `morm-mgr` package, `morm-migrations/`, `morm.toml`, `.env.example`, a sample model, and a CI workflow. `--axum` adds the app template and a systemd unit.

### Crates and tools / Configuration: `morm.toml`

**R-0103** **Detection:** `morm-admin init` finds the socket and writes it into `morm.toml` and `.env.example`, so most projects never set it by hand:

- **R-0104** Look for `.s.PGSQL.<port>` files in `/var/run/postgresql`, `/run/postgresql` and `/tmp`, plus any path given by `--socket-dir`. The filename also gives the port.
- **R-0105** If several servers are found, ask which to use. Without a terminal, take the default directory if it's among them, otherwise exit with code `3` and list the choices.
- **R-0106** If none is found, keep the default and print a warning, with the TCP settings as a commented-out alternative.
- **R-0107** Once connected, compare with `SHOW unix_socket_directories` and warn on a mismatch.

### Hooks / Outbox: events that must not be lost

- **R-0512** `morm-admin outbox retry ID` runs a failed event again.

### Migrations / Commands

- **R-0616** `baseline`: adopts an existing database as migration zero, without running anything.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0132 | suite | `morm-admin outbox retry ID` on a failed event |
