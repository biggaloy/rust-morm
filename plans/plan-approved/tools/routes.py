"""Routing rules: which step owns each block of the frozen plan."""
import re


def has(t, *pats):
    return any(re.search(p, t) for p in pats)


def route(u):
    h2, h3, t = u["h2"], u["h3"], u["text"]
    k = u["kind"]

    if h2 == "" or h2.startswith("1. ") or h2.startswith("2. "):
        return "contracts"
    if h2.startswith("15.") or h2.startswith("17."):
        return "index"

    if h2.startswith("3."):
        if h3 == "`morm-admin`":
            return "admin-core"
        if h3.startswith("Configuration"):
            if has(t, r"^\*\*Detection", r"^\d\.\s", r"Look for `.s.PGSQL", r"^`morm-admin init` finds"):
                return "admin-core"
            if t.startswith("**`morm-admin import`"):
                return "admin-core"
            return "config-pool"
        if k == "bullet":
            if has(t, r"`tls`"):
                return "config-pool"
            if has(t, r"`chrono`", r"`uuid`", r"`decimal`"):
                return "types"
            if has(t, r"`json`", r"`utoipa`"):
                return "json"
            if has(t, r"`tracing`", r"`metrics`"):
                return "telemetry"
            if has(t, r"`outbox`"):
                return "outbox"
        if has(t, r"^\*\*Registration:"):
            return "schema-diff"
        if has(t, r"How it fits together", r"`morm-mgr`"):
            return "admin-core"
        return "bootstrap"

    if h2.startswith("4."):
        if h3 == "Bases":
            return "bases-indexes"
        if h3.startswith("Struct options"):
            if has(t, r"^\| `indexes", r"^\| `extensions", r"^\| `checks", r"^\| `storage", r"^\| `base`", r"^\| `schema`"):
                return "bases-indexes"
            if has(t, r"^\| `groups`", r"^\| `scoped_groups`"):
                return "groups"
            if has(t, r"^\| `deserialize`"):
                return "views"
            if has(t, r"^\| `hooks`"):
                return "hooks"
            return "model-core"
        if h3.startswith("Field options"):
            if has(t, r"^\| `unique`", r"^\| `fk", r"^\| `deferrable`"):
                return "bases-indexes"
            if has(t, r"^\| `groups`"):
                return "groups"
            if has(t, r"^\| `secret`"):
                return "views"
            if has(t, r"^\| `min`"):
                return "validation"
            if has(t, r"^\| `version`"):
                return "writes"
            if has(t, r"^\| `skip_json_if`", r"^\| `json_string`"):
                return "json"
            return "model-core"
        if h3 == "Type inference":
            if has(t, r"^- \*\*JSON:", r"^- \*\*Floats:"):
                return "json"
            if has(t, r"^- \*\*Timestamps", r"^- \*\*NUL"):
                return "types"
            return "model-core"
        if t.startswith("**Doc comments"):
            return "bases-indexes"
        return "model-core"

    if h2.startswith("5."):
        if h3.startswith("Views") or h3.startswith("Public input"):
            if has(t, r"^- \*\*`\.apply\(\)`", r"^- \*\*Server values", r"^- \*\*An empty patch", r"^- `\.allow_empty",
                   r"^- Only the columns sent"):
                return "writes"
            return "views"
        if h3.startswith("Groups"):
            if has(t, r"^- \*\*Plain `db`", r"^- \*\*Writes on a handle", r"^- \*\*Reads on a handle", r"^- `to_json_for"):
                return "group-enforcement"
            return "groups"
        return "groups"

    if h2.startswith("6."):
        if has(t, r"^- \*\*Runs once", r"^- \*\*Trade-off", r"^- A value a pre hook", r"^- \*\*`clean` changes",
               r"^- \*\*`blocking`", r"^- `Hooks::validate", r"^- A rule that needs"):
            return "hooks"
        return "validation"

    if h2.startswith("7."):
        return "bases-indexes"

    if h2.startswith("8."):
        if h3 == "Connecting":
            return "config-pool"
        if h3 == "Get" or h3 in ("Lists", "Related rows", "Pagination") or h3.startswith("Raw SQL"):
            return "reads"
        if h3 == "Delete":
            return "writes"
        if h3 == "Insert and update":
            if has(t, r"^- \*\*Hooks and outcome", r"^- Conflict targets", r"^- \*\*An upsert", r"^- \*\*Conflict targets"):
                return "upsert"
            return "writes"
        if h3 == "Decimal":
            return "types"
        if h3 == "Query safety":
            if has(t, r"^- `sql!`"):
                return "query-tooling"
            if has(t, r"\$n` in a fragment"):
                return "statements"
            return "reads"
        if h3 == "Transactions":
            return "transactions"
        if h3 == "Deadlocks":
            if t.startswith("**Lock-order detector"):
                return "hardening"
            return "deadlocks"
        if h3.startswith("JSON"):
            return "json"
        return "reads"

    if h2.startswith("9."):
        if h3.startswith("Outbox"):
            return "outbox"
        return "hooks"

    if h2.startswith("10."):
        if h3 == "Principles" or h3 == "Source of truth" or h3 == "Typed operations":
            return "schema-diff"
        if h3 in ("What's managed",) or h3.startswith("Triggers"):
            return "managed-objects"
        if h3.startswith("Safe forms") or h3.startswith("Production") or h3.startswith("Rolling") or h3.startswith("Data migrations"):
            return "safe-forms"
        if h3.startswith("Down"):
            return "retire"
        if h3 in ("Decisions",) or h3.startswith("Proof") or h3.startswith("Files"):
            return "proof-files"
        if h3.startswith("Runtime guarantees"):
            return "execution"
        if h3 == "Commands":
            if has(t, r"^- `rehearse`", r"^- `lint`", r"^- `squash"):
                return "safe-forms"
            if has(t, r"^- `check-sql`"):
                return "query-tooling"
            if has(t, r"^- `baseline`"):
                return "admin-core"
            return "execution"
        return "schema-diff"

    if h2.startswith("11."):
        if h3.startswith("Resource estimates"):
            return "size"
        if h3 == "Telemetry":
            return "telemetry"
        if h3 == "Statements":
            return "statements"
        if h3 == "Build and benchmarks":
            return "benchmarks"
        if h3 == "Query path":
            if has(t, r"Bulk insert", r"`= ANY"):
                return "writes"
            if has(t, r"pipelined"):
                return "transactions"
            if has(t, r"Minimal dependencies"):
                return "bootstrap"
            return "statements"
        return "config-pool"

    if h2.startswith("12."):
        return route_misuse(t)

    if h2.startswith("13."):
        return "bootstrap"

    if h2.startswith("14."):
        return "readme-release"

    if h2.startswith("16."):
        if h3 == "Risks" and has(t, r"Needs proof early"):
            return "proofs"
        if h3 == "Risks" and has(t, r"Scratch databases"):
            return "schema-diff"
        if h3 == "Open items":
            return "bootstrap"
        if h3 == "Later: field-level encryption":
            return "model-core"
        return "index"

    raise SystemExit("unrouted: " + t[:80])


MISUSE = [
    (r"no filter|no `\.set|\.run\(\)`|with no limit|missing id", "writes"),
    (r"Building SQL|sort column|`OFFSET`|hidden field.*XX", "reads"),
    (r"Reading a hidden|request body into a model|password in JSON", "views"),
    (r"per-request id|Client-chosen|stale cached statement", "statements"),
    (r"empty update", "writes"),
    (r"inside a transaction or stream", "deadlocks"),
    (r"Rolling back a production", "safe-forms"),
    (r"wider than its column|disagreeing with a SQL default", "model-core"),
    (r"Swallowing an error|`\.retry\(\)`", "transactions"),
    (r"opposite order", "hardening"),
    (r"`tx\.lock`, `advisory_lock`", "deadlocks"),
    (r"duplicate of a unique", "bases-indexes"),
    (r"`Decimal` on|rounded `numeric`", "types"),
    (r"Pool settings", "size"),
    (r"nullable keyset", "reads"),
    (r"`version`", "writes"),
    (r"mask handle", "group-enforcement"),
    (r"upsert", "upsert"),
    (r"retired column|retire or rename", "retire"),
    (r"Explicit ids", "writes"),
    (r"Hashing on an async", "hooks"),
    (r"Queries in a loop", "query-tooling"),
    (r"Query values", "admin-core"),
    (r"unindexed column", "query-tooling"),
]


def route_misuse(t):
    for pat, step in MISUSE:
        if re.search(pat, t):
            return step
    if t.startswith("The compiler can't see"):
        return "query-tooling"
    raise SystemExit("unrouted misuse: " + t[:80])


TESTS = [
    # suites and whole-area lines from section 13
        (r"^\*\*Property tests:\*\* random models", "hardening"),
    (r"^\*\*Doc tests", "readme-release"),
    (r"^\*\*`#\[morm::test\]`", "query-tooling"),
    (r"^\*\*Fault injection:\*\* kill the process at every step", "recovery"),
    (r"^\*\*Property tests:\*\* random schema", "hardening"),
        (r"^\*\*Concurrency:\*\* rival migrators", "execution"),
    (r"^\*\*Retire", "retire"),
    (r"^\*\*Safe forms", "safe-forms"),
    (r"^\*\*Multi-step", "execution"),
    (r"^\*\*Forward only|^\*\*Squash", "live-modes"),
    (r"^\*\*Full chain", "hardening"),
    (r"^\*\*Fuzzing", "hardening"),
    (r"^\*\*Fault injection:\*\* kill the process and the worker|^\*\*Concurrency:\*\* rival workers|^\*\*Failures:", "outbox"),
    # specific obligations, first match wins
    (r"hidden view fields|`Deserialize` on|protected columns|derived `Serialize`", "views"),
    (r"`HasPk`", "model-core"),
    (r"unfiltered update|update with no|every write path on an `insert_only`|missing `\.version", "updates"),
    (r"list with no limit|nullable keyset|non-static", "reads"),
    (r"non-literal default|uppercase names|columns narrower|`usize`|over-long", "model-core"),
    (r"`upsert` outcomes, hooks", "hooks"),
    (r"`upsert`|`\.update_only`", "upsert"),
    (r"`self` absent|`changes\.set`|hook inputs|hooks order|hook slices|`clean` and lazy|`blocking`|`after_commit`", "hooks"),
    (r"`fetch_json` with `skip_json_if`|`fetch_json` formats|`json_string`|all list and JSON|`secret` absent", "json"),
    (r"unknown group|scoped groups|scoped mask", "groups"),
    (r"group rules on a handle", "group-enforcement"),
    (r"duplicate indexes|duplicate base|models in several schemas", "bases-indexes"),
    (r"`Decimal` and float|integer type at its limits|range CHECKs|`OutOfRange`|`InvalidText`", "types"),
    (r"`\.tag\(\)`|fragment renumbering|`shape_cache`|sharing one statement", "statements"),
    (r"`span_sample`|`sample_scope`|`slow_query`|metric totals", "telemetry"),
    (r"`doctor`", "doctor"),
    (r"`morm-admin size`|`--here`|`--measure`", "size"),
    (r"pool growth|`max_connections`|`db\.hold\(\)`|`ConnectionLost`|`max_holds`|`25006`|`db\.close\(\)`|two pools|pooler = ", "config-pool"),
    (r"cancellation|forced deadlocks|every timeout|cancelled queries|`\.lock_ordered\(\)`|advisory key|`skip_locked`|`NestedConnection`|`tx\.lock`|retry limits", "locks-timeouts"),
    (r"swallowed errors|`CommitUnknown`|savepoint", "transactions"),
    (r"`\.in_batches`|`patch`|`EmptyPatch`|`\.set\(\)` winning|`\.version\(\)` conflicts", "updates"),
    (r"`insert_many`|`\.explicit_ids\(\)`|changed defaults|`OrDefault`|array columns|^insert$", "inserts"),
    (r"keyset|tampered|`order_by_allowed`|`\.custom_plan\(\)`|`load_related`", "reads"),
    (r"`check_schema`", "rolling"),
    (r"loop detector", "query-tooling"),
    (r"views$|unknown input", "views"),
    (r"^groups$", "groups"),
]


def route_test(t, kind):
    from overrides import TEST_MOVE, TEST_REFINE
    for pat, step in TEST_MOVE + TEST_REFINE:
        if re.search(pat, t):
            return step
    for pat, step in TESTS:
        if re.search(pat, t):
            return step
    raise SystemExit(f"unrouted test: {t[:90]}")
