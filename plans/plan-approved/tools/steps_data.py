"""Hand-written header of each step. `@key` becomes that step's number."""
import re
from collections import OrderedDict
from pathlib import Path

OWNER = "the owner approves the PR"
PROOF = (Path(__file__).parent / "proof_list.md").read_text(encoding="utf-8")


def S(key, title, phase, deps, crates, areas, goal, deliver, e2e, out, **kw):
    return key, dict(title=title, phase=phase, deps=deps, crates=crates, areas=areas, goal=goal,
                     deliver=deliver, e2e=e2e, out=out, **kw)


_S = [
S("bootstrap", "Repo bootstrap and CI", "Foundation", [], "workspace, all five crates as empty shells", ["all"],
  "Create the workspace so every later step has a place to land, and CI that runs the same checks for every PR from day one.",
  ["Cargo workspace with `crates/morm`, `crates/morm-core`, `crates/morm-macros`, `crates/morm-migrate`, `crates/morm-admin`, `examples/app`, `tests/`, edition 2024, MSRV 1.99.1",
   "`ci.yml`, `nightly.yml`, `bench.yml` and `release.yml`, including Postgres 15–18 over TCP and Unix socket and a PgBouncer service. `nightly.yml` also has `workflow_dispatch`, so it can run on a branch",
   "`ci.yml` runs `plans/plan-approved/tools/check_coverage.py --code crates tests benches fuzz .github`",
   "A test helper that starts or connects to each Postgres version, and a `trybuild` harness",
   "README skeleton, included as crate docs",
   "Check that all six crate names, including `morm-mgr`, are still free on crates.io"],
  "A PR containing one trivial integration test tagged `covers: R-nnnn` passes `ci.yml` on every Postgres version and on PgBouncer. Removing the tag makes `--step` fail for that step.",
  ["Any morm feature", "Publishing crates"], approve=OWNER),
S("proofs", "Proofs of the risky parts", "Foundation", ["bootstrap"], "throwaway `spikes/` directory, deleted when the step ends", ["all"],
  "Prove on stable Rust 1.99.1 that the designs everything else depends on actually compile and work, before anything depends on them. A failed proof stops the project and goes to the owner with options.",
  ["One small spike per item below, each compiled and run",
   "`plans/plan-approved/proofs.md`: result, the final shape of each API, and any design change the owner approved",
   "`contracts.md` updated with what was learned"],
  "Every spike builds and passes on stable 1.99.1 with no nightly features, and `proofs.md` records a pass or an owner-approved change for each item.",
  ["Production code: spikes are thrown away"], approve=OWNER, extra=PROOF),
S("contracts-freeze", "Freeze the contracts", "Foundation", ["proofs"], "`morm`, `morm-core`, `morm-macros`: traits and types only", ["all"],
  "Turn `contracts.md` into compiling Rust: the shared traits, types and error enum that every later step builds on, so parallel steps can't drift apart.",
  ["Everything listed in `contracts.md`, compiled, with signatures only where a later step fills in behaviour, including the items marked \"defined in this step\"",
   "`contracts.md` matching that code exactly",
   "The parallel groups in `INDEX.md` confirmed against what `contracts.md` covers"],
  "`cargo doc` and `cargo test` pass, and a small test crate implements each trait by hand to prove the signatures are usable, including a model with no primary key and one with a composite key.",
  ["Behaviour of any feature"], approve=OWNER),

S("model-core", "`#[model]`, names and types", "Models", ["contracts-freeze"], "`morm-macros`, `morm`, `morm-core`", ["macro"],
  "Generate everything a model needs at compile time, with no runtime cost, from a plain struct.",
  ["`#[model]`, every struct and field option assigned here, `F`, generated SQL constants, `Default`, `new`, `HasPk`",
   "Name rules, `db_set`, `flatten`, metadata for each model"],
  "A model using every option assigned here expands to the expected SQL and metadata, and each compile-time rule is proven with `trybuild`.",
  ["Types and decoding (step @model-types)", "Indexes and bases (step @bases-indexes)", "Running queries through `db`"]),
S("model-types", "Type inference and decoding", "Models", ["model-core"], "`morm-macros`, `morm`", ["macro"],
  "Map Rust types to columns, and decode rows with no needless allocation.",
  ["Type inference, the integer table with range CHECKs in metadata, `from_row`, `from_row_named`, `ProductRef`, capability markers on `F`, the conversion layer"],
  "A model with every inferred type decodes rows from a real Postgres table created by hand, borrowed and owned, except the types step @types owns: `Decimal`, `Numeric`, `Range<T>`, `IpAddr`, enums, domains and composite types. Each compile-time rule is proven with `trybuild`.",
  ["Encoding exactness and `Decimal` (step @types)"]),
S("bases-indexes", "Bases, indexes and constraint names", "Models", ["model-core"], "`morm-macros`, `morm-core`", ["macro"],
  "Add bases, custom bases, indexes, constraints and the naming convention, including upsert key markers.",
  ["All built-in bases and `#[morm::base]`", "`indexes(...)`, field shortcuts, `checks`, `storage`, `schema`, `extensions`, `deferrable`, typed foreign keys, doc comments as schema comments",
   "Name generation with the 63-byte rule in `morm-core`, duplicate detection, `Product::UK` constants with their markers, and the marker for unique columns that `keyset` also uses"],
  "Models with every index form and base compile, their metadata matches expected output, and generated names match python-morm's convention, including the hashed long form.",
  ["Creating anything in a database (step @schema-diff reads this metadata)"]),
S("views", "Views, input, patch and secret", "Models", ["bases-indexes", "model-types"], "`morm-macros`, `morm`", ["macro"],
  "Generate restricted views and safe public input and output types.",
  ["`#[view]` with `read_hide`, `write_hide`, `fields`, `input`, `patch`", "`WriteOnly<T>`, `Patch<T>`, `#[col(secret)]`, generated `Serialize`, `Debug` and `Deserialize`, `#[model(deserialize)]`"],
  "A request body parses into an `input` and a `patch` view with every field state, unknown fields are rejected, and every forbidden form fails to compile.",
  ["`.apply()` running an update (step @updates)", "`groups =` on views (step @groups)"]),
S("groups", "Group declarations and masks", "Models", ["views"], "`morm-macros`, `morm`", ["macro"],
  "Global and model-scoped groups with macro-counted masks.",
  ["`morm::groups!`, `scoped_groups`, `#[model(groups)]`, `sudo`, `#[view(groups = ...)]`, `G`, `Bits<N>`, `Mask<T>`, `from_names`, `Model::groups()`, `Model::group_fields()`"],
  "An app with global and scoped groups over 64 entries builds masks, converts names at runtime, and rejects a scoped mask on another model at compile time.",
  ["Enforcement on queries (step @group-enforcement)"]),
S("validation", "Declarative rules and `clean`", "Models", ["model-core"], "`morm-macros`, `morm`", ["macro", "validation"],
  "Generate inline validation with no per-field runtime cost.",
  ["`min`, `max`, `len`, `choices`, `clean`, `db_check`, as generated functions with no function pointers"],
  "Every rule accepts and rejects at its boundary in unit tests, and a field with no rules generates no code.",
  ["When rules run in a write (step @hooks)"]),

S("config", "Config, TLS and connecting", "Runtime", ["contracts-freeze"], "`morm-core`, `morm`", ["config"],
  "`morm.toml` and environment variables, in `morm-core` so `morm-migrate` and `morm-admin` read the same config, plus connecting over sockets, TCP and TLS.",
  ["Config loading in `morm-core`, every key and variable, `[database.migration]`, `[server]`, named pool profiles and replicas as config", "`Db::connect`, `connect_as`, `connect_to`, sockets, TLS, `search_path`, startup parameters"],
  "An app connects from `morm.toml` alone, and from environment variables alone, over TCP, Unix socket and TLS, and every invalid config fails with a message naming the setting.",
  ["The pool (step @pool)", "`init` socket detection (step @admin-init)"]),
S("pool", "`morm::Pool`", "Runtime", ["config"], "`morm`", ["pool"],
  "The pool, with sizing, budget and failure rules.",
  ["`morm::Pool` on `deadpool`, dynamic size, idle shrinking, recycling, `recycle_when`, the shared `max_connections` budget through the `morm-core` function, version refusal, failover, `pool_stats()`, an internal query entry point that later steps build on"],
  "`SELECT 1` through every pool type under load grows and shrinks the pool, a Postgres restart is survived, and a pool over budget is lowered with a warning.",
  ["Statement caches (step @statements)", "`db.hold()` and pooler rules (step @hold-pooler)"]),
S("hold-pooler", "`db.hold()`, jobs and PgBouncer", "Runtime", ["pool"], "`morm`", ["pool"],
  "Session connections, and correct behaviour behind a pooler.",
  ["`db.hold()`, release on every exit path, `on_connect`, `listen`, `max_holds`, the pool side of `db.close()`, `pooler = \"transaction\"`"],
  "A held connection is released on every exit path and is never reused, and an app runs correctly behind PgBouncer with `hold` refused.",
  ["Waiting for `after_commit` on shutdown (step @hooks-more)"]),
S("types", "Numbers and special types", "Runtime", ["model-types"], "`morm`, `morm-macros`", ["macro", "types"],
  "Exact encoding and decoding for every supported type, with no silent rounding.",
  ["`Decimal` decoding with no heap use, `morm::Numeric`, every integer path, `morm::Param` for every type, `Range<T>`, `IpAddr`, timestamps, `TextEnum`, `PgEnum`, domains and composite types"],
  "Every type round-trips through a real Postgres at its limits, and a value that can't be held exactly returns an error on read and on write.",
  ["JSON formats (step @json)"]),
S("statements", "Statement caches and fragments", "Runtime", ["pool", "types"], "`morm`", ["query"],
  "The two bounded caches, shape tracking, `query_typed` fallback, fragment renumbering and the error mapping every query uses.",
  ["`statement_cache`, `shape_cache`, prepare on second use, built-SQL map, 0A000 handling, `$n` renumbering, `.tag()` text, typed error mapping"],
  "A load test with random and repeated shapes shows fixed statements staying cached, and a change to a table's result type recovers outside a transaction.",
  ["The query APIs that use the caches (steps @reads, @inserts)"]),
S("reads", "Reads", "Runtime", ["statements", "views"], "`morm`, `morm-macros`", ["query"],
  "Every way of reading, and the shared `by`/`any` builder.",
  ["`get` forms, `select`, `filter`, `filter_cols`, every list and single-result call, `order_by_allowed`, `.custom_plan()`, `.tag()` on queries, replicas through `db.pool`"],
  "A test app reads the same rows through every call and gets identical data, and each compile-time rule fails under `trybuild`.",
  ["`db.sql`, `load_related`, `keyset` (step @reads-more)", "`fetch_json` formats (step @json)", "Groups on a handle (step @group-enforcement)"]),
S("reads-more", "Raw SQL, related rows and keyset pages", "Runtime", ["reads"], "`morm`, `morm-macros`", ["query"],
  "The read paths beyond a model's own table.",
  ["`db.sql`, `db.raw`, `#[derive(morm::Row)]`, `load_related`, `keyset`, `.limit` on raw statements"],
  "Raw SQL, related rows and keyset pages return correct rows on real Postgres, including pages under concurrent writes, and each compile-time rule fails under `trybuild`.",
  ["`sql!` (step @query-tooling)", "`fetch_json` formats (step @json)"]),
S("inserts", "Inserts", "Runtime", ["statements", "views"], "`morm`, `morm-macros`", ["insert"],
  "Every way of adding rows, including bulk paths.",
  ["The internal transaction with confirmed commit that `contracts.md` lists, which step @transactions then builds `db.transaction` on", "`insert`, `insert_many` on every path, `insert_stream`, `.in_batches` on inserts, `.explicit_ids()`, `OrDefault`, statements per pattern, array columns"],
  "A million rows go through each `insert_many` path, atomically or in batches, and defaults, arrays and explicit ids behave as specified on real Postgres.",
  ["Hooks (step @hooks)", "Upsert (step @upsert)"]),
S("updates", "Updates and deletes", "Runtime", ["reads-more", "inserts"], "`morm`, `morm-macros`", ["query"],
  "Every way of changing and removing rows.",
  ["`update`, `update_q`, `update_each`, `.apply()`, `set_expr`, `.version()`, `delete` forms, `.in_batches`, `.pause_ms`, `.returning()`, `ReadOnly` on replicas"],
  "`.apply()` states, version conflicts, batches stopping and resuming, and `NotFound` all behave as specified on real Postgres.",
  ["Hooks (step @hooks)"]),
S("upsert", "Upsert", "Runtime", ["updates"], "`morm`, `morm-macros`", ["query"],
  "`upsert` with exact rules for what it overwrites and what it reports.",
  ["`db.upsert`, `db.upsert_many`, conflict targets by field, tuple and `UK`, partial index filters, `SET` list rules, `.update_only`, the empty-`SET` path, outcome reporting on every Postgres version"],
  "Upserts on every target form, with and without supplied columns, leave unsupplied values alone and report the right outcome on Postgres 15–18.",
  ["Hook calls on upsert (step @hooks-more)"]),
S("transactions", "Transactions", "Runtime", ["updates"], "`morm`", ["tx"],
  "Transactions whose commit is confirmed, never assumed.",
  ["`db.transaction`, `tx` with the same API as `db`, savepoints, `.isolation`, `.retry(n)`, per-transaction settings, `tx.set_local`, `tx.get_for_update`, confirmed commit, `TransactionAborted`, `CommitUnknown`, pipelining on `tx`"],
  "A swallowed error can never report a commit, a lost connection during `COMMIT` returns `CommitUnknown`, and a deadlock retried with `.retry(3)` succeeds with fresh rows.",
  ["Lock helpers (step @locks-timeouts)"]),
S("pool-deadlocks", "Pool deadlocks", "Runtime", ["transactions", "hold-pooler"], "`morm`", ["pool"],
  "A task can't deadlock the pool against itself, and pool waits are always finite.",
  ["`NestedConnection` in transactions and stream loops, `db.extra_conn()`, `wait_timeout`, `max_holds` on waits"],
  "Using `db` inside a transaction or a stream loop is refused, including `join!` and a stream moved between tasks, and an app with no open streams pays one atomic load.",
  ["Postgres deadlocks (step @locks-timeouts)"]),
S("locks-timeouts", "Locks, timeouts and cancellation", "Runtime", ["pool-deadlocks", "upsert"], "`morm`", ["tx"],
  "Prevent Postgres deadlocks and bound every lock wait.",
  ["`tx.lock`, `.for_key_change()`, `.lock_ordered()`, advisory locks, `.skip_locked()`, bulk sorting, single-statement retry, every timeout, cancellation"],
  "Forced deadlocks of both kinds resolve as specified, and a cancelled request's query is gone from `pg_stat_activity` with its locks released.",
  ["The lock-order detector (step @hardening)"]),
S("json", "JSON output and schemas", "Runtime", ["reads-more", "inserts"], "`morm`, `morm-macros`", ["json"],
  "Every JSON path gives the same documented output, with no `Value` or map in between.",
  ["`to_json`, `Serialize`, `ProductRef` serialization, `fetch_json`, `stream_json`, `jsonb` as `RawValue`, `json_schema`, `utoipa`, every fixed format"],
  "For one row with every type, `fetch`, `fetch_ref` and `fetch_json` produce the same JSON, and no secret appears on any path.",
  ["`to_json_for` on a handle (step @group-enforcement)"]),
S("hooks", "Hooks and write ordering", "Runtime", ["locks-timeouts", "validation"], "`morm`, `morm-macros`", ["query", "tx"],
  "Hooks with exact inputs and order on `insert`, `update` and `delete`.",
  ["`Hooks`, `#[morm::hooks]`, `Changes`, `Target`, `Committed`, `validate`, `validate_changes`, lazy `db`, `hooks(tx)`, `after_commit`, `db.tx()`, `blocking`, rule ordering, pipelined `BEGIN` and `COMMIT`"],
  "A recording model proves the exact order and inputs on `insert`, `insert_many`, `update` and `delete`, including no connection during `clean`, and `after_commit` firing only on a confirmed commit.",
  ["Upsert, batches and shutdown (step @hooks-more)"]),
S("hooks-more", "Hooks on upsert, batches and shutdown", "Runtime", ["hooks"], "`morm`", ["query", "tx"],
  "Finish hooks on the write paths with special rules.",
  ["Hooks and outcomes on `upsert`, `update_each`, `.in_batches`, batch upserts, `insert_stream`, and `db.close()` waiting for `after_commit`"],
  "A recording model proves the order and inputs on each of these paths, including no hook after an unchanged upsert, on Postgres 15–18.",
  ["Outbox (step @outbox)"]),
S("group-enforcement", "Groups on queries", "Runtime", ["json", "hooks-more", "groups"], "`morm`, `morm-macros`", ["query", "macro"],
  "Enforce groups on a `db.groups(mask)` handle for every read and write.",
  ["`db.groups`, `db.sudo`, `Forbidden`, failing closed on reads, `to_json_for`"],
  "Each read and write path, on plain `db` and on a handle, either returns only allowed data or returns `Forbidden`.",
  ["New group declarations"]),
S("outbox", "Transactional outbox", "Runtime", ["hooks-more"], "`morm`", ["outbox"],
  "Events saved with their data and delivered at least once.",
  ["`tx.emit`, `tx.emit_keyed`, the worker, leases, retries, cleanup, the `outbox` feature, the table definition"],
  "Killing the process and worker at every point never loses an event, and repeats carry the same `e.id`.",
  ["The managed `pg_outbox!` form and `morm-admin outbox` (step @admin-init); tests create the table directly"]),

S("manifest", "Registration and `morm-mgr`", "Migrations", ["bases-indexes", "model-types"], "`morm-macros`, `morm-core`, `morm-admin`", ["macro", "manifest"],
  "Get model metadata out of a project without touching the app binary.",
  ["Registration behind `cfg(morm_mgr)`, a minimal `morm-mgr` that prints the JSON manifest, the `morm-admin` code that builds it in its own target directory with that `cfg`, manifest types in `morm-core`"],
  "`morm-mgr` prints a correct manifest for models in several crates, including one nothing references, and the app binary contains none of it.",
  ["`morm-mgr` packaging for `init` (step @admin-init)"]),
S("schema-ir", "Schema IR, catalog and scratch", "Migrations", ["manifest", "config"], "`morm-migrate`", ["planner"],
  "Read any schema, from models or a live database, into one typed form.",
  ["`Schema` IR for everything the plan manages, the catalog reader, scratch database handling, creating models in a temporary database, replaying a chain"],
  "A model set created in a scratch database reads back into an IR equal to the one built from a replayed chain, on Postgres 15–18.",
  ["The diff (step @schema-diff)"]),
S("schema-diff", "Diff and typed ops", "Migrations", ["schema-ir"], "`morm-migrate`", ["planner"],
  "Diff two schemas into typed operations with inverses.",
  ["Diff, `Op` with inverse, lock, transaction and data properties, topological planning, ops for tables, columns, constraints, indexes, schemas and extensions"],
  "For a set of schema pairs, the diff produces ops whose forward and inverse SQL move a real database between both states exactly.",
  ["Writing files (step @proof-files)", "Managed objects beyond tables (step @managed-decl)"]),
S("lint", "`lint`", "Migrations", ["schema-diff", "views", "types"], "`morm-migrate`, `morm-admin`", ["planner", "admin"],
  "Report risky statements and weak schemas, gathered here from every rule in the plan.",
  ["The `morm-admin mgr` dispatcher, with one module per command", "`mgr lint` with `--url` and accepted findings, statement risk from `Op` properties, every index, `numeric`, `secret`-name and `SECURITY DEFINER` rule"],
  "Each lint rule fires on a fixture that breaks it and stays quiet on one that doesn't, and `--url` reports only tables big enough to matter.",
  ["Enforcing `lint` in the proof (step @proof-files)"]),
S("proof-files", "Proof pipeline, files and decisions", "Migrations", ["lint"], "`morm-migrate`, `morm-admin`", ["planner", "admin"],
  "Write a migration only after it's proven, with decisions as explicit data.",
  ["`makemigrations`, the five-part proof, decisions, `up.sql`, `down.sql`, `meta.json`, deterministic output, exit codes, `min_postgres` and extension checks"],
  "`makemigrations` on a changed model writes byte-identical files on repeat runs, and refuses to write when any proof part fails.",
  ["Running migrations (step @execution)"]),
S("execution", "Migration execution and recovery", "Migrations", ["proof-files"], "`morm-migrate`, `morm-admin`", ["planner", "admin"],
  "Run migrations safely, with a defined way forward from any failure.",
  ["`_morm` ledger, `migrate`, `status`, `check`, `sql`, fingerprint gates, advisory lock, lock safety, preflight"],
  "A chain of migrations runs on a real database, each gate refuses when its condition is forced, and two rival migrators can't both run.",
  ["Recovery (step @recovery)", "Safe forms (step @safe-forms)"]),
S("recovery", "Rollback and crash recovery", "Migrations", ["execution"], "`morm-migrate`, `morm-admin`", ["planner", "admin"],
  "A defined way forward from any failure.",
  ["`rollback` with `--failed` and `--orphans`, `--resume`, batch cursors, `repair`, `verify`, `refingerprint`, Postgres version handling"],
  "Killing the process at every step boundary, `--resume` and `rollback` both reach the right state, and a tampered migration is refused.",
  ["Production guards (step @live-modes)"]),
S("managed-decl", "Managed objects: declarations", "Migrations", ["recovery"], "`morm-macros`, `morm-core`", ["macro", "manifest"],
  "Declare every managed object in Rust, next to its model.",
  ["`pg_function!`, `pg_trigger!`, `pg_policy!`, `pg_procedure!`, `pg_grant!`, `#[pg_view]` with `materialized`, `file` paths, `on = Model` references, each object in the manifest"],
  "Every macro form appears correctly in the manifest, a bad `on` reference fails to compile, and none of it reaches the app binary.",
  ["Applying objects to a database (step @managed-ops)"]),
S("managed-ops", "Managed objects: catalog and ops", "Migrations", ["managed-decl", "updates"], "`morm-migrate`, `morm-macros`, `morm`", ["planner", "macro"],
  "Read each object kind from the catalog, diff it, and apply it with an exact inverse.",
  ["IR, catalog reading, diff and `Op` with inverse for each kind: functions, procedures, triggers, policies, grants, views, materialized views, enums, domains, composite types, sequences", "Running `.sql` definitions on scratch", "Placeholder roles on scratch", "Read-only view models, `db.call`, `db.refresh`"],
  "Each object kind goes from a declaration to a real database and back through the diff with no drift, on Postgres 15–18.",
  ["Dependency recreation (step @managed-deps)"]),
S("managed-deps", "Managed objects: dependencies", "Migrations", ["managed-ops"], "`morm-migrate`", ["planner"],
  "Change columns under objects that depend on them, with exact inverses.",
  ["Fixed kind order, resolution through `pg_depend`, `CREATE OR REPLACE` or drop and recreate, materialized view rebuilds"],
  "A column type change under a view, a function and a trigger plans, runs and reverses exactly on Postgres 15–18.",
  ["Unmanaged objects, which are never changed"]),
S("retire", "Retire, purge and down with data", "Migrations", ["managed-deps"], "`morm-migrate`", ["planner"],
  "Make a drop reversible, with no hidden side effects on anything still using the object.",
  ["Retire for columns and tables, `pg_depend` refusals, function source scan, `purge`, `drop now`, `down_fill`, rollback restoring data with its decisions, retired state in `verify`"],
  "A retired column stops blocking deletes, rollback restores its data and constraints, and every dependency case is refused with the right list.",
  ["`revert` (step @live-modes)"]),
S("safe-forms", "Safe forms", "Migrations", ["retire"], "`morm-migrate`", ["planner"],
  "Default to migrations that are safe on live tables.",
  ["Every row of the safe forms table, batched backfills, sync triggers for type changes, `--fast`, refusal of key type changes"],
  "Each safe form runs under live traffic with no long lock, and no write is lost during a type-change copy.",
  ["Production mode (step @live-modes)"]),
S("live-modes", "Production mode, revert and squash", "Migrations", ["safe-forms"], "`morm-migrate`, `morm-admin`", ["planner", "admin"],
  "Forward-only production, with safe ways to undo and tidy up.",
  ["`mode` with `--confirm`, `rollback` guards and the emergency override, `revert`, `rehearse`, `squash`"],
  "A new database refuses `rollback`, `revert` restores retired data through a forward migration, and a squashed chain gives the same schema as the full one.",
  ["Rolling deploys (step @rolling)"]),
S("rolling", "Rolling deploys", "Migrations", ["live-modes", "reads"], "`morm-migrate`, `morm`, `morm-macros`", ["planner", "macro"],
  "Keep old and new binaries working through a deploy.",
  ["`phase`, expand and contract, rename sync triggers, `deprecated` steps, `db.check_schema()` with its per-model column list, data migration hooks"],
  "An old binary boots and works during an expand phase, and `check_schema` reports a dropped default.",
  ["New features"],
  extra="**Waiting on the owner:** Rust data hooks need somewhere to run, because `morm-admin` holds no project code and a production host has no Rust toolchain. Stop and ask before building that part. The rest of this step can go ahead."),

S("admin-init", "`morm-admin init` and project setup", "Tools", ["retire", "outbox", "inserts", "reads-more"], "`morm-admin`, `morm-macros`", ["admin"],
  "Create a working project from one command.",
  ["`init` with socket detection and the Axum template, `morm-mgr` packaging, `scratch`, `outbox`, `baseline`, the managed `morm::pg_outbox!` form"],
  "`morm-admin init --axum` creates a project that builds, migrates and serves a request, and `outbox list` and `retry` work on it.",
  ["`doctor` (step @doctor)", "`import` (step @import)", "`size` (step @size)"]),
S("doctor", "`morm-admin doctor`", "Tools", ["admin-init"], "`morm-admin`, `morm-core`", ["doctor"],
  "One command that reports everything that's wrong with a setup, gathered here from every rule in the plan.",
  ["Every `doctor` check listed below, using the budget function in `morm-core`"],
  "`doctor` reports each check correctly when its condition is forced, and passes on a healthy project.",
  ["`size` (step @size)"]),
S("import", "`morm-admin import`", "Tools", ["admin-init"], "`morm-admin`", ["import"],
  "Generate models from any existing database, gathered here from every import rule in the plan.",
  ["`import` with exact types, names, `keep_case`, `secret`, `Decimal` choices, and a list of what models can't express"],
  "`import` on the fixture `tests/fixtures/import.sql` round-trips to no diff. Write that fixture to hold mixed-case names, composite keys, every index kind, deferrable constraints, views, triggers and an exclusion constraint.",
  ["Changing the imported database"]),
S("query-tooling", "`sql!`, `check-sql`, test support and the loop detector", "Tools", ["admin-init"], "`morm`, `morm-macros`, `morm-admin`", ["tooling", "entry", "macro"],
  "Checked raw SQL with no database at build time, and test tooling for users.",
  ["`sql!`, `check-sql` with `PREPARE` and `EXPLAIN`, including hidden columns and unquoted mixed-case names, `#[morm::test]`, the loop detector, `morm::allow_repeats`"],
  "A wrong column in `sql!` fails `check-sql`, a query in a loop fails a `#[morm::test]`, and each exclusion passes.",
  ["The lock-order detector (step @hardening)"]),
S("size", "`morm-admin size`", "Tools", ["pool", "doctor", "query-tooling"], "`morm-admin`, `morm-core`", ["size"],
  "Estimate resources from settings, and settings from resources.",
  ["`size` in both directions, `--here` with cgroup limits, `--share`, `--emit`, `--measure`, `[server]` use, exit codes, calibrated memory figures, the `doctor` line"],
  "On a real server, estimates are checked against measured memory, and `--here` gives a config that starts Postgres and the app.",
  ["`--live` (later work)"]),
S("telemetry", "Telemetry", "Tools", ["statements", "hooks-more"], "`morm`", ["telemetry", "entry"],
  "OpenTelemetry-shaped spans and metrics, cheap enough to leave on.",
  ["The warning sink moved onto `tracing`", "`tracing` and `metrics` features, `span_sample`, `sample_scope`, `slow_query`, per-connection counters and histograms, the publishing task"],
  "Spans reach an OpenTelemetry Collector with the documented fields, metrics are scraped, and a build with both features off contains no telemetry code.",
  ["Exporter setup, which belongs to apps"]),

S("benchmarks", "Benchmarks", "Finish", ["group-enforcement", "outbox", "telemetry", "query-tooling"], "`benches/`", ["bench"],
  "Measure every performance claim the plan makes, and hold it to its targets.",
  ["Criterion suite against raw tokio-postgres, SQLx, SeaORM and Diesel, every comparison listed below, `bench.yml` comparing with the last run"],
  "Every target in the requirements is met or reported to the owner with numbers. A missed target is a bug.",
  ["New features"],
  extra="Other steps defer these measurements here: `.lock_ordered()` cost, statement cache split against a single cache, prepared against `query_typed` patches, `Bits<N>` word cost, `fetch_json` against `fetch_ref`, conversion-layer overhead, the `COPY` threshold, and the array literal path."),
S("hardening", "Lock-order detector and hardening", "Finish", ["group-enforcement", "outbox", "rolling", "query-tooling", "telemetry"], "all", ["hard"],
  "The long-running proofs across the whole system.",
  ["Lock-order detector", "Long property tests for models and migrations", "Fuzzing", "Full chain on Postgres 15–18", "An audit that every compile-time guarantee has a `trybuild` case", "`cargo audit` and `cargo deny` clean"],
  "`nightly.yml` passes seven runs in a row on this step's branch, started through `workflow_dispatch`.",
  ["New features"]),
S("readme-release", "README and 0.1.0", "Finish", ["benchmarks", "hardening", "import", "size", "doctor"], "all", ["all"],
  "A README that is enough on its own, then the release.",
  ["Full README with every table", "CI check comparing the Extra calls table with the public API", "Every doc example compiled and run", "`--code` coverage clean", "A check that no `unimplemented!` from step @contracts-freeze remains", "`0.1.0` published, after the owner says so"],
  "A new project built only from the README works, `check_coverage.py --code` passes for every step, and CI is green on `main`.",
  ["Features in `later-defo-build.plan`"], approve=OWNER),
]

STEPS = OrderedDict()
for i, (k, v) in enumerate(_S, 1):
    v["n"] = i
    STEPS[k] = v
for k, v in STEPS.items():
    for d in v["deps"]:
        assert STEPS[d]["n"] < v["n"], (k, d)


def ancestors(k, seen=None):
    seen = seen if seen is not None else set()
    for d in STEPS[k]["deps"]:
        if d not in seen:
            seen.add(d)
            ancestors(d, seen)
    return seen


def wave(k, memo={}):
    if k not in memo:
        memo[k] = 1 + max((wave(d) for d in STEPS[k]["deps"]), default=0)
    return memo[k]


def beside(k):
    """Steps with no dependency path and no shared area."""
    out = []
    a = STEPS[k]
    for o, b in STEPS.items():
        if o == k or o in ancestors(k) or k in ancestors(o):
            continue
        if "all" in a["areas"] or "all" in b["areas"] or set(a["areas"]) & set(b["areas"]):
            continue
        out.append(o)
    return out


def sub(text):
    return re.sub(r"@([a-z][a-z-]+)", lambda m: f"{STEPS[m.group(1)]['n']:02d}" if m.group(1) in STEPS else m.group(0), text)


CTX = {
    "inserts": [r"^\*\*A commit is confirmed", r"^- Any Postgres error marks", r"^- `COMMIT` is sent as"],
    "outbox": [r"^\[database\]|\[outbox\]"], "telemetry": [r"\[telemetry\]"], "pool": [r"\[pool\]"],
    "size": [r"^- Connections needed", r"^- Only as many queries", r"budget = \(", r"^Pools open in one"],
    "benchmarks": [r"^\| Features compiled out|^\| `tracing` |^\| `metrics` on"],
    "updates": [r"^```rust\ndb\.insert\(&mut p\)"], "upsert": [r"^```rust\ndb\.insert\(&mut p\)"],
    "proof-files": [r"^\*\*How it fits together", r"^```\nmakemigrations"], "execution": [r"^\*\*How it fits together"],
    "recovery": [r"^```\nmakemigrations"], "live-modes": [r"^```\nmakemigrations"], "retire": [r"^```\nmakemigrations"],
    "hold-pooler": [r"\[pool\]", r"^- \*\*Shutdown"],
    "admin-init": [r"^\*\*How it fits together", r"^\*\*Registration", r"^```rust\nmorm::pg_outbox", r"^- The app role needs read", r"^- Roles are cluster"],
    "schema-ir": [r"^\*\*How it fits together"],
    "doctor": [r"^- \*\*Code Postgres doesn", r"^Pools open in one", r"^- \*\*No startup parameters", r"^```\nmorm-admin init"],
    "import": [r"^- `lint` warns about columns named"],
}


def context_for(key, reqs):
    pats = CTX.get(key, [])
    return [r for r in reqs if r["step"] != key and any(re.search(p, r["text"]) for p in pats)
            and r["step"] not in ("contracts", "index", "readme-release")]
