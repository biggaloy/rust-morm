# The stale-session state machine: design

For the owner to approve. It replaces the stale-session rule set of PR #3 (step 55, branch `step-55-enum-proofs`, head `0f26ba7`) with one state machine, as INDEX's "Step 55 twelfth review, and one consolidated stale-session design" asks. Nothing here is written into `plan-approved/` until the owner answers; then one fix agent rewrites the requirements from it and one review follows. Base: `origin/main` at the commit that adds this file.

Every owner decision listed in the brief stands. Where the machine can't keep one exactly, the section "Lettered choices" says so and gives a choice with a recommendation.

## In short

Twelve rounds found gaps because every rule did two jobs at once: it repaired the failing call and it decided something for the whole pool, and each new signal got its own mix of the two. The machine separates them.

1. **Every call is repaired locally, always.** A stale statement gets one fresh run of the same call: re-prepared straight to Postgres, unnamed behind PgBouncer. A stale session straight to Postgres gets the call retried on a newly opened connection that takes the old one's slot. Inside a transaction the call returns `Error::SchemaChanged`, and `.retry(n)`'s next attempt gets the fresh statement or the new connection. Nothing about windows, gates or counters changes what one call returns.
2. **Every pool-wide action passes one gate.** There are three: clear every connection's statement caches, add a salt behind PgBouncer (which also clears them), and replace every connection. Any of them is taken at most once per pool per `stale_cooldown`. A request that finds the window closed is remembered and taken at the first checkout after it ends. A migration, which moves `<n>`, lets one action through at once. Client input can reach the gate, but nothing it sends gets more than one action per window.
3. **Straight to Postgres, one pool-wide epoch decides which connections may be stale.** The first stale-session failure starts an epoch at the moment morm reads `<n>`. Until it ends, `stale_cooldown` later, a connection opened before that moment is replaced on its first such failure, and a connection opened after it can't hold code compiled before the change, so its failure is data and is returned as it is, at no cost. That replaces the per-statement cool-down memory and the first-failure records, and it bounds replacements to the pool's `max` per window, whatever the client does.
4. **A pending entry per statement keeps a statement fresh while a confirmation is outstanding.** Behind PgBouncer every run of it goes unnamed; straight to Postgres every connection's copy cached before the entry is prepared again. The run with the same values is the confirming run. Entries are capped and expire.
5. **Behind PgBouncer a stale session is never acted on pool-wide.** Its errors move no marker, open or discard no client connection, and after the first one the statement's errors return at once for `stale_cooldown`. It heals through `RECONNECT` or `server_lifetime`, as EJ5 decided.

What that buys, argued in section 4 and checked by the simulation in section 6:

- churn per pool: at most one pool-wide action per `stale_cooldown`, plus one per migration; straight to Postgres at most `max` stale-driven connection replacements per `stale_cooldown`, plus `max` per migration; behind PgBouncer none;
- healing: a stale statement never reaches a call outside a transaction, nor one under `.retry(n)` with `n` of at least 1; a stale session straight to Postgres stops reaching callers `stale_cooldown` after the change at the latest; behind PgBouncer, at `RECONNECT` or within `server_lifetime`;
- the silent wrong answers are two, both from changes Postgres itself can't flag, listed in section 4 (e).

| Choice | Recommendation |
|---|---|
| **MA** Straight to Postgres, which connections a stale-session failure may replace | (a) one pool-wide epoch, replacing the per-statement cool-down memory and the first-failure records; the replacement connection keeps the slot whatever its run returns; `stale_first_failure_max` goes, `stale_cooldown_max` applies behind PgBouncer only |
| **MB** The one-clear window | (a) one gate for every pool-wide action, `0A000`, `42804` and the connection generation included; a request inside the window is taken when it ends; the window never changes a call's own outcome |
| **MC** `stale_pending_expiry = 0` | (a) refused as a bare number; `"off"` and `"0s"` turn pending entries off (EP6 (a)) |
| **MD** A `search_path` set per transaction over tables of different shapes | (a) the machine handles it at the stated cost, and the README says so beside `tx.set_local` |
| **ME** The simulation's generator | (a) `proptest` as a dev-dependency of `morm`, for shrinking a failing event sequence |

## 1. The signals

Each signal is recognised as SC decided (SQLSTATE with the routine, an identifier or an untranslated `elog` message), by R-1245. The machine sorts them into two classes by what cures them, which is all it needs to know.

| Signal | Recognised by | Class | Cured by | Can client input cause it? |
|---|---|---|---|---|
| `0A000` from a cached statement | routine `RevalidateCachedQuery` | statement | a fresh statement | **Yes**, a per-transaction `search_path` (X2, X3), with no change |
| `22P02` from a cached statement casting a parameter to a non-built-in type, not naming that type (R-1245's sixth row) | routine `enum_in` and the name test | statement | a fresh statement, when it's stale | **Yes**, a client's label converted to another enum, or a row it changed (EL1, EO2) |
| Any other error from a cached statement casting a parameter to a non-built-in type (choice I), outside a transaction | none | statement | a fresh statement, when it's stale | **Yes**, every ordinary error, such as `23505` |
| `XX000` `cache lookup failed for type` from a cached statement | `elog` text | statement first: a fresh run tells it from compiled code | a fresh statement, or a new session if plpgsql | No: it needs DDL |
| `XX000` `cache lookup failed for type` from a fresh run or a non-cached statement | `elog` text | session | a new session | No |
| `42804` | one of plpgsql's three parameter routines | session | a new session, when stale | **Yes**, a record variable fed rows of the caller's choice (X1), with no change |
| `22P02` from `enum_in` holding `_morm_retired_`, no bound parameter holding it | routine and identifier | session | a new session, when stale | **Yes**, stored or transformed text (EG1, EG2, X4) |
| `22P02` holding `_morm_retired_` that a bound parameter holds | the parameter test (EF1) | none: returned as it is | | It's the client's own label |
| `XX000` `could not open relation with OID` | `elog` text | transient: retried once on the same connection outside a transaction, `SchemaChanged` inside one, as R-0981 says | | No |
| A lost connection, SQLSTATE classes `08`, `40`, `53`, `57`, or `55P03` | the connection or the SQLSTATE | not a signal; as a fresh run's outcome it neither confirms nor fails (EJ6) | | A timeout can be |
| A refused open, `53300` included | the open's error | feeds the replacement rule (EJ1) | | Indirectly, through load |
| A migration | `<n>` read from the `_morm` comment (SE) moved | event | | No |
| A schema change made by hand | nothing until one of the errors above | event | | No (it needs DDL) |
| `RECONNECT`, `server_lifetime` | nothing in morm | event: PgBouncer replaces server sessions (X10) | | No |

**A confirmation** is a fresh run of the same text with the same parameter bytes (EH2) that succeeds, with or without rows (EK4), or ends in a server error other than the first one and outside the no-confirm list (EI4, EJ6), where the cached statement or the old connection failed. `0A000`, `42804` and an `XX000` from a fresh run count as confirmed when they arrive on a connection the machine would replace, because a fresh statement or session cures them for certain (SF's certain signals, kept). No other signal raises anything without a confirmation.

## 2. The state

Per pool. Every item is stated once here, with its bound and what clears it.

| State | What it is | Bound | Set or cleared by |
|---|---|---|---|
| Marker `g<n>` or `g<n>.<salt>` | behind PgBouncer with the caches on, the comment starting each cached text (SD, SE) | one value | `<n>` read at pool start (R-1116, or its own query); adopted when a read finds it moved (a migration), which drops the salt; a salt added by the gate's "salt" action, replacing any earlier salt |
| Statement generation | a counter; a connection behind it clears both statement caches at checkout (R-0982) | one word | raised by the gate's "clear" and "salt" actions and by adopting a moved `<n>` |
| Connection generation | a counter; a connection behind it is replaced at checkout (R-0975) | one word, the same atomic word | raised by the gate's "replace" action, straight to Postgres only |
| The window | the time of the last pool-wide action and the `<n>` it was taken under, and one "due" action | O(1); "due" is one bit pair in the generation word | an action starts it; it ends `stale_cooldown` later; a request inside it sets "due", and the first checkout after it ends takes that action |
| The epoch (straight to Postgres) | start `T`, the `<n>` read at `T`, end `T + stale_cooldown`, and "suspended" | O(1) | started by a stale-session failure when there's no epoch, it has ended, or the `<n>` just read differs from its own; suspended for the rest of its life when a replacement connection can't be opened |
| Pending entries | per statement text, by a 64-bit hash: the hash of the confirming parameter bytes, a horizon, and an expiry | at most `stale_pending_max`, oldest dropped | created by a statement-class signal that hasn't been acted on pool-wide; a later signal of the same statement moves its horizon and confirming bytes, never its expiry; removed when it expires (`stale_pending_expiry` after creation), when its confirming run fails, and with every entry when a pool-wide action is taken or `<n>` moves |
| Cool-down (behind PgBouncer) | per statement text, by a 64-bit hash: an expiry | at most `stale_cooldown_max`, oldest dropped | added by a stale-session failure that got `SchemaChanged` or a failed retry; removed `stale_cooldown` later, and with every entry when a pool-wide action is taken or `<n>` moves |

Per connection: the time it was opened, the two generations it last saw, and for each cached statement the sequence number at which it was prepared (8 bytes), which the pending entry's horizon is compared with.

**What's gone:** the first-failure records and their setting `stale_first_failure_max`, and the straight-to-Postgres cool-down memory (MA). Both were per-statement tables that a client could fill, and both decided whether to replace connections; the epoch decides that with no table.

## 3. The table

The procedures the rows name:

- **Fresh run:** straight to Postgres, evict the statement on this connection, prepare it again and run it; behind PgBouncer, run it once unnamed through `query_typed`, which PgBouncer never shares.
- **Request** *clear*, *salt* or *replace*: ask the gate (section 2's window). Open, or with `<n>` moved since the last action: taken now. Closed: remembered as due, and taken at the first checkout after the window ends. A due *replace* outranks a due *clear*.
- **Old connection** (straight): morm reads `<n>` on the failing connection, one catalog read, and starts an epoch if there is none, it has ended, or `<n>` differs from the epoch's. The connection is old when it was opened before the epoch's `T` and the epoch isn't suspended.
- **Replace** (straight, outside a transaction): open a new connection as a momentary extra within the process's budget; with the budget full, or on `53300`, close the old connection first and open in its slot (EG3, EJ1). Run the call on the new connection as a fresh run. The new connection takes the old one's slot and the old one is closed, whatever that run returns, unless the new connection is lost. If it can't be opened, the caller gets the original error, the old connection keeps its slot if it's still open, and the epoch is suspended.
- **Discard** (straight, inside a transaction): after the rollback, close the connection; `.retry(n)`'s next attempt opens a new connection in that slot through the pool's ordinary open, with its backoff (EG1). If that open fails, `.retry(n)` returns its error and the epoch is suspended.
- **Entry:** create the statement's pending entry, or refresh it: horizon to now, confirming bytes to this call's, expiry unchanged. Nothing if pending entries are off.

"As it is" means `Error::Postgres` with the server's error, nothing else done. Every row costs what it says in addition to the failed call itself.

### Before any row

| Signal | Every mode and context |
|---|---|
| `22P02` whose bound parameter holds `_morm_retired_` | As it is. Costs nothing (EF1) |
| `XX000` `could not open relation with OID` | Outside a transaction, retried once on the same connection, held or not; inside one, `SchemaChanged`. Nothing raised (R-0981's transient case) |
| A statement whose pending entry exists | Straight to Postgres, a copy cached before the entry's horizon is prepared again first; behind PgBouncer the run is unnamed. If its bytes are the entry's, it's the confirming run: a confirmation requests *clear* (straight) or *salt* (pooled); the same error removes the entry and goes on below as from a fresh run; a no-confirm outcome removes the entry and returns as it is |

### A. Straight to Postgres, outside a transaction (a held connection too)

| # | Signal | morm does | Caller gets | May raise | Cost |
|---|---|---|---|---|---|
| A1 | Statement class, the `XX000` from a cached statement included | Fresh run | The fresh run's result | A confirmation, or `0A000`, requests *clear*. An `XX000` that comes back from the fresh run goes to A2 | One prepare and one run; one catalog read if the gate is closed |
| A2 | Session class, on an old connection | Replace | The new connection's run | A confirmation, or `42804` or `XX000`, requests *replace* | One catalog read, one connection, one run |
| A3 | Session class, on a connection opened since `T`, or the epoch suspended | Nothing | As it is | Nothing | One catalog read |
| A4 | Session class on a `db.hold()` connection | `SchemaChanged`; an old connection is replaced on release, counted as the epoch's replacement | `SchemaChanged`, or as it is if not old | As A2 | One catalog read; one connection on release |

### B. Straight to Postgres, inside a transaction

`.retry(n)`'s next attempt runs as a new call; without `.retry(n)` the next call does.

| # | Signal | morm does | Caller gets | May raise | Cost |
|---|---|---|---|---|---|
| B1 | `0A000` or a sixth-row `22P02` | Evict the statement on that connection; after the rollback, entry | `SchemaChanged` | `0A000` requests *clear*; the confirming run's confirmation requests *clear* | One evict; the entry |
| B2 | `XX000` from a cached statement | As B1, and if the connection is old, discard it too, which cures compiled code as well | `SchemaChanged` | The confirming run's confirmation requests *replace* | One catalog read; one connection if old |
| B3 | Session class, on an old connection | Discard | `SchemaChanged` | `42804` and `XX000` request *replace*; so does the next attempt's run of the same text and bytes on the new connection, if it confirms | One catalog read; one connection |
| B4 | Session class, on a connection opened since `T`, or the epoch suspended | Nothing | As it is, so `.retry(n)` stops | Nothing | One catalog read |
| B5 | Any other error from a cached statement casting a parameter to a non-built-in type | Nothing | As it is (R-0984) | Nothing | None |

### D. Behind PgBouncer, outside a transaction

| # | Signal | morm does | Caller gets | May raise | Cost |
|---|---|---|---|---|---|
| D1 | Statement class, the `XX000` from a cached statement included | Fresh run, unnamed; on a confirmation or `0A000`, read `<n>` | The unnamed run's result | A moved `<n>` is adopted, not gated; otherwise *salt* is requested. An `XX000` from the unnamed run goes to D2 | One unnamed run; one catalog read on a confirmation |
| D2 | Session class | If the statement is in the cool-down, nothing; otherwise retry once on another client connection, and if that fails the same way, the statement enters the cool-down | The retry's result, or as it is | Nothing, ever (EK1, EL2) | One run; under light load it usually reaches the same server session (X7) |

### E. Behind PgBouncer, inside a transaction

| # | Signal | morm does | Caller gets | May raise | Cost |
|---|---|---|---|---|---|
| E1 | Statement class, the `XX000` from a cached statement included | After the rollback, read `<n>`: if it moved, adopt it; otherwise entry, so every run of the statement goes unnamed while it lasts | `SchemaChanged` | `0A000` requests *salt*; the confirming run's confirmation requests *salt*; an `XX000` from the confirming run goes to E2 | One catalog read; unnamed runs while the entry lasts |
| E2 | Session class | If the statement is in the cool-down, nothing; otherwise the statement enters it | As it is in the cool-down; otherwise `SchemaChanged` | Nothing, ever | None |
| E3 | Any other error from a cached statement casting to a non-built-in type | Nothing | As it is | Nothing | None |

Behind PgBouncer with the caches off, as on 1.20.1 (R-0677), no statement is cached, so only D2 and E2 occur. `db.hold()` isn't available there (R-0673).

### Pending entries off

With `stale_pending_expiry = "off"` or `stale_pending_max = 0` no entry is kept. B1 and B2 still evict, so straight to Postgres each connection's copy fails at most once. Behind PgBouncer, E1 returns `SchemaChanged` on every call of the statement inside transactions, including `.retry(n)`'s attempts, until a call outside a transaction confirms it (D1), `RECONNECT`, or `server_lifetime`; and while an old type is kept, a value both types share can run on the old order meanwhile (section 4 (e)).

### F. Events

| # | Event | What changes |
|---|---|---|
| F1 | A migration commits a `tx` step | The reset and the raised generation (R-1248), the relabel (SA), the refreshes (SB). The pool learns of it at its next read of `<n>`: behind PgBouncer it adopts the marker, which drops the salt, raises the statement generation and clears every entry and cool-down entry; straight to Postgres it clears every entry, its next stale-session failure starts a new epoch, and its next request passes a closed window once. Nothing is gated, because migrations pace it. `migrate` sends `RECONNECT` where admin URLs are listed (R-1256) |
| F2 | A schema change made by hand | Nothing until a signal |
| F3 | `RECONNECT` or `server_lifetime` | PgBouncer replaces each server connection as it's released, which cures stale plpgsql and stale server statements alike (X10). No morm state changes: cool-down entries run out by themselves |
| F4 | The window ends with an action due | The first checkout after it takes the action |
| F5 | A pool-wide action is taken | *clear*: the statement generation rises. *salt*: a new salt replaces any earlier one and the statement generation rises. *replace*: the connection generation rises, so every connection is replaced at checkout, which clears its caches too. Every pending and cool-down entry is removed and a window starts. The epoch is kept |
| F6 | A checkout | One atomic read of the generation word, as now: a connection behind the connection generation is replaced, one behind the statement generation clears both caches, and a due action whose window has ended is taken |
| F7 | Time | An entry expires `stale_pending_expiry` after it was created, a cool-down entry `stale_cooldown` after, the epoch `stale_cooldown` after `T`, the window `stale_cooldown` after its action |
| F8 | A recovery conflict on a standby | Unchanged: `Serialization` inside a transaction, `CommitUnknown` during `COMMIT`, `ConnectionLost` on a held connection (SG). As a fresh run's outcome it's on the no-confirm list |

### The same, as code

For the building step, the decision procedure in one place. It's pure: no I/O, so the simulation drives the real thing (section 6).

```text
on_error(call, conn, err, ctx):
    if param_holds_retired(call): return AsIs
    if err is transient XX000:     return ctx.out ? RetrySameConn : SchemaChanged
    cls = class(err, cached = !call.fresh)        # section 1
    if cls == Statement:
        if ctx.out:
            r = fresh_run(call)
            if confirms(r, err) or err == 0A000: request(pooled ? Salt : Clear, read_n_if_needed())
            if err is XX000 and r is XX000: return session(call, conn, r, ctx)
            return r
        else:
            if !pooled: evict(conn, call.stmt)
            n = read_n(conn); if pooled and n moved: adopt(n) else:
                if err == 0A000: request(pooled ? Salt : Clear, n)
                entry(call.stmt, call.bytes)
            if !pooled and err is XX000 and old(conn, n): discard(conn); expect_confirm(call, Replace)
            return SchemaChanged
    if cls == Session: return session(call, conn, err, ctx)
    return AsIs

session(call, conn, err, ctx):
    if pooled:
        remove_entry(call.stmt)
        if in_cooldown(call.stmt): return AsIs
        if ctx.out: r = run_on_other_conn(call); if r fails the same way: cooldown(call.stmt); return r
        cooldown(call.stmt); return SchemaChanged
    n = read_n(conn)
    if !old(conn, n): return AsIs                  # opened since T, or the epoch suspended
    if err in {42804, XX000}: request(Replace, n)
    if ctx.out: r = replace(conn, call); if confirms(r, err): request(Replace, n); return r
    discard(conn); expect_confirm(call, Replace); return SchemaChanged

request(action, n):
    if window_open() or n != window.n: take(max(action, due)); else due = max(due, action)
```

## What was tested

Everything ran on throwaway servers under `.claude/scratch/` in my worktree, with `TMPDIR` set there, sockets under `~/.claude/scratch/`, all stopped and deleted afterwards.

- **Postgres 17.12**, built from ftp.postgresql.org's `postgresql-17.12.tar.bz2`, checked against its published `.sha256` (`9cf136a703d8ee2f83465c43be4e9e8e7930c82b0bfba13be7e2537be769ac55`).
- **Postgres 18.6**, the machine's own build (Manjaro package `18.6-1`), run as a throwaway cluster made with its `initdb`. Not built from a tarball.
- **PgBouncer 1.20.1** (`24992cf557d73426d7048698dffc7b019e6364d4d8757ae2cf5e2471286a2088`, the hash R-1238 pins) and **1.26.1** (`00886a2994b4844cd37110970df777772cfc42a74c400522c82fba509a3f42d9`), built from pgbouncer.org's tarballs, each checked against its `.sha256`. One of each in front of each Postgres, four in all: `pool_mode = transaction`, `default_pool_size = 3`, `min_pool_size = 3`, `server_round_robin = 1`; 1.26.1 with `max_prepared_statements = 200`, 1.20.1 with none, so morm would run there with its caches off (R-0677).
- **Clients:** `psql`, and a throwaway harness on `tokio-postgres` 0.7.18 (built offline from the local registry) for named and unnamed statements, several clients and concurrent transactions. Server sessions were told apart by `pg_backend_pid()`.
- **The simulation prototype:** a scratch crate on stable Rust 1.99.1 with no dependencies (section 6).

Unless a row says otherwise, 17.12 and 18.6 printed the same, apart from `pl_exec.c` and `plancache.c` line numbers.

| # | Claim the design rests on | Result |
|---|---|---|
| X1 | A `42804` from plpgsql's parameter routines needs no schema change: a client can cause it | **Yes.** `pick(tbl text)` does `EXECUTE format('SELECT * FROM e1.%I ...', tbl) INTO r; RETURN (r.v + 1)::text`, where table `a`'s `v` is `int` and `b`'s is `bigint`. In one session: `pick('a')` → `11`; `pick('b')` → `42804 type of parameter 4 (bigint) does not match that when preparing the plan (integer)`, routine `plpgsql_param_eval_recfield`; `pick('a')` → `11`; `pick('b')` → `42804` again. A `FOR r IN EXECUTE` loop gives the same. No DDL ran. So R-0981's "certain signal" can be paced by the caller's arguments, and under PR #3's text each such call straight to Postgres raises the connection generation, ungated, and replaces the whole pool (new finding N1) |
| X2 | A `0A000` from `RevalidateCachedQuery` needs no schema change, and fresh text doesn't stop it | **Yes.** With `search_path = t1`, `PREPARE s1 AS SELECT * FROM items WHERE id = $1`, then in a transaction `set_config('search_path', 't2', true)` and `EXECUTE s1(1)`: `0A000 cached plan must not change result type`. A new statement prepared under `t2` then fails the same way under `t1`. Unnamed runs answer under either path (EP2's spot check, on 17.12 too) |
| X3 | Behind PgBouncer the same `0A000` reaches other clients and arrives at prepare | **Yes, 1.26.1 on both.** A cached text prepared under `p7a` gives `0A000` executed under `p7b`; a client *preparing* that text, or a salted copy, under the other path gets `prepare 0A000`, because PgBouncer hands it the server statement another transaction planned. Another client preparing the original text under `p7b` gets the same |
| X4 | A `22P02` that names no retired type, from stored text, behaves as R-0981's stale one | **Yes.** A function converting a stored `_morm_retired_x` to enum `s1.st`: `22P02 invalid input value for enum s1.st: "_morm_retired_x"`, routine `enum_in`, on an old connection and on a newly opened one alike; after `UPDATE ... SET t = 'b'` the same bytes succeed on both (EP1's premise) |
| X5 | A connection opened after a change compiles plpgsql fresh, so connection age splits stale sessions from data | **Yes.** After a hand-made relabel swap: a connection that called `f` before it gets `22P02 invalid input value for enum s2._morm_retired_m1_st: "a"` on every call; one that was open but had never called `f` answers right; one opened after answers right. Age is conservative: it never calls a stale session fresh, and may call a fresh one old |
| X6 | Closing one connection lets the next open pass a role's `CONNECTION LIMIT` | **Yes.** Limit 2: the third open gets `53300 too many connections for role "s3r"` (routine `InitializeSessionUserId`); after closing one, the open succeeds (EJ1's premise) |
| X7 | Which server session a transaction reaches, behind PgBouncer | **Sequential transactions from one client reached one server session, 30 of 30**, on 1.20.1 and 1.26.1 in front of both, even with `server_round_robin = 1`. Three clients with overlapping transactions reached all three sessions, 10 each. So a retry right after a failure usually reaches the same session under light load, and another one under load |
| X8 | Plpgsql left stale in one server session: other sessions answer, unnamed | **Yes, on all four pairs.** One call compiled `f` in one session, then a hand-made relabel swap; ten rounds of three overlapping transactions, each an unnamed call: the session that compiled `f` gave `22P02 ... p1.st_old` 10 of 10, the other two answered right 10 of 10 each. So an unnamed "confirming" run can succeed on another session while the failing one stays stale (EO1's premise) |
| X9 | Behind PgBouncer a stale server statement is cured by new text, and new text can go stale again | **Yes, 1.26.1 on both.** Six clients cached `/* morm g1 */ SELECT (($1::text)::p2.st)::text` on all three sessions; after a hand-made drop of the type: named `XX000 cache lookup failed for type ...` (routine `getTypeInputInfo`) 6 of 6; unnamed right 6 of 6; salted text `g1.s1` right 6 of 6. A second hand-made drop: `g1.s1` named `XX000` 6 of 6, unnamed right, a new salt `g1.s2` right |
| X10 | `RECONNECT` heals both kinds of stale state behind PgBouncer | **Yes.** On 1.26.1, after X9 the three server pids were replaced, and new and old clients preparing the stale `g1.s1` text were right 18 of 18 and 6 of 6. On 1.20.1 and 1.26.1, the session X8 left stale gave `22P02` 5 of 5 before `RECONNECT`; afterwards three new pids answered right 15 of 15 |
| X11 | PgBouncer bounds the server statements that salts leave | **Yes.** 300 distinct texts prepared and run through one 1.26.1 client: each of three server sessions held exactly 200 in `pg_prepared_statements`, the `max_prepared_statements` value |

### What I couldn't run

- **PgBouncer 1.22**, which the CI `pooler` job also runs. It keeps prepared statements by text as 1.26.1 does (proof 16 ran 1.22.0 and 1.22.1 with the same results).
- **Postgres 15 and 16.** X1 to X6 rest on `pl_exec.c`, `plancache.c` and `enum_in` behaviour the earlier design recorded on 15 to 18; the building steps' tests run them on every version.
- **A Postgres 18 built from a tarball:** 18.6 is the distribution's build.
- **morm itself.** There's no runtime yet. The machine was run only as the prototype in section 6, against a model of Postgres and PgBouncer that the rows above calibrate.
