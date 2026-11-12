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

- `statement_cache`, `shape_cache`, prepare on second use, built-SQL map, each parameter's carrier and `($n::cast)`, the scanner's casts in fragments, 0A000 and stale type handling, the decoder choice per statement, `$n` renumbering, `.tag()` text, typed error mapping, with an online swap's shadow unique index mapped to its field

## End-to-end check

A load test with random and repeated shapes shows fixed statements staying cached, and a change to a table's result type or an enum type swap recovers outside a transaction.

## Out of scope

- The query APIs that use the caches (steps 15, 17)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Models / Type inference

- **R-0208** **NUL bytes:** Postgres rejects `\0` in text. morm doesn't scan every string for it, which would cost time on every write; Postgres's error becomes a typed `InvalidText`.

### Reading and writing / Query safety

- **R-0433** **`$n` in a fragment is local.** `comp`, `set_expr` and `.q()` fragments each count from `$1`, and morm renumbers them. The scan skips quoted strings and comments, so `'$1'` stays text. The scanner lives in `morm-core`, and it writes each parameter's cast, as `($n::type)`.
- **R-1221** **The scanner places the casts.** It follows Postgres's `scan.l` for everything that can hide a `$n`: standard strings with `''`, escape strings `E'..'` with backslashes, and `B'..'`, `X'..'`, `N'..'` and `U&'..'`; quoted identifiers with `""`, and `U&".."`; `--` comments and nested `/* */` comments; dollar quotes `$$..$$` and `$tag$..$tag$`; identifiers that hold `$`, such as `a$1`; and operator splitting, including the rule on a trailing `+` or `-`. It refuses an unterminated quote or comment. In the pass that renumbers each `$n`, it writes the cast for that parameter's slot: the column's cast for a value bound through a field, as in `comp` (choice G), and each value's own `Param` cast for a free one, in `set_expr`, `.q()`, `q_unchecked` and `sql_unchecked`. So `price < $1 AND tag = '$1' AND x = ANY($2) -- $1`, numbered from 3, becomes `price < ($3::numeric) AND tag = '$1' AND x = ANY(($4::text[])) -- $1`. A call site's value types are fixed, so its casts are too, and the scanner runs once per shape. Owner decision DH2, choice F.
- **R-0434** Typed errors: `Unique { constraint, field }`, `ForeignKey`, `NotNull`, `Check`, `Serialization`, `Deadlock`, `LockTimeout`, `TransactionTimeout`, `TransactionAborted`, `CommitUnknown`, `NestedConnection`, `PoolerMode`, `NotFound`, `EmptyPatch`, `Conflict`, `Forbidden`, `Validation`, `PoolBusy`, `ConnectionLost`, `ReadOnly`, `OutOfRange`, `InvalidText`, `InvalidOrder`, `InvalidCursor`, `Decimal`. `Unique`'s `field` maps a unique index named `_morm_new_<name>`, made from a unique index's name `<name>` through the 63-byte name function, to the field it maps `<name>` to, because while an online swap in step 39, 52 or 60 changes a column, or step 39 rebuilds an index concurrently after a type change, a write can collide on that shadow index alone, such as one whose value equals another only once rounded, or an enum label a map turns into one another row holds.

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
- **R-0716** Other dynamic SQL and overflow go through `query_typed`, which needs no prepare round trips. It needs each parameter's Postgres type, and `ToSql` doesn't give that, so parameters are `&dyn morm::Param`, a trait that adds the cast and the built-in carrier. Builder code knows both from `F`.
- **R-0718** A cached statement can go stale when a migration changes something under it, and it then fails every time until it's re-prepared. On `cached plan must not change result type` (0A000), evict, re-prepare and retry once, outside a transaction. A statement that casts a parameter to a non-built-in type, such as an enum, a domain or a composite, survives a swap of that type when it touches a table holding it, because Postgres parses it again and resolves the name again. One that touches no such table, such as `SELECT ($1::st)::text`, gets `XX000` `cache lookup failed for type` once the old type is dropped, or `22P02` naming the old type while it's kept. So outside a transaction, any error from a cached statement that casts a parameter to a non-built-in type evicts it, prepares it again and retries once, and a retry that fails again returns its error: that statement parsed once, so the error comes from a schema change and not a typo, and Postgres rolled the failed statement back, so the retry is safe. R-0981's connection replacement still comes first for `XX000` `cache lookup failed for type`. Owner decision DH2, choice I.
- **R-1220** **Every parameter has a built-in carrier and a cast.** Every statement morm sends declares each parameter in Parse as its built-in carrier and casts it in the SQL as `($n::cast)`, so Postgres never infers a parameter's type: in `sql!`, every builder, model inserts and updates, `UNNEST`, keyset pages, runtime fragments, `.limit(n)` and morm's own advisory locks, the last two as `int8`. The parentheses are needed: without them, `$1[1]` would become `$1::int8[][1]`, which Postgres reads as the type `int8[][1]` and so returns the whole array, and `$1.n` would fail with `0A000`. A non-built-in type is cast by name, so no OID is looked up or sent. Built text is cached, in R-0715's map and in `shape_cache`, with its casts, and nothing about typing costs a round trip. Owner decision DH2, choices A and C.
- **R-1104** **Decoding follows the column's real type.** When a statement is prepared, morm reads each result column's type from the row description Postgres returns, chooses each field's decoder from it once, and keeps that choice with the statement in its cache entry. A `query_typed` call, which has no cache entry, chooses once from its own result's row description. The internal query entry point then reads all of that statement's rows through `from_row` when every column has the type the binary was built against, so that path is unchanged, and through `from_row_as` with the statement's choice otherwise, deciding once before the first row and never per row. A re-prepare after a migration, on `0A000` as R-0718 says, chooses again, so a statement cached before a cutover picks the new column type's decoder. Every statement is prepared with each parameter's built-in carrier, as `query_typed` sends it. Its SQL casts the parameter to the column type the binary was built against where it's bound through a field, and otherwise to the value's own type. So Postgres converts it when it's written: a widened column takes every value the binary writes, and a value a narrowed one can't hold gets the database's error for that write.

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

- **R-0981** **Stale server state.** Two errors, from any statement, mean a session holds compiled code a re-prepare can't reach: `XX000` with the message `cache lookup failed for type`, from a dropped type, and `42804` with the message `type of parameter ... does not match that when preparing the plan`, from a plpgsql function compiled before a column or row type changed. Each repeats on every call, and a new connection clears it. Evicting a statement can't, so morm discards the connection, retries once on another outside a transaction, and raises the pool's connection generation. On a `db.hold()` connection nothing moves: morm returns `Error::SchemaChanged`, and the connection is replaced on release. A third error is transient: `XX000` with the message `could not open relation with OID`, from a caller of a `BEGIN ATOMIC` function while a cutover replaces that function or a view under it. It doesn't repeat, and a new statement on the same connection works, so the connection isn't at fault: outside a transaction morm retries once on that connection, held or not, replaces nothing and raises no generation. Tested on Postgres 15 to 18: all 3,566 such errors, with plain and cached plans, cleared on the first retry on the same connection.
- **R-0982** **Telling the rest of the pool.** Any stale statement raises a statement generation, held in the same atomic word as the pool's connection generation. A connection that checks out behind it clears both statement caches first, so connections don't each fail once per cached statement. The cost is the one atomic read a checkout already does.
- **R-0983** **Retrying is safe.** Postgres rolls back a failed statement outside a transaction, so a retry can't apply a write twice. A `42883`, or a `42804` with any other message, from a statement that isn't in a cache is never retried, the three errors in R-0981 are retried from any statement, nothing is retried once a row has reached the caller, and a retry that fails again returns its error.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0002 | suite | Unit: SQL generation |
| T-0072 | integration | `shape_cache` isolation under random and repeated shapes |
| T-0076 | integration | `InvalidText` |
| T-0084 | integration | fragment renumbering |
| T-0151 | integration | **Stale statements on Postgres 15–18:** with each parameter declared as its carrier and cast to its type, after a hand-run enum swap, with the old type dropped and with it kept, a select, a filter, an insert, an update and an `= ANY` on a cached statement on the swapped table all succeed with no error reaching the caller, and the same SQL runs clean a second time. A cached statement that touches no table of the type, such as `SELECT ($1::st)::text`, gets `XX000` with the old type dropped and `22P02` while it's kept, and recovers outside a transaction by R-0718's retry, once. The codes match what proof 14 recorded. If they differ from this design on any version, stop and ask the owner with the evidence and options. |
| T-0152 | integration | **Stale sessions:** a plpgsql trigger compiled before a hand-run swap fails with `XX000`, morm replaces the connection and the write succeeds once, other connections in the pool are replaced at checkout, and `pool_stats()` shows it. The same holds for a plpgsql function with a `%ROWTYPE` variable compiled before a hand-run column type change, which fails with `42804` and `type of parameter ... does not match that when preparing the plan`. On a `db.hold()` connection the same failures return `Error::SchemaChanged`, nothing moves, and the connection is replaced on release. With a `BEGIN ATOMIC` function and a view under it dropped and recreated by hand in a loop, a caller that gets `XX000` with `could not open relation with OID` is retried once on the same connection and succeeds, on a pool and on a `db.hold()` connection, with nothing replaced and no generation raised. Inside a transaction it returns `Error::SchemaChanged`, and a retry that fails again returns its error. |
| T-0153 | integration | **No false recovery:** a `42883` from SQL that was never cached isn't retried, a retried write applies once, and a cached statement that fails again returns its error. |
| T-0222 | integration | **Decoding follows the column's real type, on Postgres 15–18:** a cached statement on an `integer` column reads through `from_row`. After a hand-run change to `bigint`, its next run gets `0A000`, re-prepares and reads through `from_row_as` with the `bigint` decoder, and after the change back it reads through `from_row` again. A test build counts one choice per prepare and per `query_typed` call, and none per row, over a statement returning 10,000 rows, and a `query_typed` call on the changed column chooses from its own result. Every statement is prepared with its parameters' carriers and casts: after `integer` becomes `bigint`, a cached insert of an `i32` cast as `($1::int4)` succeeds, and after `bigint` becomes `integer`, a cached insert of `2^40` from an `i64` field cast as `($1::int8)` gets the database's `22003`, while one that fits succeeds. |
| T-0281 | suite | **Unit: the scanner's casts:** each of the 16 ways R-1221 lists that a `$n` can hide stays untouched, operators split as Postgres splits them, an unterminated quote or comment is refused, and a fragment is renumbered from any start with each slot's cast written as `($n::cast)` |
| T-0282 | integration | **Casts in every position, on Postgres 15–18:** a parameter written as `($n::cast)` prepares and runs, through a prepared statement and through `query_typed`, with the right answer, after a subscript `($1::int8[])[1]` and a field selection `(($1::pt.pair).n)`, under unary minus, before a cast of the user's own, in `LIMIT`, `OFFSET` and `FETCH FIRST`, in `IN (..)`, `BETWEEN`, `ARRAY[..]` and `ROW(..)`, and with `COLLATE "C"`, `CAST(.. AS int4)`, `VARIADIC`, `AT TIME ZONE`, `NOT` and `IS NULL`; and with `plan_cache_mode = force_generic_plan`, `c = ($1::citext)` with a `text` carrier, `($1::bpchar)`, an enum, a domain and `= ANY(($1::citext[]))` each use an index |
