# Step 36: `morm-admin` core

- **Phase:** Tools
- **Depends on:** 32, 24, 15, 14
- **Can run beside:** 07, 21, 23, 33, 35, 40
- **Crates:** `morm-admin`, `morm-macros`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

One tool for setup and checks.

## Deliverables

- `init` with socket detection and the Axum template, `doctor`, `scratch`, `outbox`, `baseline`, `morm-mgr` packaging and its `cfg(morm_mgr)` builds, the managed `morm::pg_outbox!` form

## End-to-end check

`morm-admin init --axum` creates a project that builds, migrates and serves a request, and `doctor` reports each of its checks correctly when the condition is forced.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> **How it fits together:** `morm-admin` holds the engine. When a command needs the models, it builds and runs `morm-mgr` to get the manifest. Commands that don't need models (`migrate`, `rollback`, `status`, `purge`) run from `morm-migrations/` alone, so a production host needs `morm-admin` and no Rust toolchain. The app binary carries no migration code or schema metadata, apart from the short per-model column list `check_schema` reads.
>
> **Registration:** models, objects and `sql!` queries register themselves through a distributed static list, with no central list to maintain. Their metadata is behind `cfg(morm_mgr)`, not a cargo feature, because `cargo build --workspace` unifies features and would pull it into the app. `morm-admin` builds `morm-mgr` in its own target directory with that `cfg` set. This needs proof early, including models in several crates, where a linker can drop a crate nothing references (section 16).
>

## Out of scope

- `import` (step 37)
- `size` (step 38)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools

| ID | Name | Kind | Job |
|---|---|---|---|
| R-0043 | `morm-mgr` | small package in each project | Links your models and prints their schema and queries as JSON. Nothing else |

### Crates and tools / `morm-admin`

**R-0056**

```
morm-admin init [path] [--axum]       # create a project
morm-admin doctor                     # check Postgres, roles, scratch, versions
morm-admin size [options]             # settings to resources, or resources to settings (section 11)
morm-admin import URL [--out DIR]     # generate models from a database
morm-admin scratch                    # check or create the scratch database
morm-admin outbox [list | retry ID]   # inspect failed events, deliver one again
morm-admin mgr <command>              # migrations and schema work (section 10)
```

**R-0057** `init` creates the `morm-mgr` package, `morm-migrations/`, `morm.toml`, `.env.example`, a sample model, and a CI workflow. `--axum` adds the app template and a systemd unit.

### Crates and tools / Configuration: `morm.toml`

**R-0102** `morm-admin doctor` reports the exact socket path it tries, and warns if `host` no longer matches a live socket.

**R-0103** **Detection:** `morm-admin init` finds the socket and writes it into `morm.toml` and `.env.example`, so most projects never set it by hand:

- **R-0104** Look for `.s.PGSQL.<port>` files in `/var/run/postgresql`, `/run/postgresql` and `/tmp`, plus any path given by `--socket-dir`. The filename also gives the port.
- **R-0105** If several servers are found, ask which to use. Without a terminal, take the default directory if it's among them, otherwise exit with code `3` and list the choices.
- **R-0106** If none is found, keep the default and print a warning, with the TCP settings as a commented-out alternative.
- **R-0107** Once connected, compare with `SHOW unix_socket_directories` and warn on a mismatch.

**R-0111** `morm-admin doctor` checks that credentials work, warns if `morm.toml` contains a literal password, and warns if Postgres can log query values: `log_parameter_max_length` and `auto_explain.log_parameter_max_length` default to full values, which puts anything sent as a parameter, including `secret` columns, in the server log.

### Migrations / Source of truth

**R-0533** `doctor` compares the Postgres major version and installed extensions of scratch and the target.

### Migrations / Down that can restore data

- **R-0556** `doctor` runs the same scan.

### Migrations / Runtime guarantees

- **R-0587** `doctor` reports the server version.

### Migrations / Commands

- **R-0604** `baseline`: adopts an existing database as migration zero, without running anything.

### Performance and memory / `max_connections`

**R-0634** `morm-admin doctor` runs the same check.

### Performance and memory / Behind a pooler

- **R-0654** `doctor` checks that they're there.
- **R-0660** `doctor` warns if `MORM_MIGRATION_URL` doesn't point at Postgres directly.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0765 | Query values, such as password hashes, reaching the Postgres log | `doctor` warns when statements are logged and `log_parameter_max_length` isn't `0` |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0110 | integration | `doctor` warnings on parameter logging |
