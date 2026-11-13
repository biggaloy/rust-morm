# Stale statements and functions across a type swap: design

For the owner to approve. It answers EE1, EB, EC, ED, EE6 and EE11 from `step-55-open-items.md` as one design, with EA (enum and composite result columns read as `::text`) taken as decided. Nothing here is written into `plan-approved/` until the owner answers. Base: `origin/main` at `bc68ffe`; PR #3's spikes at `d8bd613`.

## In short

Four mechanisms, each tested on Postgres 15.20, 16.16, 17.12 and 18.7, straight, behind PgBouncer 1.20.1 and 1.22.1 with three server connections, and on a streaming standby:

1. **The reset.** Every `tx` step a migration runs ends with two catalog-only statements: it creates and drops a placeholder operator in `_morm`. Postgres then marks every cached plan invalid in every session of that database, at commit, as if each had run `DISCARD PLANS`. A cached statement is analysed again on its next use, so it resolves type names again. This fixes, with no error reaching any app, every stale statement that touches no table of the type (EE1's silent case included), and every `%ROWTYPE`, record and trigger function that would give `42804`. It works for server sessions behind PgBouncer, which morm can't reach, and it replays on a standby. It leaves two things: a statement whose result type changed (`0A000`, which needs a re-prepare to choose new decoders), and a plpgsql variable declared with the old type, which is fixed when the function compiles.
2. **The relabel.** When morm keeps an old enum type under a retired name, as the online cutover does until `purge`, it renames each of the type's labels to `_morm_retired_<label>` in the same transaction. Any code still compiled against the old type then fails with `22P02` naming the retired type, instead of running silently on the old order. The reviewer's fix alone isn't enough: with the relabel and no reset, a cached comparison of two constants still returned the old answer, silently, on every version.
3. **Recompiling functions morm doesn't manage.** A no-op `ALTER FUNCTION ... COST <same>` changes the function's catalog row, so every session compiles it again on its next call, behind PgBouncer and on a standby too. In the swap's transaction, morm runs it on each unmanaged plpgsql function the walk finds that the migrating role may alter. The rest stay in R-1040's decision, whose text changes.
4. **Behind PgBouncer, a shared marker.** A cached statement's text starts with `/* morm g<n> */`, where `<n>` is a generation every process reads from the catalog. Each `tx` step raises it with the reset. A process started after a migration therefore never shares a stale server statement. A stale error from a cached statement, at prepare or at execute, retries once through an unnamed statement, which is never stale, and then moves the process to the current marker.

Around these: stale errors are recognised without reading translated text (SQLSTATE with the routine, an identifier, or an untranslated `elog` message); a standby's recovery conflict that ends a connection inside a transaction becomes a retryable `Serialization`; and the narrowing pragma is refused.

| Choice | Recommendation |
|---|---|
| **SA** (EE1) How stale code is kept off the old type | (a) the reset and the relabel together |
| **SB** (ED) plpgsql compiled before a swap | (a) refresh unmanaged functions with a no-op `ALTER FUNCTION` where morm may; a `22P02` naming a retired type is a stale session; the decision covers the rest and says `RECONNECT` behind PgBouncer |
| **SC** (EB) Recognising stale errors in any language | (a) SQLSTATE with the routine, an identifier or an untranslated message, pinned per version by tests |
| **SD** (EC, EE11b) Stale server statements behind PgBouncer | (a) the shared marker, unnamed retry, prepare-time errors handled as execute-time ones |
| **SE** Where the marker lives | (a) the `_morm` schema's comment |
| **SF** When the statement generation rises | (a) only for a confirmed stale statement |
| **SG** (EE11a) A standby ends a connection inside a transaction | (a) `Serialization` in a transaction, retried by `.retry(n)`; `ConnectionLost` on a hold |
| **SH** (EE6) Pragmas that hide R-1080's narrowing | (a) refuse `performance_warnings` in any form; a function with `disable:check` gets its own line in the narrowing decision |

Each choice, its evidence and its options are below, then what changes in each file, the README notices, and what I couldn't run.

## What was tested

Everything ran on throwaway servers under `.claude/scratch/` in my worktree, with `TMPDIR` set there, all stopped and deleted afterwards.

- **Postgres** 15.20, 16.16, 17.12 and 18.7, built from ftp.postgresql.org's tarballs, each checked against its published SHA-256, with `--enable-nls` so German messages are real. One primary each, and for EE11 a streaming hot standby of each, made with `pg_basebackup -R`, `hot_standby_feedback = off` and `max_standby_streaming_delay = 3s`.
- **PgBouncer** 1.20.1 (SHA-256 `24992cf557d73426d7048698dffc7b019e6364d4d8757ae2cf5e2471286a2088`, the one R-1238 pins) and 1.22.1 (`2b018aa6ce7f592c9892bb9e0fd90262484eb73937fd2af929770a45373ba215`), built from pgbouncer.org's tarballs, each checked against its `.sha256`. Transaction mode, `default_pool_size = 3`, `min_pool_size = 3`, `server_round_robin = 1`. Three instances per Postgres: 1.22.1 with `max_prepared_statements = 200`, 1.22.1 with `0`, and 1.20.1, which has no prepared-statement support. Every pooler run below reached all three server sessions, counted by `pg_backend_pid()` from each client.
- **plpgsql_check** 2.10.16, built from the `v2.10.16` tag's tarball on GitHub (SHA-256 `b4734ffac4110348e8b963abb5b4f97cf0ade819ab92a74f0ddd47c4141288e3`), against each of the four builds, extension version `2.10`. Not the PGDG package: I can't install packages here, and the source is the same release proof 18 used.
- **Clients:** `psql` for single-session cases, and a throwaway Rust harness on `tokio-postgres` 0.7.18 (proof 13's lockfile, built offline) for named and unnamed statements, several clients, PgBouncer and the standby. The harness casts every parameter as `($n::type)` with a built-in carrier, as R-1220 says, and reads results the way morm would.
- Results were compared across versions by diffing the printed tables with OIDs and ports masked. Unless a row says otherwise, all four versions printed the same.
- Names used below: E1 is SA's table; E2 the swap and `ALTER FUNCTION` as a role that isn't superuser; E5 the messages in German and English; E6 the pragmas; E7 repeated calls of a stale function; E8 a transaction open across the swap, and what the relabel and reset wait on; E9 two variables of the type, and SQL-language functions. The PgBouncer and standby runs are named by their sections.

### The swap every test ran

The harness runs (SB, SD, SG) used this swap, one transaction from another session, shaped like the online cutover with the old type kept. The `psql` runs (E1 to E9) used the same shape with shorter names, with the relabel and the reset switched on and off as each table says:

```sql
ALTER TYPE st RENAME TO _morm_retired_m2_st;          -- R-0995's retired name
CREATE TYPE st AS ENUM ('c','b','a','d');             -- reordered, one label added
ALTER TABLE t RENAME COLUMN s TO _morm_retired_m2_s;  -- the retired column keeps the old type
ALTER TABLE t ADD COLUMN s st;
UPDATE t SET s = _morm_retired_m2_s::text::st;
ALTER TABLE t ALTER COLUMN n TYPE bigint;             -- a column type change in the same step
-- relabel (SA):
ALTER TYPE _morm_retired_m2_st RENAME VALUE 'a' TO '_morm_retired_a';  -- and 'b', 'c'
-- reset (SA), and the marker (SD, SE):
CREATE OPERATOR _morm.### (LEFTARG = int4, RIGHTARG = int4, FUNCTION = int4eq);
DROP OPERATOR _morm.### (int4, int4);
COMMENT ON SCHEMA _morm IS 'morm statement generation 2';
```

The statements and functions cached before it, in each session:

| Name | What |
|---|---|
| S1 | `SELECT ($1::st)::text`, touching no table |
| S2 | `SELECT (($1::st) < ($2::st))::text` with `'a'` and `'c'`: true before, false after |
| S3 | `SELECT n FROM t WHERE id = ($1::int4)`, whose result type changes |
| S4 | a function with a `t%ROWTYPE` variable returning `(r.n + 1)::text` |
| S5 | a function with `DECLARE v st; v := p;`, comparing `v < 'c'::st`, never refreshed |
| S6 | `UPDATE t SET s = ...` firing a `BEFORE UPDATE` trigger function that compares `NEW.s` |

## SA (EE1). Keeping stale code off the old type

### Evidence

Straight connections, `psql` `PREPARE` and plpgsql, the same on 15.20, 16.16, 17.12 and 18.7. "Silent" means a wrong answer with no error:

| After the swap | No relabel, no reset | Relabel only | Reset only | Relabel and reset |
|---|---|---|---|---|
| `EXECUTE b_lt('a','c')`, `($1::st) < ($2::st)` | `t`, **silent** | `22P02` naming the retired type | `f`, right | `f`, right |
| `EXECUTE b_const`, `'a'::st < 'c'::st` | `t`, **silent** | `t`, **silent** | `f`, right | `f`, right |
| `EXECUTE b_audit('c')`, an `INSERT ... WHERE 'a'::st < ($1::st)` | inserts the row, **silent** | `22P02` | inserts nothing, right | inserts nothing, right |
| A function `RETURN ('a'::st < 'c'::st)` | `true`, **silent** | `true`, **silent** | `false`, right | `false`, right |
| A function with `DECLARE v st; v := p;` comparing `v < 'c'::st` | `true`, **silent** | `22P02` naming the retired type | `42883 operator does not exist: st_old < st` | `22P02` naming the retired type |
| A function with two such variables, comparing `v < w` | | | `true`, **silent** (E9) | |
| A `%ROWTYPE` function | `42804` (`plpgsql_param_eval_recfield`) | `42804` | right | right |
| `EXECUTE` of a statement returning the changed column | `0A000` | `0A000` | `0A000` | `0A000` |

- **Why the relabel alone isn't enough:** Postgres folds a constant such as `'a'::st` into the cached plan as the label's OID. A relabel changes the label's text, not its OID, so the folded comparison still uses the old order. Only analysing the statement again fixes it.
- **Why the reset alone isn't enough:** a plpgsql variable's type is fixed when the function compiles, and plan invalidation doesn't recompile functions. So `DECLARE v st` stays on the old type, and with labels both types share it runs silently.
- **What the reset is:** `plancache.c` registers `PlanCacheSysCallback` on the `pg_namespace` and `pg_operator` syscaches, on 15 to 18 alike, and the callback runs `ResetPlanCache()`, which marks every saved plan invalid, protocol-level statements and plpgsql's expressions included. Creating and dropping an operator sends that invalidation to every session of the database at commit. I tested four forms on all four versions: creating and dropping a schema, `GRANT USAGE ON SCHEMA` and the operator pair each reset; `ALTER SCHEMA ... OWNER TO` its current owner is a no-op and didn't. The operator pair needs only `CREATE` on `_morm` and leaves nothing behind, so it's the one to use. `DISCARD PLANS` in the session gave the same table as the reset, so the reset is `DISCARD PLANS` for every session at once.
- **As a role that isn't superuser:** a role owning the type and a schema for the operator, with nothing else, ran the type rename, the relabel and the operator pair in one transaction on all four versions (E2).
- **E5 again with the reset in the swap:** the `%ROWTYPE` function and the `record` function that gave `42804` were right, and so was a cached `SELECT ($1::st)::text`; the `DECLARE` function gave `22P02` naming the retired type, and a statement returning the changed column `0A000`, on all four versions. Once the retired type was dropped, as `purge` does, the `DECLARE` function in that old session gave `XX000 cache lookup failed for type`, which R-0981 already handles, and the cached statement stayed right.
- **SQL-language functions** over the changed table, one `RETURNS SETOF t` and one returning the changed column, were right after the reset in a session that had called them before (E9, all four versions).
- **It waits on nothing:** with `lock_timeout = '200ms'`, the relabel and the operator pair committed while another session sat idle in a `REPEATABLE READ` transaction that had read the table (E8, all four versions).
- **Its cost:** `pgbench -M prepared -S`, 8 clients, 10 s, on 18.7: 176,149 and 167,832 tps with no reset, 164,899 tps with a reset sent every 0.2 s (46 resets). A migration sends one per `tx` step, so the cost is within noise. A heavier statement costs one more planning the first time it runs after a step.
- **Behind PgBouncer and on a standby:** see SD and SG. The reset reached every server session behind 1.20.1 and 1.22.1, and it replayed on every standby.

### A transaction already open at the cutover

Postgres applies a committed catalog change to an open transaction only when that transaction next takes a lock. In a `READ COMMITTED` transaction begun before the swap, a cached `($1::st) < ($2::st)` returned the old answer after the swap committed, until the same transaction read another table; from then on it returned the new one (E8, all four versions). This isn't a stale state: it ends with the transaction and isn't repeated by the next. Those statements run as they would have a moment before the cutover, as for any DDL Postgres runs, morm's or not, and a transaction that touches a swapped table can't straddle the cutover at all, because the cutover takes that table's lock. The README says so (notice 3).

### Options

- **(a) Recommended. The reset and the relabel, together.**
  - Every `tx` step, in `up`, `down`, `rollback`, `--resume` and `purge`, ends with the operator pair and raises the generation (SD, SE). One rule, no list of kinds, as R-1038 does for the walk. `no-tx` and `batch` steps change no type, so they don't need it.
  - Whenever morm keeps an enum type under a retired name (the online cutover, R-0995's retire of a type with its columns), it renames each label to `_morm_retired_<label>` in the same transaction, through the 63-byte name function, because a label has the same limit. The ledger already records the type's labels. Down and rollback read a retired column by mapping each placeholder back through those recorded labels in their own expression, and never rename the labels back, which would make stale code silent again. `purge` drops the type as before. Restoring a retired type to live (a rollback of a retire) renames type and labels back, which is right, because it's live again with the same OID.
  - User labels may not start with `_morm_`: `#[pg_enum(value = "...")]` refuses it at compile time, and `import` and `check_schema` name such a label.
  - The retired column shows `_morm_retired_pending` and so on until `purge`; the README says so.
- **(b) The relabel only,** as the reviewer proposed. Constant-only comparisons, in cached statements and in functions, stay silent on the old order (the table above).
- **(c) The reset only.** Unmanaged plpgsql with a variable of the type stays silent on the old order for labels both types share.
- **(d) Drop the old type at cutover,** keeping retired values as `text`. This rewrites the table, which the online form exists to avoid, and conflicts with R-1050.

## SB (ED). plpgsql compiled before a swap

### Evidence

- A function with `DECLARE v st` compiled before the swap kept failing on every call after it, through named, unnamed and simple-protocol statements, until something recompiled it (18.7: 12 calls through the harness and 6 through `psql`, inside and outside a transaction). After the relabel each failure is `22P02 invalid input value for enum <schema>._morm_retired_m2_st: "a"`, routine `enum_in`.
- **A no-op `ALTER FUNCTION f(text) COST 100`**, its cost unchanged, run by the function's owner (not superuser) from another session, made the next call in the failing session compile the function again and return the right answer, on all four versions. Postgres recompiles a plpgsql function when its `pg_proc` row's `xmin` or `ctid` changes, and any `ALTER FUNCTION` writes a new row. Behind PgBouncer 1.20.1 and 1.22.1 the same `ALTER` fixed every server session (6 of 6 calls right after it, on each), and the stale session no client can reach was fixed with it.
- On each standby, the same `ALTER` run on the primary fixed the standby session that had the function compiled, once replayed (all four versions).
- Without it, behind PgBouncer, a new client reached the same stale server session: S5 failed 6 of 6 times for new clients on every pooler instance.
- The reset doesn't fix a `DECLARE` (SA's table), and neither do `DISCARD PLANS` or `DISCARD ALL` (proof 15).

### Options

- **(a) Recommended.**
  - Managed functions keep R-1039's `CREATE OR REPLACE` refresh.
  - In the same transaction, morm runs a no-op `ALTER FUNCTION <regprocedure> COST <its current cost>` on each unmanaged plpgsql function or procedure the walk finds, where the migrating role owns it or is a member of its owner. It changes nothing in the definition, so it isn't changing the unmanaged object, only making every session compile it again, as a reconnect would. The decision R-1040 opens still lists every unmanaged object, because the refresh fixes staleness, not logic.
  - A `22P02` from `enum_in` whose message holds `_morm_retired_` is a stale session, as R-0981's two errors are: a direct connection is replaced and the call retried once outside a transaction; a held one returns `SchemaChanged` and is replaced on release; inside a transaction it's `SchemaChanged`. The name is an identifier, so it's in the message in every language (SC).
  - R-1040's text names, for each unmanaged function, whether morm refreshed it, and says: morm apps connected straight to Postgres recover by themselves from a dropped type, a changed parameter type and a retired type's labels; behind PgBouncer a new client reaches the same server session, so run `RECONNECT` on each PgBouncer after the migration for the functions morm couldn't refresh; other services must reconnect.
- **(b) No `ALTER`.** Every unmanaged function stays stale in every session that compiled it until that session ends; behind PgBouncer that's until `RECONNECT` or `server_lifetime`. The relabel keeps them loud.
- **(c) Always drop the old type in the swap.** Conflicts with R-1050, and only turns `22P02` into `XX000`.

## SC (EB). Recognising stale errors in any language

### Evidence

With `lc_messages = 'de_DE.utf8'` and `'C'`, on all four versions (E5):

| Stale error | SQLSTATE | German message | Routine |
|---|---|---|---|
| A `%ROWTYPE` field after `integer` to `bigint` | `42804` | `Typ von Parameter 4 (bigint) stimmt nicht mit dem überein, als der Plan vorbereitet worden ist (integer)` | `plpgsql_param_eval_recfield` |
| A `record` variable after a composite is recreated | `42804` | `Typ von Parameter 2 (comp) stimmt nicht mit dem überein, ...` | `plpgsql_param_eval_generic_ro` |
| A retired type's label | `22P02` | `ungültiger Eingabewert für Enum _morm_retired_m1_st: »a«` | `enum_in` |
| A cached plan whose result type changed | `0A000` | `gecachter Plan darf den Ergebnistyp nicht ändern` | `RevalidateCachedQuery` |
| A dropped type | `XX000` | `cache lookup failed for type <n>`, in English | `getTypeInputInfo`, `getTypeOutputInfo` and others |

- `pl_exec.c` raises the parameter-type error from exactly three routines on 15.20, 16.16, 17.12 and 18.7: `plpgsql_param_eval_recfield`, `plpgsql_param_eval_generic` and `plpgsql_param_eval_generic_ro`. `RevalidateCachedQuery` raises the `0A000` on all four; 18 also has the same message in `sql_postrewrite_callback`, for a SQL function whose body stops or starts returning a row type, which its own comment says probably can't happen. That one isn't a stale statement, so it isn't treated as one.
- `XX000 cache lookup failed for type` and `could not open relation with OID` come from `elog`, which is never translated: the German run printed them in English.
- Identifiers are never translated, so `_morm_retired_m1_st` is in the German message as it is in the English one.
- The recovery-conflict routine differs by version (`ProcessInterrupts` on 15 and 16, `ProcessRecoveryConflictInterrupt` on 17 and 18), so SG doesn't use it.

### Options

- **(a) Recommended. No translated text is read.** morm recognises each stale error by a signal that doesn't change with `lc_messages`:

  | Error | Signal | Kind |
  |---|---|---|
  | `0A000` from a cached statement | routine `RevalidateCachedQuery` | stale statement (R-0718) |
  | `42804` from any statement | routine one of the three above | stale session (R-0981) |
  | `XX000` from any statement | message starting `cache lookup failed for type` (`elog`) | stale session (R-0981) |
  | `XX000` from any statement | message starting `could not open relation with OID` (`elog`) | transient (R-0981) |
  | `22P02` from any statement | routine `enum_in`, message holding `_morm_retired_` | stale session (SB) |
  | `22P02` from a cached statement that casts a parameter to a non-built-in type | routine `enum_in`, and the message doesn't hold the cast's type name, with or without its schema, as a whole identifier (Postgres prints the schema only when the type isn't on the `search_path`, as E5 and the harness showed) | stale statement (R-0718, R-0984), for a swap made outside morm |
  | Any error from a cached statement that casts a parameter to a non-built-in type | none: retried once, as choice I says, and stale only if the retry succeeds (SF) | stale statement |

  An English message is accepted as a second signal for `0A000` and `42804`, so a future version that renames a routine still works on an English server. T-0151 and T-0152 assert each routine on 15 to 18, so supporting a new major version means these tests run on it first, and a renamed routine fails CI there. A miss is never silent: the error reaches the app as `Error::Postgres`.
- **(b) Require English messages.** The startup statement reads `current_setting('lc_messages')` and refuses a pool whose setting doesn't start with `C`, `POSIX` or `en`, with `Error::Config` naming `ALTER ROLE <role> SET lc_messages = 'C'` (a superuser's command) or `GRANT SET ON PARAMETER lc_messages TO <role>`. Message matching stays. Behind PgBouncer one client's `SET lc_messages` would reach other clients' sessions, and a server initialised in another language refuses every morm app until someone changes the role. It needs a loud README notice.

## SD (EC, EE11b). Stale server statements behind PgBouncer

### Evidence

Behind PgBouncer 1.22.1 with `max_prepared_statements = 200` and three server sessions, six clients each cached S1 to S6 under the marker they read (`g1`), ran them concurrently five times so every server session held them, then the swap ran straight on Postgres. Each cell is six calls, one per client, the same on all four versions:

| After the swap, with the reset | S1 | S2 | S3 | S4 | S5 | S6 |
|---|---|---|---|---|---|---|
| Old clients, their cached statements | right | right | `0A000` | right | `22P02` retired | right |
| New clients preparing the old text (`g1`), as a new process with R-0975's per-pool counter would | right | right | **`0A000` at prepare** | right | `22P02` retired | right |
| New clients preparing under the marker they read (`g2`) | right | right | right | right | `22P02` retired | right |
| Old clients, the same text unnamed | right | right | right | right | `22P02` retired | right |
| After the no-op `ALTER FUNCTION` on S5's function | | | | | right | |

| After the swap, without the reset | S1 | S2 | S3 | S4 | S5 | S6 |
|---|---|---|---|---|---|---|
| Old clients, cached | `22P02` | `22P02` | `0A000` | `42804` | `22P02` | `42804` |
| New clients, old text | `22P02` | `22P02` | `0A000` at prepare | `42804` | `22P02` | `42804` |
| New clients, the marker they read: still `g1`, because nothing raised it | `22P02` | `22P02` | `0A000` at prepare | `42804` | `22P02` | `42804` |
| Old clients, unnamed | right | right | right | `42804` | `22P02` | `42804` |

Behind 1.22.1 with `max_prepared_statements = 0` and behind 1.20.1, where morm runs every statement unnamed (R-0677), every call was right with the reset apart from S5 (`22P02` until the `ALTER`); without it, S4 and S6 gave `42804` and S5 `22P02` to old and new clients alike, because the server sessions were stale, while S1 to S3 were right, being unnamed.

Straight to Postgres the same clients needed nothing new: with the reset, only S3 failed (`0A000`, fixed by R-0718's re-prepare) and S5 (`22P02`, fixed by a new connection or the `ALTER`).

So:

- With the reset, the only stale server statement left behind PgBouncer is one whose result type changed. With EA, an enum or composite swap never changes a result type morm reads, so this comes from a column type change (R-1104) or a view's shape. It's permanent for that text on that server connection, for every client, and it arrives at prepare as often as at execute.
- A marker every process reads cures it the moment the migration raises it, and an unnamed statement always works.
- A marker nothing raises (the second table's third row) cures nothing. That's EE4 for a per-pool counter.

### Options

- **(a) Recommended. A shared marker, read from the catalog.**
  - With `pooler = "transaction"` and the caches on, the text morm prepares for a cached statement starts with `/* morm g<n> */`, before any `.tag()`. `<n>` is the database's statement generation (SE). It's one fixed comment per generation, not per request, so R-0755 holds, and `pg_stat_statements` ignores it (R-0714). Straight to Postgres, or with the caches off, there's no marker.
  - A pool reads `<n>` once when it starts, in the startup statement once step 41 exists and in one catalog query before that or with `check_schema = "off"`.
  - On a stale-statement signal (SC) from a cached statement, at prepare or at execute, outside a transaction: morm runs the call once through `query_typed`, which PgBouncer never shares. If that succeeds, the cached text was stale: morm reads `<n>` again on that connection, and adopts it if it moved; if it didn't (DDL made outside morm), morm adds a random 64-bit salt, `/* morm g<n>.<salt> */`, kept until `<n>` next moves, so the process's texts collide with no stale server statement. Either way it raises the statement generation (R-0982), so every connection drops its cached statements at checkout. If the unnamed run fails too, the error is returned and nothing changes.
  - Inside a transaction there's no retry: a signal returns `SchemaChanged` (R-0984), and after the rollback morm reads `<n>` and applies the same rule, so `.retry(n)`'s next attempt prepares fresh text.
  - A stale-session signal behind PgBouncer (`42804`, `XX000` cache lookup, `22P02` retired) is retried once on another client connection, which may reach a fresh server session, but doesn't raise R-0975's connection generation: replacing client connections can't reach a server session, so it would only churn. The reset, the managed refresh and SB's `ALTER` cure these; R-1040's decision covers the rest.
  - Cost: one catalog read at pool start, and on a stale error one unnamed run plus one catalog read, once per process per generation. While old and new processes run side by side, each server connection holds a set of statements per generation in use, which `max_prepared_statements` bounds.
- **(b) Caches off behind any pooler.** Every statement is unnamed, so none can be stale, at the cost of parsing and planning every query on the server. The reset and SB are still needed for functions. R-0677 changes to require `statement_cache = 0` and `shape_cache = 0` with `pooler = "transaction"`.
- **(c) The migration sends `RECONNECT`** to each PgBouncer listed in `[migrations]`, which needs each one's admin console and a complete list, and does nothing for a PgBouncer it isn't told about.
- **(d) A random marker per process** (EE4's option b). Nothing collides, but processes never share server statements, so each server connection holds one set per process.

## SE. Where the marker lives

The app role must read it with no grant, in the startup statement, and every migration must move it, monotonically, in the step's own transaction.

- **(a) Recommended. The `_morm` schema's comment,** `morm statement generation <n>`, set with `COMMENT ON SCHEMA _morm` in each `tx` step after the operator pair. Any role reads it from `pg_description`, which needs no grant; it read the same through PgBouncer as straight. A database with no `_morm` reads as `0`. `COMMENT ON SCHEMA` needs the schema's owner, so `migrate`, `rollback` and `purge` preflight check that the migrating role owns `_morm` and has `CREATE` on it, and refuse with exit code `2`, naming `ALTER SCHEMA _morm OWNER TO <role>`. `morm-admin status` shows the generation.
- **(b) The ledger,** such as the latest `_morm.history` id, which every app role would need `USAGE` on `_morm` and `SELECT` on the table to read.

## SF. When the statement generation rises

R-0982 raises the pool's statement generation on "any stale statement", which clears every connection's caches. With choice I, R-0718 retries any error from a cached statement that casts a parameter to a non-built-in type, so a client sending a bad enum label could clear the caches on every request.

- **(a) Recommended.** A statement is stale, and raises the generation, only when its error is one of SC's certain signals (`0A000` from `RevalidateCachedQuery`, `22P02` naming a retired type, `XX000` cache lookup, the three `42804` routines), or when the retry on a fresh statement succeeds where the cached one failed. A retry that fails the same way returns its error and raises nothing.
- **(b) As written.**

## SG (EE11a). A standby ends a connection inside a transaction

### Evidence

The swap ran on the primary with the reset; sessions were on its standby, with `max_standby_streaming_delay = 3s`. The same on 15 to 18:

| Standby session | What happened |
|---|---|
| Idle in a `REPEATABLE READ` transaction that had read the table | Ended after the delay. The client's connection task received `FATAL 40001 terminating connection due to conflict with recovery` while idle; the next statement got `tokio-postgres`'s `connection closed`, with no database error attached |
| Idle in a `READ COMMITTED` transaction that had read the table | The same |
| Running a query on the table during replay | `ERROR 40001 canceling statement due to conflict with recovery` after 3.5 s; the connection stayed |
| Idle, not in a transaction, holding a prepared statement on the table | Untouched; its next run was right |
| Cached statements and functions (S1 to S5) | Exactly as on the primary with the reset: S1, S2 and S4 right, S3 `0A000`, S5 `22P02` retired. So the reset and the relabel replay. After a no-op `ALTER FUNCTION` on the primary replayed, S5 was right |
| The swap's replay | Waited 3.0 to 3.1 s behind the idle transactions |

A transaction ended this way never committed, and on a standby it can't have written anything. R-0461 already retries a single statement's `40001` once.

### Options

- **(a) Recommended.**
  - morm's connection task keeps the error that closed the connection.
  - When a statement in a transaction finds the connection closed and that error was a `FATAL` with `40001`, and the statement wasn't the `COMMIT`, morm returns `Error::Serialization` and discards the connection, so `.retry(n)` runs the transaction again on a fresh one, as Postgres's own SQLSTATE says it may. A loss during `COMMIT` stays `CommitUnknown` (R-0440).
  - On a `db.hold()` connection it's `Error::ConnectionLost`, as for a restart (R-0697), naming the recovery conflict, because the session's state is gone. R-0699's `on_connect` applies when it reconnects.
  - The README says when a standby does this (notice 4).
- **(b) `ConnectionLost` everywhere,** never retried. Every replayed DDL that conflicts reaches the app as an error, though the transaction is safe to run again.

## SH (EE6). Pragmas that hide R-1080's narrowing

### Evidence

plpgsql_check 2.10.16, extension `2.10`, on all four versions. A function with `v int` reading a `bigint` column, checked with `all_warnings => true`:

| Line in the function | R-1080's finding |
|---|---|
| none | `performance 42804 cast "bigint" value to "integer" type` |
| `PERFORM 'PRAGMA:DISABLE:PERFORMANCE_WARNINGS';` | **gone** |
| the same in lower case, `'pragma:disable:performance_warnings'` | **gone** |
| `PERFORM plpgsql_check_pragma('disable:performance_warnings');` | **gone** |
| `aux int := plpgsql_check_pragma('disable:performance_warnings');` in `DECLARE` | **gone** |
| `PERFORM 'PRAGMA:DISABLE:CHECK';` in a block around the read | **gone** |
| `disable:other_warnings`, `extra_warnings`, `security_warnings` and `compatibility_warnings` | reported |
| `disable:performance_warnings` in an inner block that ends before the read | reported |
| `disable:performance_warnings` then `enable:performance_warnings` | reported |
| `'PRAGMA: disable : performance_warnings'`, with spaces | reported, with `WARNING unsupported pragma` |

The two `plpgsql_check_pragma(...)` forms fail at run time with `42883` in a database without the extension, so R-1066's real call in `_morm_proof` stops them unless the app defines a stub function. The short forms are strings to Postgres and run anywhere.

R-1092's decision already lists every function the walk finds on a column change, so a hidden narrowing still reaches a refusing decision, but without the line, the types and the plain statement that the function fails once a value doesn't fit.

### Options

- **(a) Recommended.**
  - Before checking, as R-1093 does for `@plpgsql_check_options`, morm searches each checked function's `prosrc` for `performance_warnings`, in any case, and a hit stops `makemigrations` with exit code `2`, naming the function, with no file written. It isn't a decision, and a `proof` doesn't get past it. Nothing morm acts on is a `performance` finding apart from R-1080's, so disabling them has no use in a managed function.
  - A function holding `disable:check`, in any case, that R-1076 checks on a change to a column's type, length or precision gets its own line in R-1080's decision, naming the function and saying its check is off in places, so a narrowing there can't be seen. Its default refuses, as R-1080's does.
- **(b) README only:** list `disable:performance_warnings` and `disable:check` in R-1079's loud pragma notice.

## What changes, in outline

No new IDs here: where a new requirement or test obligation is needed, it's named as one, and the building PR gives it the next free ID.

### `contracts.md`

- `Error::SchemaChanged`'s comment: replace PR #3's lines with "recognised by SQLSTATE with the routine, an identifier or an untranslated `elog` message, never translated text (SC)", and add the `22P02` naming a retired type.
- `Error::Serialization`: also a `FATAL 40001` recovery conflict that ended the connection inside a transaction before `COMMIT` (SG). `Error::ConnectionLost`: on a held connection, names that conflict.
- `PoolStats` gains a counter of stale statements retried unnamed and the pool's current marker (`g<n>` or `g<n>.<salt>`).
- Conventions: `_morm` holds the statement generation in its comment, owned by the migrating role; `_morm.###` is used for an instant in each `tx` step; enum labels starting `_morm_` are reserved.
- The internal query entry point's comment: behind a pooler with caches on, cached text carries the marker.
- PR #3's proof 13 comment: drop the Q1 sentence (EA answers it) and record the composite decoder's NULL fields and bounds (EE9), as the fix PR does anyway.

### `proofs.md` (step 55's section, in PR #3)

- Proof 14: "right" becomes "runs on the old type, silently" for shared labels with the old type kept, with SA's table; the relabel-only and reset rows.
- Proof 15: the reset and the no-op `ALTER` rows; `DISCARD PLANS` named against R-0994 (EE7).
- Proof 16: SD's two tables, pool of three, 1.20.1 and 1.22.1 with and without prepared statements; the prepare-time `0A000`.
- Proof 17: SG's table, and that the reset and relabel replay.
- Proof 18: SH's table.
- Q1 to Q4 replaced by the owner's answers to EA and SA to SH.

### Requirements and tests, by step

- **Step 11.** R-0975: unchanged in meaning; add that stale-session errors behind a pooler don't raise the connection generation (SD).
- **Step 12.** R-0697: a held connection ended by a recovery conflict returns `ConnectionLost` naming it (SG). R-1238: each pooler instance runs `default_pool_size` of at least 3, with `min_pool_size` equal and `server_round_robin = 1`, so tests reach several server sessions (EE2's lesson). T-0104 asserts at least two server sessions reached.
- **Step 14.**
  - R-0718: signals from SC; behind a pooler the retry is unnamed and moves the marker; a stale signal at prepare is handled as at execute.
  - R-0981: signals from SC; the `22P02` naming a retired type joins the stale-session errors; behind a pooler no connection generation.
  - R-0982: SF.
  - R-0983: "any other routine" in place of "any other message".
  - A new requirement for the marker (SD, SE): the text, when it's read, the salt, the unnamed retry and its cost.
  - T-0151: the hand-run swap is SA's swap, with the reset and relabel and once without them; add S2's order-sensitive statement and a constant-only one, run with the old type kept; assert each routine on 15 to 18; in German too.
  - T-0152: assert the three `42804` routines and `22P02` naming a retired type, in German too.
  - T-0153: a bad label from a client raises no generation (SF).
  - A new test obligation in the `pooler` job, on 1.20.1, 1.22 and 1.26.1 with a pool of at least 3: SD's tables, a new process after a migration preparing under the new marker with no stale error, hand-run DDL moving to a salt, and a prepare-time `0A000` retried unnamed.
- **Step 20.** R-0984: signals from SC. A new requirement for SG's `Serialization` mapping, with a new test obligation in T-0172's standby job: `.retry(n)` on a `read_only` pool succeeds after a recovery conflict ends its idle transaction, a loss during `COMMIT` stays `CommitUnknown`, and a held connection gets `ConnectionLost` naming the conflict.
- **Step 33.** A new requirement: every `tx` step ends with the operator pair and the generation comment; preflight checks `_morm`'s owner and `CREATE` (SE). Its test: a step's commit invalidates another session's cached plan on 15 to 18, and a step that rolls back sends nothing.
- **Step 37.** R-0994: `DISCARD PLANS` clears some shapes (`%ROWTYPE`, a trigger) and not a variable of the type; the step's reset covers every session; the refresh stays for managed functions (EE7).
- **Step 38.** R-0995: a retired enum type's labels are renamed to `_morm_retired_<label>`; `purge` unchanged; restoring a retired type renames them back. T-0122 or a new test: a stale session gets `22P02` naming the retired type.
- **Step 41.** R-1116 and R-1234: the startup statement also reads the generation when the pool is behind a pooler.
- **Step 51.** R-1012: no change beyond step 33's rule.
- **Step 52.** R-1049: the cutover relabels the old type; R-1050: values stay, shown as placeholders until `purge`. T-0175: an order-sensitive cached statement and a constant-only comparison, from an old binary, give the new answer after cutover, and nothing runs silently on the old type.
- **Step 56.** R-0977: `#[pg_enum(value = "...")]` refuses a label starting `_morm_`.
- **Step 58.** R-1023: restoring rows maps the placeholders back through the labels in `op_data`.
- **Step 59.**
  - R-1039: the no-op `ALTER FUNCTION` for unmanaged functions the role may alter (SB).
  - R-1040: the new text (SB).
  - R-1080 and R-1093: SH.
  - T-0168: the `ALTER` case and the relabel case; T-0171: SD's and SB's PgBouncer cases with a pool of at least 3; T-0172: SG's table, and the reset and relabel on the standby.
  - A new test obligation for SH's table on 15 to 18.
- **Step 50.** The README notices below.

An INDEX Decisions entry names every changed ID, and `coverage.md` and `manifest.json` follow, in the fix PR.

## README notices

Each refuses, silences a check or stops a developer, so each is loud. What each must cover:

1. **Retired enum labels (SA).** After an online swap the retired column holds every old value, but a query on it shows `_morm_retired_<label>` until `purge`; the ledger maps each back, and down and rollback restore the real labels. Labels starting `_morm_` are refused at compile time. Code compiled against the old type fails with `22P02` naming the retired type instead of giving old answers.
2. **Functions morm doesn't manage, and PgBouncer (SB).** morm refreshes the ones the migrating role owns. For the rest, the decision lists them: connected straight to Postgres, morm apps recover by themselves; behind PgBouncer, run `RECONNECT` on each PgBouncer after the migration, because a new client reaches the same server session; other services must reconnect.
3. **A transaction open at the cutover (SA).** Statements in it that take no new lock run as they would have just before the cutover, as with any DDL in Postgres. Keep transactions short around a migration.
4. **Standbys (SG).** Replaying a migration's DDL ends a standby session sitting in a transaction that read an affected table, after `max_standby_streaming_delay`. A transaction gets `Error::Serialization`, which `.retry(n)` retries; a `db.hold()` connection gets `ConnectionLost` and loses its session state. Replay waits up to that delay behind such a session, so the standby lags. Don't sit in a transaction on a `read_only` pool.
5. **Hidden narrowings (SH).** `performance_warnings` in a managed plpgsql function's source stops `makemigrations`; `disable:check` around a read of a changed column opens a decision, because either can hide a value that no longer fits. Joins R-1079's pragma notice.
6. **DDL outside morm, behind PgBouncer (SD).** morm's migrations move every process to fresh statement text. A change made by hand costs each process one failed call, retried, before it moves; inside a transaction it's `SchemaChanged`, which `.retry(n)` retries.
7. **Only under SC (b): English server messages.** Which setting, the `ALTER ROLE` fix, and that the pool refuses to start otherwise.

## What I couldn't run

- **PgBouncer 1.26.1 and 1.19.1:** not built here; the task named 1.20.1 and 1.22. 1.26.1 keeps prepared statements by text as 1.22 does (proof 16 ran it with the same results), and 1.19.1 only matters for T-0300's refusal. The CI `pooler` job runs both.
- **PGDG packages:** Postgres and plpgsql_check were built from source, as above, since I can't install packages here. Same releases' sources, checked by SHA-256 (plpgsql_check's tag tarball has no published checksum; its hash is recorded above).
- **Languages other than German,** and Postgres 19, which morm doesn't support (R-0943 names 15 to 18).
- **SC (b)'s settings** (`ALTER ROLE ... SET lc_messages`, `GRANT SET ON PARAMETER`) weren't run again here; the reviewer ran them on 18.6 and 17.11.- **morm itself:** there's no runtime yet, so the harness stands in for morm's statements. Its shapes follow R-1220, but the retry, marker and recognition logic above are designed here, not run end to end; the new test obligations prove them in their steps.
- **The reset's cost under a realistic workload:** measured only with `pgbench -S`; step 48 measures what it needs.

The scripts, harness and servers were throwaway and are deleted. Every result the design rests on is in this file, and the swap and statements above are enough to run them again.
