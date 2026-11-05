# Step 27: `morm-admin` core

- **Phase:** Tools
- **Depends on:** 23, 05
- **Can run beside:** 28, 29, 30
- **Crates:** `morm-admin`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

One tool for setup, checks and importing existing databases.

## Deliverables

- `init`, `doctor`, `import`, `scratch`, `outbox`, `baseline`, `morm-mgr` packaging and `cfg(morm_mgr)` builds, Axum template, socket detection

## End-to-end check

`morm-admin init --axum` creates a project that builds, migrates and serves a request. `import` on the madmin-style test schema round-trips to no diff.

`doctor` gathers checks other steps defined: credentials, pooler role settings and parameter logging (09), scratch match (21), function scan (25), real roles (26). Step 28 adds the `size` line.

## Out of scope

- `size` (step 28)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools

| ID | Name | Kind | Job |
|---|---|---|---|
| R-0043 | `morm-mgr` | small package in each project | Links your models and prints their schema and queries as JSON. Nothing else |

**R-0044** **How it fits together:** `morm-admin` holds the engine. When a command needs the models, it builds and runs `morm-mgr` to get the manifest. Commands that don't need models (`migrate`, `rollback`, `status`, `purge`) run from `morm-migrations/` alone, so a production host needs `morm-admin` and no Rust toolchain. The app binary carries no migration code or schema metadata, apart from the short per-model column list `check_schema` reads.

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

**R-0102** **Detection:** `morm-admin init` finds the socket and writes it into `morm.toml` and `.env.example`, so most projects never set it by hand:

- **R-0103** Look for `.s.PGSQL.<port>` files in `/var/run/postgresql`, `/run/postgresql` and `/tmp`, plus any path given by `--socket-dir`. The filename also gives the port.
- **R-0104** If several servers are found, ask which to use. Without a terminal, take the default directory if it's among them, otherwise exit with code `3` and list the choices.
- **R-0105** If none is found, keep the default and print a warning, with the TCP settings as a commented-out alternative.
- **R-0106** Once connected, compare with `SHOW unix_socket_directories` and warn on a mismatch.

**R-0113** **`morm-admin import`** is a simple starting point for any existing database. It reads the catalog only, never rows, and writes model files that match the schema exactly: types, nullability, defaults, indexes, constraints and their names. Nothing is "improved". Anything models can't express, such as exclusion constraints or partitioned tables, is listed in its output and stays unmanaged. `morm-admin mgr baseline` then records that state as migration zero. Views, hooks and validation are added by hand afterwards.

### Migrations / Commands

- **R-0578** `baseline`: adopts an existing database as migration zero, without running anything.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0735 | Query values, such as password hashes, reaching the Postgres log | `doctor` warns when statements are logged and `log_parameter_max_length` isn't `0` |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0105 | integration | `doctor` warnings on parameter logging |
