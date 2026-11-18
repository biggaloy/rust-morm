# Step 42: `morm-admin init` and project setup

- **Phase:** Tools
- **Depends on:** 38, 27, 17, 16
- **Can run beside:** 08, 23, 26, 47, 51, 52, 53, 54, 58, 59, 60
- **Crates:** `morm-admin`, `morm-macros`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Create a working project from one command.

## Deliverables

- `init` with socket detection and the Axum template, `morm-mgr` packaging, `scratch`, `outbox`, `baseline`, the managed `morm::pg_outbox!` form
- The `morm.toml` `init` creates is step 10's shipped `crates/morm-core/morm.toml`, embedded in `morm-admin` and written byte for byte, apart from a found socket's `host` and `port`, written loudly, and the `.env.example` beside it, as R-1280 says; the CI workflow it creates connects to its service container with `sslmode = disable`, loudly, as R-1281 says

## End-to-end check

`morm-admin init --axum` creates a project that builds, migrates and serves a request, and `outbox list` and `retry` work on it.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> **How it fits together:** `morm-admin` holds the engine. When a command needs the models, it builds and runs `morm-mgr` to get the manifest. Commands that don't need models (`migrate`, `rollback`, `status`, `purge`) run from `morm-migrations/` alone, so a production host needs `morm-admin` and no Rust toolchain. The app binary carries no migration code or schema metadata, apart from the short per-model column list `check_schema` reads.
>
> **Registration:** models, objects and `sql!` queries register themselves through a distributed static list, with no central list to maintain. Their metadata is behind `cfg(morm_mgr)`, not a cargo feature, because `cargo build --workspace` unifies features and would pull it into the app. `morm-admin` builds `morm-mgr` in its own target directory with that `cfg` set. This needs proof early, including models in several crates, where a linker can drop a crate nothing references (section 16).
>
> ```rust
> morm::pg_outbox!();                                    // declares the table, tracked by migrations
>
> db.transaction(async |tx| {
>     tx.insert(&mut order).await?;
>     tx.emit("order_placed", &OrderPlaced { id: order.id }).await?;
>     Ok(())
> }).await?;
>
> morm::outbox::worker(&db)                              // in the app, or its own process
>     .on("order_placed", async |e: Event<OrderPlaced>| send_email(e.id, e.payload).await)
>     .run().await?;
> ```
>
> - The app role needs read and write access on the table, so it's a normal managed table in the app's schema, not in `_morm`.
>
> - Roles are cluster-wide, so they aren't created. `morm-admin scratch` makes placeholder roles so policies and grants can run on scratch.
>

## Out of scope

- `doctor` (step 43)
- `import` (step 44)
- `size` (step 46)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools

| ID | Name | Kind | Job |
|---|---|---|---|
| R-0043 | `morm-mgr` | small package in each project | Links your models and prints their schema and queries as JSON. |

### Crates and tools / `morm-admin`

**R-0057**

```
morm-admin init [path] [--axum]       # create a project
morm-admin doctor                     # check Postgres, roles, scratch, versions
morm-admin size [options]             # settings to resources, or resources to settings (section 11)
morm-admin import URL [--out DIR]     # generate models from a database
morm-admin scratch                    # check or create the scratch database
morm-admin outbox [list | retry ID]   # inspect failed events, deliver one again
morm-admin mgr <command>              # migrations and schema work (section 10)
```

**R-0058** `init` creates the `morm-mgr` package, `morm-migrations/`, `morm.toml`, `.env.example`, a sample model, and a CI workflow. `--axum` adds the app template and a systemd unit.

- **R-0955** `morm-admin init [path] [--axum]`, `morm-admin scratch` to check or create the scratch database, and `morm-admin outbox [list | retry ID]` to inspect failed events and deliver one again.

### Crates and tools / Configuration: `morm.toml`

**R-0104** **Detection:** `morm-admin init` finds the socket and writes it into `morm.toml` and `.env.example`, so most projects never set it by hand:

- **R-0105** Look for `.s.PGSQL.<port>` files in `/var/run/postgresql`, `/run/postgresql` and `/tmp`, plus any path given by `--socket-dir`. The filename also gives the port.
- **R-0106** If several servers are found, ask which to use. Without a terminal, take the default directory if it's among them, otherwise exit with code `3` and list the choices.
- **R-0107** If none is found, keep the default, writing the shipped `morm.toml` unchanged, and print a warning naming the directories searched, with the TCP settings as a commented-out alternative to paste: `# host = "127.0.0.1"`, `# port = 5432` and `# sslmode = "disable"`, the last with its `# WARNING:` line saying that `sslmode` defaults to `require`, so a local server reached over TCP without TLS needs it, and naming the README notice on `sslmode`; `.env.example` holds the same as `MORM_DB_HOST`, `MORM_DB_PORT` and `MORM_DB_SSLMODE`, commented out, with the same warning.
- **R-0108** Once connected, compare with `SHOW unix_socket_directories` and warn on a mismatch.

**R-1280** **The `morm.toml` and `.env.example` `init` writes, with a found socket written loudly.** `init` writes step 10's shipped `crates/morm-core/morm.toml`, embedded in `morm-admin`, as the new project's `morm.toml`, byte for byte, apart from what detection finds. When the socket found is the default one, `/var/run/postgresql/.s.PGSQL.5432`, or none is found, the file is the shipped one exactly. When it's anywhere else, in another directory, on another port in the default directory, or both, `init` writes each of `host` and `port` whose found value isn't its default, `/var/run/postgresql` and `5432`, as an active line in place of its `${MORM_DB_HOST}` or `${MORM_DB_PORT}` placeholder, such as `host = "/run/postgresql"` or `port = 5433`, keeping the line's comments, with a loud `# WARNING:` line above it. The warning says that a value written here overrides the host or port of `DATABASE_URL`, so a deployment whose `DATABASE_URL` names another server connects here instead; that before deploying elsewhere the `${MORM_DB_HOST}` and `${MORM_DB_PORT}` placeholders go back, or `MORM_DB_HOST` and `MORM_DB_PORT` are set, which override the file; and it names the README notice on it, "A `host` or `port` written in `morm.toml` overrides `DATABASE_URL`". Nothing else in the file changes, so it still loads with no error. `.env.example` gets the found directory and port as `MORM_DB_HOST` and `MORM_DB_PORT`, and with none found R-0107's commented-out alternative. `init` prints each line it wrote with the warning's text. At startup, step 10's config loading warns whenever such a line overrides a different value in `DATABASE_URL`. Owner decision: `init` writes a found socket loudly (ER2).

**R-1281** **`init`'s CI workflow connects without TLS, on purpose and loudly.** The CI workflow `init` creates runs Postgres as a service container, which its job reaches over TCP without TLS, so with `sslmode`'s default, `require`, step 10's, every connection would be refused. So every connection it sets up uses `sslmode = disable`: `MORM_DB_SSLMODE: disable` in the job's `env`, and `?sslmode=disable` on each full URL it sets, such as `DATABASE_URL`, `MORM_MIGRATION_URL` and `MORM_SCRATCH_URL`, with a comment above them saying that `disable` is right here only because the service container runs beside the job on its own runner, that a deployment keeps the default, and naming the README notice on `sslmode`. Owner decision: `sslmode` defaults to `require`; the fourteenth review's ER5.

### Hooks / Outbox: events that must not be lost

- **R-0516** `morm-admin outbox retry ID` runs a failed event again.
- **R-0959** `morm::pg_outbox!()` declares the outbox table as a managed table, tracked by migrations.

### Migrations / Commands

- **R-0621** `baseline`: adopts an existing database as migration zero, without running anything.
- **R-0954** `baseline`.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0132 | suite | `morm-admin outbox retry ID` on a failed event |
| T-0324 | suite | **`init`'s files, with and without a found socket:** a test in `morm-admin` runs `init`'s writing with each detection result and compares what it writes: with the default socket, `/var/run/postgresql/.s.PGSQL.5432`, and with none found, `morm.toml` equals `crates/morm-core/morm.toml` byte for byte; with `/run/postgresql/.s.PGSQL.5432`, `/tmp/.s.PGSQL.5432`, `/var/run/postgresql/.s.PGSQL.5433` and `/run/postgresql/.s.PGSQL.5433`, it differs from the shipped file only in the `host` and `port` lines whose found value isn't the default, written active, and a `# WARNING:` line above each, saying the value overrides `DATABASE_URL`'s, how to put the placeholder back or set `MORM_DB_HOST` and `MORM_DB_PORT`, and naming the README notice, and it loads with no error; `init` prints each line it wrote with its warning; `.env.example` holds the found `MORM_DB_HOST` and `MORM_DB_PORT`; with none found, the printed warning and `.env.example` hold R-0107's commented-out TCP alternative with `sslmode = "disable"` and its warning; loading the written `morm.toml` with `DATABASE_URL` naming another host and port gives the file's host and port with step 10's startup warning naming both, and with `MORM_DB_HOST` and `MORM_DB_PORT` set gives theirs with no warning; and the CI workflow `init` creates sets `MORM_DB_SSLMODE: disable`, puts `?sslmode=disable` on every URL it sets, and has R-1281's comment naming the README notice on `sslmode` |
