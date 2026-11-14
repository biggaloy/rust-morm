# Step 55 independent review, eighth round

PR #3 (`step-55-enum-proofs`), head `1887f07`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes. This round covers the fix for the seventh review's EK1 to EK8, `git diff 1a49674 origin/step-55-enum-proofs` (one commit, `1887f07`), and checks that nothing earlier regressed. The owner decided EK1: behind a pooler a stale `22P02` moves no marker and raises nothing, the statement enters the 60-second cool-down after its first `SchemaChanged`, and healing there stays with `RECONNECT` and `server_lifetime`.

## What I read

- **On the PR branch:** `rules.md`, `contracts.md`, `proofs.md` (step 55's section, with the new "seventh review" bullets and proof 16 again), and every changed requirement and test in full: R-0981, R-0984, R-1238, R-1246, R-1249, R-1256, R-1258, T-0311, T-0312 and T-0316. For the rule set I also re-read R-0982, R-1245 (the signal table), R-1040, R-1250, R-1253, R-1257 and T-0171.
- **On `origin/main`:** INDEX's Decisions entries "Step 55 sixth review, EJ1 to EJ9" (with EJ5) and "Step 55 seventh review, EK1 to EK8", including the EK IDs and the fix's own choices, and `reviews/step-55-review-7.md`.
- **The PR body.**

## What I checked and ran

- **CI on the head.** `1887f07` is green: `ci` on push (run 43609126276) and on the pull request (run 43609131058), 18 of 18 jobs each, including `coverage`, `pooler`, `msrv`, `features`, `trybuild`, `doc`, `unit`, `clippy`, `fmt` and integration. The diff since `1a49674` touches only `plans/plan-approved/` (8 files). No code, test or workflow changed.
- **Coverage checker,** on the head:
  - `check_coverage.py --step 55 --code crates tests benches fuzz .github`: **1258 requirements, 316 test obligations, 0 problems**;
  - the plain form: the same.
- **Manifest and INDEX.** Comparing the manifests at `1a49674` and `1887f07` ID by ID: R-0981, R-0984, R-1238, R-1246, R-1249, R-1256, R-1258, T-0311, T-0312 and T-0316 change; none is added or removed; `frame` and `master_sha256` are unchanged. That is exactly INDEX's EK entry, "Written in at `1887f07`". Against `origin/main` the set of changed or new IDs is the same 75 as at `1a49674`, all covered by INDEX's step 55 entries. `coverage.md` changes only T-0316's summary; the totals stand.
- **Spot checks, on throwaway servers.** I built PgBouncer 1.20.1, 1.22.1 and 1.26.1 from pgbouncer.org's tarballs, each checked against its published SHA-256 (`24992cf5…2088`, which is also R-1238's pin, `2b018aa6…a215` and `00886a29…42d9`, all matched), in front of the system's Postgres 18.6. Everything ran under `.claude/scratch/` in my worktree, with `TMPDIR` set there; the PgBouncers' Unix sockets sat in a short folder under `~/.claude/scratch/`, because the worktree's path is too long for a socket. PgBouncer was in transaction mode with `default_pool_size = 1`. The clients were asyncpg 0.31.0, whose `fetch` uses the extended protocol, and psql, which uses the simple one.

  | Claim | Result |
  |---|---|
  | EK5: the admin console refuses the extended protocol | Yes, on 1.20.1, 1.22.1 and 1.26.1: `extended query protocol not supported by admin console`. psql's simple-protocol `SHOW CONFIG` and `RECONNECT` worked on all three |
  | EK2: `SHOW CONFIG` reports `so_reuseport` | Yes, as `so_reuseport\|0\|0\|no` on 1.20.1, 1.22.1 and 1.26.1 (key, value, default, changeable), and `so_reuseport\|1\|0\|no` from each of two 1.26.1 processes sharing one port with `so_reuseport = 1` |
  | EK2: each process's own Unix socket reaches that process alone | Yes. With both processes stale after a relabel swap, `RECONNECT` through process 1's socket healed process 1 only (process 2 still gave `22P02` naming `_morm_retired_m1_st`); `RECONNECT` through process 2's socket then healed it. A third process given process 1's `unix_socket_dir` refused to start: `FATAL unix socket is in use, cannot continue` |
  | EK6: with one server connection, a new client after a command reaches the same server session unless something reconnected it | Yes, on all three versions: two clients in a row, the same `pg_backend_pid()` |
  | EK4's T-0311 case, an `UPDATE ... SET c = f($1)` matching no row | See EL5: on the stale session a `VOLATILE` `f` gave `UPDATE 0` with no error; only an `IMMUTABLE` `f`, folded at planning, gave the `22P02` |
  | R-1245's sixth row, a `22P02` from a cached statement that casts a parameter, holding another enum's name | A client can produce it. See EL1 |

## EK1 to EK8: are they fixed?

| Finding | Verdict and evidence |
|---|---|
| EK1 | **Fixed as the owner decided.** R-0984: behind a pooler such a `22P02` "moves no marker, adds no salt and raises no generation", is `SchemaChanged` once, then "enters step 14's memory, so for 60 seconds its `22P02` returns as it is", and "never clears a statement cache"; healing is `RECONNECT` or `server_lifetime`. R-1246 adds the same, "inside a transaction or out", and names EK1. R-1249 and R-1258 say it. T-0312's pooler case asserts the marker unmoved, no cache dropped and the memory filled, and a `lower($1)` case gives one `SchemaChanged` and nine `22P02`. This fixes the retired-label `22P02`. Two neighbouring paths behind a pooler still clear caches, one of them at a client's will (EL1, EL2) |
| EK2 | **Fixed as recommended,** with a gap. R-1256 reads `so_reuseport` from `SHOW CONFIG` after each `RECONNECT` over TCP and reports `1` as a failed send; R-1258 says "every PgBouncer process" and to list each process's own Unix socket. Both claims hold on 1.26.1 (above). But from another host no URL to such a PgBouncer can succeed, the README doesn't say so, and the Unix-socket branch has no test (EL3) |
| EK3 | **Fixed.** R-1256: a run that stops with an error after committing any `tx` step sends `RECONNECT` before it exits with that step's error; R-1258 says so; T-0316 adds the failing step after a cutover |
| EK4 | **Fixed in the rules.** R-0981 and R-0984 confirm on success "with or without rows". T-0311's new case can't reach the retry as written (EL5) |
| EK5 | **Fixed.** R-1256 sends `RECONNECT` and `SHOW CONFIG` as simple queries, with the console's own refusal quoted; confirmed on all three versions |
| EK6 | **Fixed in substance,** with a contradiction. T-0316 runs on its own 1.20.1, 1.22 and 1.26.1 instances with one server connection and proves "nothing sent" with a released session; confirmed above. R-1238's new bullet contradicts the bullet after it, owner decision EA, and T-0316 lost "on every server session the pool holds" (EL4) |
| EK7 | **Fixed for what it named.** The PR body's README and "use this when" lines are corrected, and R-1256 names plain `RECONNECT` and step 59. Attribution of this fix's own choices is short again (EL7) |
| EK8 | **Fixed.** R-1258 states the `Error::Config` refusal and that after a failed send you run `RECONNECT` by hand, because `migrate` again sends nothing |

Nothing earlier regressed. Only `plan-approved/` changed, and the checker, CI and the ID lists agree. SA to SH, EF1, EG1 to EG3, EH1 to EH8, EI1 to EI9 and EJ1 to EJ9 read consistently with the new text; EL1 and EL2 are older paths that EK1's reasoning now shows up, not regressions.

## The rule set behind a pooler, path by path

| Path | Heals? Bounded? |
|---|---|
| Retired-label `22P02`, outside a transaction | Retry once on another connection, which with PgBouncer's LIFO reuse usually fails; the memory then returns it at once for 60 seconds. Nothing raised. Bounded |
| Retired-label `22P02`, inside a transaction (EK1) | `SchemaChanged` once, then the cool-down; nothing moved or raised. Bounded at one `SchemaChanged` per statement per minute |
| Stored or client text holding `_morm_retired_`, either way | As the two rows above. Bounded, no cache clearing |
| A real stale server session | Heals through `RECONNECT` at the end of each migration, or when a run stops after a committed `tx` step, on each listed PgBouncer process, including each `so_reuseport` process through its own Unix socket (confirmed above); otherwise within `server_lifetime` (confirmed in the seventh review) |
| `so_reuseport` reached over TCP | Reported as a failed send, exit `1`, every migration; heals within `server_lifetime` or by hand (EL3) |
| **Another enum's `22P02` from a cached statement that casts a parameter, inside a transaction** | **Not bounded: the marker moves and every connection's caches are cleared on every call, and a client can trigger it with an ordinary enum label (EL1)** |
| `42804` or `XX000` `cache lookup failed for type` from a real stale session, inside a transaction | Marker moved and caches cleared on every failing call, for up to `server_lifetime`, though new text can't cure it (EL2). Not a client's to trigger |
| `0A000`, outside or inside | One marker move per process per generation, as SD to SF intend |

So behind a pooler the rule set is sound for the retired-label `22P02`, which EK1 was about: no client text clears a cache through it, and a real stale session heals through `RECONNECT`, every `so_reuseport` process and a stopped run included, or through `server_lifetime`. It isn't yet sound overall, because of EL1, and EL2 leaves the same churn EK1 removed on the other two stale-session signals.

## Findings

### EL1. Blocks approval. R-0984, R-1246, R-1245, R-0982: inside a transaction behind a pooler, a client's enum label can move the marker and clear every connection's caches on every call

- **Where:**
  - R-1245's sixth row: "`22P02` from a cached statement that casts a parameter to a non-built-in type | routine `enum_in`, and the message doesn't hold that cast's type name as a whole identifier ... | stale statement, from a swap made outside morm". It's a stale-statement signal on its own, with no retry to confirm it.
  - R-0984 lists that `22P02` among its signals: morm "returns `Error::SchemaChanged` ... and clears the statement ...; behind a pooler, once the transaction has rolled back, it reads the statement generation and moves its marker as step 14 says ..., apart from a `22P02` from `enum_in` holding `_morm_retired_`".
  - R-1246, inside a transaction: "the signal returns `Error::SchemaChanged`, and once the transaction has rolled back morm reads `<n>` on that connection and applies the same rule", which is: adopt `<n>` if it moved, otherwise add a new random salt, and "Either way it raises the statement generation ... so every connection drops its cached statements at checkout".
- **What's wrong:**
  - That `22P02` isn't only a swap made outside morm. Any statement that casts a parameter to an enum (or `citext`, or a domain) and converts something to a different enum raises it when the value doesn't fit. On 18.6, straight and behind 1.26.1, in and out of a transaction: a prepared `SELECT (r8.conv(($1::r8.kind)))::text`, where `conv` returns `k::text::r8.other`, gave `22P02`, routine `enum_in`, `invalid input value for enum other: "b"` for `b`, a valid label of `kind`. A prepared `UPDATE r8.t SET status = ($2::text)::r8.other WHERE email = ($1::public.citext)` gave the same for a client's `zzz`. Neither message holds the cast's type name (`kind`, `citext`), so both match the row. A `PgEnum` can carry `b`, so morm's own encoding doesn't stop it.
  - Behind a pooler, inside a transaction, each such call moves the marker to a new salt and raises the statement generation, so every connection in the process drops both statement caches and prepares every statement again under new text, and PgBouncer holds one more set of server statements per salt. With `.retry(n)`, each call does it `n + 1` times. A client sending such a label on every request clears the caches on every request. That's what R-0982 rules out ("so a client sending a bad enum label can't clear the caches on every request") and what owner decision SF forbids (the statement generation rises "only for a confirmed stale statement", and "a `22P02` is confirmed only by a retry that succeeds, because its text can come from a client"). It's the same harm EK1 blocked approval for, through the neighbouring row of R-1245.
  - Outside a transaction it's bounded: R-1246's unnamed retry fails, "its error is returned and nothing changes". Straight to Postgres inside a transaction nothing is cleared, but the client's `22P02` comes back as `Error::SchemaChanged`, after `.retry(n)` has run the closure `n + 1` times.
- **Evidence:** the quoted text of R-1245, R-0984, R-1246, R-0982 and SF (proofs.md, proof 16's decision paragraph); the two runs above.
- **Fix, a design choice for the owner:**
  - **(a) Recommended. Confirm it as step 14 does, through `.retry(n)`'s next attempt.** Inside a transaction this `22P02` returns `SchemaChanged` and evicts that one statement, and moves the marker only if `<n>` has moved, which only a migration does. With `<n>` unmoved, `.retry(n)`'s next attempt runs that statement unnamed behind a pooler, or re-prepared straight. If it succeeds with the same text and parameter bytes (EH2's test), the cached text was stale: then the salt is added and the generation raised, as R-1246's first bullet does outside a transaction. If it fails the same way, the error returns as it is, as `Error::Postgres`, and nothing moves. With no `.retry(n)`, nothing moves, and the statement's next run outside a transaction confirms through R-1246's first bullet. A client's label then costs one unnamed run per call and never clears a cache. R-1253 says that inside a transaction a change made by hand heals through `.retry(n)`'s next attempt. T-0312 or T-0306 adds the `conv` case behind PgBouncer: no marker move, no cache dropped, the `22P02` returned as it is on the second attempt.
  - **(b)** Keep the move, but put the statement in step 14's memory after it, keyed by its text without the marker, so the same `22P02` from that statement returns as it is for 60 seconds. That bounds a client's label to one cache clearing per statement per minute, but a client still triggers it.
  - **(c)** Keep the rule and document it. That doesn't remove the contradiction with R-0982 and SF.

### EL2. Should fix. R-0984, R-0982, R-1246: behind a pooler, `42804` and `XX000` from a real stale session still move the marker and clear caches inside a transaction, on every failing call, though new text can't cure them

- **Where:**
  - R-0984 lists `42804` and `XX000` `cache lookup failed for type` among its signals and, behind a pooler, "moves its marker as step 14 says", exempting only the retired-label `22P02`.
  - R-0982: a statement "is confirmed stale when its error is one of R-1245's certain signals, `0A000` ..., `XX000` `cache lookup failed for type` or `42804`", and a confirmed stale statement raises the statement generation.
  - R-1246's new sentence gives the reason the retired-label `22P02` is exempt: "a stale-session signal, not a stale-statement one: it moves no marker ... because new text can't reach the compiled function that raised it". R-1245's table calls `42804` and that `XX000` "stale session" too.
  - R-0981, outside a transaction behind a pooler: "discards and raises nothing".
- **What's wrong:** these two errors come from code compiled in the server session, like the retired-label `22P02`. Proof 16's table shows new text doesn't cure them behind PgBouncer: without the reset, S4 and S6 gave `42804` to "Old clients, unnamed" as to cached ones; only `DISCARD PLANS`, a no-op `CREATE OR REPLACE` or `RECONNECT` cleared them. So behind a pooler, inside a transaction, each failing call after a hand-made column change, or an `XX000` after a type is dropped while sessions that compiled against it live, moves the marker to a new salt and clears every connection's caches, for up to `server_lifetime` (an hour by default) when no admin URL is listed. That's the churn EK1's third point showed "buys nothing, even for a real stale session". Outside a transaction R-0981 raises nothing for the same errors, so the two paths disagree, and R-0982 can be read either way. A client can't trigger these, so this doesn't block approval.
- **Evidence:** the quoted text; proof 16's second table in `proofs.md`.
- **Fix, a design choice for the owner, since it extends EK1 to the other two stale-session signals:**
  - **(a) Recommended.** R-1246's sentence covers all three stale-session signals of R-1245 behind a pooler, in or out of a transaction: they move no marker, add no salt and raise no statement generation, and stay `SchemaChanged` inside a transaction. R-0982 names that exception. Healing stays with `RECONNECT` and `server_lifetime`, as R-1258 says. T-0312's pooler case adds a `%ROWTYPE` function after a hand-run column change: `SchemaChanged`, marker unmoved, no cache dropped.
  - **(b)** Keep it, and say in R-1258 that behind PgBouncer each such failing call inside a transaction clears the process's statement caches until the server connection is replaced.

### EL3. Should fix. R-1256, R-1258, T-0316: with `so_reuseport`, morm can succeed only on the PgBouncer's own host, the README doesn't say so, and the Unix-socket branch has no test

- **Where:** R-1256: "after each `RECONNECT` over TCP, morm reads `SHOW CONFIG` ... and when `so_reuseport` is `1` it reports that PgBouncer as a failed send, naming the setting and saying to list each process's own Unix socket instead". R-1258: "list each process's own Unix socket; morm reads `so_reuseport` from `SHOW CONFIG` and reports a shared address as a failed send". T-0316: "A stubbed console whose `SHOW CONFIG` reports `so_reuseport` as `1` is reported as a failed send naming the setting, and one reporting `0` isn't."
- **What's wrong:**
  - The check reports every TCP URL to such a PgBouncer as failed, not only a shared address. `SHOW CONFIG` can't tell morm whether the address it used is shared (a process can also listen on an address of its own on the same port), so this is the right call. But a Unix socket can only be reached on the same host, so `migrate` run from another host, such as a deploy runner, can never succeed against an `so_reuseport` PgBouncer: every migration reports it failed and exits `1`. R-1258's "reports a shared address as a failed send" reads as if a per-process TCP address would work, and it doesn't say morm must run on each PgBouncer's host. That's a surprise with no README line.
  - Nothing tests that a Unix-socket URL skips the check and succeeds against an `so_reuseport = 1` PgBouncer, the one configuration R-1258 tells users to set up. The `1` branch is tested only against a stub. Real PgBouncer reports `so_reuseport|1|0|no` (above), so a real instance is cheap.
- **Evidence:** the two `so_reuseport` processes above: each one's Unix socket reached that process alone, and `SHOW CONFIG` through it said `1`; a shared Unix socket refused to start.
- **Fix:**
  - R-1258: "With `so_reuseport`, several processes share one address and a connection reaches only one of them, so morm must reach each process through its own Unix socket, which works only from the PgBouncer's own host. Any TCP URL to such a PgBouncer is reported as a failed send on every migration. From another host, leave it out and let `server_lifetime` heal it, or run `RECONNECT` on each process yourself." R-1256's printed message says the same.
  - T-0316: run the `1` case against a real 1.26.1 instance with `so_reuseport = 1`. Its TCP URL is reported as a failed send naming the setting, and its Unix-socket URL gets `RECONNECT` with no `SHOW CONFIG` and heals the function. Keep the stub if wanted.

### EL4. Should fix. R-1238, T-0316: the `RECONNECT` test's instances contradict EA's three-connection rule, T-0316 lost "every server session", and their cache settings are unstated

- **Where:** R-1238's new bullet: "one more instance of each of 1.20.1, 1.22 and 1.26.1 runs with `default_pool_size = 1` ... Owner decision on EK6." The next bullet: "Each instance's config sets `default_pool_size = 3` or more, `min_pool_size` the same and `server_round_robin = 1` ... A pool of one hid a failure ... Owner decision EA." And: "The 1.22 and 1.26.1 instances, apart from the third 1.26.1 instance and the `RECONNECT` test's, set `max_prepared_statements = 200` explicitly ... The 1.19.1 and 1.20.1 configs leave the key out, and morm's pools on them set `statement_cache = 0` and `shape_cache = 0`."
- **What's wrong:**
  - As written, the new instances break the "Each instance" rule, an owner decision, and nothing says they're its exception.
  - At `1a49674` T-0316 checked the heal "on every server session the pool holds". With one server connection that clause is gone, so nothing proves that `RECONNECT` replaces several server sessions, idle and busy, which R-1256 promises ("closes each server connection of every database on that PgBouncer").
  - Leaving `max_prepared_statements` out gives 0 on 1.22 and 200 on 1.26.1, which 1.24.0 changed. So the three instances don't run morm the same way, and the text doesn't say what `statement_cache` and `shape_cache` morm's pools use there. On 1.22 with caches on and the key at 0, a pool of one hides EA's failure: the case that bullet exists to prevent.
- **Fix:**
  - R-1238 says the `RECONNECT` instances are the one exception to the three-connection rule, and why: T-0316's "nothing sent" check needs a new client to reach the same server session.
  - Each `RECONNECT` instance gets a second `[databases]` entry with `pool_size = 3` and `min_pool_size = 3`. T-0316 asserts that after `RECONNECT` all three of its server pids are replaced, which also proves "every database".
  - R-1238 states those instances' `max_prepared_statements` (such as 0 on all three) and that morm's pools on them set `statement_cache = 0` and `shape_cache = 0`, or sets 200 on 1.22 and 1.26.1 as elsewhere.

### EL5. Minor. T-0311, PR body: the new "`UPDATE` matching no row" case can't reach the retry as written

- **Where:** T-0311: "an `UPDATE ... SET c = f($1)` retry that succeeds on a fresh connection, matching no row, confirms and raises the generation". PR body, EK4: "such as an `UPDATE` that matches nothing".
- **What's wrong:** for the retry to happen, the first run on the stale session must raise the `22P02`. An `UPDATE` matching no row never evaluates its `SET` list, so a plain plpgsql `f`, which is `VOLATILE`, is never called. On 18.6, with a `DECLARE v st` function compiled before a relabel swap, `UPDATE r8.u SET c = r8.fv($1) WHERE id = -1` returned `UPDATE 0` on the stale session with no error, while `WHERE id = 1` gave the `22P02`. Only an `IMMUTABLE` `f`, which the planner folds with the bound value, raised it with no row matched. A builder following the text gets a case that never retries.
- **Fix:** "an `UPDATE ... SET c = f($1)` matching one row, whose retry on a fresh connection succeeds and returns only its command tag, confirms and raises the generation". That's EK4's real case. Or keep "matching no row" and say `f` is `IMMUTABLE`, so the planner calls it. Correct the PR body's example to match.

### EL6. Minor. R-1040, R-1250, R-1257: EK2's and EK3's wording didn't reach the other places that describe the send

- **Where:**
  - R-1257's "use this when" line: "list every PgBouncer, and read the notice first".
  - R-1250: "list every PgBouncer's admin console in `[migrations] pgbouncer_admin_urls` and `migrate` sends each one `RECONNECT` after every migration".
  - R-1040's decision text: "at the end of the migration when `[migrations] pgbouncer_admin_urls` lists it".
- **What's wrong:** R-1258 now says "every PgBouncer process", warns about `so_reuseport`, and says `RECONNECT` is also sent "when a run stops partway after one of its steps has committed" (EK2, EK3). The other three still say "every PgBouncer" and "after every migration" only. A reader of the shorter line can list one `so_reuseport` address, or expect no send after a stopped cutover.
- **Fix:** R-1257: "list every PgBouncer process"; R-1250: "every PgBouncer process's admin console ... after every migration, and when a run stops after a committed step"; R-1040: "at the end of the migration, or when the run stops after a committed step, when ... lists it".

### EL7. Minor. R-1238, R-1256, PR body: this fix's own choices are attributed wrongly or not named

- **Where and what's wrong:**
  - INDEX's EK entry names two choices "the owner approves through PR #3": "the `RECONNECT` test gets its own PgBouncer 1.20.1, 1.22 and 1.26.1 instances; the `so_reuseport` check runs only after a `RECONNECT` over TCP".
  - R-1238 labels the first an "Owner decision on EK6" rather than a choice of this fix.
  - R-1256's list of "this fix's choices" doesn't name the second, and the PR body's choices list (1 to 20) doesn't have it.
  - The PR body's EJ5 bullet still says "such a `22P02` stays `SchemaChanged`, so `.retry(n)` can reach another server session", and choice 16 cites EJ5, with no note that EK1 refined it, as the EG3 and EH5 bullets have.
- **Fix:**
  - R-1238 ends its bullet with "with this fix's choice the owner approves through PR #3: the `RECONNECT` test's own instances".
  - R-1256's choices add "the `so_reuseport` check only after a `RECONNECT` over TCP".
  - The PR body adds that choice as 21, and "Refined by EK1: the statement then enters the cool-down" to the EJ5 bullet.

## What I couldn't run

- **morm itself.** There is no runtime yet. EL1's marker move, EL2 and EL4 come from the rules' text; EL1's error shapes, EL3 and EL5 come from Postgres and PgBouncer themselves.
- **Postgres 15 to 17.** I ran 18.6 only. The `enum_in` messages and routine EL1 relies on are the ones proof 14 and T-0151 already pin on 15 to 18.
- **`so_reuseport` on 1.20.1 and 1.22.** I ran the two-process case on 1.26.1. 1.20.1 and 1.22.1 report the setting in `SHOW CONFIG` the same way.
- **PgBouncer 1.22 from the runner's apt sources.** I built 1.22.1 from pgbouncer.org's tarball. CI's apt build may be another 1.22 patch release.

## Verdict

**Not approved yet.**

What holds:

- every changed ID is in INDEX's EK entry at `1887f07`, and the PR body's ID lists match;
- the checker reports 0 problems, and CI is green on the head;
- EK1 is fixed as the owner decided, and EK2 to EK8 as the seventh review recommended;
- the console's refusal of the extended protocol, `SHOW CONFIG`'s `so_reuseport` row, healing each `so_reuseport` process through its own Unix socket, and the one-connection "nothing sent" check all hold on real PgBouncers;
- straight to Postgres the rule set stays sound, as the seventh review found;
- behind a pooler, the retired-label `22P02` now costs at most one `SchemaChanged` per statement per minute and clears no cache, and a real stale session heals through `RECONNECT`, every `so_reuseport` process and a stopped run included, or through `server_lifetime`.

What remains:

- **Blocks approval:** EL1. Inside a transaction behind a pooler, a client's enum label that a statement converts to another enum moves the marker and clears every connection's caches on every call, against R-0982 and SF. This needs a design choice; (a) is recommended.
- **Should fix:**
  - EL2: `42804` and `XX000` from a real stale session still clear caches behind a pooler inside a transaction (a design choice; (a) recommended);
  - EL3: `so_reuseport` works only from the PgBouncer's host, unsaid, and the Unix-socket branch is untested;
  - EL4: the `RECONNECT` instances contradict EA, T-0316 lost "every server session", and their cache settings are unstated.
- **Minor:**
  - EL5: T-0311's "`UPDATE` matching no row";
  - EL6: R-1040, R-1250 and R-1257's wording;
  - EL7: attribution of this fix's choices.
