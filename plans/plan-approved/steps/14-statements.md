# Step 14: Statement caches and fragments

- **Phase:** Runtime
- **Depends on:** 11, 12, 13, 56
- **Can run beside:** 07, 08, 09, 29, 30, 31, 32, 33, 34, 35
- **Crates:** `morm`, `morm-core`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

The two bounded caches, shape tracking, `query_typed` fallback, fragment renumbering and the error mapping every query uses.

## Deliverables

- `statement_cache`, `shape_cache`, prepare on second use, built-SQL map, 0A000 and stale type handling, `$n` renumbering, `.tag()` text, typed error mapping

## End-to-end check

A load test with random and repeated shapes shows fixed statements staying cached, and a change to a table's result type or an enum type swap recovers outside a transaction.

## Out of scope

- The query APIs that use the caches (steps 15, 17)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Models / Type inference

- **R-0208** **NUL bytes:** Postgres rejects `\0` in text. morm doesn't scan every string for it, which would cost time on every write; Postgres's error becomes a typed `InvalidText`.

### Reading and writing / Query safety

- **R-0433** **`$n` in a fragment is local.** `comp`, `set_expr` and `.q()` fragments each count from `$1`, and morm renumbers them. The scan skips quoted strings and comments, so `'$1'` stays text. The scanner lives in `morm-core`.
- **R-0434** Typed errors: `Unique { constraint, field }`, `ForeignKey`, `NotNull`, `Check`, `Serialization`, `Deadlock`, `LockTimeout`, `TransactionTimeout`, `TransactionAborted`, `CommitUnknown`, `NestedConnection`, `PoolerMode`, `NotFound`, `EmptyPatch`, `Conflict`, `Forbidden`, `Validation`, `PoolBusy`, `ConnectionLost`, `ReadOnly`, `OutOfRange`, `InvalidText`, `InvalidOrder`, `InvalidCursor`, `Decimal`.

### Performance and memory / `morm::Pool`

- **R-0625** **Statement caches:** two bounded LRUs on each connection, below.

### Performance and memory / Statements

- **R-0703** Two bounded LRUs per connection. Evicted statements are closed when dropped, which also frees Postgres memory.

| ID | Setting | Default | Holds |
|---|---|---|---|
| R-0704 | `statement_cache` | 256 | SQL fixed at compile time: `get` and `delete` by primary key, a model's static `insert`, `sql!`, `db.sql()` and `db.raw()` |
| R-0705 | `shape_cache` | 128 | SQL put together at runtime: `select` builders including `filter_cols` and `.q()`, `get_by`, `get_where`, `get_q`, `delete_by`, `delete_where`, `delete_q`, inserts with defaulted fields, `update`, `update_q`, `.apply()`, and any query with a `.tag()` |

- **R-0706** **Why two:** each combination of columns is different SQL text, so a 10-field patch has up to 1,023 shapes, and a client chooses which one it sends. morm can't tell a fixed `.set()` chain from a conditional one, so the split is by how the SQL is made. A fixed chain is one busy shape and stays cached.
- **R-0707** **A shape is prepared the second time it's seen.** Its first use runs through `query_typed`, in one round trip, with no cache entry. A small fixed table of shape hashes per connection records what has been seen, and a collision only prepares a shape one use early.
- **R-0708** `.limit(n)` is always a bound parameter, so a client's page size can't create new shapes.
- **R-0709** **Worst case:** random one-off shapes are never prepared and evict nothing. A client repeating hundreds of shapes can churn `shape_cache`, but it can't touch `statement_cache`.
- **R-0711** **Query tags:** `.tag("orders.list")` adds a fixed comment, `/* orders.list */`, so server logs, `auto_explain` and `pg_stat_activity` show which code ran a query. It costs nothing unless used.
- **R-0712** It takes only `&'static str`, so it can't hold a request id, which would make every query text unique. Characters outside letters, digits, `.`, `_`, `:` and `-` become `_`, so a tag can't close the comment.
- **R-0713** One tag is one extra statement: the same SQL under two tags is prepared twice.
- **R-0714** `pg_stat_statements` ignores comments, so those two calls merge into one entry showing the first tag seen. Use logs for tags, and `pg_stat_statements` for totals per query.
- **R-0715** Built SQL text is kept in a small bounded map keyed by model and column bitmask, so a repeated shape doesn't rebuild the string.
- **R-0716** Other dynamic SQL and overflow go through `query_typed`, which needs no prepare round trips. It needs each parameter's Postgres type, and `ToSql` doesn't give that, so parameters are `&dyn morm::Param`, a trait that adds the type. Builder code knows the type from `F`.
- **R-0718** A cached statement can go stale when a migration changes something under it, and it then fails every time until it's re-prepared. On `cached plan must not change result type` (0A000), evict, re-prepare and retry once, outside a transaction. Do the same on `42883` or `42804` from a cached statement with a parameter of a non-built-in type, which is what an enum parameter returns when a swap keeps the old type: that statement parsed once, so this is a schema change and not a typo.

### Performance and memory / Query path

- **R-0720** SQL for `get` and `delete` by primary key is a compile-time `&'static str`, as are a model's column list and its insert when it has no defaulted optional fields (section 8).
- **R-0723** Positional decoding; borrowed parameters (`&dyn morm::Param`), not boxed.
- **R-0725** Rows carry no per-row metadata.
- **R-0726** No dynamic dispatch in hot paths.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0755 | A per-request id in SQL, which would defeat every statement cache | `.tag()` takes only `&'static str`, and no trace comments are added |
| R-0757 | Client-chosen fields evicting hot statements | Runtime-built SQL has its own cache, and one-off shapes are never prepared |
| R-0764 | A stale cached statement after a migration or an enum swap | Re-prepared or replaced automatically outside a transaction. Inside one, step 20 returns `Error::SchemaChanged`. |

### Managed enums

- **R-0981** **Stale server state.** Two errors, from any statement, mean a session holds compiled code a re-prepare can't reach: `XX000` with the message `cache lookup failed for type`, from a dropped type, and `42804` with the message `type of parameter ... does not match that when preparing the plan`, from a plpgsql function compiled before a column or row type changed. Each repeats on every call, and a new connection clears it. Evicting a statement can't, so morm discards the connection, retries once on another outside a transaction, and raises the pool's connection generation. On a `db.hold()` connection nothing moves: morm returns `Error::SchemaChanged`, and the connection is replaced on release.
- **R-0982** **Telling the rest of the pool.** Any stale statement raises a statement generation, held in the same atomic word as the pool's connection generation. A connection that checks out behind it clears both statement caches first, so connections don't each fail once per cached statement. The cost is the one atomic read a checkout already does.
- **R-0983** **Retrying is safe.** Postgres rolls back a failed statement outside a transaction, so a retry can't apply a write twice. A `42883`, or a `42804` with any other message, from a statement that isn't in a cache is never retried, the two errors in R-0981 are retried from any statement, nothing is retried once a row has reached the caller, and a retry that fails again returns its error.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0002 | suite | Unit: SQL generation |
| T-0072 | integration | `shape_cache` isolation under random and repeated shapes |
| T-0076 | integration | `InvalidText` |
| T-0084 | integration | fragment renumbering |
| T-0151 | integration | **Stale statements on Postgres 15–18:** after a hand-run enum swap, with the old type dropped and with it kept, a select, a filter, an insert, an update and an `= ANY` on a cached statement all succeed with no error reaching the caller, and the same SQL runs clean a second time. The codes match what proof 14 recorded. If they differ from this design on any version, stop and ask the owner with the evidence and options. |
| T-0152 | integration | **Stale sessions:** a plpgsql trigger compiled before a hand-run swap fails with `XX000`, morm replaces the connection and the write succeeds once, other connections in the pool are replaced at checkout, and `pool_stats()` shows it. The same holds for a plpgsql function with a `%ROWTYPE` variable compiled before a hand-run column type change, which fails with `42804` and `type of parameter ... does not match that when preparing the plan`. On a `db.hold()` connection the same failures return `Error::SchemaChanged`, nothing moves, and the connection is replaced on release. |
| T-0153 | integration | **No false recovery:** a `42883` from SQL that was never cached isn't retried, a retried write applies once, and a cached statement that fails again returns its error. |
