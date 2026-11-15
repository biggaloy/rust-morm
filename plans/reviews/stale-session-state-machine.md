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

## 4. Guarantees, argued

Notation: `W` is `stale_cooldown` (60 s by default), `E` is `stale_pending_expiry` (60 s), `L` is PgBouncer's `server_lifetime` (3600 s by default), and `max` is the pool's `max`. A change is a schema change committed at time `t0`, by a migration or by hand. Something is stale when it was prepared or compiled before `t0` and fails or answers wrongly after it. An error is visible when it reaches the caller and a fresh session with a fresh statement wouldn't return it. Every bound is per pool: a process with several pools on one database, such as `[pools.jobs]`, has each pool's (EP7).

### (a) Every stale session heals within a stated time

**H1. Straight to Postgres, stale statements.**

- Outside a transaction none is ever visible: A1 answers the call with a fresh run.
- Inside one, a connection's stale copy fails at most once, because B1 evicts it. While the statement's entry lasts, every other connection's copy cached before the entry is prepared again before it runs. So with `.retry(n)`, `n` at least 1, none is visible, and without it at most one `SchemaChanged` reaches a caller per stale connection, `max` in all.
- Every connection's stale statements, of every text, are cleared within `W` of the first confirmation or `0A000`: the request is taken at once, or at the first checkout after the window ends.

**H2. Straight to Postgres, stale sessions.** From `t0 + W` on, a stale session is never visible outside a transaction, nor inside one under `.retry(n)` with `n` at least 1; without `.retry(n)`, each connection holding one fails one transaction and is replaced.

- Why `t0 + W`: a connection holding code compiled before `t0` was opened before `t0`. Any epoch in force after `t0 + W` started after `t0`, because an epoch lasts `W`. Its `T` is then after `t0`, so every such connection is old in it, and its first failure replaces it (A2, B3). A new epoch starts at the first failure after the last one ends, so there is always one to apply.
- Before `t0 + W`, the only visible case is a connection opened between an earlier epoch's `T` and `t0`: its failure returns as it is until that epoch ends.
- A connection that is replaced compiles everything fresh (X5), so it fails no more for that change. The first confirmation, or `42804` or `XX000` on an old connection, also requests *replace*, which replaces the rest within `W`.
- **When Postgres refuses connections,** a failed replacement suspends the epoch, and A3 and B4 return the error as it is until it ends. Healing then comes within `W` of the first moment Postgres accepts a connection again, as EI5 and EJ1 already accepted for a refused open. Closing the old connection first on `53300` or a full budget means one server slot is enough (X6).

**H3. Behind PgBouncer, stale statements.**

- Outside a transaction none is ever visible: D1 answers with an unnamed run.
- Inside one, the first call per process returns `SchemaChanged`. While its entry lasts every run of that statement is unnamed and right, so with `.retry(n)`, `n` at least 1, nothing is visible.
- The salt comes within `W` of the first confirmation or `0A000`.
- Without any confirmation, a statement run only inside transactions, with no `.retry(n)` and different values each time, returns `SchemaChanged` once per `E` per process. It heals at the first call outside a transaction, at `RECONNECT`, which also clears stale server statements (X10), or within `L`. R-1253 states this case.

**H4. Behind PgBouncer, stale sessions.** morm can't reach a server session, so it heals when PgBouncer replaces it: at once after `RECONNECT`, which `migrate` sends after every migration where admin URLs are listed (R-1256, X10), and otherwise within `L`. Meanwhile calls that reach another server session are answered (X8), and morm adds nothing pool-wide.

These hold whatever clients send, because a client can only start an epoch early or take the window, and both still heal: every replacement it causes replaces an old connection with a fresh one, and every pool-wide action it causes clears or replaces everything stale. No table a client can fill decides whether a stale session is healed.

**Two conditions, both stated in the README:**

- H1's and H3's "nothing visible under `.retry(n)`" needs pending entries on, and no more than `stale_pending_max` statements pending at once. An entry pushed out only costs that statement one more `SchemaChanged`.
- The bound for a stale statement that no call ever errors on is the silent case in (e).

### (b) Churn is bounded, whatever clients send

**C1. Pool-wide actions:** at most one per `W`, plus one per migration.

- `request` takes an action only when the window is open or `<n>` moved since the last action, and a due action waits for the window's end.
- So two actions less than `W` apart have a migration between them.
- An action clears every connection's statement caches, or behind PgBouncer adds a salt, or straight to Postgres replaces up to `max` connections at checkout.

**C2. Straight to Postgres, stale-driven replacements:** at most `max` per `W`, plus `max` per migration.

- A new epoch starts only after the last one ended, or when `<n>` moved.
- Inside an epoch only connections opened before `T` are replaced, each at most once, because its replacement is opened after `T`. There are at most `max` of them.
- A failed open suspends the epoch, so at most two opens fail per epoch: one beside the old connection, one in its slot on `53300`.

**C3. New connections from stale handling,** straight to Postgres: C2's replacements plus C1's *replace* actions, at most `2 × max` per `W`, plus `2 × max` per migration.

**C4. Behind PgBouncer:**

- stale handling never opens or discards a client connection;
- at most one salt per `W` plus one adoption per migration;
- the server statements salts leave behind are bounded by PgBouncer's `max_prepared_statements` per server connection (X11).

**C5. Per call:**

- outside a transaction, at most one fresh run, one catalog read and, behind PgBouncer for a stale session, one retry on another connection;
- inside one, at most one catalog read after the rollback;
- while a statement has an entry, each of its runs is unnamed behind PgBouncer, or prepared again once per connection straight. A client's label can create the entry, so it can keep a statement running unnamed for every caller, one entry per `E` (EN1, EO7). Each entry's life is capped at `E`: a later signal moves its horizon, never its expiry.

**Why no input escapes these:**

- the only things that decide a pool-wide action or a replacement are the window, the epoch and a connection's age, and none is keyed by anything a client chooses;
- tables a client can fill, the pending entries keyed by statement and the cool-down, only change per-call outcomes, and are capped;
- values, stored text, a row changed between two runs (EO2, EP1), the timing of `.retry(n)`'s backoff, a `search_path` per transaction (EP2, X2, X3) and a record variable's shape (X1) can each produce signals at the client's rate, but each signal reaches the same gate and epoch.

### (c) No growth past `max` or the budget

- The slot rules are R-0981's: the replacement connection is a momentary extra within the process's budget, and with the budget full, or on `53300`, the old connection is closed first and the new one opens in its slot.
- `max` bounds the slots; the budget, which every pool in the process shares, bounds connections.
- A replacement takes the old connection's slot whatever its run returns, so the pool's size is unchanged, except after a failed open once the old one was closed, which leaves the slot to the pool's next checkout (EI6).
- Behind PgBouncer no client connection is added.

### (d) Bounded memory

- Pending entries: at most `stale_pending_max` per pool, each about 32 bytes, keyed by statement text, so client values add none.
- Cool-down entries: at most `stale_cooldown_max`, each about 16 bytes.
- The epoch, the window and the due action: a few words per pool.
- Each cached statement: an 8-byte prepare sequence.
- The ceilings of 4096 (EO4) cap the tables at about 200 KB per pool.
- Server side, PgBouncer bounds the salts' statements (X11).

### (e) Silent wrong answers, exactly

Two, both from changes Postgres itself gives no error for. The README states both.

1. **A transaction open across a cutover,** morm's or anyone's: statements in it that take no new lock see the catalog as it was before (SA's notice 3, unchanged).
2. **A type swap made outside morm that keeps the old type without renaming its labels.**
   - A cached statement, or plpgsql compiled before the swap, that compares or converts a label both types share answers by the old type's order, with no error.
   - It ends for a statement when the statement is prepared again: within `W` of the pool's first confirmation or `0A000` after the swap, when its own stale error is repaired, or when its connection is replaced.
   - For plpgsql it ends when the session ends: straight to Postgres, the connection's replacement; behind PgBouncer, `RECONNECT` or `L`.
   - If nothing in the pool ever errors, it lasts until the connection or server session is replaced.
   - Behind PgBouncer, while the statement has an entry, its runs are unnamed and right. Between entries, and always with pending entries off, the old order can come back.

morm's own migrations leave neither of these, apart from the first: the reset re-analyses every cached plan and the relabel makes compiled code fail loudly (SA).

## 5. The gaps, closed

### EP1 to EP8

| Finding | Closed by |
|---|---|
| EP1: straight to Postgres, a row changed during `.retry(n)`'s backoff makes the confirming attempt replace the whole pool every round | The connection generation is raised only through the gate (C1), at most once per `W`. The epoch, not a per-statement record, decides replacements (C2), and an action doesn't reset it, so the round after a confirmation finds every connection opened since `T` and returns the error as it is (B4). The cycle costs at most `max` replacements and one *replace* per `W` |
| EP2: a per-transaction `search_path` makes `0A000` clear the caches on every switch | `0A000`'s request goes through the gate like every other (MB). The call is still repaired locally, and inside a transaction the entry keeps the statement fresh, so a switch costs one `SchemaChanged` per `E`. X2 and X3 add that fresh text can't cure a switch and that behind PgBouncer it arrives at prepare and in other clients; MD's notice says what a schema-per-tenant app pays |
| EP3: inside the window, R-1246 and R-0982 describe different calls; `0A000` behind PgBouncer reads both ways | The window never changes a call's outcome: inside or outside it, a statement-class signal gets its fresh run and the caller its result. Only the pool-wide action waits. `0A000` is gated like the rest, and a new salt replaces the old one (F5) |
| EP4: a second hand-made change within the window fails calls for up to `2 × stale_cooldown`, unstated | Gone with the in-window "treat as plpgsql" path. A second change inside the window costs what the first did: outside a transaction nothing visible, inside one a `SchemaChanged` and then fresh runs, and the pool-wide action comes at the window's end (H1, H3) |
| EP5: no test shows the window reopening or a second change healing | The deferred action makes "the window reopens" an observable event: a request inside it is taken at the first checkout after it ends. T-0306 and T-0312 assert it with `stale_cooldown = "1s"`, and the simulation asserts the actions' spacing (section 6) |
| EP6: `stale_pending_expiry = 0` is both off and refused | MC: `0` and `"0"` are bare numbers, refused; `"off"` and `"0s"` turn entries off |
| EP7: per pool or per process; `<n>` straight to Postgres; the setting's text | Every bound is per pool (section 4). Straight to Postgres, `<n>` is read on a stale-session failure (the epoch) and on a request while the window is closed, and nowhere else. R-0060's comment and the use-when line name the window, the epoch and the cool-down |
| EP8: R-1268's small-values line overstates the cost behind PgBouncer | `stale_cooldown_max` now governs only the cool-down behind PgBouncer, where a value too small costs a retry on another connection outside a transaction or a `SchemaChanged` inside one, per call, and never a connection. R-1268 says that |

### New gaps this design found and closes

| # | Gap in PR #3's text | Evidence | Closed by |
|---|---|---|---|
| N1 | Straight to Postgres, `42804` is a certain signal that raises the connection generation at once, ungated, and a caller choosing a record's shape causes it with no change, so it can replace the whole pool on every call | X1 | `42804` reaches the gate (C1) and the epoch (C2); on a connection opened since `T` it's returned as it is |
| N2 | Behind PgBouncer, a `0A000` from a per-transaction `search_path` arrives at prepare and in other clients, and no salt can cure it | X3 | Gated (MB); the entry keeps the statement unnamed while it lasts; MD's notice |
| N3 | Straight to Postgres, an `XX000` from a plain cached statement went straight to connection replacement, which the epoch would refuse on a newer connection, leaving a stale statement failing | the prototype | A1 and B2: every `XX000` from a cached statement gets a fresh run first; inside a transaction an old connection is also discarded |
| N4 | A second statement-class signal while an entry lasts left copies cached between the two signals stale, so `.retry(n)` could fail twice | the prototype | A later signal moves the entry's horizon and confirming bytes, not its expiry (section 2) |
| N5 | Behind PgBouncer, "retry on another connection" was described as reaching a fresh server session | X7 | The cost is stated as it is: sequential transactions usually reach the same session; under load another |

### Every earlier stale-session gap

The machine replaces the rules these findings patched, so each is closed by a structural rule rather than by its own sentence:

| Finding | Where the machine closes it |
|---|---|
| EE1, EE4, EE5, EE11, EF5 | Kept as designed: reset and relabel (SA), the shared marker read at pool start (SD, SE), the refresh (SB), prepare-time signals handled as execute-time ones (F1, D1, E1) |
| EF1, EG2 | The parameter test, before any row; stored or transformed text costs at most C2's replacements and nothing else |
| EG1, EH1, EI1, EJ2, EJ7 | The epoch: in a transaction only an old connection is discarded; a connection opened since `T` returns the error as it is, so `.retry(n)` stops; `<n>` moved starts a new epoch, so a later migration's stale sessions are never taken for data; `T` is the time `<n>` was read, so a migration that commits between the failure and the read is inside the epoch |
| EG3, EH5, EI6, EJ1 | Section 3's replace: momentary extra, close first on a full budget or `53300`, slot kept, a failed open suspends the epoch so it isn't retried every call |
| EH2, EM4 | A confirmation needs the same text and bytes |
| EI4, EJ6, EK4 | Section 1's definition of a confirmation and the no-confirm list |
| EI5 | H2's refused-connection clause |
| EI7 | Discard: `.retry(n)`'s attempt opens through the pool's ordinary open, and its failure is returned |
| EJ5, EK1, EL2, EM2 | Behind PgBouncer a stale session is never acted on pool-wide; the cool-down makes its errors return at once |
| EL1, EM1, EM3, EN1, EN2, EN4, EO7 | Pending entries, one per statement: the confirming run, unnamed or re-prepared runs while it lasts, removed on a failed confirmation or any pool-wide action, and an `XX000` from a fresh run going to the session rows |
| EN3, EO8 | The pending check stays one relaxed atomic read per cached run while no entry exists (R-1269) |
| EN5, EO4, EO6, EO10 | Entries expire after `E`; settings keep their ceilings; tests set the caps and expiry (section 7) |
| EO1, EO2 | The gate, now covering every pool-wide action, with the window never changing a call's outcome |
| EO3 | MA removes `stale_first_failure_max`; `stale_cooldown_max` keeps its floor of 1 behind PgBouncer, where it costs only per-call work |
| EO5 | "Pending entries off" in section 3 |
| EO9 | Section 7's notices state each setting by name |

## 6. The randomised simulation test

The guarantees are about sequences: twelve reviews found sequences of ordinary calls, row changes and retries that the rules handled badly one step at a time. So the test drives the machine with random sequences against a model and checks every guarantee after every event.

### How the building step shapes it

- **The machine is one pure module** in `morm`, such as `stale.rs`: the pool's state (section 2) and functions that take a signal, the call's context and the clock, and return a decision: answer with a fresh run, replace, discard, return `SchemaChanged`, return as it is, request an action. The pool and connection code carries the decisions out. The simulation drives the same module, with the model carrying them out, so it tests the code that ships, not a copy.
- **Which step:** step 14 writes the module with rows A, D and F; step 20 adds rows B and E and the simulation, as a new test obligation, because step 20 is the first step with the pool (11), the pooler mode (12), the caches (14) and transactions (20) all in place.
- **How it runs in CI:** a unit test in `crates/morm`, so it runs in `ci`'s workspace test job (`cargo test --locked --workspace --exclude morm-tests --all-targets`), with no Postgres: 256 sequences of 2,000 events, a few seconds. `nightly`'s `cargo test --locked --workspace` runs it with `PROPTEST_CASES=20000`. A failure shrinks to a shortest failing sequence (ME), saved under `crates/morm/proptest-regressions/` and committed, so it reruns on every build.

### The model

- **Postgres:** the database generation `<n>`; versions for the enum type, a column's type and the plpgsql functions; whether the last hand-made swap dropped the type or kept it without a relabel; stored text, good or holding `_morm_retired_`; and whether new connections are refused, by a role limit at the pool's size or entirely.
- **Sessions:** each compiles a function at its first call and keeps that version; remembers the first record shape a function saw (X1); and holds its prepared statements with the versions and `search_path` they were planned under.
- **Straight to Postgres** a connection is its own session. **Behind PgBouncer** there are `k` server sessions holding prepared statements by text, each transaction reaching the client connection's last session or, under load, another (X7); sessions are replaced after `server_lifetime`, and all at once by `RECONNECT`.
- **Statements:** a cached enum cast that touches no table (`SELECT (($1::text)::st)::text`); a statement whose result type a column change alters (`0A000`); the same under a per-transaction `search_path` (X2); a plpgsql `DECLARE v st` function; a function converting stored text (X4); and a record-shape function (X1).
- **Events:** calls outside a transaction and in `.retry(0)` to `.retry(2)` transactions, whose attempts may bind other values; client labels, good, bad and holding `_morm_retired_`; stored text toggled at the client's pace, also between `.retry(n)`'s attempts (EP1); `search_path` and record-shape choices; hand-made swaps (drop, keep, column change); migrations (the reset re-analyses every plan, a column may change, functions are refreshed or not, `RECONNECT` sent or not); refused opens; fresh runs ending in a timeout (the no-confirm list).
- **The oracle:** what a fresh session with a fresh statement returns for the same call.
- **Settings:** defaults, pending entries off, caps of 1, and `stale_cooldown` five times `stale_pending_expiry`; each in both modes, with and without abusive clients.

### What it asserts

| # | Invariant | Guarantee |
|---|---|---|
| I1 | Two pool-wide actions are at least `stale_cooldown` apart unless `<n>` moved between them | C1 |
| I2 | A request the window deferred is taken by the first checkout after the window ends | EP5 |
| I3 | Two epochs start at least `stale_cooldown` apart unless `<n>` moved; an epoch replaces at most the connections open at its start | C2 |
| I4 | Stale-driven connection opens stay within `2 × max + 2` per `stale_cooldown` plus `max` per migration; behind PgBouncer there are none | C3, C4 |
| I5 | The pool's slots never pass `max`; open connections never pass `max + 1` or the budget | (c) |
| I6 | Pending and cool-down entries never pass their caps | (d) |
| I7 | A stale statement is never visible outside a transaction, nor under `.retry(n)`, `n` at least 1, with entries on and enough of them | H1, H3 |
| I8 | From the last change, or the end of the last refusal, plus `stale_cooldown`: a stale session straight to Postgres is never visible outside a transaction or under `.retry(n)`, and without it fails at most once per connection; behind PgBouncer a stale session is never visible after `RECONNECT` or `server_lifetime`; a stale statement in transactions with no `.retry(n)` fails at most once per `stale_pending_expiry` | H2, H3, H4 |
| I9 | A wrong answer with no error comes only from a copy cached before a hand-made swap that kept the old type, and never later than `stale_cooldown` after the pool's first confirmation following the last change | (e) |

### The prototype's results

I built the model and the machine in a scratch crate (about 800 lines, stable Rust 1.99.1, a seeded generator and no dependencies, without shrinking), and ran 3,000 seeds of 4,000 events for each of nine variants, about 100 million calls in all:

| Variant | Calls | Pool-wide actions | Stale-driven opens | Failing seeds |
|---|---|---|---|---|
| Straight | 11.9 M | 40,690 | 43,778 | 0 |
| Straight, abusive clients | 11.3 M | 75,125 | 139,201 | 0 |
| Straight, abusive, entries off | 11.3 M | 75,364 | 140,318 | 0 |
| Straight, abusive, caps of 1 | 11.3 M | 75,392 | 139,913 | 0 |
| Behind PgBouncer | 11.9 M | 30,207 | 0 | 0 |
| Behind PgBouncer, abusive | 11.3 M | 79,425 | 0 | 0 |
| Behind PgBouncer, abusive, entries off | 11.3 M | 78,375 | 0 | 0 |
| Behind PgBouncer, abusive, caps of 1 | 11.3 M | 78,849 | 0 | 0 |
| Behind PgBouncer, abusive, `stale_cooldown` 5 × `stale_pending_expiry` | 11.3 M | 19,272 | 0 | 0 |

The prototype found two gaps in my first draft of the machine, both now in it: N3 (an `XX000` from a plain cached statement on a connection newer than the epoch was returned as it was) and N4 (a second signal while an entry lasted).

To show the invariants catch what they're for, I broke the machine one way at a time and ran 50 seeds of each variant:

| Broken machine | Caught by |
|---|---|
| No gate: every request taken at once | I1, in every seed |
| No epoch: every stale-session failure replaces the connection | I4 ("stale opens over the rate bound"), and I7 |
| PR #3's rule: `42804` and `XX000` straight raise the connection generation at once, ungated (N1) | I1, in 48 to 50 of 50 seeds |
| An epoch that never ends | I8 and I7: stale sessions and statements still visible after the bound |
| No replacement on an old connection | I8 and I7 |
| No deferral: a request inside the window is dropped | I2, in 48 of 50 seeds |
| No fresh run inside the window (EO1's plpgsql path for fresh text) | I7 |

**What the prototype doesn't model,** which the building step's simulation adds: several processes behind one PgBouncer, sharing server statements by text with salts of their own; PgBouncer's `max_prepared_statements` eviction; held connections; several pools in one process; a failing read of `<n>`; the parameter test's forms beyond a flag; and SG's recovery conflicts as no-confirm outcomes.

## 7. The rewrite plan

One fix agent rewrites these from this design, on PR #3's branch. No new IDs appear here: where a requirement or test obligation is new, it says so, and the fix gives it the next free ID.

### Requirements

| ID | Step | Fate | What it says after the rewrite |
|---|---|---|---|
| R-1245 | 14 | Rewritten | Section 1's table: each signal, how it's recognised, its class, what cures it, and whether client input can cause it. The definition of a confirmation and the no-confirm list. SC's language, routine and version text kept as is |
| R-0718 | 14 | Rewritten | The statement class, both modes, both contexts: rows A1, B1, B2, D1, E1; the fresh run; pending entries (one per statement, horizon, confirming bytes, refresh, expiry, removal, entries off, cap); a confirmation or `0A000` requests *clear* or *salt*. Choice I unchanged |
| R-0981 | 14 | Rewritten | The session class straight to Postgres: the epoch (start, `T`, `<n>`, end, suspension), old and new connections, *replace* with R-0981's slot rules and the new connection keeping the slot, rows A2 to A4, B3, B4; the parameter test; the transient `XX000`; certain signals requesting *replace* |
| R-0982 | 14 | Rewritten | The three pool-wide actions and the gate: the window, the due action at the first checkout after it, one action per migration, what an action clears, and that the epoch is kept; why the window never changes a call's own outcome |
| R-0983 | 14 | Edited | Keeps "any other routine"; adds that a stale statement gets one fresh run and no other retry |
| R-1246 | 14 | Rewritten | Behind PgBouncer: the marker, the salt (a new one replaces the last), adoption of a moved `<n>`, unnamed fresh runs, entries making every run unnamed, rows D2 and E2 (no pool-wide action, one retry on another connection, the cool-down), `pool_stats()`, and the costs of section 4 (C4, C5) |
| R-1262 | 14 | Rewritten | Four settings, as below; MC's rule for zero; floors and ceilings; values fixed at start |
| R-0984 | 20 | Rewritten, shorter | Inside a transaction: when the call gets `SchemaChanged` and when the error as it is; rows B1 to B5 and E1 to E3 by reference; `.retry(n)`'s next attempt on a new connection after a discard, and on fresh statements through the entry; what happens with no `.retry(n)`. The first-failure records go |
| R-0985 | 20 | Unchanged | |
| R-0975 | 11 | Edited | The connection generation rises only through R-0982's *replace* action; behind a pooler, never |
| R-0060 | 10, and its copies in 11, 12, 27, 47 | Edited | The stale block has four lines; `stale_cooldown`'s comment names the window, the epoch and the cool-down (EP7) |
| R-1259, R-1260 | 10 | Unchanged | |
| R-1261 | 10 | Removed | `MORM_POOL_STALE_FIRST_FAILURE_MAX` goes with its setting (MA) |
| R-1263 to R-1266 | 50 | Rewritten | The use-when lines below |
| R-1267 | 50 | Removed | Its setting goes (MA) |
| R-1268 | 50 | Rewritten | The settings notice below |
| R-1249, R-1250, R-1253, R-1258 | 50 | Rewritten | The notices below |
| New, step 50 | 50 | New | MD's notice beside `tx.set_local` |
| R-1040 | 59 | Edited | The decision text states the costs of section 4 in its terms, for straight and behind PgBouncer |
| R-1256 | 59 | Edited | Adds that `RECONNECT` also clears stale server statements (X10) |
| R-1269 | 48 | Edited | The checkout's due-action check rides on the generation word's one atomic read; the horizon compare runs only while an entry exists; both measured as part of the cached-statement path |
| R-1116, R-1248, R-0697 | 41, 33, 12 | Unchanged | |

**The settings after the rewrite (R-1262):**

| Setting | Default | Governs | Allowed |
|---|---|---|---|
| `stale_cooldown` | `"60s"` | the window for pool-wide actions; straight to Postgres the epoch's length; behind PgBouncer the cool-down | `"1s"` to `"10m"`; shorter, `"0s"`, `0` or `"off"` refused |
| `stale_cooldown_max` | `64` | cool-down entries per pool, behind PgBouncer; straight to Postgres it holds nothing | `1` to `4096` |
| `stale_pending_expiry` | `"60s"` | how long a pending entry lasts from its creation | `"off"` or `"0s"`, which turn entries off, or up to `"10m"`; `0` and `"0"` are bare numbers, refused (MC) |
| `stale_pending_max` | `64` | pending entries per pool | `0` (off) to `4096` |

### `contracts.md`

- `PoolStats`: the counters the rewrite names: epochs started, stale-driven replacements, pool-wide actions taken and deferred, the sizes of the pending and cool-down tables, and the marker, `g<n>` or `g<n>.<salt>`.
- `Error::SchemaChanged`'s comment: returned only where morm repaired something for the next attempt (a statement evicted or an entry made, or a connection discarded); where it repaired nothing, the server's error is returned as `Error::Postgres`.

### `proofs.md`, step 55's section

A record of this design's evidence, X1 to X11, with versions and hashes, and the owner's answers to MA to ME beside the earlier ones.

### Test obligations

| ID | Step | Fate | What it covers after the rewrite |
|---|---|---|---|
| T-0306 | 14 | Rewritten | Behind PgBouncer, rows D1, D2 and E1 on 1.20.1, 1.22 and 1.26.1 with three server sessions. Keeps proof 16's statements. Adds, with `stale_cooldown = "1s"`: a second hand-made drop inside the window answered right outside a transaction, with the salt taken at the first checkout after the window (EP4, EP5); plpgsql stale in one of three sessions (X8) costing at most one salt per window over ten windows (EO1); per-transaction `search_path` flips answered right outside a transaction and at most one salt per window, with the prepare-time `0A000` (X3); `RECONNECT` clearing a stale server statement (X10) |
| T-0311 | 14 | Rewritten | Straight to Postgres, rows A2 to A4: an old connection replaced once, with the slot rules' branches (room in the budget, full budget, `53300`, every open refused, which suspends the epoch, a `statement_timeout` before the conversion, a `23505` after it, an `UPDATE` returning only its tag); stored text costing at most the connections open at `T`, then nothing until the epoch ends; X1's record-shape `42804` costing at most `max` replacements and one *replace* per window over ten windows |
| T-0312 | 20 | Rewritten, shorter | Inside transactions, both modes: B1 to B5 and E1 to E3 each once; EP1's cycle, a row changed during the backoff ten times, costing at most `max` discards and one *replace* per window; two migrations each starting an epoch; no `.retry(n)`; entries off; the cool-down; caps of 2 |
| T-0314 | 14 | Rewritten | The epoch and the cool-down: discards stop at the connections open at `T` and resume after the epoch; a moved `<n>` starts a new one; behind PgBouncer, errors return at once for `stale_cooldown`, and a cap of 2 drops the oldest |
| T-0317 | 14 | Rewritten | The four settings, defaults, overrides, floors and ceilings; `0` and `"0"` refused for `stale_pending_expiry`, `"off"` and `"0s"` accepted; `stale_first_failure_max` refused as an unknown key |
| T-0316 | 59 | Edited | Adds that after `RECONNECT` a new client preparing a stale text gets a fresh server statement (X10) |
| T-0152 | 14 | Edited | "The pool's other connections are replaced at checkout" comes from the *replace* action, at most once per window |
| T-0151, T-0153 | 14 | Unchanged | |
| New, step 20 | 20 | New | The simulation, I1 to I9 (section 6) |

### Fix choices that change

These were the fix's own choices, approved through PR #3, not owner decisions. They change with the machine, and the rewrite's PR body lists them:

- a failed retry's new connection was closed and the first kept its slot (EG3's fix); now the new connection keeps the slot (MA);
- first-failure records cleared when the connection generation moves, and the 64-entry per-statement memory straight to Postgres (EI1's and EG2's fixes); both go (MA);
- "unnamed runs for other values apply behind a pooler only" (EN1's fix); straight to Postgres an entry now makes each connection prepare its older copy again, the same rule in that mode's terms;
- "the one-clear window limits only confirmation-driven raises, not a `0A000`" (EO1's fix); every pool-wide action is gated (MB);
- "`0` also means off" for `stale_pending_expiry` (EO10's fix); MC.

### README notices

Each is loud, beside the row the old one sits by. What each must cover:

1. **R-1249, retired enum labels.** Its first and last paragraphs stay. The costs paragraph says:
   - outside a transaction a call is retried once on a newly opened connection that takes the old one's place;
   - inside one, `.retry(n)` runs again on a newly opened connection, and with no `.retry(n)` each connection that had the function compiled fails one transaction and is replaced;
   - a label you send holding `_morm_retired_` gets its `22P02` at once;
   - text holding it that a function reads or builds costs each connection open at its first failure one replacement, then nothing, again at most once per `stale_cooldown` per pool and after each migration, and never more than the pool's `max` per `stale_cooldown`;
   - if Postgres refuses new connections, replacements pause for `stale_cooldown`;
   - behind PgBouncer such a statement returns `SchemaChanged` once, then its error at once for `stale_cooldown`, until PgBouncer replaces the server connection.
2. **R-1250, functions morm doesn't manage.** The same costs straight to Postgres. Behind PgBouncer, `RECONNECT` or `server_lifetime`, as now.
3. **R-1253, a schema change made outside morm,** in both modes, no longer behind PgBouncer only:
   - outside a transaction a stale statement never reaches you;
   - inside one you get `SchemaChanged` once per connection straight to Postgres, or once per `stale_pending_expiry` per process behind PgBouncer, and `.retry(n)` hides it;
   - morm clears its caches at most once per `stale_cooldown` per pool;
   - behind PgBouncer, a statement run only inside transactions, with no `.retry(n)` and different values each time, fails once per `stale_pending_expiry` until it runs outside a transaction, `RECONNECT` or `server_lifetime`;
   - the two silent cases of section 4 (e), in plain words;
   - run schema changes through morm, and use `.retry(n)`.
4. **R-1258, a stale function behind PgBouncer.** As now, adding that `RECONNECT` also clears stale server statements, and that for a stale function morm never clears its caches or adds a salt.
5. **R-1268, the stale-session settings.** For each of the four, what a longer and a shorter value cost:
   - `stale_cooldown`, longer: fewer clears and replacements under abuse; but straight to Postgres a session made stale inside an epoch can fail as it is that long, a refused open pauses replacements that long, and behind PgBouncer a remembered statement's errors return at once that long. Shorter: up to `max` replacements and one cache clear per `stale_cooldown`.
   - `stale_pending_expiry`, longer: a bad label makes a statement run unnamed, or prepared again, for every caller that long. Shorter: statements run only in transactions fail more often.
   - The caps: memory, and what a value smaller than the number of statements pending at once costs: one more `SchemaChanged` for a statement pushed out, never a connection.
   - What's refused, and what turning entries off does (section 3, "Pending entries off").
6. **New, beside `tx.set_local` (MD).**
   - A `search_path` set per transaction over tables of different shapes makes a cached statement fail with `0A000` on a switch.
   - Outside a transaction morm runs it fresh; inside one, `SchemaChanged` once per `stale_pending_expiry` per statement and process, then fresh runs.
   - morm clears its caches for it at most once per `stale_cooldown`.
   - Behind PgBouncer it arrives at prepare and for other clients too.
   - Use `.retry(n)`, or keep tenant schemas the same shape.
7. **The use-when lines,** R-1263 to R-1266:
   - `stale_cooldown`: you want stale-session handling, and the caches' clearing, to react sooner or later than once a minute;
   - `stale_cooldown_max`: behind PgBouncer, more than 64 statements can be hitting stale server sessions at once;
   - `stale_pending_expiry`: your transactions retry later than a minute, or a bad label should stop making a statement run unnamed sooner; `"off"` turns entries off, so read the notice first;
   - `stale_pending_max`: more than 64 statements can be waiting for a confirmation at once; `0` turns entries off.

## Lettered choices

### MA. Straight to Postgres, which connections a stale-session failure may replace

PR #3 decides this per statement: a cool-down memory after a failed fresh-connection retry (EG2), first-failure records inside transactions (EH1), each with the statement generation (EI1). Both are tables a client can fill, and both decide replacements. EP1 showed a confirmation clears the records, so the cycle repeats; and a client feeding more statements than the cap brings back a connection per call (EO3).

- **(a) Recommended. One epoch per pool** (section 2, rows A2 to A4, B3, B4).
  - A connection opened before the epoch's `T` is replaced on its first stale-session failure. One opened after `T` can't hold code compiled before the change, so its failure returns as it is.
  - The epoch lasts `stale_cooldown`, and a moved `<n>` starts a new one.
  - The replacement keeps the old connection's slot whatever its run returns.
  - It keeps the purpose of EG1, EG2, EH1, EI1 and EJ1: a client's text costs at most the pool's `max` replacements per `stale_cooldown`, whatever it sends and however many statements; a real stale session heals within `stale_cooldown`, a later migration's included; one server slot is enough on `53300`.
  - It changes their mechanism: the memory and the records go, and with them `stale_first_failure_max` (an EN5 setting) and its floor (EO3).
  - Its cost against PR #3: stored text fed to one statement can cost up to `max` replacements per `stale_cooldown` instead of one; in exchange the bound doesn't depend on how many statements are fed, and no table can be filled to delay healing.
- **(b) Keep the memory and the records,** add EP1 (a) (the connection generation under the window, and no second raise for a connection opened since the first) and a per-pool cap of `max` replacements per `stale_cooldown`. Bounded, but the cap can be spent by text fed to many statements while a real stale session waits, the memory still holds a really stale statement's error for `stale_cooldown` after a failed retry, and five settings stay.

### MB. The one-clear window

EO1 limited confirmation-driven clears to one per `stale_cooldown` and sent a stale signal from freshly salted text down the plpgsql path. That path is what made a second hand-made change fail calls for up to twice `stale_cooldown` (EP4), left `0A000` outside the limit (EP2), left the connection generation outside it (EP1), and read two ways (EP3).

- **(a) Recommended. One gate for every pool-wide action, with deferral.**
  - *clear*, *salt* and *replace* all pass it, whatever asked: `0A000`, `42804`, `XX000` or a confirmation.
  - A request inside the window is taken at the first checkout after it ends.
  - A call's own outcome never depends on the window: a signal from freshly salted text gets its fresh run like any other.
  - This keeps EO1's and EO2's bound and extends it to every action (EP1 (a), EP2 (a)). It replaces EO1's "treated as plpgsql, with no further clear" with "answered by a fresh run, with no further clear until the window ends".
- **(b) EO1 as written,** with EP4's cost of up to twice `stale_cooldown` stated, `0A000` left out of the window with EP2 (b)'s notice, and the connection generation handled by MA (b).

### MC. `stale_pending_expiry = 0` (EP6)

- **(a) Recommended.** `0` and `"0"` are bare numbers, refused like every other bare duration; `"off"` and `"0s"` turn entries off.
- **(b)** The integer `0` and `"0"` are accepted for this key alone, meaning off.

### MD. A `search_path` set per transaction over tables of different shapes (EP2, X2, X3)

- **(a) Recommended.** The machine handles it: outside a transaction a fresh run answers; inside, one `SchemaChanged` per `stale_pending_expiry` per statement and process, then fresh runs; at most one cache clear per `stale_cooldown`. A loud README notice beside `tx.set_local` says so (notice 6).
- **(b)** Also, a transaction that sets `search_path` through `tx.set_local` runs all its statements unnamed, so no cached plan meets another path. That removes these errors for morm's own API at the cost of parsing and planning every statement in such transactions; a `SET LOCAL search_path` sent as raw SQL still goes through the machine.

### ME. The simulation's generator

- **(a) Recommended. `proptest`** as a dev-dependency of `morm`: a failing sequence shrinks to the few events that matter and is saved as a regression. `rules.md` asks for a reason for a new crate in the PR; this is it.
- **(b)** A seeded generator in the test itself, as the prototype used: no new crate, but a failure is reported as a seed and thousands of events, with no shrinking.

Servers, the harness, the prototype and all scratch files were throwaway and are deleted. Every result the design rests on is in this file, and section 6 says enough to build the simulation again.
