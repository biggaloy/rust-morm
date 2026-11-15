# Step 55 independent review, eleventh round

PR #3 (`step-55-enum-proofs`), head `c4b4ce2`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes. This round covers the fix for the tenth review's EN1 to EN5 and the owner's stale-session settings, `git diff a463237 origin/step-55-enum-proofs` (one commit, `c4b4ce2`), and checks that nothing earlier regressed.

The owner decided:

- behind a pooler, while a confirmation is pending, runs with other values go unnamed;
- pending checks expire after 60 seconds;
- every stale-session tunable is a per-pool `morm.toml` setting with today's value as its default, an env override and a "use this when" line;
- a value that turns a protection off gets a loud notice, or is refused if it would allow unbounded churn.

## What I read

- **On the PR branch:** `rules.md`, `contracts.md`, and `proofs.md` (step 55's section, the EH6 and EM3 bullets and the new "tenth review" bullets). I read every changed and new ID in full, old and new: R-0060, R-0981, R-0984, R-1246, R-1253, R-1258, R-1259 to R-1268, T-0306, T-0312, T-0314 and T-0317. For the rule set I also re-read R-0718, R-1238, R-1245, R-1040, R-1249, R-1250 and step 48's deferred-measurement list.
- **On `origin/main`:** INDEX's Decisions entries "Step 55 ninth review, EM1 to EM6" and "Step 55 tenth review, EN1 to EN5", with the EN IDs, the settings and the fix's own choices, and `reviews/step-55-review-10.md`.
- **The PR body.**

## What I checked and ran

- **CI on the head.** `c4b4ce2` is green: `ci` on push (run 43687675581) and on the pull request (run 43687680340), 36 of 36 checks passing. The diff touches only `plans/plan-approved/`: `coverage.md`, `proofs.md`, `manifest.json`, and steps 10, 11, 12, 14, 20, 27, 47 and 50. No code, test or workflow changed.
- **Coverage checker,** run on a `git archive` of the head:
  - `check_coverage.py --step 55 --code crates tests benches fuzz .github`: **1268 requirements, 317 test obligations, 0 problems**;
  - the plain form gives the same.
- **Manifest against INDEX.** I compared the manifests at `a463237` and `c4b4ce2` ID by ID:
  - changed: R-0060, R-0981, R-0984, R-1246, R-1253, R-1258, T-0306, T-0312 and T-0314;
  - new: R-1259 to R-1268 and T-0317;
  - none removed, and `frame` and `master_sha256` unchanged.

  That's exactly INDEX's EN entry, "Written in at `c4b4ce2`", and the PR body's list.
- **R-0060's copies.** The `morm.toml` block in steps 11, 12, 27 and 47 is byte for byte R-0060's, including the five new lines. The env variables `MORM_POOL_STALE_*` follow R-0088 to R-0100 and R-1118, and R-0101 applies them to the startup pool as for every other pool setting.
- **`test-hooks`.** Nothing in `crates/`, `tests/`, `.github/` or `plans/plan-approved/steps/` names it any more. The only mention left is `proofs.md`'s EH6 history bullet, which now says "Superseded ... the feature is gone".
- **Spot checks on throwaway Postgres 18.6,** a cluster under `.claude/scratch/` in my worktree with `TMPDIR` set there and its socket under `~/.claude/scratch/`. The client was asyncpg 0.31.0 and `psql`. I tried to build PgBouncer 1.26.1 from pgbouncer.org's tarball, whose SHA-256 `00886a29…3f42d9` matched the published `.sha256`, but running the build was refused in this session, so the PgBouncer side below rests on PgBouncer's transaction-mode routing and on the plan's own statements about it.

  | Claim | Result on 18.6 |
  |---|---|
  | A plpgsql function with `DECLARE v st`, compiled in session A; then by hand `ALTER TYPE st RENAME TO st_old; CREATE TYPE st ...; DROP TYPE st_old` | Session A: `SELECT f($1::text)` gives `XX000` `cache lookup failed for type 16385` both prepared and unnamed. Session B, which hadn't called `f`, gives `a` unnamed, and `d` for a label only the new type has. See EO1 |
  | Same text and same parameter bytes can fail, then succeed, after the client changes its own row | `PREPARE u2(int, text, kind) AS UPDATE t SET o = conv($2) WHERE id = $1 AND k = $3`, where `conv` returns `p::other`, run as `EXECUTE u2(1,'zzz','a')`: first `22P02` `invalid input value for enum other: "zzz"` while row 1 has `k = 'a'`, then `UPDATE 0` once row 1's `k` is `'b'`, then `22P02` again when the client sends `'b'`. `enum_in` is `STABLE` (`provolatile = s`), so the conversion runs only for a matched row. See EO2 |

## EN1 to EN5: are they fixed?

| Finding | Verdict and evidence |
|---|---|
| EN1 | **Fixed as the owner decided.** R-1246's "Other values meanwhile": "Behind a pooler, while an entry exists for a statement's text, its runs with other parameter bytes go unnamed too ... but they confirm nothing and don't touch the entry. Straight to Postgres they run as usual." R-1253's title is now "can fail calls until each process confirms it", and its body states the once-per-expiry failure and the old-order window. R-1258 says "once a run of the same call has failed again". T-0312 adds the different-values case, the shared-label order and the expiry. Gaps: what an other-values unnamed run does when it itself ends in a stale signal (EO7), and the multi-server-session case (EO1) |
| EN2 | **Fixed as recommended, and the lifecycle reads the same everywhere it's stated.** R-1246 defines it in three sub-bullets: bounds and clearing ("the pool's statement generation moves, a confirmation's salt included, or `<n>` or the connection generation moves"), one-shot removal ("takes the entry out of the set, so only one run confirms it"), and the three outcomes. R-0984 points to step 14 for them. `proofs.md`'s EM3 and EN2 bullets and the PR body's EN2 bullet and choices 23 and 24 use the same words. T-0312 tests each clearing trigger on its own, "ten more runs with the same values raise the statement generation no further", the `c` third outcome and a `statement_timeout` cancel. One sentence in R-0984 lags EN4 (EO9) |
| EN3 | **Fixed in R-1246's cost bullet,** word for word as recommended. But the benchmark it promises isn't in step 48, so nothing builds it (EO8) |
| EN4 | **Fixed.** R-1246: "inside a transaction a plpgsql `XX000` then enters step 14's cool-down ... and outside one it goes on to R-0981's retry on another connection, as R-0718 says". T-0306 adds the outside-a-transaction case reaching a fresh server session |
| EN5 | **Fixed as the owner decided:** `stale_pending_expiry`, `"60s"` by default. But no test shows the protection itself: a same-values run after the expiry adds no salt (EO6). A long expiry also widens the window again (EO4) |

The settings:

- R-1262 defines the five settings and their bounds.
- R-1259 to R-1261 add the env variables, R-1263 to R-1267 the "use this when" lines, and R-1268 the loud notice.
- T-0317 tests the defaults, the overrides and the refusals at start.
- T-0314 now shortens the cool-down with `stale_cooldown = "1s"`.

Names, env variables and placement match the existing pool settings. The bounds have gaps in both directions (EO3, EO4), the pending-off notice is wrong for straight connections (EO5), and two of the settings' effects are never tested (EO6).

Nothing earlier regressed in the text: SA to SH, EF1, EG1 to EG3, EH1 to EH8, EI1 to EI9, EJ1 to EJ9, EK1 to EK8, EL1 to EL7 and EM1 to EM6 read consistently with it. EO1 and EO2 sit in rules that EM1 and EL1 introduced, and this round's changes widen both. Earlier rounds didn't see them: EO1 needs more than one server session, and EO2 needs a repeated cycle.

## The rule set, path by path

Straight to Postgres:

| Path | Heals? Bounded? |
|---|---|
| Retired-label `22P02`, stored or client text, `42804`, `XX000` | As in the seventh to tenth reviews: a new connection confirms, and stored or client text costs at most the connections open at its first failure, again after each migration. **But** with `stale_cooldown_max` or `stale_first_failure_max` set to `1`, two such statements in turn cost a connection on every call (EO3) |
| Sixth-row `22P02`, a hand-made swap | The pending run, prepared again, raises the statement generation. Heals |
| Sixth-row `22P02` from a client's label | Fails its pending run and nothing moves, **unless the client changes its own row between the two calls**: then the same bytes succeed and every cached statement in the process is dropped, as often as the client repeats it (EO2) |

Behind a pooler:

| Path | Heals? Bounded? |
|---|---|
| Retired-label `22P02`, `42804` | One `SchemaChanged`, then the cool-down; no cache cleared. Heals through `RECONNECT` or `server_lifetime`. Sound |
| Hand-made swap's stale statement, same values (EM3) | One `SchemaChanged` per process, then a salt. Heals |
| The same with other values (EN1) | Right answers, run unnamed, one `SchemaChanged` per process per `stale_pending_expiry`; heals at the next same-values call, outside a transaction, or by PgBouncer. Bounded and stated in R-1253 |
| `XX000` from plpgsql in **some** server sessions, outside or inside a transaction | **Not sound.** The unnamed or pending run is a new transaction, so PgBouncer can hand it a server session where the function compiles fresh. It then succeeds, and morm takes it for a stale statement: a salt is added, the statement generation raised, and every connection's caches dropped. This repeats on each call that reaches a stale session, until `server_lifetime` (EO1) |
| Sixth-row `22P02` from a client's label | The same client-made cycle as straight (EO2) |
| A stale server session after a migration | `RECONNECT` or `server_lifetime`. Unchanged |

Across all these paths, nothing grows past `max` or the budget, because every unnamed or pending run reuses its own connection. Every set is bounded by its setting, though the settings themselves have no ceiling (EO4). No path gives a silent wrong answer beyond what R-1253 and R-1268 state.

**The rule set isn't sound yet.** EO1 lets one stale server session clear every process's caches on every call that reaches it. EO2 lets a client clear a process's caches as often as it repeats a three-call cycle.

## Findings

### EO1. Blocks approval. R-1246, R-0718, R-1245, R-0984, R-1258, T-0306, T-0312: behind PgBouncer, the confirming unnamed run can reach a different server session, so plpgsql left stale in one session is taken for a stale statement, and every call that reaches that session clears every cache

- **Where:**
  - R-1246's first bullet: "Behind a pooler an `XX000` `cache lookup failed for type` from a cached statement is handled here as well ...: if its unnamed run succeeds, it was a stale statement, as above; if it fails, it came from plpgsql". R-1245's third row and R-0718 say the same.
  - R-1246's "confirming run", in a new transaction after the rollback: it "confirms when it succeeds ... or ends in a server error other than the first one".
  - R-1258: "morm keeps its statement caches meanwhile".
- **What's wrong:**
  - In transaction mode PgBouncer assigns a server connection per transaction. The failed cached run and the unnamed run that follows it outside a transaction are two transactions. A pending run is a later transaction too. Each can reach a different server session.
  - The plan says so itself:
    - R-1238 sets `server_round_robin = 1` "so a test's clients reach several server sessions, as production does", and notes that "a pool of one hid a failure that only shows when a client's round trips reach different server connections";
    - R-0981 says the retry on another connection "may reach a fresh server session";
    - T-0306's new sentence relies on one.
  - The spot check shows the Postgres side. After the type is dropped by hand, session A gives `XX000` for the function both prepared and unnamed, while session B, which hadn't compiled it, answers unnamed.
  - So when plpgsql is stale in some server sessions and not others, the call fails in A and its unnamed run succeeds in B. Each of these happens:
    - morm adds a salt and raises the statement generation, so every connection drops its cached statements, every pending entry is cleared, and the statement never enters the cool-down;
    - the next call that reaches A does it all again.

    That's a full cache clear on every call that reaches a stale session, in every process, until PgBouncer replaces A, an hour by default. Pending entries are cleared each time, so a real hand-made stale statement elsewhere loses its confirmation too.
  - The common way to get here is a migration whose unmanaged function morm couldn't refresh (R-1250), followed by `purge` dropping the retired type, with no `pgbouncer_admin_urls`. R-1258's notice then says the opposite: "morm keeps its statement caches meanwhile".
  - The same holds for the sixth-row `22P02` when it comes from plpgsql compiled against a type renamed by hand, and for EN2's third outcome, an unnamed run in B ending in another error.
  - T-0306's and T-0312's `DECLARE v st` cases can't see this. Their server sessions all compiled the function before the change, so the unnamed run fails wherever it lands.
- **Evidence:** the spot check's first row; R-1238's text; transaction-mode assignment per transaction.
- **Fix, a design choice for the owner, since EM1 decided that one unnamed run tells the two apart:**
  - **(a) Recommended. Confirm only once per window, and never from fresh text.**
    - With `<n>` unmoved, a pool adds a salt, or straight raises the statement generation for a pending confirmation, at most once per `stale_cooldown`.
    - A stale signal from a cached statement prepared under the salt the pool added last isn't a stale statement, because that text was prepared after the change. It takes the plpgsql path: the cool-down, no salt.
    - A confirmation that comes inside the window still returns the run's own result, which is the right answer, and moves nothing.
    - This bounds the churn at one cache clear per process per `stale_cooldown`, and it also closes EO2.
    - R-1246, R-0984, R-1245's row and R-0718 say so. R-1258's caches sentence and R-1253 state the rate.
  - **(b) Confirm inside one server session.** Run the cached statement and the unnamed run in one transaction, with a savepoint between them, so PgBouncer keeps the same server session. A stale statement then shows as "cached fails, unnamed succeeds" in the same session.
    - The cost is a few round trips on the rare path.
    - Statements that can't run in a transaction block, such as a `CALL` of a procedure that commits, need a rule of their own, because their `2D000` would otherwise count as "another error" and confirm.
  - Either way, T-0306 and T-0312 add the following, behind PgBouncer with `server_round_robin = 1` and at least three server connections, where one server session never called the `DECLARE v st` function before the drop: calls outside and inside transactions raise the statement generation at most as the rule allows, and the plpgsql `XX000` reaches the cool-down.

### EO2. Should fix. R-1246, R-0984, R-1262: a client can make the same text and parameter bytes fail and then succeed by changing its own row, and each such pair clears every cache in the process, as often as it likes

- **Where:**
  - R-1246's confirming run "confirms when it succeeds, with or without rows, or ends in a server error other than the first one". R-0984 adds the straight-to-Postgres re-prepare, which "raises" the statement generation.
  - R-1246's bounds: "A data change long after the failure therefore can't be taken for a stale statement."
- **What's wrong:**
  - The spot check shows a common statement shape where the conversion runs only for a matched row: `UPDATE t SET o = conv($2) WHERE id = $1 AND k = $3` with `$3::kind`. It qualifies as a sixth-row statement: it casts a parameter to a non-built-in type, and the `22P02` names `other`. With row 1's `k` at `'a'`, `(1,'zzz','a')` gives `22P02`; after the client sets its own row's `k` to `'b'`, through any ordinary call, the same bytes give `UPDATE 0`.
  - So a client repeats three calls: the bad label, a change to its row, the bad label again. Each round confirms a "stale statement".
    - Behind a pooler, that adds a salt and raises the statement generation.
    - Straight, it raises the statement generation.
    - Either way every connection in the process drops its cached statements.
  - EN5's expiry limits how long a round may take, 60 seconds, not how many rounds there are. EN2's third outcome widens it: an error other than the first one, such as a check or unique violation the changed row reaches, confirms too.
  - The tenth review rated EN5 minor because "it needs that kind of function". The spot check shows it needs only a per-row conversion and a row the client can change, so client-triggerable cache clearing at the client's request rate remains, straight and behind a pooler.
- **Evidence:** the spot check's second row; R-1246's and R-0984's text.
- **Fix, a design choice for the owner:**
  - **(a) Recommended.** EO1's (a): with `<n>` unmoved, at most one confirmation-driven salt or statement-generation raise per pool per `stale_cooldown`. A client then costs at most one cache clear per process per minute, and a real hand-made change still heals on its first confirmation. R-1246 states it, and T-0312 adds the three-call cycle repeated ten times within a minute, raising the generation once.
  - **(b)** Confirm only on success with rows or a command tag that touched a row, not `UPDATE 0`. That narrows the case but doesn't close it: a different error, or a row the client can create, still confirms.

### EO3. Should fix. R-1262, R-1268, R-1264, R-1267: `stale_cooldown_max = 1` and `stale_first_failure_max = 1` are allowed, and with two statements fed such text each costs a connection on every call, the churn the refusal of `0` exists to stop

- **Where:** R-1262: `stale_cooldown_max` "at least `1`; `0` is `Error::Config`, for the same reason", and `stale_first_failure_max` "at least `1`; `0` is `Error::Config`, because with no record stored text in transactions would cost a discarded connection on every call". R-1268: "without them text holding a retired label would open or discard a connection on every call".
- **What's wrong:**
  - Both sets drop the oldest entry.
  - With a size of 1, two statements failing in turn evict each other. Each failure of statement 1 is then a first failure again:
    - outside a transaction, a new connection opened and closed (R-0981);
    - inside one, straight to Postgres, a connection discarded (R-0984).

    That happens on every call. A cutover with stored retired labels read by two functions is enough.
  - R-1264 says as much for 65 statements at the default. A floor of 1 makes it a two-statement case, so the bound refuses `0` but allows the same churn one step up. The owner's rule is "refused if it would allow unbounded churn".
- **Fix, a design choice for the owner:**
  - **(a) Recommended.** Raise both floors to the default, `64`, and refuse lower values with `Error::Config`. Larger values stay allowed for apps with more such statements.
  - **(b)** Keep `1`, and R-1268 says loudly that a value below the number of statements that can be fed such text at once costs those statements a connection on every call.

  Either way, T-0317 tests the new floor.

### EO4. Should fix. R-1262, R-1268, R-1265: no setting has a ceiling; a large `stale_pending_max` lets a client fill memory, a long `stale_pending_expiry` reopens EN5's window, and a long `stale_cooldown` delays a real stale session's healing by as long, and the notice names only one of these

- **Where:** R-1262's "Allowed" column: `stale_pending_expiry` "`0` or more", `stale_pending_max` "`0` or more", the others "at least". R-1268: "A longer `stale_cooldown` makes a statement whose confirming connection Postgres refused wait longer before morm tries it again." R-1265's use-when encourages "more than 60 seconds".
- **What's wrong:**
  - **Pending entries are keyed by parameter bytes the client chooses.** Each bad label in a transaction adds one entry, so with `stale_pending_max = 100000000` a client can fill the set: memory grows with client input, up to the setting. The memory and first-failure sets are keyed by statement text, which the app bounds; this one isn't.
  - **`stale_pending_expiry`.** A value of hours brings back EN5's "data change long after the failure", and EO2's window grows with it. Behind a pooler, a client's bad label also makes every caller's runs of that statement unnamed for that long (EO7). R-1268 names neither.
  - **`stale_cooldown`.** A value of days keeps a remembered statement's error returning at once, with no retry on a new connection, until the connection generation moves. Straight to Postgres, a real stale session that was being fed such text at a cutover then fails that long. R-1268 mentions only the refused-connection case, not R-0981's first case, "a statement that was being fed such text when a cutover happened", or step 20's transactions.
- **Fix:**
  - R-1262 gives each setting a ceiling, refused with `Error::Config` naming the key and the bound. For example: `stale_cooldown` and `stale_pending_expiry` at most `"1h"`, PgBouncer's default `server_lifetime`; each `_max` at most `65536`.
  - R-1268 says what a longer `stale_cooldown` and a longer `stale_pending_expiry` cost, in the terms above.
  - T-0317 tests each ceiling.

### EO5. Should fix. R-1268, R-1262, T-0312, PR body: what turning pending checks off does is misstated straight to Postgres, never defined behind a pooler, and tested by an outcome no requirement gives

- **Where:**
  - R-1268: "Then, behind PgBouncer or straight, a statement a change made by hand left stale, run only inside transactions, heals only when it runs outside one, `.retry(n)` doesn't heal it". R-1262: "inside a transaction a hand-made change heals only when the statement runs outside one".
  - T-0312: "with `stale_pending_expiry = 0` no pending check is kept and `.retry(2)`'s second attempt returns its error as it is".
  - PR body: "the two allowed at `0` only slow healing".
- **What's wrong:**
  - **Straight to Postgres the notice is wrong.** The sixth-row `22P02` "evicts that one statement" (R-0984), so each connection fails once and prepares fresh text, and `.retry(n)` heals once its attempt reaches a connection that has already failed. It heals within the pool's size, as the tenth review's straight table found, not "only when it runs outside one". An `XX000` from a cached statement straight is a certain signal (R-0982) and needs no pending check at all.
  - **Behind a pooler, no requirement says what the call returns.** With pending checks off, the stale signal inside a transaction gives `SchemaChanged` again on every attempt, or some other result. T-0312 asserts "returns its error as it is", which reads as `Error::Postgres` and which nothing in R-1246 or R-0984 gives. It also doesn't say which statement it means.
  - **The PR body contradicts R-1268.** R-1268 warns that a kept type's shared label runs on the old order with no error, which is more than slow healing.
  - **`stale_pending_max = 0`'s behaviour is untested.** T-0317 only starts a pool with it.
- **Fix:**
  - R-1246 adds a sentence for checks turned off. Behind a pooler, inside a transaction, the signal returns `Error::SchemaChanged` on every call, as before EL1, or whatever the owner prefers; straight to Postgres, each connection evicts its own copy as R-0984 says.
  - R-1262 and R-1268 limit the "heals only when ..." sentence to behind PgBouncer, and say that straight it heals as each connection fails once.
  - T-0312 asserts the outcome R-1246 gives, for both `stale_pending_expiry = 0` and `stale_pending_max = 0`, straight and behind PgBouncer.
  - The PR body says what R-1268 says.

### EO6. Should fix. T-0312, T-0317: a build that hard-codes 64 entries, or never expires a same-values entry, passes every test

- **Where:**
  - T-0312: "a 65th pending confirmation drops the oldest", "a 65th recorded first failure drops the oldest record", and the expiry tested only through other values ("until the pending check expires ... after which the next failing call gives `SchemaChanged`").
  - T-0317 tests start-up, overrides and refusals only.
- **What's wrong:**
  - `rules.md` says "A requirement that holds several rules needs tests for each". R-1262 says each setting "governs" its number, but only `stale_cooldown` and `stale_cooldown_max` are tested at a non-default value (T-0314's `"1s"` and `2`). For `stale_pending_max` and `stale_first_failure_max`, only the default of 64 is ever exercised, so a build that ignores the setting passes.
  - EN5's own protection is never tested: after the expiry, a run with the same values that now succeeds adds no salt. A build that expires entries only for the other-values rule passes T-0312.
- **Fix:** T-0312 adds:
  - with `stale_pending_max = 2`, the third pending confirmation drops the oldest;
  - with `stale_first_failure_max = 2`, the third record drops the oldest;
  - with `stale_pending_expiry = "1s"`, a same-values run after two seconds that succeeds raises no statement generation and adds no salt.

### EO7. Minor. R-1246, R-1258: an other-values unnamed run that itself ends in a stale signal has no defined outcome, and the cost of a client's label is understated

- **Where:**
  - R-1246's "Other values meanwhile" and R-0984's signal list, which apply to an `XX000` or `22P02` "from a cached statement".
  - R-1258: "once a run of the same call has failed again, its error at once".
  - R-1246's summary: "So a client's label costs one unnamed run per call and never clears a cache."
- **What's wrong:**
  - While an entry exists, a plpgsql `XX000` called with other values now runs unnamed and fails again. That run isn't from a cached statement, so no rule says whether it returns `SchemaChanged`, returns `Error::Postgres`, enters the cool-down, or, outside a transaction, goes on to R-0981's retry. R-1258's "once a run ... has failed again" depends on that.
  - The same goes for a client's bad label with other bytes.
  - A client's label now also makes every caller's runs of that statement unnamed for up to `stale_pending_expiry`. That's one unnamed run per call of that statement in the process, not per client call. It's the owner's decision, but R-1246's summary and the README don't say it.
- **Fix:**
  - R-1246: an other-values unnamed run that ends in R-1245's plpgsql signal counts as that statement's failed confirmation. It enters the cool-down, returns as `Error::Postgres` inside a transaction, and goes on to R-0981's retry outside one. Any other error returns as it is.
  - The summary sentence says "and, behind a pooler, runs that statement unnamed for every caller for up to `stale_pending_expiry`". R-1253 or R-1268 carries the same line.
  - T-0312 adds the other-values plpgsql case.

### EO8. Minor. R-1246, step 48: EN3's benchmark is promised in step 14's text but missing from step 48, which only reads its own file

- **Where:** R-1246: "Step 48's benchmarks cover the cached-statement path with the count at zero." Step 48's list, "Other steps defer these measurements here: ...", and R-0717 don't include it. `rules.md`: a step session reads its own step file "and nothing else", and "Performance claims are measured in step 48".
- **What's wrong:** the hot-path claim EN3 added is never measured, because step 48's session won't see it.
- **Fix:** add the pending check, with the count at zero against no check, to step 48's deferred list and as a step 48 requirement (new ID, `coverage.md` and manifest), with an INDEX note.

### EO9. Minor. R-0984, R-1040, R-1249, R-1250, R-1253, R-1262: text that still states the old fixed numbers, or lags EN4

- **Where and what:**
  - R-0984: "stored or client text costs at most one `SchemaChanged` per statement per minute". R-1253: "can fail once a minute per process". R-1040, R-1249 and R-1250: "returns at once for 60 seconds" and "for up to 60 seconds". Now that these are settings, each should say "per `stale_cooldown`" or "per `stale_pending_expiry`", "60 seconds by default", as R-0981 and R-1258 now do.
  - R-0984: "when it fails the same way, the error returns as it is ..., and a plpgsql `XX000` enters step 14's memory". R-1246 now sends it to R-0981's retry when the pending run is outside a transaction (EN4). R-0984 should add "inside a transaction", or point to R-1246 as it does for the bounds.
  - R-1262: "Every number the stale-session rules use is a pool setting". INDEX's EN entry says "the retry count and hash and salt widths are design, not settings". It should say "every tunable", naming the five.

### EO10. Minor. R-0060, R-1262, T-0317: `stale_pending_expiry` turns off with `0`, while every other pool duration that can be turned off takes `"off"`

- **Where:** R-0060: `max_lifetime = "off"`, `statement_timeout = "off"`, `slow_query ... "off"`, and `wait_timeout ... can't be off`, against `stale_pending_expiry = "60s" ... 0 turns that off`. T-0317 uses `stale_pending_expiry = 0` (an integer) and `stale_cooldown = "0"` (a string).
- **What's wrong:** a user who writes `stale_pending_expiry = "off"` by the file's own convention, or `stale_cooldown = "off"`, gets behaviour no requirement states. It's also unclear whether a duration key accepts a bare integer.
- **Fix:**
  - R-1262: `stale_pending_expiry` takes `"off"`, and `0` or `"0s"` as the same.
  - `stale_cooldown = "off"` is `Error::Config`, as `wait_timeout` "can't be off".
  - R-0060's comment says `"off"`.
  - T-0317 tests both spellings.

## What I couldn't run

- **PgBouncer.** Running its build was refused in this session, so EO1's routing rests on transaction-mode PgBouncer assigning a server connection per transaction, and on R-1238, R-0981 and T-0306 saying a client's round trips reach different server sessions. The Postgres side ran on 18.6.
- **Postgres 15 to 17.** I ran 18.6 only. Both spot checks rest on long-standing plpgsql caching and `enum_in`'s volatility.
- **morm itself.** There is no runtime yet. Every outcome under the rules comes from their text.

## Verdict

**Not approved yet.**

What holds:

- every changed and new ID is in INDEX's EN entry at `c4b4ce2`, and the PR body's lists match;
- the checker reports 0 problems, and CI is green on the head;
- EN1 to EN5 are written in as decided or recommended, and EN2's lifecycle is complete and stated the same in R-1246, R-0984, `proofs.md` and the PR body;
- the settings have consistent names and env variables, R-0060's copies match, and `test-hooks` is gone;
- nothing passes `max` or the budget, and no path gives a silent wrong answer beyond what the README states.

What remains:

- **Blocks approval:**
  - EO1: behind PgBouncer, plpgsql left stale in one server session passes for a stale statement when the unnamed run reaches another session, and then every call that reaches it clears every cache until `server_lifetime` (a design choice; (a) recommended).
- **Should fix:**
  - EO2: a client clears a process's caches as often as it repeats a three-call cycle on its own row (a design choice; (a), the same rate limit as EO1's, recommended);
  - EO3: floors of `1` allow per-call churn with two statements (a design choice; (a) recommended);
  - EO4: no ceilings, and the notice names only one cost;
  - EO5: pending-off is misstated straight, undefined behind a pooler, and tested against no rule;
  - EO6: two settings and EN5's protection are untested at non-default values.
- **Minor:**
  - EO7: an other-values unnamed run's own stale signal, and the cost of a client's label;
  - EO8: EN3's benchmark isn't in step 48;
  - EO9: text still stating fixed numbers, and R-0984 lagging EN4;
  - EO10: `0` against `"off"`.
