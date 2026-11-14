# Step 55 independent review, seventh round

PR #3 (`step-55-enum-proofs`), head `1a49674`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes. This round covers the fix for the sixth review's EJ1 to EJ9 and the owner's EJ5 decision, `git diff 14bcb6f origin/step-55-enum-proofs` (commits `1b818c5` and `1a49674`), and checks that nothing earlier regressed. The owner has decided that stale sessions heal by themselves: the automatic fresh-connection retry stays; behind PgBouncer, `migrate` sends `RECONNECT` where admin URLs are configured, and elsewhere sessions heal within `server_lifetime`; morm never ends a server session itself.

## What I read

- **On the PR branch:** `rules.md`, `contracts.md`, `proofs.md` (step 55's section, including the new "sixth review" bullets), and every changed and new requirement and test in full: R-0981, R-0984, R-1040, R-1050, R-1249, R-1250, R-1256, R-1257, R-1258, T-0311, T-0312, T-0315 and T-0316. For the rule set I also re-read R-0982, R-0718, R-1245, R-1246 (the marker behind a pooler), R-1248 (the reset and the statement generation), R-0985 and R-0435 (what `.retry(n)` retries), R-1238 and T-0171 (the `pooler` job), R-0583, R-0618 and R-0996.
- **On `origin/main`:** INDEX's Decisions entries from "Stale-statement design, SA to SH" through "Step 55 sixth review, EJ1 to EJ9", including EJ5's decision, its IDs and the fix's own choices, and `reviews/step-55-review-6.md`.
- **The PR body.**

## What I checked and ran

- **CI on the head.** `1a49674` is green: `ci` on push (run 43606507072) and on the pull request (run 43606512034), every job, including `coverage`, `pooler`, `msrv`, `features`, `trybuild`, `doc` and integration on 15 to 18 over TCP and Unix sockets. The diff since `14bcb6f` touches only `plans/plan-approved/` (8 files). No code, test or workflow changed.
- **Coverage checker,** on the head:
  - `check_coverage.py --step 55 --code crates tests benches fuzz .github`: **1258 requirements, 316 test obligations, 0 problems**;
  - the plain form: the same.
- **Manifest and INDEX.** Comparing manifests ID by ID: `14bcb6f` to `1b818c5` changes R-0981, R-0984, R-1040, R-1050, R-1249, R-1250, T-0311, T-0312 and T-0315; `1b818c5` to `1a49674` changes R-1040 and R-1250 and adds R-1256, R-1257, R-1258 and T-0316. None is removed, and `frame` and `master_sha256` are unchanged. That is exactly INDEX's EJ entry: "Written in at `1b818c5`" and "EJ5 written in at `1a49674`". Against `origin/main`, all 75 changed or new IDs are covered by INDEX's step 55 entries. `coverage.md` gains R-1256 to R-1258 and T-0316 and the new totals.
- **Spot checks, on throwaway servers.** I built PgBouncer 1.20.1 and 1.26.1 from pgbouncer.org's tarballs, each checked against its published SHA-256 (`24992cf5…2088` and `00886a29…42d9`, both matched), and Postgres 17.12 from ftp.postgresql.org's tarball, checked the same way (`9cf136a7…ac55`, matched). I used the system's 18.6 too. Everything ran under `.claude/scratch/` in my worktree, with `TMPDIR` set there, PgBouncer in transaction mode. The client was asyncpg with its statement cache off, and the stale function was `f(p text)` with `DECLARE v st; v := p`, called before a relabel swap made by hand: rename `st` to `_morm_retired_m1_st`, rename its labels, create a new `st`.

  | Claim | Result, on 17.12 and 18.6, behind 1.20.1 and 1.26.1 |
  |---|---|
  | R-1256: after the swap, a new client reaches the same server session and gets `22P02` naming the retired type | Yes, same backend pid, `invalid input value for enum _morm_retired_m1_st: "a"`, with pools of one and of two server connections |
  | R-1256: a transaction begun before `RECONNECT` runs on, on the same server session, and commits | Yes, same pid before and after, committed |
  | R-1256: after `RECONNECT`, the next client reaches a new server session and gets the right answer, on every server session the pool holds | Yes, both server pids replaced, `f('a')` returns `a` |
  | Choice 19: plain `RECONNECT` reconnects every database on that PgBouncer | Yes, a second database's server pid was replaced too |
  | R-1256 and R-1258: with nothing sent, the session heals within `server_lifetime` | Yes, with `server_lifetime = 3` and a call every 0.2 seconds, a new server pid answered right after about 3 seconds |
  | EJ1: after a `53300`, closing one connection frees the server slot at once | Yes. With a role at its `CONNECTION LIMIT`, `too many connections for role`, `53300`; closing one connection and opening another at once was accepted 300 of 300 times, over TCP and a Unix socket. With the server full (`remaining connection slots are reserved`, `53300`), 300 of 300 too |
  | EJ5's choice: behind a pooler, `.retry(n)`'s next attempt "can reach another server session" | Rarely, with PgBouncer's defaults. With two warm server sessions, 20 transactions in a row from one client, and 20 from new client connections, all reached the same server session (LIFO, `server_round_robin = 0`). See EK1 |
  | `RECONNECT` through a TCP address shared by two PgBouncer processes with `so_reuseport = 1` | Reached one process only. The other kept its stale server session and answered `22P02` 3 times out of 8 after `RECONNECT` returned success. See EK2 |
  | The admin console and the extended query protocol | Refused on both versions: `extended query protocol not supported by admin console`. See EK5 |

## EJ1 to EJ9 and EJ5: are they fixed?

| Finding | Verdict and evidence |
|---|---|
| EJ1 | **Fixed as the owner decided.** R-0981: a `53300` refusal of the extra connection closes the stale one and opens the new one in its slot; another failure leaves the first in its slot; a failure after the close leaves the slot to the pool. T-0311 confirms with the role's `CONNECTION LIMIT` still at the pool's size, and covers the refuse-everything case with `ALLOW_CONNECTIONS false`, which isn't `53300`. On 17.12 and 18.6 the freed slot is accepted at once (above). R-1249 says so |
| EJ2 | **Fixed, as recommended.** R-0984 takes the record's time at the generation read after the rollback; a failure on an older connection with the generation unmoved keeps the record and discards that connection; a moved generation replaces the record. T-0312 adds the more-than-eight-calls case. The reset raises the generation in its own transaction after the step (R-1248), so a failure in that short window can still count as a failed confirmation, but it's bounded by the 60-second memory and heals at the next failure after the generation moves |
| EJ3 | **Fixed.** T-0312's swaps raise the `_morm` comment as a `tx` step does, and a no-`.retry(n)` case with a surviving record reaches the generation comparison |
| EJ4 | **Fixed.** R-1249 states the exit-`2` refusal and the `_<run>` rewriting, and that logs and `pg_stat_activity` differ from the files |
| EJ5 | **Fixed as the owner decided,** with gaps. R-1256, R-1257, R-1258, R-1040, R-1250 and T-0316 write in `RECONNECT` after every migration where admin URLs are configured, `server_lifetime` elsewhere, and no `pg_terminate_backend`; R-0984 keeps no record behind a pooler. `RECONNECT`'s claims hold on 17.12 and 18.6 behind both versions (above). But removing the record leaves stored text in a transaction behind a pooler unbounded (EK1); `RECONNECT` reaches one process of an `so_reuseport` group (EK2); a migration that stops after its cutover sends nothing (EK3); and T-0316's "sends nothing" proof can't fail (EK6) |
| EJ6 | **Fixed,** with the fix's own list (classes `08`, `40`, `53`, `57` and `55P03`) named as a choice in R-0981 and the PR body. The new wording leaves a successful statement that returns no rows unclassified (EK4) |
| EJ7 | **Fixed.** "And again after each migration" is in R-0984, R-1249, R-1040 and R-1250 |
| EJ8 | **Fixed.** R-1050 and T-0315 cover `UESCAPE` and `$` inside a bare identifier |
| EJ9 | **Fixed for what it named.** R-0984, R-0981 and R-1050 name the EI fix's choices; the PR body's choices 16 to 18 list them and the EH5 bullet notes EI6 and EJ1. EJ5's own choices are named in R-1256, but one is missing there, and the PR body has two wrong lines (EK7) |

Nothing earlier regressed outside the points below. Only `plan-approved/` changed, and the checker, CI and the ID lists agree. SA to SH, EF1, the `22P02` narrowing, SF, EG1 to EG3, EH1 to EH8 and EI1 to EI9 still read consistently with the new text, apart from EK1's contradiction with R-0982.

## The rule set, path by path

| Path | Heals? Bounded? |
|---|---|
| Outside a transaction, straight, open accepted (budget room or full) | Yes, on the first call: confirmed, generation raised, pool replaced at checkout |
| Outside, `53300` refusal | Yes, on the first call: the stale connection is closed and its slot reused, accepted at once on 17.12 and 18.6 |
| Outside, the open fails even in the stale connection's slot (failover, `ALLOW_CONNECTIONS false`) | Yes, after up to 60 seconds, with the emptied slot reopened fresh by the pool (documented) |
| Outside, a retry ending in a listed error | Yes, after up to 60 seconds (documented) |
| Inside, `.retry(n)` | Yes: discard, attempt on a new connection, confirmed. An attempt ending in `40001` or `40P01` puts the statement in the memory, so `.retry(n)`'s next run, on a pooled connection, can return the raw `22P02` for up to 60 seconds. Bounded, and each such round replaces at least the discarded connection |
| Inside, no `.retry(n)` | Yes: each stale connection fails one transaction and is discarded; a later cutover heals through the generation comparison |
| Inside, the attempt's open refused | Yes: the discard freed a server slot first, and the pool's open has its backoff |
| Stored or client text, straight | Bounded: one connection per statement per minute outside, a round of discards per migration inside |
| `db.hold()` | Yes, on release |
| Behind PgBouncer, admin URLs listed | Yes, at the end of the migration, without interrupting transactions (confirmed above), apart from EK2 (`so_reuseport`) and EK3 (a stopped migration) |
| Behind PgBouncer, no URLs | Yes, within `server_lifetime` (confirmed above) |
| **Stored or client text inside a transaction, behind PgBouncer** | **Not bounded**: `SchemaChanged` and a full cache flush of the process on every call, for as long as the text is there (EK1) |

No path grows the pool past `max` or the process past its budget. The 64-entry caps are owner-approved choices (EG2, EI1), so their thrash beyond 64 distinct statements isn't raised again.

So the rule set is sound straight to Postgres, in and out of transactions, with and without `.retry(n)`, with a full budget and with refused opens. It isn't yet sound behind PgBouncer, because of EK1, and EK2 and EK3 leave the "at once" promise short.

## Findings

### EK1. Blocks approval. R-0984, R-1246, R-1249, R-1258, T-0312: behind a pooler, inside a transaction, text holding `_morm_retired_` now churns every connection's statement cache on every call, with no bound

- **Where:**
  - R-0984: "behind a pooler, once the transaction has rolled back, it reads the statement generation and moves its marker as step 14 says". And, new in EJ5's fix: "Behind a pooler morm keeps no record and discards nothing ...: such a `22P02` is `Error::SchemaChanged` unless the parameter test or step 14's memory says otherwise".
  - R-1246, inside a transaction: "the signal returns `Error::SchemaChanged`, and once the transaction has rolled back morm reads `<n>` on that connection and applies the same rule". That rule is: adopt `<n>` if it moved, otherwise add a new random salt, and "Either way it raises the statement generation ... so every connection drops its cached statements at checkout".
- **What's wrong:**
  - **Nothing behind a pooler fills the memory from a transaction any more.** A failed confirmation needs a record, and there is none behind a pooler. So a statement whose function converts stored text, or a client's text it changes, returns `SchemaChanged` on every call, for as long as that text exists. Each call moves the marker to a new salt and raises the statement generation, so every connection in the process drops both statement caches at its next checkout and prepares every statement again under new text. On PgBouncer that also means new server-side statements on every server connection. With `.retry(n)` each call repeats this `n + 1` times.
  - **A client can do it on purpose.** R-0981 itself names `lower(p)`: a client sending `_MORM_RETIRED_X` passes the parameter test (its text doesn't hold `_morm_retired_`), and the function turns it into `_morm_retired_x`. Behind PgBouncer, inside a transaction, every such request clears every connection's caches. That contradicts R-0982, owner decision SF and EF1's reason: "so a client sending a bad enum label can't clear the caches on every request". It also contradicts SF's rule that the generation is raised only for a confirmed stale statement, since nothing in a transaction behind a pooler confirms this `22P02`.
  - **The churn buys nothing, even for a real stale session.** This `22P02` comes from a plpgsql variable compiled in the server session. New statement text runs in the same server session and calls the same compiled function, so it fails the same way; only `RECONNECT` or `server_lifetime` heals it (R-1256). And `.retry(n)`'s next attempt mostly reaches the same server session: with PgBouncer's default LIFO reuse, 20 of 20 transactions in a row, from one client or from new client connections, reached the same server (above). So without admin URLs, a real stale session behind PgBouncer also flushes the process's caches on every failing call for up to `server_lifetime`, an hour by default.
  - **The README is wrong there.** R-1249 and R-1250 say such text "costs one new connection, or inside transactions at most the pool's connections ... and then that statement's `22P02` returns at once for 60 seconds". Behind PgBouncer, inside a transaction, it never stops. R-1258 doesn't mention it.
  - T-0312's pooler case checks only that nothing is discarded and that the error stays `SchemaChanged`; nothing checks the marker or the caches.
  - Before EJ5's fix this was already mostly true, because behind a pooler morm's client connections are never replaced, so no connection was ever "opened after the record". EJ5's change makes it certain.
- **Evidence:** the quoted text of R-0984, R-1246, R-0982 and R-0981; the LIFO runs above on 1.20.1 and 1.26.1.
- **Fix, a design choice for the owner:**
  - **(a) Recommended.** Behind a pooler, a `22P02` from `enum_in` holding `_morm_retired_` in a transaction moves no marker and raises no generation, because new statement text can't reach a server session's compiled function. Keep `SchemaChanged`, as EJ5's choice says, but when a `.retry(n)` run's last attempt, or a call with no `.retry(n)`, ends in that same `22P02` for the same statement and parameter bytes, put the statement in step 14's memory, as outside a transaction. That bounds stored and client text to one `SchemaChanged` per statement per minute. It doesn't slow healing, which behind PgBouncer comes from `RECONNECT` or `server_lifetime`, not from morm. R-1249 and R-1258 then say that behind PgBouncer such a statement's `22P02` returns at once for 60 seconds after its first `SchemaChanged`. T-0312's pooler case asserts no marker move, no cache drop and the memory filled after the first call.
  - **(b)** Only stop the marker move and the raise for this signal behind a pooler, and say in R-1249 and R-1258 that behind PgBouncer such text inside a transaction returns `SchemaChanged` on every call and `.retry(n)` runs the closure `n + 1` times. That's simpler and bounds the cost per call, but the wrong error kind lasts as long as the data does.
  - **(c)** Keep the rule and document it. That doesn't remove the contradiction with R-0982 and SF.

### EK2. Should fix. R-1256, R-1258, T-0316: `RECONNECT` through an address that several PgBouncer processes share reaches only one of them, and morm reports success

- **Where:** R-1256 sends `RECONNECT` to "each listed admin console". R-1258: "List every PgBouncer: one left out keeps its stale sessions until `server_lifetime`".
- **What's wrong:** a common way to use more than one core is several PgBouncer processes on one port with `so_reuseport = 1`. Each process has its own pools, and an admin connection to the shared address reaches one process, picked by the kernel. On 1.26.1, with two such processes in front of 18.6, `RECONNECT` through the shared TCP address returned success. One process's server session was replaced. The other kept its stale one, and new clients got `22P02` 3 times out of 8 afterwards. morm would exit `0` and say nothing, and that process's sessions would stay stale until `server_lifetime`. Under R-1258's wording the user did list "every PgBouncer".
- **Evidence:** the run above. Both versions report the setting in `SHOW CONFIG` (`so_reuseport|1|0|no` on 1.26.1, `so_reuseport|0|0|no` on 1.20.1).
- **Fix:**
  - R-1258 says "every PgBouncer process". With `so_reuseport`, list each process's own Unix socket (its own `unix_socket_dir`), because a shared address reaches only one of them.
  - R-1256: after `RECONNECT` over TCP, morm reads `SHOW CONFIG`. If `so_reuseport` is `1`, it reports that PgBouncer as a failed send (host, port and the reason, exit `1`), because it can't know it reached every process.
  - T-0316 adds a unit test of that check.

### EK3. Should fix. R-1256, R-1258, T-0316: a migration that stops after its cutover has committed sends no `RECONNECT`, even with admin URLs listed

- **Where:** R-1256 sends `RECONNECT` "at the end of every migration it applies, after that migration's last step has committed and the ledger records it as applied".
- **What's wrong:** an online swap commits its cutover in its own `tx` step, and later steps of the same migration can fail (R-1053, R-1128, R-0584: `--resume` and `rollback --failed` exist for that). Then the swapped type is live, but the migration isn't applied, so nothing is sent. Behind PgBouncer every server session that compiled an unmanaged function stays stale until the operator resumes or rolls back, or until `server_lifetime` (an hour by default). That's during an incident, when the user configured admin URLs precisely so this heals "at once" (R-1257, R-1258). It still heals within `server_lifetime`, so the owner's bound holds, but the README's promise doesn't. This doesn't change the owner's choice of when to send for a migration that finishes.
- **Fix:**
  - R-1256: a run that stops with an error after committing any `tx` step of a migration also sends `RECONNECT` to each listed PgBouncer before it exits, reporting failed sends as now.
  - R-1258 says "after every migration, and when one stops partway".
  - T-0316 adds a migration whose step after the cutover fails: the function is healed for the next client after release, and `migrate` exits with the step's error.

### EK4. Minor. R-0981, R-0984: "returns rows" leaves a successful statement with no result rows unclassified

- **Where:** R-0981: "It confirms a stale session when it returns rows, or a server error other than ...". R-0984: "ends as step 14's retry confirms, with rows or a server error outside step 14's list".
- **What's wrong:** EJ6's rewording replaced "gets past the conversion ... whatever happens next" with "returns rows". An `INSERT`, an `UPDATE` or a `CALL` that calls the function and succeeds returns a command tag, not rows, and an `UPDATE` matching no row returns zero rows. Read literally, such a retry neither confirms nor fails nor is in the no-confirm list, so the text doesn't say what happens.
- **Fix:** "when it succeeds, with or without rows, or ends in a server error other than ..." in R-0981, and "with success or a server error" in R-0984. T-0311 adds an `UPDATE ... SET c = f($1)` retry that confirms.

### EK5. Minor. R-1256: the admin console takes only the simple query protocol

- **What's wrong:** PgBouncer's console refuses the extended protocol: `extended query protocol not supported by admin console`, on 1.20.1 and 1.26.1. morm's ordinary path sends parameterised, prepared statements, so a send built on it fails every time, and R-1256 treats that as a failed send: the migration stays applied and exits `1`. T-0316 would catch it, but the requirement should say how the command is sent.
- **Fix:** R-1256 says morm sends `RECONNECT` (and EK2's `SHOW CONFIG`) as a simple query, with no parameters.

### EK6. Minor. T-0316: its "sends nothing" proof can't fail, it skips 1.22, and its `RECONNECT` can disturb T-0171

- **Where:** T-0316: "`--plan`, a run with nothing pending and a type change made by hand send nothing, shown by a held server session keeping its `pg_backend_pid()` across them"; "behind PgBouncer 1.20.1 and 1.26.1".
- **What's wrong:**
  - `RECONNECT` never closes a server connection while a transaction holds it. That's R-1256's own claim, and it held on both versions above. So a held server session keeps its pid whether or not anything was sent, and the assertion passes for an implementation that sends `RECONNECT` on `--plan`.
  - R-1238 runs the pooler tests of later steps on every pair, 1.22 included, and T-0171 in the same step names 1.20.1, 1.22 and 1.26.1. T-0316 leaves 1.22 out.
  - T-0316 sends `RECONNECT` to the `pooler` job's shared PgBouncer, which reconnects every database on it. T-0171 asserts that an unmanaged function "stays stale on the server sessions that compiled it until `RECONNECT`". Run in parallel, T-0316 can heal T-0171's sessions early and make it fail.
- **Fix:**
  - Show "nothing sent" with a released server session. With one server connection, a new client after the command reaches the same `pg_backend_pid()` and the function is still stale. Or check `SHOW SERVERS`, or PgBouncer's log, for no reconnect.
  - Run T-0316 on 1.20.1, 1.22 and 1.26.1.
  - Run it serially with the other pooler tests, or against its own PgBouncer instances.

### EK7. Minor. PR body and attribution

- The PR body ends "No new public call, so no "use this when" line", but R-1257 adds a "use this when" line for the new setting `[migrations] pgbouncer_admin_urls`.
- The PR body's "README notices (step 50)" bullet lists R-1249 to R-1255 and not R-1258, the new notice, or R-1257.
- R-1256's list of "this fix's choices" omits one INDEX's EJ entry and the PR body's choice 19 name: plain `RECONNECT`, for every database on that PgBouncer. INDEX also names "R-1256 and T-0316 live in step 59" as a choice, which the step placement shows but no text names.
- **Fix:** correct the two PR body lines, and add "plain `RECONNECT`, for every database on that PgBouncer" to R-1256's choices.

### EK8. Minor. R-1258: two surprises of the new setting have no README line

- **Where:** R-1256: "a URL whose database isn't `pgbouncer` is `Error::Config`", and "`--plan`, `rehearse`, a run that applies nothing ... send nothing".
- **What's wrong:**
  - The `Error::Config` refusal isn't in R-1258 or R-1257.
  - After a failed send, the natural step is to run `migrate` again. That applies nothing, so it sends nothing and exits `0`. R-1258 says only "`migrate` reports which PgBouncer and exits with code `1`". The fix is `RECONNECT` by hand, which only the printed message says.
- **Fix:** R-1258 adds: "Each URL must name the `pgbouncer` database, or morm stops with `Error::Config`. If a send fails, run `RECONNECT` on that PgBouncer's console yourself: running `migrate` again applies nothing and sends nothing."

## What I couldn't run

- **morm itself.** There is no runtime yet. EK1, EK3, EK4 and EK6's first point are shown from the rules' text. EK1's LIFO evidence and EK2's result come from PgBouncer itself.
- **Postgres 15 and 16, and PgBouncer 1.22.** I ran 17.12 and 18.6 behind 1.20.1 and 1.26.1 only. `RECONNECT`'s claims held on all four pairs, and the `53300` slot-reuse held on 17.12 and 18.6.
- **`so_reuseport` on 1.20.1.** I ran the two-process case on 1.26.1 only. 1.20.1 reports the setting in `SHOW CONFIG` the same way.
- **A loaded server.** The `53300` close-then-open check ran on an idle machine. Under load, a slower backend exit could lose the race. R-0981 then leaves the slot to the pool, which reopens it fresh, so that path is bounded anyway.

## Verdict

**Not approved yet.**

What holds:

- every changed ID is in INDEX's EJ entry at the commits it names, and the PR body's ID lists match;
- the checker reports 0 problems, and CI is green on the head;
- EJ1 to EJ4 and EJ6 to EJ9 are fixed;
- EJ5's `RECONNECT` claims hold on 17.12 and 18.6 behind 1.20.1 and 1.26.1: a transaction in flight keeps its server session, the next client gets a fresh one, and every database on that PgBouncer is reconnected;
- straight to Postgres, the retry, confirmation, cool-down and pool rules now heal every path within a bounded time, never grow past `max` or the budget, and give no silent wrong answer.

What remains:

- **Blocks approval:** EK1. Behind a pooler, inside a transaction, text holding `_morm_retired_` clears every connection's statement caches on every call, a client can trigger it, and it contradicts R-0982 and SF. This needs a design choice; (a) is recommended.
- **Should fix:**
  - EK2: `RECONNECT` reaches one process of an `so_reuseport` group, and morm reports success;
  - EK3: a migration that stops after its cutover sends no `RECONNECT`.
- **Minor:**
  - EK4: "returns rows";
  - EK5: the simple query protocol;
  - EK6: T-0316's proof, its versions and its isolation;
  - EK7: PR body and attribution;
  - EK8: two README lines.
