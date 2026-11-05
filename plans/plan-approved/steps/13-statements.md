# Step 13: Statement caches and fragments

- **Phase:** Runtime
- **Depends on:** 10, 12
- **Can run beside:** 05, 06, 07, 08, 11, 25, 26, 27, 28, 29
- **Crates:** `morm`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

The two bounded caches, shape tracking, `query_typed` fallback, fragment renumbering and the error mapping every query uses.

## Deliverables

- `statement_cache`, `shape_cache`, prepare on second use, built-SQL map, 0A000 handling, `$n` renumbering, `.tag()` text, typed error mapping

## End-to-end check

A load test with random and repeated shapes shows fixed statements staying cached, and a change to a table's result type recovers outside a transaction.

## Out of scope

- The query APIs that use the caches (steps 14, 15)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Reading and writing / Query safety

- **R-0423** **`$n` in a fragment is local.** `comp`, `set_expr` and `.q()` fragments each count from `$1`, and morm renumbers them. The scan skips quoted strings and comments, so `'$1'` stays text.
- **R-0424** Typed errors: `Unique { constraint, field }`, `ForeignKey`, `NotNull`, `Check`, `Serialization`, `Deadlock`, `LockTimeout`, `TransactionTimeout`, `TransactionAborted`, `CommitUnknown`, `NestedConnection`, `PoolerMode`, `NotFound`, `EmptyPatch`, `Conflict`, `Forbidden`, `Validation`, `PoolBusy`, `ConnectionLost`, `ReadOnly`, `OutOfRange`, `InvalidText`, `InvalidOrder`, `InvalidCursor`, `Decimal`.

### Performance and memory / Statements

- **R-0684** Two bounded LRUs per connection. Evicted statements are closed when dropped, which also frees Postgres memory.

| ID | Setting | Default | Holds |
|---|---|---|---|
| R-0685 | `statement_cache` | 256 | SQL fixed at compile time: `get` and `delete` by primary key, a model's static `insert`, `sql!`, `db.sql()` and `db.raw()` |
| R-0686 | `shape_cache` | 128 | SQL put together at runtime: `select` builders including `filter_cols` and `.q()`, `get_by`, `get_where`, `get_q`, `delete_by`, `delete_where`, `delete_q`, inserts with defaulted fields, `update`, `update_q`, `.apply()`, and any query with a `.tag()` |

- **R-0687** **Why two:** each combination of columns is different SQL text, so a 10-field patch has up to 1,023 shapes, and a client chooses which one it sends. morm can't tell a fixed `.set()` chain from a conditional one, so the split is by how the SQL is made. A fixed chain is one busy shape and stays cached.
- **R-0688** **A shape is prepared the second time it's seen.** Its first use runs through `query_typed`, in one round trip, with no cache entry. A small fixed table of shape hashes per connection records what has been seen, and a collision only prepares a shape one use early.
- **R-0689** `.limit(n)` is always a bound parameter, so a client's page size can't create new shapes.
- **R-0690** **Worst case:** random one-off shapes are never prepared and evict nothing. A client repeating hundreds of shapes can churn `shape_cache`, but it can't touch `statement_cache`. The `tracing` feature logs heavy churn.
- **R-0691** **Query tags:** `.tag("orders.list")` adds a fixed comment, `/* orders.list */`, so server logs, `auto_explain` and `pg_stat_activity` show which code ran a query. It costs nothing unless used.
- **R-0692** It takes only `&'static str`, so it can't hold a request id, which would make every query text unique. Characters outside letters, digits, `.`, `_`, `:` and `-` become `_`, so a tag can't close the comment.
- **R-0693** One tag is one extra statement: the same SQL under two tags is prepared twice.
- **R-0694** `pg_stat_statements` ignores comments, so those two calls merge into one entry showing the first tag seen. Use logs for tags, and `pg_stat_statements` for totals per query.
- **R-0695** Built SQL text is kept in a small bounded map keyed by model and column bitmask, so a repeated shape doesn't rebuild the string.
- **R-0696** Other dynamic SQL and overflow go through `query_typed`, which needs no prepare round trips. It needs each parameter's Postgres type, and `ToSql` doesn't give that, so parameters are `&dyn morm::Param`, a trait that adds the type. Builder code knows the type from `F`.
- **R-0698** `cached plan must not change result type` (0A000): evict and re-prepare once outside a transaction, typed error inside.

### Performance and memory / Query path

- **R-0699** SQL for `get` and `delete` by primary key is a compile-time `&'static str`, as are a model's column list and its insert when it has no defaulted optional fields (section 8).
- **R-0702** Positional decoding; borrowed parameters (`&dyn morm::Param`), not boxed.
- **R-0704** Rows carry no per-row metadata.
- **R-0705** No dynamic dispatch in hot paths.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0734 | A per-request id in SQL, which would defeat every statement cache | `.tag()` takes only `&'static str`, and no trace comments are added |
| R-0736 | Client-chosen fields evicting hot statements | Runtime-built SQL has its own cache, and one-off shapes are never prepared |
| R-0743 | A stale cached statement after a migration | Re-prepared automatically outside a transaction |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0002 | suite | Unit: SQL generation |
| T-0072 | integration | `shape_cache` isolation under random and repeated shapes |
| T-0084 | integration | fragment renumbering |
