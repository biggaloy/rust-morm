# Step 55 independent review, ninth round

PR #3 (`step-55-enum-proofs`), head `b10fa3f`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes. This round covers the fix for the eighth review's EL1 to EL7, `git diff 1887f07 origin/step-55-enum-proofs` (one commit, `b10fa3f`), and checks that nothing earlier regressed. The owner decided two things:

- **EL1:** a client-triggerable `22P02` is confirmed through `.retry(n)`'s next attempt, and the marker moves only if that attempt succeeds.
- **EL2:** behind a pooler, `42804` and `XX000` follow EK1: no marker, the cool-down, and healing through `RECONNECT` and `server_lifetime`.

## What I read

- **On the PR branch:** `rules.md`, `contracts.md`, `proofs.md` (step 55's section, the new "eighth review" bullets, and proofs 14 and 16 again), and every changed requirement and test in full: R-0982, R-0984, R-1040, R-1238, R-1245, R-1246, R-1250, R-1253, R-1256, R-1257, R-1258, T-0311, T-0312 and T-0316. For the rule set I also re-read R-0718, R-0981, R-0983 and R-1249.
- **On `origin/main`:** INDEX's Decisions entries "Stale-statement design, SA to SH", "Step 55 seventh review, EK1 to EK8" and "Step 55 eighth review, EL1 to EL7", including the EL IDs and the fix's own choices, and `reviews/step-55-review-8.md`.
- **The PR body.**

## What I checked and ran

- **CI on the head.** `b10fa3f` is green: `ci` on push (run 43634569011) and on the pull request (run 43634573392), 36 of 36 checks across the two. The diff since `1887f07` touches only `plans/plan-approved/` (7 files). No code, test or workflow changed.
- **Coverage checker,** on the head:
  - `check_coverage.py --step 55 --code crates tests benches fuzz .github`: **1258 requirements, 316 test obligations, 0 problems**;
  - the plain form gives the same.
- **Manifest and INDEX.** I compared the manifests at `1887f07` and `b10fa3f` ID by ID:
  - R-0982, R-0984, R-1040, R-1238, R-1245, R-1246, R-1250, R-1253, R-1256, R-1257, R-1258, T-0311, T-0312 and T-0316 change;
  - none is added or removed, and `frame` and `master_sha256` are unchanged.

  That's exactly INDEX's EL entry, "Written in at `b10fa3f`", and the PR body's list. Against `origin/main` the set of changed or new IDs is the same 75 as at `1887f07` (30 changed and 18 new requirements, 11 changed and 16 new tests), all covered by INDEX's step 55 entries.
- **Spot checks, on throwaway servers.** I built PgBouncer 1.26.1 from pgbouncer.org's `pgbouncer-1.26.1.tar.gz`. Its SHA-256, `00886a2994b4844cd37110970df777772cfc42a74c400522c82fba509a3f42d9`, matched the published `.sha256` file. One instance ran in front of each of Postgres 18.6 and 17.11, in transaction mode with `max_prepared_statements = 200` and one server connection. Everything ran under `.claude/scratch/` in my worktree with `TMPDIR` set there, apart from the Unix sockets, which sat in a short folder under `~/.claude/scratch/`. The client was asyncpg 0.31.0: `prepare` gives a named statement, and `statement_cache_size = 0` gives unnamed ones.

  | Claim | Result, the same on 18.6 and 17.11 |
  |---|---|
  | EL1: a client's valid label of one enum, converted to another enum, gives the `22P02` R-1245's sixth row matches, and the unnamed confirming run fails too | Yes. Behind PgBouncer, a cached `SELECT (r9.conv((($1::text)::r9.kind)))::text` gave `22P02` `invalid input value for enum r9.other: "b"` (`enum_in`) for `b`, and so did the same text run unnamed. So under EL1 a client's label returns its error on `.retry(n)`'s second attempt and moves nothing |
  | EL2's premise, "new text doesn't cure" `XX000` `cache lookup failed for type` | **Only for plpgsql.** After a hand-run swap that dropped the old type (rename `st` to `st_old`, create `st`, drop `st_old`), a cached `SELECT (($1::text)::r9.st)::text` gave `XX000` `cache lookup failed for type <n>` (`getTypeOutputInfo`) on every run. A new client preparing the same text through PgBouncer reached the same server session and got the same `XX000`. The same text run unnamed, and the text with a new `/* morm g0.1 */` marker, both returned the right answer. A plpgsql `DECLARE v r9.st` function in that session kept giving `XX000` (`getTypeInputInfo`) when run unnamed. Straight to Postgres, a new connection cured the cached statement. See EM1 |
  | A hand-run swap that keeps the old type, inside transactions behind PgBouncer, with the same text prepared again in each transaction | Three transactions in a row each gave `22P02` `invalid input value for enum r9.st_old: "d"`, and new text in a fourth returned `d`. See EM3 |

## EL1 to EL7: are they fixed?

| Finding | Verdict and evidence |
|---|---|
| EL1 | **Fixed as the owner decided.** R-0984 confirms the sixth-row `22P02` through `.retry(n)`'s next attempt, unnamed behind a pooler or prepared again straight, and moves the marker or raises the generation only "if it succeeds with the same text and the same parameter bytes". A failure "returns as it is, as `Error::Postgres`, and nothing moves". R-1246's second bullet and R-0982 say the same, and R-1245's row now reads "once its retry succeeds". T-0312 adds the `conv` case straight and behind PgBouncer, called ten times, with the marker unmoved and no cache dropped, plus the hand-made-swap case that confirms. The spot check above holds. The "same parameter bytes" gate has no test (EM4), and without `.retry(n)` a real stale statement inside transactions doesn't heal and the README doesn't say so (EM3) |
| EL2 | **Written in as the owner decided,** but the decision's premise holds only for plpgsql. `XX000` `cache lookup failed for type` also comes from a cached statement, which new text cures (EM1). The cool-down it relies on isn't defined for these two errors anywhere but the README (EM2) |
| EL3 | **Fixed as recommended.** R-1258 says `migrate` must reach each process through its own Unix socket, which works only from that host, that any TCP URL is a failed send on every migration, and what to do from another host. R-1256's message says the same, and morm sends no `SHOW CONFIG` over a Unix socket. T-0316 runs both branches against a real 1.26.1 instance run as two `so_reuseport = 1` processes |
| EL4 | **Fixed as recommended.** R-1238: each `RECONNECT` instance has a `pool_size = 1` database, stated as "the one exception to the three-connection rule", and a `pool_size = 3`, `min_pool_size = 3` one. EA's bullet names the exception. All set `max_prepared_statements = 0`, with morm's caches off. T-0316 asserts that all three server pids of the three-connection database are replaced. The choice isn't fully attributed (EM5) |
| EL5 | **Fixed.** T-0311's case is "an `UPDATE ... SET c = f($1)` matching one row, whose retry on a fresh connection succeeds and returns only its command tag", and the PR body's EL5 line matches |
| EL6 | **Fixed** in R-1257 ("every PgBouncer process"), R-1250 ("every PgBouncer process's admin console ... and when a run stops after a committed step") and R-1040 ("or when the run stops after a committed step"). The PR body still quotes R-1257's old wording (EM6) |
| EL7 | **Fixed for what it named.** R-1238 names "the `RECONNECT` test's own instances", R-1256 adds "the `so_reuseport` check run only after a `RECONNECT` over TCP", the PR body adds choice 21, and the EJ5 bullet and choice 16 note EK1's refinement. This fix's own new choices are attributed short again (EM5) |

Nothing earlier regressed in the text: SA to SH, EF1, EG1 to EG3, EH1 to EH8, EI1 to EI9, EJ1 to EJ9 and EK1 to EK8 read consistently with it. EM1 is a regression in behaviour, inside a transaction, that EL2's fix brought in.

## The rule set, path by path

Straight to Postgres:

| Path | Heals? Bounded? |
|---|---|
| Retired-label `22P02`, stored or client text, `42804`, `XX000` | As the seventh and eighth reviews found: a new connection confirms, the pool is replaced, and stored or client text costs at most the connections open at its first failure, again after each migration. Unchanged |
| Sixth-row `22P02`, a client's label, inside a transaction | `SchemaChanged`, then `.retry(n)`'s re-prepared attempt fails and returns `Error::Postgres`; nothing raised. Bounded |
| Sixth-row `22P02`, a hand-made swap, inside a transaction | The statement is evicted; the next attempt, or any next call, prepares it again and succeeds. Bounded per connection |

Behind a pooler:

| Path | Heals? Bounded? |
|---|---|
| Retired-label `22P02` (EK1) | One `SchemaChanged` per statement per minute; no cache cleared. Heals through `RECONNECT` or `server_lifetime` |
| Sixth-row `22P02`, a client's label (EL1) | One `SchemaChanged` and one unnamed run per call; never clears a cache. **Sound** |
| Sixth-row `22P02`, a hand-made swap, with `.retry(n)` | The second attempt runs unnamed, succeeds, and the salt and the generation move. One failed attempt per process |
| Sixth-row `22P02`, a hand-made swap, without `.retry(n)` and never run outside a transaction | `SchemaChanged` on every call until PgBouncer replaces the server connection (EM3). Bounded by `server_lifetime`, and unsaid |
| `42804`, `XX000` from plpgsql (EL2) | No marker, no cache cleared; heals through `RECONNECT` or `server_lifetime`. The cool-down they "enter" has no defined effect on them (EM2) |
| **`XX000` from a cached statement, after a hand-run swap that drops a type** | **Never retried unnamed and never moves the marker, in or out of a transaction, so every process's cached statement fails on every call until `server_lifetime`, though new text cures it (EM1)** |
| A real stale server session after a migration | `RECONNECT` on each listed PgBouncer process, a stopped run included; otherwise `server_lifetime`. Unchanged |

Across all these paths: no client text clears a cache, nothing grows past `max` or the budget, and no path gives a silent wrong answer. Behind a pooler the rule set is sound for client and stored text. It isn't sound yet for a schema change made by hand, because of EM1, and EM3 leaves a quieter case of the same kind.

## Findings

### EM1. Blocks approval. R-1245, R-0718, R-0981, R-0982, R-0984, R-1246, R-1253: behind a pooler, `XX000` `cache lookup failed for type` from a cached statement is a stale statement that new text cures, but no path ever gives it new text

- **Where:**
  - R-1245's third row: "`XX000` from any statement | message starting `cache lookup failed for type` | stale session".
  - R-0718: "Without that reset, as after a swap made by hand, one that touches no such table, such as `SELECT ($1::st)::text`, gets `XX000` `cache lookup failed for type` once the old type is dropped ... R-0981's handling still comes first for R-1245's stale-session signals."
  - R-0981, outside a transaction behind a pooler: "it retries once on another connection ... and discards and raises nothing".
  - R-1246's third bullet, new: "R-1245's three stale-session signals, `42804`, `XX000` `cache lookup failed for type` and a `22P02` ... move no marker, add no salt and raise nothing, inside a transaction or out". R-0982, new: they "raise no statement generation at all, because new statement text runs in the same server session and can't cure them". R-0984, new: "The same holds behind a pooler for step 14's other two stale-session signals".
  - `proofs.md`, the EL2 bullet: "proof 16's table above showed new text doesn't cure them".
- **What's wrong:**
  - `XX000` `cache lookup failed for type` doesn't only come from plpgsql. Proof 14's table records it from a plain cached statement: "No table of the type: `SELECT ($1::st)` | `XX000` `cache lookup failed for type <n>` (`getTypeInputInfo`), every run", cured by "a re-prepare on the same connection, `query_typed` and a new connection". Proof 16 says the client sees the same behind PgBouncer, "sometimes at the prepare itself (`prepare: XX000 ... (getBaseTypeAndTypmod)`)".
  - I reproduced it behind 1.26.1 on 18.6 and 17.11 (above). After a hand-run swap that dropped the old type:
    - the cached statement and a new client's prepare of the same text both gave `XX000`;
    - the same text run unnamed, and text with a new marker, both worked.

    So it's the stale statement R-1253's marker exists for. Only the plpgsql `XX000` kept failing when run unnamed.
  - The rules give that statement no new text, in or out of a transaction:
    - Outside a transaction, R-0718 puts R-0981 first, and R-0981 retries on another client connection. Through PgBouncer that reaches a server statement prepared from the same text, so it fails again.
    - Inside a transaction, since EL2, there's `SchemaChanged`, the cool-down and no marker. Before `b10fa3f` the marker moved there, so this fix turned a path that healed after one failure into one that never heals.
  - So behind PgBouncer, after a type is dropped by hand (a hand-run swap, or `DROP TYPE` and `CREATE TYPE` again), every process's cached statements that cast a parameter to that type fail on every call until PgBouncer replaces each server connection, an hour by default. No migration runs, so nothing sends `RECONNECT`.
  - That contradicts:
    - owner decision SD: "a stale signal at prepare or execute retries once unnamed, then adopts the new `n` or ... a random salt";
    - R-0718's own example, which names this statement as one its retry handles;
    - R-1253's loud notice: "each process's first call to a statement the change left stale fails, is retried once unnamed and succeeds".

    No client can raise this error, so a fix doesn't reopen EL1.
- **Evidence:** proof 14's table and its first bullet; proof 16's "What the client sees" and "Re-preparing doesn't clear a stale statement"; the spot check above.
- **Fix, a design choice for the owner, since it refines EL2's premise:**
  - **(a) Recommended. Confirm it with one unnamed run, as the sixth-row `22P02` is confirmed.**
    - Behind a pooler, an `XX000` `cache lookup failed for type` from a cached statement is run once more unnamed, with the same text and parameter bytes: outside a transaction through R-1246's first bullet, before R-0981's retry on another connection, and inside one through `.retry(n)`'s next attempt, as EL1 does.
    - If the unnamed run succeeds, the cached text was stale: adopt `<n>` or add the salt, and raise the statement generation. If it fails, it came from compiled code: EL2's path, with no marker, the cool-down, and `RECONNECT` or `server_lifetime`.
    - The spot check shows the plpgsql case fails the unnamed run, so a real stale session still never moves the marker, and EL2's churn stays gone. Straight to Postgres nothing changes.
    - The text changes:
      - R-1245's third row's kind becomes "stale session; from a cached statement behind a pooler, a stale statement once its unnamed run succeeds";
      - R-0718's precedence sentence names the exception;
      - R-0981, R-0982, R-0984 and R-1246's third bullet say "from plpgsql" where they now say the error can't be cured;
      - `proofs.md`'s EL2 bullet cites proof 14's row.
    - Tests: T-0306 (outside a transaction) and T-0312 (inside) add, behind PgBouncer with two processes, a cached `SELECT (($1::text)::st)::text` after a hand-run swap that drops the old type: each process fails one call, the salt is added and the generation raised, and the next call is right. The `DECLARE v st` function's `XX000` fails its unnamed run, and the marker stays where it was.
  - **(b)** Keep the rule, and say in R-0718 and loudly in R-1253 that behind PgBouncer a type dropped by hand leaves every cached statement that casts to it failing until `server_lifetime` or a `RECONNECT` run by hand. That still contradicts SD.

### EM2. Should fix. R-0981, R-0984, R-1246, T-0312: the cool-down that `42804` and `XX000` "enter" behind a pooler has no defined effect on them

- **Where:**
  - R-0984, new: `42804` and `XX000` "return `Error::SchemaChanged`, move no marker, add no salt, raise no generation, and the statement then enters step 14's memory".
  - R-1246, new: "Inside a transaction they stay `Error::SchemaChanged`, and the statement then enters step 14's 60-second cool-down."
  - R-0981 defines what the memory does: "meanwhile a `22P02` holding `_morm_retired_` from that statement is returned at once, with no new connection and nothing raised". R-0984's "When ... step 14's pool remembers the statement, it returns as it is, not as `Error::SchemaChanged`" sits in the paragraph about that `22P02`.
  - Only R-1258's README notice says what happens to the other two: "returns `Error::SchemaChanged` once, and then its error at once for 60 seconds".
- **What's wrong:**
  - No requirement says that a remembered statement's `42804` or `XX000` returns at once as `Error::Postgres`. So a builder following R-0981 and R-0984 fills the memory with no effect: each call returns `SchemaChanged`, and `.retry(n)` runs the closure `n + 1` times on every call. The README promises more than the rules build.
  - Outside a transaction behind a pooler, the rules don't say whether a remembered statement still gets R-0981's retry on another connection for these two errors.
  - T-0312's new cases assert "the statement in the memory", but not what that does. The `22P02` case does assert it: "so its next call, in `.retry(n)`'s next attempt or later, returns its `22P02` at once".
- **Fix:**
  - R-0981's memory sentence: "meanwhile a `22P02` holding `_morm_retired_` from that statement, and behind a pooler a `42804` or an `XX000` `cache lookup failed for type` from it (after EM1, from plpgsql), is returned at once, as `Error::Postgres`, with no retry, no new connection and nothing raised". R-0984 says the same inside a transaction. This matches R-1258.
  - T-0312's `%ROWTYPE` and dropped-type cases add "and its next call, in `.retry(n)`'s next attempt, returns its `42804` (`XX000`) at once, as `Error::Postgres`". T-0306's pooler case adds the same outside a transaction.

### EM3. Should fix. R-1253, R-0984, R-1246: behind PgBouncer, inside transactions without `.retry(n)`, a statement a hand-made swap left on another enum never heals, and the README doesn't say so

- **Where:**
  - R-0984: "With no `.retry(n)` nothing moves, and the statement's next run outside a transaction confirms through step 14's retry." R-1246's second bullet says the same.
  - R-1253's notice: "Inside a transaction that call returns `Error::SchemaChanged`, which `.retry(n)` retries; a `22P02` naming another enum heals through `.retry(n)`'s next attempt ...".
- **What's wrong:**
  - Before EL1, the first `SchemaChanged` moved the marker, so the next call healed with or without `.retry(n)`. Now only `.retry(n)`'s next attempt confirms.
  - Evicting the statement doesn't help behind PgBouncer: the next prepare of the same text gets the stale server statement back. On 18.6 and 17.11, three transactions in a row, each preparing the same text again, each gave `22P02` naming `st_old`; new text in a fourth worked.
  - So an app that uses transactions without `.retry(n)` gets `SchemaChanged` on every call of that statement until PgBouncer replaces the server connection or the statement runs outside a transaction. The same holds for an app that retries `SchemaChanged` itself, which the error's name invites. Nothing gets worse and nothing is wrong silently, but it's a surprise, and the notice beside `pooler = "transaction"` implies the opposite.
- **Fix, a design choice for the owner, since EL1 named `.retry(n)`'s next attempt:**
  - **(a) Recommended.** Keep the pending confirmation in the pool, keyed by a hash of the text and the parameter bytes, at most 64 entries with the oldest dropped, and cleared when `<n>` moves. Then that statement's next run with the same bytes runs unnamed and confirms, whether it's `.retry(n)`'s next attempt, the app's own retry or a later call. A client's label still costs one unnamed run and clears nothing. T-0312 adds two transactions with no `.retry(n)` after a hand-made swap: `SchemaChanged`, then success, with the salt added.
  - **(b)** Keep the rule, and add to R-1253: "Without `.retry(n)`, such a statement run only inside transactions keeps returning `Error::SchemaChanged` until it runs outside a transaction or PgBouncer replaces the server connection, so use `.retry(n)`."

### EM4. Minor. T-0312: nothing tests EL1's "same parameter bytes" gate

- **Where:** R-0984 and R-1246 move the marker on EL1's confirming attempt "only if it succeeds with the same text and the same parameter bytes". T-0312's new cases bind `b` on both attempts, or confirm a hand-made swap.
- **What's wrong:** the gate is what stops a closure that binds another value on its second attempt from raising the generation, but no case reaches it on EL1's path. The retired-label path has the matching case ("a closure that binds a different value on its second attempt, or doesn't call the function then, raises no generation").
- **Fix:** T-0312 adds, behind PgBouncer and straight: a `.retry(2)` closure that binds `b` to the `conv` statement on its first attempt and `a` on its second succeeds, with no salt added, the marker unmoved and no generation raised.

### EM5. Minor. R-0984, R-1238, PR body: this fix's own choices are attributed short

- **Where:** INDEX's EL entry names two choices "the owner approves through PR #3":
  - "each `RECONNECT` test instance has a second, three-connection database beside the one-connection one, which is the one stated exception to EA's three-connection rule";
  - "straight to Postgres, EL1's confirming attempt re-prepares the statement and raises the statement generation on success".
- **What's wrong:**
  - R-0984's list of "this fix's choices" still names only the records cleared and no record kept behind a pooler, and doesn't name the second choice.
  - R-1238 names "the `RECONNECT` test's own instances" (EK6's choice) and not the three-connection database.
  - The PR body's choices 1 to 21 don't have the straight re-prepare. Choice 20 covers the databases.
- **Fix:** R-0984 adds "straight to Postgres, EL1's confirming attempt prepared again, raising the statement generation on success"; R-1238 adds "and the second, three-connection database". The PR body adds choice 22 for the re-prepare.

### EM6. Minor. PR body, R-1258: two places keep the old "every PgBouncer" wording

- **Where and what's wrong:**
  - The PR body's last paragraph quotes R-1257 as "list every PgBouncer, and read the notice first". R-1257 now says "every PgBouncer process".
  - R-1258's "To heal it at once" paragraph says "list every PgBouncer's admin console", while the same notice's next paragraph says "List every PgBouncer process" and R-1250 says "every PgBouncer process's admin console".
- **Fix:** the PR body quotes R-1257 as it stands, and R-1258 says "list every PgBouncer process's admin console".

## What I couldn't run

- **morm itself.** There is no runtime yet. EM1's and EM3's outcomes under the rules come from their text. The error shapes, and what new text, the same text and an unnamed run do, come from Postgres and PgBouncer themselves.
- **Postgres 15 and 16.** I ran 18.6 and 17.11. Proof 14 already pins the cached statement's `XX000` on 15 to 18, straight.
- **PgBouncer 1.20.1 and 1.22.** I ran 1.26.1 only. Proof 16 records the shared stale server statement, `XX000` among its errors, on 1.20.1 and 1.22.1. The `so_reuseport` and two-database claims of EL3 and EL4 were checked on real PgBouncers by the eighth review, or follow from PgBouncer's documented settings. I didn't run them again.

## Verdict

**Not approved yet.**

What holds:

- every changed ID is in INDEX's EL entry at `b10fa3f`, and the PR body's lists match;
- the checker reports 0 problems, and CI is green on the head;
- EL1 is fixed as the owner decided: behind a pooler a client's enum label now costs one unnamed run per call and never clears a cache. That closes the last client-triggerable path, and the spot check confirms the confirming run fails for such a label;
- EL3 to EL7 are fixed as the eighth review recommended;
- straight to Postgres the rule set stays sound;
- behind a pooler no client or stored text clears a cache, nothing passes `max` or the budget, no answer is silently wrong, and every stale server session heals through `RECONNECT` or `server_lifetime`.

What remains:

- **Blocks approval:** EM1. A type dropped by hand leaves cached statements failing behind PgBouncer until `server_lifetime`, though new text cures them, against SD and R-1253. EL2's fix extended this inside transactions. It needs a design choice; (a) is recommended.
- **Should fix:**
  - EM2: the cool-down for `42804` and `XX000` is defined only in the README;
  - EM3: without `.retry(n)`, a hand-made swap's stale statement inside transactions behind PgBouncer doesn't heal, and the README doesn't say so (a design choice; (a) recommended).
- **Minor:**
  - EM4: EL1's parameter-bytes gate is untested;
  - EM5: this fix's choices are attributed short;
  - EM6: the PR body and R-1258 keep the old "every PgBouncer" wording.
