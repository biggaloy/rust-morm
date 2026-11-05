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
    m = re.match(r"^(-|\d+\.)\s", u["text"])
    prefix = m.group(0) if m else ""
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
    (r"over-long and colliding", "bases-indexes"), (r"`db\.close\(\)`", "hooks-more"),
    (r"columns narrower|`usize`", "model-types"),
    (r'pooler = "', "locks-timeouts"),
]
EXTRA_TESTS = [("every compile-time guarantee has a `trybuild` case", "trybuild", "hardening")]
TEST_REFINE = [
    (r"pool growth|`max_connections`|`25006`|two pools", "pool"),
    (r"`db\.hold\(\)`|`ConnectionLost`|`max_holds`", "hold-pooler"),
    (r"`NestedConnection`", "pool-deadlocks"),
]


# ---- second review ----------------------------------------------------------

MOVE2 = [  # (start of block, step)
    ("| `insert_only` |", "upsert"), ("Compile-time checks: unknown columns", "bases-indexes"),
    ("- On existing tables, migrations build", "safe-forms"), ("- **Read-only works:**", "updates"),
    ("**Views write too.**", "updates"), ("- **Inserts:** a view", "inserts"),
    ("- It never appears in `Serialize`", "group-enforcement"),
    ("```rust\nlet db = db.groups(", "group-enforcement"), ("- **Generic code:**", "group-enforcement"),
    ("- **Scoped bits are per model,**", "group-enforcement"), ("**Read replicas** work", "reads"),
    ("- **Statement caches:** two bounded", "statements"), ("- **`SET LOCAL` still works", "locks-timeouts"),
    ("- **Prepared statements** need", "locks-timeouts"), ("- **NUL bytes:**", "statements"),
    ("- **Input:** a view with", "hooks"), ("- **Holds can't use up the pool.**", "hold-pooler"),
    ("- **`citext`**", "reads"), ("| `default_fn` |", "inserts"), ("- an empty `impl Hooks`", "hooks"),
    ("```\nmorm-admin init [path]", "admin-init"),
    ("- `morm`, `morm-macros`, `morm-migrate`", "readme-release"),
    ("**Everything an application's schema", "schema-ir"), ("Tables and other objects with no definition", "retire"),
]
INFO2 = [  # examples and labels: the rules around them carry the tests
    "```\nmorm-admin init [path]", "```\nmakemigrations [--plan]",
    "1. Never trust generated", "2. Check the real database", "3. Every failure has", "4. Down is a first-class", "5. Nothing destructive",
    "- **Migrations must bypass the pooler.**", "Crons, background jobs and service workers", "```rust\nloop {\n    let batch",
    "| Normal request, job or cron", "| Websocket query", "| Several statements that must be atomic", "| A very long query",
    "| A heavy job in the same process", "| `LISTEN`, temp tables", "| Server push from Postgres",
    "```rust\nuse morm::{model, Timestamp};", "```rust\nlet mut p = Product {", "```rust\npub enum Patch<T>",
    "```rust\ndb.insert(&mut p).await?;", "```rust\n#[derive(morm::Row)]", "```rust\nmorm::pg_outbox!();",
    "Cargo features of `morm`:", "**Custom bases** are ordinary", "There are two kinds of group:",
    "**Postgres deadlocks:**", "**Limiting the damage:**", "All are `morm-admin mgr <command>`:",
    "**What the numbers mean:**", "**Cheap by construction:**", "- **Needs proof early:**", "Migration tests:", "Outbox tests:",
]
PIECES2 = [  # (start of block, exact text, step, new block or None)
    ("- `Default` (unless `no_default`)", ", `json_schema()`", "json", "`json_schema()` is generated per model."),
    ("Compile-time checks: unknown columns", "Postgres checks `filter` and `expr` on scratch, because a macro can't parse SQL.", "proof-files", None),
    ("- **Session features refuse.**", "`tx.advisory_lock` is unaffected, because it ends with the transaction.", "locks-timeouts", None),
    ("- **Connection limits:** `SHOW", " and the `max_connections` check in `size`", "size", "With `pooler = \"transaction\"`, the `max_connections` check in `size` is skipped."),
    ("- `cached plan must not change", ", typed error inside", "transactions", "`cached plan must not change result type` (0A000) inside a transaction returns a typed error."),
    ("- **Worst case:** random one-off", "The `tracing` feature logs heavy churn.", "telemetry", None),
    ("- The SQL must be `&'static str`", "`sql!` also registers it for `check-sql`.", "query-tooling", None),
    ("- **Array columns:**", ", `upsert` and `update_each`", "updates", "Array columns work through `update_each` the same way."),
    ("| Large, with `.returning()`", " or an `upsert`", "upsert", "A large `upsert` uses `UNNEST` in chunks in one transaction, because `COPY` can't handle conflicts."),
    ("| `generated`, `identity`, `check`", ", and `.set()` on it is a compile error", "updates", "`.set()` on a generated column is a compile error."),
    ("- `Product::F.<field>`", "; `Product::F.alias(\"p\")` for JOINs, with prefix decoding for two models in one row", "reads", "`Product::F.alias(\"p\")` works for JOINs, with prefix decoding for two models in one row."),
    ("- **`#[col(version)]`:**", " and `upsert`", "upsert", "`upsert` also increments a `version` column."),
    ("- **`.apply()` acts as one", "so hooks, `clean`, rules, `Changes<Self>` and group masks work unchanged, and `clean` runs only on fields that were sent.", "hooks", "`.apply()` goes through hooks, `clean`, rules, `Changes<Self>` and group masks exactly as `.set()` does, and `clean` runs only on fields that were sent."),
    ("**`.retry(n)`** is available", "Each attempt starts with no queued `after_commit` events.", "hooks", None),
    ("**Automatic retry:**", "Retry covers only the SQL, so hooks don't run again and `RETURNING` fills the row only on success.", "hooks", None),
    ("**Automatic retry:**", "`hooks(tx)` statements are transactions, so they follow `.retry(n)`.", "hooks", None),
    ("- **Visibility:** `pool_stats()`", "Postgres's deadlock detail is logged through `tracing`, with no parameter values.", "telemetry", None),
    ("- `user.to_json()`, or", "`user.to_json_for(mask)` limits output to groups.", "group-enforcement", None),
    ("- **Failures retry with backoff.**", ", and `morm-admin outbox retry ID` runs it again", "admin-init", "`morm-admin outbox retry ID` runs a failed event again."),
    ("- **Fingerprint gate.**", "Rollback checks the reverse.", "recovery", "**Fingerprint gate:** rollback checks the fingerprints in reverse."),
    ("| `morm-mgr` |", " Nothing else", "rolling", "`morm-mgr` also runs a migration's Rust data hooks, and nothing else. Build it in CI and ship the binary with the migrations. `migrate --hooks ./morm-mgr` and `rollback --hooks ./morm-mgr` call it as `morm-mgr hook ID up` or `down`, and a migration with hooks refuses to run without it, or if that binary doesn't contain the hook, because `morm-admin` holds no project code. A hook connects with the migration role and runs as a `no-tx` step, so it must be safe to run again on `--resume`."),
    ("- Roles are cluster-wide", ", and `doctor` checks the real ones exist", "doctor", "`doctor` checks that the real roles exist."),
]
SPLIT2 = [  # fifth-pass sizing: (old step, block pattern, new step)
    ("model-core", r"^(\| `i8`|\| `u8`|\| `u16`|\| `u32`|\| `u64`|\| `i128`|\| `u128`|\| `NonZero|\| `char`)", "model-types"),
    ("model-core", r"^`i16` →|^- \*\*Integers Postgres|^- \*\*`usize`|^- \*\*A column must hold|^- \*\*A wider column|^- \*\*`numeric` is larger|^- \*\*`citext`", "model-types"),
    ("model-core", r"^- positional decoding|^Encrypted columns|^- `F` columns carry|^- Encode and decode|^\| A Rust type wider", "model-types"),
    ("schema-diff", r"^No snapshots|^1\. Replay|^2\. Create the models|^3\. Read both|^- \*\*Scratch databases", "schema-ir"),
    ("execution", r"^- \*\*A failed migration|^- \*\*Crash recovery|^- \*\*Tamper refusal|^- \*\*LIFO rollback|^- \*\*Postgres versions|^- `verify`", "recovery"),
    ("admin-core", r"^(- )?(`morm-admin doctor`|`doctor`)|^\| Query values", "doctor"),
    ("reads", r"^morm doesn't load relations|^```rust\nlet orders|^It runs one `WHERE|^`OFFSET 100000|^```rust\nlet page|^- The last key|^- \*\*A nullable key|^- A cursor is|^- Writes between|^`db\.sql\(\)`|^```rust\n#\[derive|^- Every list and single|^- `#\[derive\(morm::Row|^- The SQL must|^- \*\*The list rule|^- It uses the same|^\| Paging with|^\| A nullable keyset", "reads-more"),
    ("managed-decl", r"^- \*\*Models over views|^- Macros also generate|^- Roles are cluster|^- A `\.sql` file", "managed-ops"),
    ("hooks", r"^- \*\*Hooks and outcome|^- An insert runs|^- An update runs|^- `pre_update` can't|^- `Unchanged`|^- A batch upsert|^Hooks run once per batch|^\| `upsert` \||^- \*\*Shutdown|^- \*\*`update_each`", "hooks-more"),
]


def final_fix(units):
    out = []
    for u in units:
        t = u["text"]
        for start, step in MOVE2:
            if t.startswith(start):
                u["step"] = step
        if any(t.startswith(s) for s in INFO2):
            u["forced_info"] = True
        extra = []
        for start, find, step, new in PIECES2:
            if t.startswith(start):
                assert find in u["text"], find
                u["text"] = u["text"].replace(find, "").replace("  ", " ").strip()
                extra.append((new or find, step))
        out.append(u)
        for txt, step in extra:
            v = dict(u)
            if u["kind"] == "row":
                v["kind"] = "bullet"
                v["head"] = None
            v["text"] = ("- " if v["kind"] == "bullet" else "") + txt
            v["step"] = step
            v["forced_info"] = False
            out.append(v)
    out.extend(NEW_BLOCKS)
    for u in out:
        for old, pat, new in SPLIT2:
            if u["step"] == old and re.search(pat, u["text"]):
                u["step"] = new
        if u["step"] == "admin-core":
            u["step"] = "admin-init"
    return out


def nb(text, step, h2, h3):
    return dict(kind="bullet", h2=h2, h3=h3, h4="", text="- " + text, head=None, line=0, step=step,
                forced_info=False, orig_sha="")


# owner-confirmed additions after the split; INDEX.md lists them
NEW_BLOCKS = [
    nb("`db.upsert_many(&mut rows, target)` is the batch form of `upsert`, taking the same conflict targets. It's the call the plan's \"batch upsert\" and \"`upsert` batches\" mean.", "upsert", "8. Reading and writing", "Insert and update"),
    nb("Array columns work through `upsert` and `upsert_many` the same way as `insert_many`.", "upsert", "8. Reading and writing", "Insert and update"),
    nb("`makemigrations [--plan] [--json] [-n name] [--decide ID=OPT] [--decisions F] [--empty] [--fast]`, with `--empty` writing an empty migration for hand-written steps, and `rebase-chain` fixing a chain marked `diverged`.", "proof-files", "10. Migrations", "Commands"),
    nb("`migrate [--plan] [--json] [--yes] [--to ID] [--resume] [--phase expand|contract]`, where `--yes` skips the prompt and `--to ID` stops after that migration. `status [--json]`. `check`. `sql ID` prints a migration's SQL.", "execution", "10. Migrations", "Commands"),
    nb("`rollback [--last N | --to ID | --orphans | --failed] [--plan] [--yes] [--accept-data-loss] [--production --reason R]`. `verify [--json]`. `repair --reason R`. `refingerprint`.", "recovery", "10. Migrations", "Commands"),
    nb("`revert ID [--plan] [--decide ID=OPT]`. `mode [development --confirm NAME | production]`. `squash --to ID`. `rehearse --url URL`.", "live-modes", "10. Migrations", "Commands"),
    nb("`purge --older-than D | --now ID`.", "retire", "10. Migrations", "Commands"),
    nb("`baseline`.", "admin-init", "10. Migrations", "Commands"),
    nb("`morm-admin init [path] [--axum]`, `morm-admin scratch` to check or create the scratch database, and `morm-admin outbox [list | retry ID]` to inspect failed events and deliver one again.", "admin-init", "3. Crates and tools", "`morm-admin`"),
    nb("`morm-admin mgr <command>` runs migration and schema work, and each `mgr` command lives in its own module.", "lint", "10. Migrations", "Commands"),
    nb("`morm::pg_outbox!()` declares the outbox table as a managed table, tracked by migrations.", "admin-init", "9. Hooks", "Outbox: events that must not be lost"),
]

TEST_SPLIT["**Failures:** backoff, `max_attempts`, `failed` events kept, `retry`, and cleanup after `retention`."] = [
    ("**Failures:** backoff, `max_attempts`, `failed` events kept, and cleanup after `retention`", "outbox"),
    ("`morm-admin outbox retry ID` on a failed event", "admin-init")]
TEST_MOVE += [
    (r"a scoped mask on another model", "group-enforcement"), (r"every write path on an `insert_only`", "upsert"),
    (r"`InvalidText`", "statements"), (r"`secret` absent", "group-enforcement"),
    (r"`upsert` outcomes, hooks|`db\.close\(\)`", "hooks-more"), (r"array columns on every", "updates"),
    (r"^`doctor`|`doctor` warnings", "doctor"),
    (r"keyset|tampered|`load_related`", "reads-more"),
]
