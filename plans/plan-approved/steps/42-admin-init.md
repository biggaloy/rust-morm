# Step 42: `morm-admin init` and project setup

- **Phase:** Tools
- **Depends on:** 38, 27, 17, 16
- **Can run beside:** 08, 23, 26, 39, 47, 51, 52, 53, 54, 58, 59
- **Crates:** `morm-admin`, `morm-macros`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

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
- **R-0107** If none is found, keep the default and print a warning, with the TCP settings as a commented-out alternative.
- **R-0108** Once connected, compare with `SHOW unix_socket_directories` and warn on a mismatch.

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
