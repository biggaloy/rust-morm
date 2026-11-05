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
   "`ci.yml`, `nightly.yml`, `bench.yml` and `release.yml`, including Postgres 15–18 over TCP and Unix socket and a PgBouncer service",
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
  ["`#[model]`, every struct and field option assigned here, `F`, generated SQL constants, `from_row`, `from_row_named`, `ProductRef`, `Default`, `new`, `HasPk`",
   "Name rules, type inference, the integer table with range CHECKs in metadata, `db_set`, `insert_only`, `flatten`, capability markers and the conversion layer"],
  "A model using every option expands to the expected SQL, and rows decode from a real Postgres table created by hand. All compile-time rules are proven with `trybuild`.",
  ["Indexes and bases (step @bases-indexes)", "Running queries through `db`"]),
S("bases-indexes", "Bases, indexes and constraint names", "Models", ["model-core"], "`morm-macros`, `morm-core`", ["macro"],
  "Add bases, custom bases, indexes, constraints and the naming convention, including upsert key markers.",
  ["All built-in bases and `#[morm::base]`", "`indexes(...)`, field shortcuts, `checks`, `storage`, `schema`, `extensions`, `deferrable`, typed foreign keys, doc comments as schema comments",
   "Name generation with the 63-byte rule in `morm-core`, duplicate detection, `Product::UK` constants with their markers, and the marker for unique columns that `keyset` also uses"],
  "Models with every index form and base compile, their metadata matches expected output, and generated names match python-morm's convention, including the hashed long form.",
  ["Creating anything in a database (step @schema-diff reads this metadata)"]),
S("views", "Views, input, patch and secret", "Models", ["bases-indexes"], "`morm-macros`, `morm`", ["macro"],
  "Generate restricted views and safe public input and output types.",
  ["`#[view]` with `read_hide`, `write_hide`, `fields`, `input`, `patch`", "`WriteOnly<T>`, `Patch<T>`, `#[col(secret)]`, generated `Serialize`, `Debug` and `Deserialize`, `#[model(deserialize)]`"],
  "A request body parses into an `input` and a `patch` view with every field state, unknown fields are rejected, and every forbidden form fails to compile.",
  ["`.apply()` running an update (step @updates)", "`groups =` on views (step @groups)"]),
S("groups", "Group declarations and masks", "Models", ["views"], "`morm-macros`, `morm`", ["macro"],
  "Global and model-scoped groups with macro-counted masks.",
  ["`morm::groups!`, `scoped_groups`, `#[model(groups)]`, `sudo`, `#[view(groups = ...)]`, `G`, `Bits<N>`, `Mask<T>`, `from_names`, `Model::groups()`, `Model::group_fields()`"],
  "An app with global and scoped groups over 64 entries builds masks, converts names at runtime, and rejects a scoped mask on another model at compile time.",
  ["Enforcement on queries (step @group-enforcement)"]),
S("validation", "Declarative rules and `clean`", "Models", ["model-core"], "`morm-macros`, `morm`", ["validation"],
  "Generate inline validation with no per-field runtime cost.",
  ["`min`, `max`, `len`, `choices`, `clean`, `db_check`, as generated functions with no function pointers"],
  "Every rule accepts and rejects at its boundary in unit tests, and a field with no rules generates no code.",
  ["When rules run in a write (step @hooks)"]),

S("config", "Config, TLS and connecting", "Runtime", ["contracts-freeze"], "`morm-core`, `morm`", ["config"],
  "`morm.toml` and environment variables, in `morm-core` so `morm-migrate` and `morm-admin` read the same config, plus connecting over sockets, TCP and TLS.",
  ["Config loading in `morm-core`, every key and variable, `[database.migration]`, `[server]`, named pool profiles and replicas as config", "`Db::connect`, `connect_as`, `connect_to`, sockets, TLS, `search_path`, startup parameters"],
  "An app connects from `morm.toml` alone, and from environment variables alone, over TCP, Unix socket and TLS, and every invalid config fails with a message naming the setting.",
  ["The pool (step @pool)", "`init` socket detection (step @admin-core)"]),
S("pool", "`morm::Pool`", "Runtime", ["config"], "`morm`", ["pool"],
  "The pool, with sizing, budget and failure rules.",
  ["`morm::Pool` on `deadpool`, dynamic size, idle shrinking, recycling, `recycle_when`, the shared `max_connections` budget, version refusal, failover, `pool_stats()`, an internal query entry point that later steps build on"],
  "`SELECT 1` through every pool type under load grows and shrinks the pool, a Postgres restart is survived, and a pool over budget is lowered with a warning.",
  ["Statement caches (step @statements)", "`db.hold()` and pooler rules (step @hold-pooler)"]),
S("hold-pooler", "`db.hold()`, jobs and PgBouncer", "Runtime", ["pool"], "`morm`", ["pool"],
  "Session connections, and correct behaviour behind a pooler.",
  ["`db.hold()`, release on every exit path, `on_connect`, `listen`, `max_holds`, the pool side of `db.close()`, `pooler = \"transaction\"`"],
  "A held connection is released on every exit path and is never reused, and an app runs correctly behind PgBouncer with `hold` refused.",
  ["Waiting for `after_commit` on shutdown (step @hooks)"]),
S("types", "Numbers and special types", "Runtime", ["model-core"], "`morm`, `morm-macros`", ["types"],
  "Exact encoding and decoding for every supported type, with no silent rounding.",
  ["`Decimal` decoding with no heap use, `morm::Numeric`, every integer path, `morm::Param` for every type, `Range<T>`, `IpAddr`, timestamps, `TextEnum`, `PgEnum`, domains and composite types, `InvalidText`"],
  "Every type round-trips through a real Postgres at its limits, and a value that can't be held exactly returns an error on read and on write.",
  ["JSON formats (step @json)"]),
S("statements", "Statement caches and fragments", "Runtime", ["pool", "types"], "`morm`", ["query"],
  "The two bounded caches, shape tracking, `query_typed` fallback, fragment renumbering and the error mapping every query uses.",
  ["`statement_cache`, `shape_cache`, prepare on second use, built-SQL map, 0A000 handling, `$n` renumbering, `.tag()` text, typed error mapping"],
  "A load test with random and repeated shapes shows fixed statements staying cached, and a change to a table's result type recovers outside a transaction.",
  ["The query APIs that use the caches (steps @reads, @inserts)"]),
S("reads", "Reads", "Runtime", ["statements", "views"], "`morm`, `morm-macros`", ["query"],
  "Every way of reading, and the shared `by`/`any` builder.",
  ["`get` forms, `select`, `filter`, `filter_cols`, every list and single-result call, `db.sql`, `db.raw`, `#[derive(morm::Row)]`, `load_related`, `keyset`, `order_by_allowed`, `.custom_plan()`, `.tag()` on queries"],
  "A test app reads the same rows through every call and gets identical data, and each compile-time rule fails under `trybuild`.",
  ["`fetch_json` formats (step @json)", "Groups on a handle (step @group-enforcement)"]),
S("inserts", "Inserts", "Runtime", ["statements", "views"], "`morm`, `morm-macros`", ["insert"],
  "Every way of adding rows, including bulk paths.",
  ["`insert`, `insert_many` on every path, `insert_stream`, `.in_batches` on inserts, `.explicit_ids()`, `OrDefault`, statements per pattern, array columns"],
  "A million rows go through each `insert_many` path, atomically or in batches, and defaults, arrays and explicit ids behave as specified on real Postgres.",
  ["Hooks (step @hooks)", "Upsert (step @upsert)"]),
S("updates", "Updates and deletes", "Runtime", ["reads", "inserts"], "`morm`, `morm-macros`", ["query"],
  "Every way of changing and removing rows.",
  ["`update`, `update_q`, `update_each`, `.apply()`, `set_expr`, `.version()`, `delete` forms, `.in_batches`, `.pause_ms`, `.returning()`, `ReadOnly` on replicas"],
  "`.apply()` states, version conflicts, batches stopping and resuming, and `NotFound` all behave as specified on real Postgres.",
  ["Hooks (step @hooks)"]),
S("upsert", "Upsert", "Runtime", ["updates"], "`morm`, `morm-macros`", ["query"],
  "`upsert` with exact rules for what it overwrites and what it reports.",
  ["Conflict targets by field, tuple and `UK`, partial index filters, `SET` list rules, `.update_only`, the empty-`SET` path, outcome reporting on every Postgres version"],
  "Upserts on every target form, with and without supplied columns, leave unsupplied values alone and report the right outcome on Postgres 15–18.",
  ["Hook calls on upsert (step @hooks)"]),
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
S("json", "JSON output and schemas", "Runtime", ["reads"], "`morm`, `morm-macros`", ["json"],
  "Every JSON path gives the same documented output, with no `Value` or map in between.",
  ["`to_json`, `Serialize`, `ProductRef` serialization, `fetch_json`, `stream_json`, `jsonb` as `RawValue`, `json_schema`, `utoipa`, every fixed format"],
  "For one row with every type, `fetch`, `fetch_ref` and `fetch_json` produce the same JSON, and no secret appears on any path.",
  ["`to_json_for` on a handle (step @group-enforcement)"]),
S("hooks", "Hooks and write ordering", "Runtime", ["locks-timeouts", "validation"], "`morm`, `morm-macros`", ["query", "tx"],
  "Hooks with exact inputs and order, on every write path, including upsert and shutdown.",
  ["`Hooks`, `#[morm::hooks]`, `Changes`, `Target`, `Committed`, `validate`, `validate_changes`, lazy `db`, `hooks(tx)`, `after_commit`, `db.tx()`, `blocking`, rule ordering, pipelined `BEGIN` and `COMMIT`, hooks on upsert and batches, `db.close()` waiting for `after_commit`"],
  "A recording model proves the exact order and inputs on every write path, including no connection during `clean`, and `after_commit` firing only on a confirmed commit.",
  ["Outbox (step @outbox)"]),
S("group-enforcement", "Groups on queries", "Runtime", ["json", "hooks", "groups"], "`morm`", ["query"],
  "Enforce groups on a `db.groups(mask)` handle for every read and write.",
  ["`db.groups`, `db.sudo`, `Forbidden`, failing closed on reads, `to_json_for`"],
  "Each read and write path, on plain `db` and on a handle, either returns only allowed data or returns `Forbidden`.",
  ["New group declarations"]),
S("outbox", "Transactional outbox", "Runtime", ["hooks"], "`morm`", ["outbox"],
  "Events saved with their data and delivered at least once.",
  ["`tx.emit`, `tx.emit_keyed`, the worker, leases, retries, cleanup, the `outbox` feature, the table definition"],
  "Killing the process and worker at every point never loses an event, and repeats carry the same `e.id`.",
  ["The managed `pg_outbox!` form and `morm-admin outbox` (step @admin-core); tests create the table directly"]),

S("manifest", "Registration and `morm-mgr`", "Migrations", ["bases-indexes"], "`morm-macros`, `morm-core`", ["macro", "manifest"],
  "Get model metadata out of a project without touching the app binary.",
  ["Registration behind `cfg(morm_mgr)`, a minimal `morm-mgr` that prints the JSON manifest, manifest types in `morm-core`"],
  "`morm-mgr` prints a correct manifest for models in several crates, including one nothing references, and the app binary contains none of it.",
  ["`morm-mgr` packaging for `init` (step @admin-core)"]),
S("schema-diff", "Schema IR, scratch and typed ops", "Migrations", ["manifest", "config"], "`morm-migrate`", ["planner"],
  "Read schemas into a typed IR and diff them into typed operations with inverses.",
  ["`Schema` IR, catalog reader, scratch database handling, diff, `Op` with inverse, lock, transaction and data properties, topological planning"],
  "For a set of schema pairs, the diff produces ops whose forward and inverse SQL move a real database between both states exactly.",
  ["Writing files (step @proof-files)", "Managed objects beyond tables (step @managed-decl)"]),
S("lint", "`lint`", "Migrations", ["schema-diff"], "`morm-migrate`", ["planner"],
  "Report risky statements and weak schemas, gathered here from every rule in the plan.",
  ["`mgr lint` with `--url` and accepted findings, statement risk from `Op` properties, every index, `numeric`, `secret`-name and `SECURITY DEFINER` rule"],
  "Each lint rule fires on a fixture that breaks it and stays quiet on one that doesn't, and `--url` reports only tables big enough to matter.",
  ["Enforcing `lint` in the proof (step @proof-files)"]),
S("proof-files", "Proof pipeline, files and decisions", "Migrations", ["lint"], "`morm-migrate`", ["planner"],
  "Write a migration only after it's proven, with decisions as explicit data.",
  ["`makemigrations`, the five-part proof, decisions, `up.sql`, `down.sql`, `meta.json`, deterministic output, exit codes, `min_postgres` and extension checks"],
  "`makemigrations` on a changed model writes byte-identical files on repeat runs, and refuses to write when any proof part fails.",
  ["Running migrations (step @execution)"]),
S("execution", "Migration execution and recovery", "Migrations", ["proof-files"], "`morm-migrate`, `morm-admin`", ["planner", "admin"],
  "Run migrations safely, with a defined way forward from any failure.",
  ["`_morm` ledger, `migrate`, `rollback` with `--failed`, `status`, `verify`, `check`, `repair`, `refingerprint`, `sql`, fingerprint gates, advisory lock, preflight, crash recovery"],
  "Killing the process at every step boundary, `--resume` and `rollback` both reach the right state, and two rival migrators can't both run.",
  ["Safe forms (step @safe-forms)"]),
S("managed-decl", "Managed objects: declarations", "Migrations", ["execution", "updates"], "`morm-migrate`, `morm-macros`, `morm`", ["planner"],
  "Declare everything a schema depends on, and read it back through the catalog.",
  ["`pg_function!`, `pg_trigger!`, `pg_policy!`, `pg_procedure!`, `pg_grant!`, `#[pg_view]`, materialized views, enums, domains, read-only view models, `db.call`, `db.refresh`, placeholder roles on scratch"],
  "Each object kind goes from a declaration to a real database and back through the diff with no drift, on Postgres 15–18.",
  ["Dependency recreation (step @managed-deps)"]),
S("managed-deps", "Managed objects: dependencies", "Migrations", ["managed-decl"], "`morm-migrate`", ["planner"],
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
S("rolling", "Rolling deploys", "Migrations", ["live-modes", "reads"], "`morm-migrate`, `morm`", ["planner"],
  "Keep old and new binaries working through a deploy.",
  ["`phase`, expand and contract, rename sync triggers, `deprecated` steps, `db.check_schema()` with its per-model column list, data migration hooks"],
  "An old binary boots and works during an expand phase, and `check_schema` reports a dropped default.",
  ["New features"]),

S("admin-core", "`morm-admin` core", "Tools", ["retire", "outbox", "inserts", "reads"], "`morm-admin`, `morm-macros`", ["admin"],
  "One tool for setup and checks.",
  ["`init` with socket detection and the Axum template, `doctor`, `scratch`, `outbox`, `baseline`, `morm-mgr` packaging and its `cfg(morm_mgr)` builds, the managed `morm::pg_outbox!` form"],
  "`morm-admin init --axum` creates a project that builds, migrates and serves a request, and `doctor` reports each of its checks correctly when the condition is forced.",
  ["`import` (step @import)", "`size` (step @size)"]),
S("import", "`morm-admin import`", "Tools", ["admin-core"], "`morm-admin`", ["import"],
  "Generate models from any existing database, gathered here from every import rule in the plan.",
  ["`import` with exact types, names, `keep_case`, `secret`, `Decimal` choices, and a list of what models can't express"],
  "`import` on the fixture `tests/fixtures/import.sql` round-trips to no diff. Write that fixture to hold mixed-case names, composite keys, every index kind, deferrable constraints, views, triggers and an exclusion constraint.",
  ["Changing the imported database"]),
S("size", "`morm-admin size`", "Tools", ["pool", "admin-core"], "`morm-admin`, `morm-core`", ["size"],
  "Estimate resources from settings, and settings from resources.",
  ["`size` in both directions, `--here` with cgroup limits, `--share`, `--emit`, `--measure`, `[server]` use, exit codes, calibrated memory figures, the `doctor` line"],
  "On a real server, estimates are checked against measured memory, and `--here` gives a config that starts Postgres and the app.",
  ["`--live` (later work)"]),
S("query-tooling", "`sql!`, `check-sql`, test support and the loop detector", "Tools", ["admin-core"], "`morm`, `morm-macros`, `morm-admin`", ["tooling"],
  "Checked raw SQL with no database at build time, and test tooling for users.",
  ["`sql!`, `check-sql` with `PREPARE` and `EXPLAIN`, including hidden columns and unquoted mixed-case names, `#[morm::test]`, the loop detector, `morm::allow_repeats`"],
  "A wrong column in `sql!` fails `check-sql`, a query in a loop fails a `#[morm::test]`, and each exclusion passes.",
  ["The lock-order detector (step @hardening)"]),
S("telemetry", "Telemetry", "Tools", ["statements", "hooks"], "`morm`", ["telemetry"],
  "OpenTelemetry-shaped spans and metrics, cheap enough to leave on.",
  ["`tracing` and `metrics` features, `span_sample`, `sample_scope`, `slow_query`, per-connection counters and histograms, the publishing task"],
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
  "`nightly.yml` passes for seven runs in a row.",
  ["New features"]),
S("readme-release", "README and 0.1.0", "Finish", ["benchmarks", "hardening", "import", "size"], "all", ["all"],
  "A README that is enough on its own, then the release.",
  ["Full README with every table", "CI check comparing the Extra calls table with the public API", "Every doc example compiled and run", "`--code` coverage clean", "`0.1.0` published, after the owner says so"],
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
    "outbox": [r"^\[database\]|\[outbox\]"], "telemetry": [r"\[telemetry\]"], "pool": [r"\[pool\]"],
    "hold-pooler": [r"\[pool\]"],
    "size": [r"^- Connections needed", r"^- Only as many queries", r"budget = \(", r"^If `min` or `max`", r"^Pools open in one"],
    "benchmarks": [r"^\| Features compiled out|^\| `tracing` |^\| `metrics` on"],
    "updates": [r"^```rust\ndb\.insert\(&mut p\)"], "upsert": [r"^```rust\ndb\.insert\(&mut p\)"],
    "proof-files": [r"^\*\*How it fits together"], "execution": [r"^\*\*How it fits together"],
    "admin-core": [r"^\*\*How it fits together", r"^\*\*Registration"],
    "schema-diff": [r"^\*\*How it fits together"],
}


def context_for(key, reqs):
    pats = CTX.get(key, [])
    return [r for r in reqs if r["step"] != key and any(re.search(p, r["text"]) for p in pats)
            and r["step"] not in ("contracts", "index", "readme-release")]
