"""Corrections from the independent review of the first split."""
import hashlib
import re


def sha(t):
    return hashlib.sha256(t.encode()).hexdigest()[:12]


# whole blocks moved to another step
MOVE = {
    "e66e6b531f61": "reads", "ad810fa8098b": "transactions", "f6d883aaa6e4": "statements",
    "58d759310ed5": "model-core", "bd371fde1668": "bases-indexes", "eaff890a515d": "groups",
    "598e93fa8c27": "hooks", "6af77e2dbc1e": "hooks", "ffd4e3b0fa83": "json",
    "18fba30523b2": "hooks", "91f866a73fa6": "hooks", "a61a34c38869": "hooks", "09c164bb38ec": "hooks",
    "33619bbb4bbc": "execution", "c66c6a8a3b6e": "execution", "ce33d6801b1f": "execution",
    "9a847096655b": "execution", "f8c471530391": "retire", "c4fa09341219": "benchmarks",
    "f318c5d7ebb5": "lint", "e5cd51d6040f": "lint", "4c27bbfd8478": "import",
    "a2a26228038f": "proof-files", "593511b55fc9": "proof-files", "5536a8a15c1e": "manifest",
}
# headings and labels: nothing to build
INFO = {"09e09762d2d1", "91833ef44890", "f2471c70770d", "eca389cfa91f", "d4c1aaff82ad"}

# (block sha, exact text to take out, step, text of the new block or None to reuse it)
PIECES = [
    ("06ba092fcf58", "Hooks run once per batch.", "hooks", None),
    ("f40054f187ea", "**Hooks and outcome.** `upsert` holds a whole row, so `pre_insert` always runs, before Postgres decides.", "hooks", None),
    ("f40054f187ea", "An insert runs `post_insert` and fires `Inserted(rows)`.", "hooks", None),
    ("f40054f187ea", "An update runs `post_update`, with `Changes` holding the `SET` columns, and fires `Updated`.", "hooks", None),
    ("f40054f187ea", "`pre_update` can't run, because the outcome isn't known yet.", "hooks", None),
    ("f40054f187ea", "`Unchanged` writes nothing, so no post hook or `after_commit` runs.", "hooks", None),
    ("f40054f187ea", "A batch upsert fires one `Upserted { inserted, updated }` event per statement, with counts.", "hooks", None),
    ("593511b55fc9", ", and `morm` refuses to connect to a server below 15", "pool", "`morm` refuses to connect to a server below 15."),
    ("17a4c0797523", "Queries joined on one `tx` are pipelined on its connection.", "transactions", None),
    ("ac5c05b0c0ae", "`read_only = true` makes a write on that pool fail with `ReadOnly` at once, sets `default_transaction_read_only`, and exempts it from the `25006` discard rule.", "updates", None),
    ("35c823a0aabc", "`.set()` checks once per call, and `to_json_for` works out its field list once per mask, so output cost doesn't grow with the number of rows.", "group-enforcement", None),
    ("20b843c3f651", "Hand-written SQL must then quote the name, and `check-sql` catches the mistake in registered queries.", "query-tooling", None),
    ("e15275987506", "but `lint` warns, and ", "lint", "**Unbounded `numeric`** with `Decimal`: `lint` warns."),
    ("ff61480428dd", ", and `check-sql` flags queries selecting hidden columns", "query-tooling", "`check-sql` flags queries selecting hidden columns."),
    ("a2a26228038f", "so `doctor` compares the Postgres major version and installed extensions of scratch and the target, and ", "admin-core", "`doctor` compares the Postgres major version and installed extensions of scratch and the target."),
    ("1a68d35b2fb7", ", and `doctor` warns if it doesn't", "admin-core", "`doctor` warns if `MORM_MIGRATION_URL` doesn't point at Postgres directly."),
    ("c6c882372fa9", "`#[derive(TextEnum)]` maps a Rust enum to `text` with a CHECK constraint, the recommended form, because adding or removing a value is an ordinary migration.", "types", None),
    ("c6c882372fa9", "`#[derive(PgEnum)]` maps to a Postgres enum, mainly to match existing tables, because removing a value from one is very hard.", "types", None),
    ("c6c882372fa9", "Domains and composite types are also supported.", "types", None),
]

TOOL_STEPS = [(r"`morm-admin doctor`|`doctor`", "admin-core"), (r"`morm-admin import`|`import`", "import"),
              (r"`morm-admin mgr lint`|`lint`", "lint")]


def split_auto(u):
    """Move sentences that start with a later tool's name into that tool's step."""
    if u["kind"] not in ("bullet", "para") or u["h2"].startswith(("1.", "2.", "14.", "15.", "16.", "17.")):
        return []
    out = []
    prefix = "- " if u["text"].startswith("- ") else ""
    body = u["text"][len(prefix):]
    parts = re.split(r"(?<=[.])\s+(?=[`*A-Z])", body)
    if len(parts) == 1:
        return []
    keep = []
    for p in parts:
        hit = None
        for pat, step in TOOL_STEPS:
            if re.match(pat, p) and u["step"] != step and not (step == "admin-core" and u["step"] == "size"):
                hit = step
        if hit:
            out.append((p, hit))
        else:
            keep.append(p)
    if out:
        u["text"] = prefix + " ".join(keep)
    return out


def apply_pieces(units, route_fn):
    """Route every block, then cut out pieces. Returns the final block list."""
    out = []
    for u in units:
        h = sha(u["text"])
        u["orig_sha"] = h
        u["step"] = MOVE.get(h) or route_fn(u)
        u["forced_info"] = h in INFO
        extra = []
        for bsha, find, step, new in PIECES:
            if bsha != h:
                continue
            assert find in u["text"], find
            u["text"] = u["text"].replace(find, "").replace("  ", " ").strip()
            extra.append((new or find, step))
        extra += split_auto(u)
        body = re.sub(r"^-\s*", "", u["text"]).strip(" ,.")
        if body and not (u["text"].startswith("- ") and not body):
            out.append(u)
        for txt, step in extra:
            v = dict(u)
            v["text"] = ("- " if u["kind"] == "bullet" else "") + txt
            v["step"] = step
            v["forced_info"] = False
            out.append(v)
    return out


def refine(u, step):
    """Oversized steps were split: send each block to its part."""
    h3, t = u["h3"], u["text"]
    if step == "config-pool":
        if u["h2"].startswith(("3.", "8.")):
            return "config"
        if h3 in ("`morm::Pool`", "Dynamic size", "Sizing guidance", "`max_connections`"):
            return "pool"
        return "hold-pooler"
    if step == "writes":
        if h3 == "Delete" or u["h2"].startswith(("5.", "4.")):
            return "updates"
        if re.search(r"= ANY|no filter|no `\.set|`\.run\(\)`|missing id|empty update|`version`", t) and u["h2"].startswith(("11.", "12.")):
            return "updates"
        if re.match(r"- (An update never|\*\*`update` and `delete`|`updated_at`|\*\*Request-driven|\*\*`#\[col\(version|\*\*Lost updates|\*\*`now\(\)`)", t):
            return "updates"
        return "inserts"
    if step == "deadlocks":
        if re.match(r"(- \*\*A second connection|- `db\.extra_conn|- \*\*Pool waits|- \*\*Holds can|\*\*Pool deadlocks|There are two kinds)", t) or "inside a transaction or stream" in t:
            return "pool-deadlocks"
        return "locks-timeouts"
    if step == "safe-forms":
        if h3.startswith("Production") or re.match(r"- `(rehearse|squash)", t) or "Rolling back a production" in t:
            return "live-modes"
        if h3.startswith(("Rolling", "Data migrations")):
            return "rolling"
        return "safe-forms"
    if step == "managed-objects":
        if t.startswith("**Dependencies are handled") or t.startswith("- **Order is automatic"):
            return "managed-deps"
        return "managed-decl"
    if step == "admin-core" and t.startswith("**`morm-admin import`"):
        return "import"
    if step == "schema-diff" and t.startswith("**Registration"):
        return "manifest"
    return step


# test obligations: exact text -> list of (text, step)
TEST_SPLIT = {
    "**Unit:** macro output, SQL generation, diff, naming.": [
        ("Unit: macro output", "model-core"), ("Unit: SQL generation", "statements"),
        ("Unit: diff", "schema-diff"), ("Unit: naming", "bases-indexes")],
    "every get, update and delete form": [("every get form", "reads"), ("every update and delete form", "updates")],
    "views": [("views through every read", "reads"), ("views through every write", "updates")],
    "**Op tests:** every op's inverse, with and without data, and every dependency recreate path.": [
        ("**Op tests:** every op's inverse, with and without data", "schema-diff"),
        ("every dependency recreate path", "managed-deps")],
}
TEST_MOVE = [
    (r"^groups$", "group-enforcement"), (r"models in several schemas", "reads"),
    (r"every integer type at its limits", "json"), (r"range CHECKs", "schema-diff"),
    (r"`\.limit\(\)` sharing", "reads"), (r"`\.tag\(\)` in server", "reads"),
    (r"over-long and colliding", "bases-indexes"), (r"`db\.close\(\)`", "hooks"),
    (r'pooler = "', "locks-timeouts"),
]
EXTRA_TESTS = [("every compile-time guarantee has a `trybuild` case", "trybuild", "hardening")]
TEST_REFINE = [
    (r"pool growth|`max_connections`|`25006`|two pools", "pool"),
    (r"`db\.hold\(\)`|`ConnectionLost`|`max_holds`", "hold-pooler"),
    (r"`NestedConnection`", "pool-deadlocks"),
]
