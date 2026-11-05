"""Hand-written header of each step. Requirements are added by split.py."""
from collections import OrderedDict

OWNER = "the owner approves the PR"
RT = "`morm`"
MAC = "`morm-macros`"
MIG = "`morm-migrate`"
ADM = "`morm-admin`"

_S = [
("bootstrap", dict(title="Repo bootstrap and CI", phase="Foundation", deps=[], crates="workspace, all four crates as empty shells",
  approve=OWNER,
  goal="Create the workspace so every later step has a place to land, and CI that runs the same checks for every PR from day one.",
  deliver=["Cargo workspace with `crates/morm`, `crates/morm-macros`, `crates/morm-migrate`, `crates/morm-admin`, `examples/app`, `tests/`, edition 2024, MSRV 1.99.1",
           "`ci.yml`, `nightly.yml`, `bench.yml` and `release.yml`, including Postgres 15–18 over TCP and Unix socket and a PgBouncer service",
           "`tools/check_coverage.py` copied into the repo and run by `ci.yml` in `--code` mode",
           "A test helper that starts or connects to each Postgres version, and a `trybuild` harness",
           "README skeleton, included as crate docs",
           "Check that the five crate names are still free on crates.io"],
  e2e="A PR containing one trivial integration test tagged `covers: R-nnnn` passes `ci.yml` on every Postgres version and on PgBouncer. Removing the tag makes the coverage job fail.",
  out=["Any morm feature", "Publishing crates"])),

("proofs", dict(title="Proofs of the risky parts", phase="Foundation", deps=["bootstrap"], crates="throwaway `spikes/` directory, deleted when the step ends",
  approve=OWNER,
  goal="Prove on stable Rust 1.99.1 that the designs everything else depends on actually compile and work, before anything depends on them. A failed proof stops the project and goes to the owner with options.",
  deliver=["One small spike per item below, each compiled and run",
           "`plans/plan-approved/proofs.md`: result, code location of the spike, the final shape of each API, and any design change the owner approved",
           "`contracts.md` updated with what was learned"],
  e2e="Every spike builds and passes on stable 1.99.1 with no nightly features, and `proofs.md` records a pass or an owner-approved change for each item.",
  extra="""## Proof list

| # | Proof | Pass means |
|---|---|---|
| 1 | `#[view]` generating sibling structs from one attribute list | Hidden fields are absent, `F` differs per view, and the error message for a hidden field is readable |
| 2 | `#[morm::base]` through a generated `macro_rules!`, across crates, base extending a base | Fields appear as plain fields, with no nesting |
| 3 | Macro-counted `Bits<N>` with an associated type | `db.groups()` is generic over the mask on stable, and a mask for another model is a compile error |
| 4 | `async |tx|` closures with `.retry(n)` | A closure mutating a captured row doesn't compile, and the plain `FnOnce` form still works without `.retry` |
| 5 | Hook signatures, lazy `db: &impl Conn`, `#[morm::hooks]` | Users write `async fn`, futures are `Send` under Axum, the macro records overrides, and a hook with no query checks out nothing |
| 6 | Borrowed `ProductRef<'r>` | Rows decode with no per-field allocation and serialize directly |
| 7 | `NestedConnection` for stream loops | Registry on first poll, re-registration on task change, `join!`, a stream moved between tasks, and one atomic load when no stream is open |
| 8 | Upsert conflict-target markers across macro expansions | A column with no unique key is a compile error, and `UK` constants work for expression and partial indexes |
| 9 | `morm::Param` with `query_typed` | Works for every supported type, `Option<T>`, arrays and `NULL`, through `&dyn morm::Param`, with no boxing |
| 10 | Registration behind `cfg(morm_mgr)` | Models in several crates are all found, including a crate nothing references, and no metadata reaches the app binary under `cargo build --workspace` |
| 11 | The loop detector | The same statement repeating in a short window is detected at low cost, and the listed exclusions work |
| 12 | `deadpool` with our own manager | Dynamic size, idle shrinking and checkout with no extra round trip all work |
""",
  out=["Production code: spikes are thrown away"])),

("contracts-freeze", dict(title="Freeze the contracts", phase="Foundation", deps=["proofs"], crates="`morm`, `morm-macros`: traits and types only",
  approve=OWNER,
  goal="Turn `contracts.md` into compiling Rust: the shared traits, types and error enum that every later step builds on, so parallel steps can't drift apart.",
  deliver=["Public traits, the `Error` enum and shared types compiled in `morm`, with signatures only where a later step fills in behaviour",
           "`contracts.md` matching that code exactly",
           "Parallel groups in `INDEX.md` confirmed against what `contracts.md` covers"],
  e2e="`cargo doc` and `cargo test` pass, and a small test crate implements each trait by hand to prove the signatures are usable.",
  out=["Behaviour of any feature"])),

("model-core", dict(title="`#[model]`, names and types", phase="Models", deps=["contracts-freeze"], crates=f"{MAC}, {RT}",
  goal="Generate everything a model needs at compile time, with no runtime cost, from a plain struct.",
  deliver=["`#[model]`, every struct and field option assigned here, `F`, generated SQL constants, `from_row`, `from_row_named`, `ProductRef`, `Default`, `new`, `HasPk`",
           "Name rules, type inference, the integer table with range CHECKs, `db_set`, `insert_only`, `flatten`, capability markers and the conversion layer"],
  e2e="A model using every option expands to the expected SQL, and rows decode from a real Postgres table created by hand. All compile-time rules are proven with `trybuild`.",
  out=["Indexes and bases (step 05)", "Running queries through `db`"])),

("bases-indexes", dict(title="Bases, indexes and constraint names", phase="Models", deps=["model-core"], par=["views", "groups", "validation"], crates=f"{MAC}",
  goal="Add bases, custom bases, indexes, constraints and the naming convention, including upsert key markers.",
  deliver=["All built-in bases and `#[morm::base]`", "`indexes(...)`, field shortcuts, `checks`, `storage`, `schema`, `extensions`, `deferrable`, typed foreign keys, doc comments as schema comments",
           "Name generation with the 63-byte rule, duplicate detection, and `Product::UK` constants with their markers"],
  e2e="Models with every index form and base compile, their schema metadata matches expected output, and generated names match python-morm's convention, including the hashed long form.",
  out=["Creating anything in a database (step 21 reads this metadata)"])),

("views", dict(title="Views, input, patch and secret", phase="Models", deps=["model-core"], par=["bases-indexes", "groups", "validation"], crates=f"{MAC}, {RT}",
  goal="Generate restricted views and safe public input and output types.",
  deliver=["`#[view]` with `read_hide`, `write_hide`, `groups`, `fields`, `input`, `patch`", "`WriteOnly<T>`, `Patch<T>`, `#[col(secret)]`, generated `Serialize`, `Debug` and `Deserialize`, `#[model(deserialize)]`"],
  e2e="A request body parses into an `input` and a `patch` view with every field state, unknown fields are rejected, and every forbidden form fails to compile.",
  out=["`.apply()` running an update (step 13)"])),

("groups", dict(title="Group declarations and masks", phase="Models", deps=["model-core"], par=["bases-indexes", "views", "validation"], crates=f"{MAC}, {RT}",
  goal="Global and model-scoped groups with macro-counted masks.",
  deliver=["`morm::groups!`, `scoped_groups`, `#[model(groups)]`, `sudo`, `G`, `Bits<N>`, `Mask<T>`, `from_names`, `Model::groups()`, `Model::group_fields()`"],
  e2e="An app with global and scoped groups over 64 entries builds masks, converts names at runtime, and rejects a scoped mask on another model at compile time.",
  out=["Enforcement on queries (step 19)"])),

("validation", dict(title="Declarative rules and `clean`", phase="Models", deps=["model-core"], par=["bases-indexes", "views", "groups"], crates=f"{MAC}, {RT}",
  goal="Generate inline validation with no per-field runtime cost.",
  deliver=["`min`, `max`, `len`, `choices`, `clean`, `db_check`, as generated functions with no function pointers"],
  e2e="Every rule accepts and rejects at its boundary in unit tests, and a field with no rules generates no code.",
  out=["When rules run in a write (step 18)"])),

("config-pool", dict(title="Config, connecting and the pool", phase="Runtime", deps=["contracts-freeze"], par=["model-core", "types", "schema-diff"], crates=RT,
  goal="`morm.toml`, environment variables and `morm::Pool`, with every sizing, safety and failure rule.",
  deliver=["Config loading, `Db::connect`, `connect_as`, `connect_to`, TLS, sockets, named pools, replicas",
           "`morm::Pool` on `deadpool`, dynamic size, recycling, `max_connections` budget shared by pools, failover, `db.hold()`, `db.close()`, `pool_stats()`",
           "`pooler = \"transaction\"` behaviour"],
  e2e="An app starts from `morm.toml` alone, runs `SELECT 1` through every pool type over TCP and socket, grows and shrinks under load, survives a Postgres restart, and shuts down cleanly. The same flow passes behind PgBouncer.",
  out=["Statement caches (step 11)", "Deadlock rules (step 16)"])),

("types", dict(title="Numbers and special types", phase="Runtime", deps=["model-core"], par=["config-pool"], crates=RT,
  goal="Exact encoding and decoding for every supported type, with no silent rounding.",
  deliver=["`Decimal` decoding with no heap use, `morm::Numeric`, every integer path, `morm::Param`, `Range<T>`, `IpAddr`, timestamps, `TextEnum`, `PgEnum`, domains and composite types, `InvalidText`"],
  e2e="Every type round-trips through a real Postgres at its limits, and a value that can't be held exactly returns an error on read and on write.",
  out=["JSON formats (step 17)"])),

("statements", dict(title="Statement caches and fragments", phase="Runtime", deps=["config-pool", "types"], crates=RT,
  goal="The two bounded caches, shape tracking, `query_typed` fallback, fragment renumbering and tags.",
  deliver=["`statement_cache`, `shape_cache`, prepare on second use, built-SQL map, 0A000 handling, `$n` renumbering, `.tag()`"],
  e2e="A load test with random and repeated shapes shows fixed statements staying cached, and a migration changing a table's result type recovers outside a transaction.",
  out=["The query APIs that use the caches (steps 12, 13)"])),

("reads", dict(title="Reads", phase="Runtime", deps=["statements", "views"], par=["writes"], crates=RT,
  goal="Every way of reading: `get`, lists, raw SQL, related rows and keyset pages.",
  deliver=["`get` forms, `select`, `filter`, `filter_cols`, every list and single-result call, `db.sql`, `db.raw`, `#[derive(morm::Row)]`, `load_related`, `keyset`, `order_by_allowed`, `.custom_plan()`, typed errors"],
  e2e="A test app reads the same rows through every call and gets identical data, and each compile-time rule fails under `trybuild`.",
  out=["`fetch_json` formats (step 17)", "Groups on a handle (step 19)"])),

("writes", dict(title="Writes", phase="Runtime", deps=["statements", "views"], par=["reads"], crates=RT,
  goal="Every way of writing except upsert, including bulk paths.",
  deliver=["`insert`, `insert_many` on every path, `insert_stream`, `.explicit_ids()`, `OrDefault`, `update`, `update_q`, `update_each`, `.apply()`, `set_expr`, `.version()`, `delete` forms, `.in_batches`, `.pause_ms`"],
  e2e="A million rows go through each `insert_many` path, atomically or in batches, and defaults, arrays, `.apply()` states and version conflicts behave as specified on real Postgres.",
  out=["Hooks (step 18)", "Upsert (step 14)"])),

("upsert", dict(title="Upsert", phase="Runtime", deps=["writes", "bases-indexes"], crates=RT,
  goal="`upsert` with exact rules for what it overwrites and what it reports.",
  deliver=["Conflict targets by field, tuple and `UK`, partial index filters, `SET` list rules, `.update_only`, the empty-`SET` path, outcome reporting on every Postgres version"],
  e2e="Upserts on every target form, with and without supplied columns, leave unsupplied values alone and report the right outcome on Postgres 15–18.",
  out=["Hook calls on upsert (step 18)"])),

("transactions", dict(title="Transactions", phase="Runtime", deps=["writes"], par=["deadlocks"], crates=RT,
  goal="Transactions whose commit is confirmed, never assumed.",
  deliver=["`db.transaction`, savepoints, `.isolation`, `.retry(n)`, per-transaction settings, `tx.set_local`, confirmed commit, `TransactionAborted`, `CommitUnknown`, pipelined `BEGIN` and `COMMIT`"],
  e2e="A swallowed error can never report a commit, a lost connection during `COMMIT` returns `CommitUnknown`, and a deadlock retried with `.retry(3)` succeeds with fresh rows.",
  out=["Lock helpers (step 16)"])),

("deadlocks", dict(title="Deadlocks, locks and timeouts", phase="Runtime", deps=["transactions"], crates=RT,
  goal="Prevent both kinds of deadlock and bound every wait.",
  deliver=["`NestedConnection`, `extra_conn`, `tx.lock`, `.for_key_change()`, `.lock_ordered()`, advisory locks, `.skip_locked()`, bulk sorting, single-statement retry, every timeout, cancellation"],
  e2e="Forced deadlocks of both kinds resolve as specified, and a cancelled request's query is gone from `pg_stat_activity` with its locks released.",
  out=["The lock-order detector (step 32)"])),

("json", dict(title="JSON output and schemas", phase="Runtime", deps=["reads"], par=["writes"], crates=RT,
  goal="Every JSON path gives the same documented output, with no `Value` or map in between.",
  deliver=["`to_json`, `Serialize`, `ProductRef` serialization, `fetch_json`, `stream_json`, `jsonb` as `RawValue`, `json_schema`, `utoipa`, every fixed format"],
  e2e="For one row with every type, `fetch`, `fetch_ref` and `fetch_json` produce the same JSON, and no secret appears on any path.",
  out=["`to_json_for` on a handle (step 19)"])),

("hooks", dict(title="Hooks and write ordering", phase="Runtime", deps=["writes", "upsert", "transactions", "validation"], crates=f"{RT}, {MAC}",
  goal="Hooks with exact inputs and order, on every write path.",
  deliver=["`Hooks`, `#[morm::hooks]`, `Changes`, `Target`, `Committed`, `validate`, `validate_changes`, lazy `db`, `hooks(tx)`, `after_commit`, `db.tx()`, `blocking`, rule ordering"],
  e2e="A recording model proves the exact order and inputs on every write path, including no connection during `clean`, and `after_commit` firing only on a confirmed commit.",
  out=["Outbox (step 20)"])),

("group-enforcement", dict(title="Groups on queries", phase="Runtime", deps=["reads", "writes", "json", "groups"], crates=RT,
  goal="Enforce groups on a `db.groups(mask)` handle for every read and write.",
  deliver=["`db.groups`, `db.sudo`, `Forbidden`, failing closed on reads, `to_json_for`"],
  e2e="Each read and write path, on plain `db` and on a handle, either returns only allowed data or returns `Forbidden`.",
  out=["New group declarations"])),

("outbox", dict(title="Transactional outbox", phase="Runtime", deps=["hooks"], crates=RT,
  goal="Events saved with their data and delivered at least once.",
  deliver=["`morm::pg_outbox!`, `tx.emit`, `tx.emit_keyed`, the worker, leases, retries, cleanup, the `outbox` feature"],
  e2e="Killing the process and worker at every point never loses an event, and repeats carry the same `e.id`.",
  extra="`pg_outbox!` declares a managed table. Until step 26 exists, tests create it directly; step 26 adds the managed form to its own tests.",
  out=["`morm-admin outbox` (step 27)"])),

("schema-diff", dict(title="Schema IR, scratch and typed ops", phase="Migrations", deps=["bases-indexes"], par=["reads", "writes", "transactions"], crates=MIG,
  goal="Read schemas into a typed IR and diff them into typed operations with inverses.",
  deliver=["Model registration behind `cfg(morm_mgr)` and a minimal `morm-mgr` that prints the manifest, which later migration steps need before step 27 packages it", "`Schema` IR, catalog reader, scratch database handling, diff, `Op` with inverse, lock, transaction and data properties, topological planning"],
  e2e="For a set of schema pairs, the diff produces ops whose forward and inverse SQL move a real database between both states exactly.",
  out=["Writing files (step 22)", "Managed objects beyond tables (step 26)"])),

("proof-files", dict(title="Proof pipeline, files and decisions", phase="Migrations", deps=["schema-diff"], crates=MIG,
  goal="Write a migration only after it's proven, with decisions as explicit data.",
  deliver=["`makemigrations`, the five-part proof, decisions, `up.sql`, `down.sql`, `meta.json`, deterministic output, exit codes"],
  e2e="`makemigrations` on a changed model writes byte-identical files on repeat runs, and refuses to write when any proof part fails.",
  out=["Running migrations (step 23)"])),

("execution", dict(title="Migration execution and recovery", phase="Migrations", deps=["proof-files"], crates=f"{MIG}, {ADM}",
  goal="Run migrations safely, with a defined way forward from any failure.",
  deliver=["`_morm` ledger, `migrate`, `rollback`, `status`, `verify`, `check`, `repair`, `refingerprint`, `sql`, fingerprint gates, advisory lock, preflight, crash recovery"],
  e2e="Killing the process at every step boundary, `--resume` and `rollback` both reach the right state, and two rival migrators can't both run.",
  out=["Safe forms (step 24)"])),

("safe-forms", dict(title="Safe forms, production mode and deploys", phase="Migrations", deps=["execution"], par=["retire", "managed-objects"], crates=f"{MIG}, {ADM}",
  goal="Default to migrations that are safe on live tables, and to forward-only production.",
  deliver=["Every safe form, batched backfills, `mode`, `revert`, `rollback --failed`, emergency override, `squash`, `lint`, `rehearse`, expand and contract, sync triggers, `check_schema`, data hooks"],
  e2e="Each safe form runs under live traffic with no long lock, a production database refuses `rollback` by default, and an old binary boots during an expand phase.",
  out=["Retire and purge (step 25)"])),

("retire", dict(title="Retire, purge and down with data", phase="Migrations", deps=["execution"], par=["safe-forms", "managed-objects"], crates=MIG,
  goal="Make a drop reversible, with no hidden side effects on anything still using the object.",
  deliver=["Retire for columns and tables, `pg_depend` refusals, function source scan, `purge`, `drop now`, `down_fill`, rollback restoring data with its decisions"],
  e2e="A retired column stops blocking deletes, rollback restores its data and constraints, and every dependency case is refused with the right list.",
  out=["Managed object recreation (step 26)"])),

("managed-objects", dict(title="Managed objects", phase="Migrations", deps=["execution"], par=["safe-forms", "retire"], crates=f"{MIG}, {MAC}",
  goal="Manage everything a schema depends on, with exact inverses.",
  deliver=["`pg_function!`, `pg_trigger!`, `pg_policy!`, `pg_procedure!`, `pg_grant!`, `#[pg_view]`, materialized views, enums, domains, dependency recreation, `db.call`, `db.refresh`", "`morm::pg_outbox!` as a managed table, with tests"],
  e2e="A column type change under a view, a function and a trigger plans, runs and reverses exactly on Postgres 15–18.",
  out=["Unmanaged objects, which are never changed"])),

("admin-core", dict(title="`morm-admin` core", phase="Tools", deps=["execution", "bases-indexes"], par=["size", "query-tooling", "telemetry"], crates=ADM,
  goal="One tool for setup, checks and importing existing databases.",
  deliver=["`init`, `doctor`, `import`, `scratch`, `outbox`, `baseline`, `morm-mgr` packaging and `cfg(morm_mgr)` builds, Axum template, socket detection"],
  e2e="`morm-admin init --axum` creates a project that builds, migrates and serves a request. `import` on the madmin-style test schema round-trips to no diff.",
  extra="`doctor` gathers checks other steps defined: credentials, pooler role settings and parameter logging (09), scratch match (21), function scan (25), real roles (26). Step 28 adds the `size` line.",
  out=["`size` (step 28)"])),

("size", dict(title="`morm-admin size`", phase="Tools", deps=["config-pool", "admin-core"], par=["query-tooling", "telemetry"], crates=ADM,
  goal="Estimate resources from settings, and settings from resources.",
  deliver=["`size` in both directions, `--here` with cgroup limits, `--share`, `--emit`, `--measure`, `[server]` use, exit codes, calibrated memory figures"],
  e2e="On a real server, estimates are checked against measured memory, and `--here` gives a config that starts Postgres and the app.",
  out=["`--live` (later work)"])),

("query-tooling", dict(title="`sql!`, `check-sql`, test support and the loop detector", phase="Tools", deps=["reads", "admin-core"], par=["size", "telemetry"], crates=f"{RT}, {MAC}, {ADM}",
  goal="Checked raw SQL with no database at build time, and test tooling for users.",
  deliver=["`sql!`, `check-sql` with `PREPARE` and `EXPLAIN`, `#[morm::test]`, the loop detector, `morm::allow_repeats`"],
  e2e="A wrong column in `sql!` fails `check-sql`, a query in a loop fails a `#[morm::test]`, and each exclusion passes.",
  out=["The lock-order detector (step 32)"])),

("telemetry", dict(title="Telemetry", phase="Tools", deps=["statements", "hooks"], par=["size", "query-tooling"], crates=RT,
  goal="OpenTelemetry-shaped spans and metrics, cheap enough to leave on.",
  deliver=["`tracing` and `metrics` features, `[telemetry]`, `span_sample`, `sample_scope`, `slow_query`, per-connection counters and histograms, the publishing task"],
  e2e="Spans reach an OpenTelemetry Collector with the documented fields, metrics are scraped, and a build with both features off contains no telemetry code.",
  out=["Exporter setup, which belongs to apps"])),

("benchmarks", dict(title="Benchmarks", phase="Finish", deps=["group-enforcement", "outbox", "telemetry"], crates="`benches/`",
  goal="Measure every performance claim the plan makes, and hold it to its targets.",
  deliver=["Criterion suite against raw tokio-postgres, SQLx, SeaORM and Diesel, every comparison listed below, `bench.yml` comparing with the last run"],
  e2e="Every target in the requirements is met or reported to the owner with numbers. A missed target is a bug.",
  extra="Other steps defer these measurements here: `.lock_ordered()` cost, statement cache split against a single cache, prepared against `query_typed` patches, `Bits<N>` word cost, `fetch_json` against `fetch_ref`, conversion-layer overhead, the `COPY` threshold, and the array literal path.",
  out=["New features"])),

("hardening", dict(title="Lock-order detector and hardening", phase="Finish", deps=["group-enforcement", "outbox", "safe-forms", "retire", "managed-objects"], par=["benchmarks"], crates="all",
  goal="The long-running proofs across the whole system.",
  deliver=["Lock-order detector", "Long property tests for models and migrations", "Fuzzing", "Full chain on Postgres 15–18", "`cargo audit` and `cargo deny` clean"],
  e2e="`nightly.yml` passes for seven runs in a row.",
  out=["New features"])),

("readme-release", dict(title="README and 0.1.0", phase="Finish", deps=["benchmarks", "hardening", "admin-core", "size", "query-tooling"], crates="all", approve=OWNER,
  goal="A README that is enough on its own, then the release.",
  deliver=["Full README with every table", "CI check comparing the Extra calls table with the public API", "Every doc example compiled and run", "`--code` coverage clean", "`0.1.0` published, after the owner says so"],
  e2e="A new project built only from the README works, `check_coverage.py --code` passes for every step, and CI is green on `main`.",
  out=["Features in `later-defo-build.plan`"])),
]

STEPS = OrderedDict()
for i, (k, v) in enumerate(_S, 1):
    v["n"] = i
    STEPS[k] = v
