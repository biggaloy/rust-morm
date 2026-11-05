# Step 28: `morm-admin size`

- **Phase:** Tools
- **Depends on:** 09, 27
- **Can run beside:** 29, 30
- **Crates:** `morm-admin`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Estimate resources from settings, and settings from resources.

## Deliverables

- `size` in both directions, `--here` with cgroup limits, `--share`, `--emit`, `--measure`, `[server]` use, exit codes, calibrated memory figures

## End-to-end check

On a real server, estimates are checked against measured memory, and `--here` gives a config that starts Postgres and the app.

## Out of scope

- `--live` (later work)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Performance and memory / Resource estimates: `morm-admin size`

**R-0609** One command works in both directions, so nobody has to add up pool size, `work_mem` and caches by hand.

**R-0610** **Settings to resources.** With no sizing flags, it reads every pool in `morm.toml`, then the live `max_connections`, `shared_buffers`, `work_mem`, `hash_mem_multiplier`, maintenance and parallel worker settings:

**R-0611**

```
$ morm-admin size

Pools                 max  x processes  connections
  default              40         2          80
  jobs                 10         1          10
  Total                                      90 of 97 available

Postgres memory       typical    worst case
  shared_buffers        2.0G        2.0G
  Connections           0.5G        0.9G
  Queries (work_mem)    0.4G        5.6G   <- exceeds RAM
  Maintenance           0.3G        0.8G
  Total                 3.2G        9.3G   of 8G RAM

warning: worst case exceeds RAM. Lower work_mem to 16MB, or max to 25.
```

**R-0612** (Figures show the format only.) Pools on the same host and port are added together, and each server is reported separately. It exits with `1` when connections or the typical total don't fit, so CI can catch a bad config. A worst case over RAM is a warning, and `doctor` prints the same result in one line.

**R-0613** **Resources to settings.** It prints suggested `morm.toml` and `postgresql.conf` values, each with a one-line reason:

**R-0614**

```
morm-admin size --here                                  # this machine
morm-admin size --ram 8G --cores 4                      # a server you describe
morm-admin size --here --qps 2000 --query-ms 5          # with a known workload
morm-admin size --here --share 0.5                      # the server is shared with other apps
morm-admin size --here --emit postgres                  # only one block: morm or postgres
```

- **R-0615** **`--here`** is for running on the machine that will host Postgres, including before Postgres is installed, since it needs no connection. It reads core count and RAM from the machine and takes the smaller of those and any cgroup limit, so a container or systemd slice gets a config for what it can really use. `--ram` and `--cores` override either value.
- **R-0616** If `morm.toml` points at a remote host, `--here` warns that this machine isn't the database server. A Unix socket or `localhost` counts as local.
- **R-0617** Suggestions cover `[server]`, `[pool]` `min`, `max` and `processes`, plus `max_connections`, `shared_buffers`, `effective_cache_size`, `work_mem` and `maintenance_work_mem`.
- **R-0618** With no workload flags, it assumes the defaults from "Sizing guidance" and prints that assumption.
- **R-0619** **It only prints.** It never changes Postgres or writes a file, and `--emit` makes the output easy to redirect.

**R-0620** **What the numbers mean:**

- **R-0621** **Estimates, shown as a range.** One query can use `work_mem` several times, once per sort or hash, multiplied by `hash_mem_multiplier` and parallel workers. Real use is usually near the typical figure, and the worst case shows only what's possible.
- **R-0622** **Per-connection memory** depends on how many statements are cached and how complex they are. The built-in defaults are calibrated in milestone 2 and aren't measured for your app.
- **R-0623** **`--measure`** replaces those defaults: it prepares your app's real statements on one connection and reads `pg_backend_memory_contexts`. It needs the models through `morm-mgr`, so run it in development or CI against a test database, not on a production host.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0722 | Pool settings that don't fit the server | `morm-admin size` exits with `1`, so CI can catch it; `doctor` reports it |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0106 | integration | `morm-admin size` estimates checked against measured memory |
| T-0107 | integration | `--here` with and without cgroup limits |
| T-0108 | integration | `--measure` |
