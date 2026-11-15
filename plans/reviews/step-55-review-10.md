# Step 55 independent review, tenth round

PR #3 (`step-55-enum-proofs`), head `a463237`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes. This round covers the fix for the ninth review's EM1 to EM6, `git diff b10fa3f origin/step-55-enum-proofs` (one commit, `a463237`), and checks that nothing earlier regressed. The owner decided two things:

- **EM1:** behind a pooler, an `XX000` `cache lookup failed for type` from a cached statement is confirmed with one unnamed run. Success moves the marker; failure follows EL2's no-marker, cool-down path, now only for plpgsql.
- **EM3:** the pending confirmation is kept in the pool, keyed by text and parameter bytes, so a later call confirms and heals.

## What I read

- **On the PR branch:** `rules.md`, `contracts.md` (its `_morm` comment and query entry point), `proofs.md` (step 55's section, the EL2 bullet and the new "ninth review" bullets), and every changed requirement and test in full, old and new: R-0718, R-0981, R-0982, R-0984, R-1238, R-1245, R-1246, R-1253, R-1258, T-0306 and T-0312. For the rule set I also re-read R-1256 and R-1257.
- **On `origin/main`:** INDEX's Decisions entries "Step 55 eighth review, EL1 to EL7" and "Step 55 ninth review, EM1 to EM6", with the EM IDs and the fix's own choice, and `reviews/step-55-review-9.md`.
- **The PR body.**

## What I checked and ran

- **CI on the head.** `a463237` is green: `ci` on push (run 43683925354) and on the pull request (run 43683928719), 36 of 36 checks across the two. The diff since `b10fa3f` touches only `plans/plan-approved/` (6 files: `proofs.md`, steps 12, 14, 20 and 50, and `manifest.json`). No code, test or workflow changed.
- **Coverage checker,** on the head:
  - `check_coverage.py --step 55 --code crates tests benches fuzz .github`: **1258 requirements, 316 test obligations, 0 problems**;
  - the plain form gives the same.
- **Manifest and INDEX.** I compared the manifests at `b10fa3f` and `a463237` ID by ID:
  - R-0718, R-0981, R-0982, R-0984, R-1238, R-1245, R-1246, R-1253, R-1258, T-0306 and T-0312 change;
  - none is added or removed, and `frame` and `master_sha256` are unchanged.

  That's exactly INDEX's EM entry, "Written in at `a463237`", and the PR body's list. Against `origin/main` the set of changed or new IDs is the same 75 as at `b10fa3f` (30 changed and 18 new requirements, 11 changed and 16 new tests), all covered by INDEX's step 55 entries.
- **Spot checks, on throwaway servers.** I built PgBouncer 1.26.1 from pgbouncer.org's `pgbouncer-1.26.1.tar.gz`. Its SHA-256, `00886a2994b4844cd37110970df777772cfc42a74c400522c82fba509a3f42d9`, matched the published `.sha256` file. One instance ran in front of each of Postgres 18.6 and 17.11, in transaction mode, with `max_prepared_statements = 200` and one server connection. Everything ran under `.claude/scratch/` in my worktree with `TMPDIR` set there, apart from the Unix sockets, which sat in a short folder under `~/.claude/scratch/`. The client was asyncpg 0.31.0: `prepare` gives a named statement, and `statement_cache_size = 0` gives unnamed ones. Every call ran in its own transaction.

  | Claim | Result, the same on 18.6 and 17.11 |
  |---|---|
  | EM1: after a hand-run swap that drops the old type, a cached `SELECT (($1::text)::st)::text` gets `XX000`, and the same text run unnamed works | Yes: the cached statement gave `XX000` `cache lookup failed for type 16386`, and so did a new client preparing the same text; unnamed, `a` returned `a` |
  | EM3: the same text with other values heals | **No.** The cached statement gave the same `XX000` for `a`, `b` and `d`, and a new client's prepare of it for `b` and `d`. Only a run unnamed, or new text, cures it. See EN1 |
  | The same, after a hand-run swap that keeps the old type and reverses its order (`c, b, a, d`), for a cached `SELECT ($1::text)::st < ($2::text)::st` | `a < d` and `b < d` each gave `22P02` `invalid input value for enum st_old: "d"`; `a < c` gave `true` silently, while unnamed it gave `false`, the right answer. See EN1 |
  | The confirming run can end in an error other than the first one | Yes. The stale `XX000` statement given `c`, a label only the dropped type had, gave `XX000` cached and `22P02` `invalid input value for enum st: "c"` unnamed, an error the rules give no outcome for. See EN2 |

## EM1 to EM6: are they fixed?

| Finding | Verdict and evidence |
|---|---|
| EM1 | **Fixed as the owner decided.** R-1245's third row now reads "stale session; from a cached statement behind a pooler, a stale statement once its unnamed run succeeds, and a stale session from plpgsql once it fails". R-0718's precedence sentence names the exception, and R-0981 sends that `XX000` to R-1246 first. R-1246's first bullet handles it outside a transaction "before step 14's retry on another connection", and its second bullet confirms it inside one through the pending confirmation. R-0982, R-0984 and R-1246's third bullet now say "from plpgsql compiled in the server session". `proofs.md`'s EL2 bullet cites proof 14's row. T-0306 (outside) and T-0312 (inside) add the two-process `SELECT (($1::text)::st)::text` case, salt added and generation raised, and the `DECLARE v st` case, marker unmoved. The spot check above holds. Two loose ends: the outside-a-transaction failure path disagrees between R-1246 and R-0718 (EN4), and what follows a confirming run that ends in a third outcome isn't said (EN2) |
| EM2 | **Fixed as recommended.** R-0981's memory sentence returns a remembered statement's `22P02`, and behind a pooler its `42804` or plpgsql `XX000`, "at once, as `Error::Postgres`, with no retry, no new connection and nothing raised". R-0984 and R-1246's third bullet say the same inside a transaction, and so does R-1258. T-0312's `%ROWTYPE` and `DECLARE` cases and T-0306's pooler case assert it |
| EM3 | **Written in as the owner decided,** with the bounds as the fix's own choice: 64 entries per pool, a 64-bit hash of text and parameter bytes, cleared when `<n>` or the connection generation moves. T-0312 adds two transactions with no `.retry(n)`, an app's own retry loop, and the 65th entry. But calls with other values still never heal behind PgBouncer, against R-1253's loud title (EN1); what removes an entry after a successful confirmation isn't said, and read as written each later run with the same bytes clears the caches again (EN2); and the check runs on every cached statement's run, a cost R-1246 doesn't state (EN3) |
| EM4 | **Fixed as recommended.** T-0312: "a `.retry(2)` closure that binds `b` to the `conv` statement on its first attempt and `a`, which fits, on its second, behind PgBouncer and straight, succeeds with no salt added, the marker unmoved and no statement generation raised". A build that confirmed on any next run would fail it |
| EM5 | **Fixed.** R-0984's choices add "straight to Postgres the confirming run prepared again, raising the statement generation on success"; R-1238's add "the second, three-connection database on each"; the PR body adds choice 22 |
| EM6 | **Fixed.** R-1258 says "list every PgBouncer process's admin console", and the PR body quotes R-1257 as it stands, "list every PgBouncer process, and read the notice first" |

Nothing earlier regressed in the text: SA to SH, EF1, EG1 to EG3, EH1 to EH8, EI1 to EI9, EJ1 to EJ9, EK1 to EK8 and EL1 to EL7 read consistently with it.

## The rule set, path by path

Straight to Postgres:

| Path | Heals? Bounded? |
|---|---|
| Retired-label `22P02`, stored or client text, `42804`, `XX000` | As the seventh to ninth reviews found: a new connection confirms, the pool is replaced, and stored or client text costs at most the connections open at its first failure, again after each migration. Unchanged |
| Sixth-row `22P02`, a client's label | `SchemaChanged`, then the pending run, prepared again, fails and returns `Error::Postgres`; nothing raised. Bounded |
| Sixth-row `22P02`, a hand-made swap | The pending run, prepared again, succeeds and raises the statement generation; a run with other values fails once per pooled connection, each evicting its own copy. Heals within the pool's size |

Behind a pooler:

| Path | Heals? Bounded? |
|---|---|
| Retired-label `22P02` (EK1), `42804` | One `SchemaChanged` per statement per minute, then the error at once (EM2); no cache cleared. Heals through `RECONNECT` or `server_lifetime` |
| Sixth-row `22P02`, a client's label (EL1) | One `SchemaChanged` and one unnamed run per call; clears nothing. **Sound** |
| `XX000` from a cached statement, a type dropped by hand, outside a transaction (EM1) | One failed run and one unnamed run per process; salt added. **Sound** |
| `XX000` from plpgsql, outside a transaction (EM1) | Cached run, unnamed run, retry on another connection, then the memory. Bounded; but see EN4 for a pending run |
| A hand-made swap's stale statement inside a transaction, retried with the same values (EL1, EM1, EM3) | One `SchemaChanged` per process, then salt added. **Heals** |
| The same, called with **other values** each time, with no `.retry(n)` and no call outside a transaction | `SchemaChanged` on every call until PgBouncer replaces the server connection, and, with the old type kept, a shared label's answer silently on the old order meanwhile. Bounded by `server_lifetime`, and the README says the opposite (EN1) |
| A pending confirmation that succeeds | Salt added and statement generation raised. Nothing in the text removes the entry, so read as written each later run with the same bytes re-salts and clears every cache (EN2) |
| A real stale server session after a migration | `RECONNECT` on each listed PgBouncer process, a stopped run included; otherwise `server_lifetime`. Unchanged |

Across all these paths: nothing grows past `max` or the budget, because every pending run reuses its own connection. The pending confirmations, the first-failure records and the memory are each at most 64 per pool. No path gives a wrong answer silently that it didn't before, apart from EN1's kept-type case, which this design lengthens. Straight to Postgres the rule set stays sound. Behind a pooler it heals client and stored text and EM1's `XX000`, but it isn't sound yet: EN2's churn is one reading away, and EN1's case is unsaid.

## Findings

### EN1. Should fix. R-1253, R-1258, R-0984, R-1246: behind PgBouncer, a statement a hand-made change left stale, run inside transactions with other values each time, fails on every call until `server_lifetime`, and the loud notice says it costs one failed call

- **Where:**
  - R-1253's title: "**Behind PgBouncer, a schema change made outside morm costs each process one failed call.**" Its body: "a `22P02` naming another enum, and `cache lookup failed for type` from a statement casting to a type dropped by hand, heal through that statement's next run with the same values, whether by `.retry(n)`, your own retry or a later call".
  - R-1246's second bullet: "A run of that statement with other parameter bytes confirms nothing and runs as usual." R-0984: "A run with other parameter bytes confirms nothing."
  - R-1258's last paragraph: a `cache lookup failed for type` from code compiled in the server session "returns `Error::SchemaChanged` once, and then its error at once for 60 seconds".
- **What's wrong:**
  - "Runs as usual" means prepared under the same marker. Behind PgBouncer that returns the stale server statement. The spot check shows it fails for every value: `XX000` for `a`, `b` and `d` with the type dropped, and `22P02` naming `st_old` for any pair holding `d` with it kept.
  - So an app that calls such a statement in transactions without `.retry(n)`, with values that change from call to call (a user id, a timestamp), gets `Error::SchemaChanged` on every call in every process, until PgBouncer replaces the server connection, an hour by default, or the statement runs outside a transaction. The pending confirmation never matches. That's the case EM3 was about, and EM3's fix heals only the same-values retry. It's bounded and nothing clears a cache, but the loud notice says one failed call per process.
  - With the old type kept, the same window runs a shared label on the old type's order with no error: the spot check's `a < c` gave `true` from the cached statement and `false`, the right answer, unnamed. R-0718 records this ("which no retry can see"), but no README notice says it. Before EL1, the first `SchemaChanged` moved the marker and ended it; now it lasts as long as no same-values retry comes.
  - R-1258's "`SchemaChanged` once, and then its error at once for 60 seconds" also holds only for a same-values retry. A plpgsql `XX000` called with other values each time gives `SchemaChanged` on every call, because only a failed pending run fills the memory. That costs no more than the error itself, but the notice says otherwise.
- **Evidence:** the spot check's second and third rows; R-1246's and R-0984's text above.
- **Fix, a design choice for the owner, since EM3 keyed the confirmation by parameter bytes:**
  - **(a) Recommended. Keep the rule and say it.**
    - R-1253's title: "**Behind PgBouncer, a schema change made outside morm can fail calls until each process confirms it.**"
    - R-1253's body adds: "A call with other values isn't a confirmation: inside a transaction it returns `Error::SchemaChanged` again, so a statement run only in transactions, without `.retry(n)` or your own retry of the same call, keeps failing until it runs outside a transaction or PgBouncer replaces the server connection, within `server_lifetime`. While the old type is kept, a value both types share runs on the old type's order, with no error, until then. Use `.retry(n)`."
    - R-1258's last paragraph: "returns `Error::SchemaChanged`, and once a retry of the same call has failed, its error at once for 60 seconds".
    - T-0312 adds, behind PgBouncer, after a hand-made swap: two transactions with no `.retry(n)` and different values each give `SchemaChanged`, with no salt added.
  - **(b) Heal other values too, with no confirmation from them.** While a pending entry exists for a text, a run of it with other bytes runs unnamed and confirms nothing. Those calls then get the right answer, with no `SchemaChanged` and no old-order answer, and no cache is cleared. The cost is one unnamed run per call while the entry lasts. It needs a bound, because a client's bad label can create an entry: the 64-entry limit and the generation moves already bound it, and a time limit would too (EN5). If the owner takes (b), R-1246 and R-0984 say so, and T-0312 tests it.

### EN2. Should fix. R-0984, R-1246, T-0312, INDEX, PR body: the pending confirmation's lifecycle isn't defined, and read as written a successful confirmation clears every cache again on each later run with the same values

- **Where:**
  - R-0984: the pending confirmation is "all cleared when `<n>` or the connection generation moves ... That statement's next run with the same parameter bytes ... runs it unnamed behind a pooler, or prepared again straight to Postgres. If it succeeds, the cached text was stale, and morm then adds the salt and raises the statement generation ...; if it fails the same way, the error returns as it is, as `Error::Postgres`, nothing moves".
  - R-1246's second bullet: the same, and on a failure "the pending confirmation is dropped". It says nothing on success.
  - R-1246 defines `<n>` as "the database's statement generation, read from the `_morm` schema's comment". A salt doesn't move it, and neither does the pool's own statement generation, which R-0982 raises on each confirmation.
  - INDEX's EM entry: "cleared when either generation moves". The PR body's choice 23 and `proofs.md`'s EM3 bullet: "cleared when the statement or connection generation moves". T-0312: "all are cleared when the statement generation moves".
- **What's wrong:**
  - **Success doesn't remove the entry.** Only a failure drops it (R-1246), R-0984 removes it in neither case, and the clearing triggers, `<n>` and the connection generation, don't move when a confirmation adds a salt or raises the pool's statement generation. So, read as written, the entry outlives its own confirmation: every later run with the same bytes runs unnamed (or prepared again straight), succeeds, adds a new salt and raises the statement generation, and every connection drops its cached statements at checkout. A statement called often with one value, such as `status = 'active'`, would clear the pool's caches on each call, straight to Postgres and behind a pooler. That's the churn the rule set exists to prevent. "The next run" suggests a one-shot entry, and the INDEX and PR body wording suggests the pool's statement generation clears it, but the requirement text a builder follows says neither.
  - **The other entries after a confirmation.** If only `<n>` and the connection generation clear them, each other pending entry, once matched, re-salts again, although the first salt already gave every text fresh marks. That's bounded at 64, but it's waste the INDEX wording ("either generation") seems to rule out.
  - **A third outcome.** The confirming run can end in an error other than the first one: the spot check's stale `XX000` statement given `c` gave `22P02` `invalid input value for enum st: "c"` unnamed, which shows the cached text was stale but is neither "succeeds" nor "fails the same way". It can also end in one of R-0981's errors that end a statement before it reaches the conversion (`08`, `40`, `53`, `57`, `55P03`). For EL1's straight-to-Postgres retired-label attempt R-0984 defines all three outcomes; for the pending run it doesn't say whether the entry stays, whether the marker moves, or what the caller gets.
  - **The texts disagree on what clears the entries:** `<n>` (R-0984, R-1246), "either generation" (INDEX), "the statement or connection generation" (PR body, `proofs.md`), and "the statement generation" (T-0312), where "statement generation" means both the database's `<n>` and the pool's counter in R-0982. T-0312 tests one trigger, not both.
- **Evidence:** the text above; the spot check's fourth row.
- **Fix:**
  - R-1246 and R-0984 say: "The pending run takes the entry out of the set, so only one run confirms it. Every entry is cleared when the pool's statement generation moves, a confirmation's salt included, or `<n>` or the connection generation moves."
  - They define the third outcome the way R-0981 does: a confirming run that ends in a server error other than the first one, and outside R-0981's list, confirms. The caller gets that error, and the salt and the generation move, since `XX000` and a sixth-row `22P02` came from the cached text. One that ends in an error on that list, or a lost connection, neither confirms nor fails: the entry is dropped, nothing moves, and the caller gets that error. If the owner prefers that only a success confirms, the text says a different error drops the entry and moves nothing.
  - INDEX's EM entry, the PR body's choice 23, `proofs.md` and T-0312 use the same words.
  - T-0312 adds:
    - after a confirmed pending run, ten more runs with the same values raise the statement generation no further;
    - the entries are cleared when the connection generation moves;
    - a confirming run that ends in another error (the dropped type's `c`) gives the outcome the rule picks.

### EN3. Should fix. R-1246, R-0984: the pending confirmation is checked on every cached statement's run, and the rules don't state or bound that cost

- **Where:**
  - R-1246's second bullet and R-0984: "That statement's next run with the same parameter bytes ... in a transaction or not, runs it unnamed". To know whether a run is that statement's next run, morm must look in the pool-wide set before every cached statement's run, on every connection, because the other connections still hold the stale statement.
  - R-1246's cost bullet: "The cost is one catalog read when a pool starts, and on a stale error one unnamed run and one catalog read, once per process per generation."
- **What's wrong:**
  - The step-14 memory and the first-failure records are read only after an error. The pending confirmation is the first of these sets read before a run, on the hottest path in morm. `rules.md` puts raw performance first, and "Hot paths have no needless allocation, dynamic dispatch or per-row state".
  - Nothing tells a builder to keep that check off the common path. A lock on the shared set, or a hash of the parameter bytes, on every call would meet the text as written. R-1246's cost line now leaves out a per-call cost the design adds.
- **Fix:** R-1246 adds to its cost bullet: "While no confirmation is pending, the check is one relaxed atomic read of the pool's pending count per run. The text's hash, which the statement cache already holds, is compared only when the count isn't zero, and the parameter bytes are hashed only when the text matches." R-0984 points to it. Step 48's benchmarks cover the cached-statement path with the count at zero.

### EN4. Minor. R-1246, R-0718, R-0981: outside a transaction, a plpgsql `XX000`'s failed pending run skips the retry on another connection that R-0718 gives every failed unnamed run

- **Where:**
  - R-0718: "only a failed unnamed run goes on to R-0981". R-0981, behind a pooler: "it retries once on another connection, which may reach a fresh server session", and a failed retry fills the memory.
  - R-1246's second bullet, for the pending run, "in a transaction or not": "If it fails the same way, its error returns as it is, as `Error::Postgres`, the pending confirmation is dropped and nothing moves; a plpgsql `XX000` then enters step 14's cool-down".
- **What's wrong:** outside a transaction the two say different things. Under R-0718 the call gets one try on another connection, which can reach a fresh server session and succeed, and the memory is filled only if that fails too. Under R-1246 the call returns `XX000` at once and the statement goes straight into the 60-second cool-down, so every call in that minute fails, even where another server session would have answered. A builder has to pick one.
- **Fix:** R-1246's second bullet: "If it fails the same way inside a transaction, ...; outside one, a plpgsql `XX000` goes on to R-0981's retry on another connection, as R-0718 says, and the pending confirmation is dropped." T-0306 adds, outside a transaction, a pending `DECLARE v st` call whose unnamed run fails and whose retry reaches a fresh server session and succeeds.

### EN5. Minor. R-1246, R-0984, INDEX: with no time limit, a pending confirmation can be matched long after the failure, when a data change, not a stale statement, makes the same values work

- **Where:** INDEX's EM entry: "with no time-based expiry". R-1246: a pending entry is matched by "the app's own retry or a later call".
- **What's wrong:**
  - "Same text, same bytes, now succeeds" proves a stale statement only if nothing else changed between the two runs. A sixth-row `22P02` can come from a function that converts a client's label through data the app reads, as R-0981 says of stored text ("a function converting text it read from a table").
  - If that data changes after the failure, by an admin fixing a row or by the client through another call, the same values then succeed. morm takes that for a stale statement, adds a salt and clears the pool's caches.
  - Under EL1 that needed the change to land between two attempts of one `.retry(n)`. With no expiry it can land at any time before the entry is pushed out or a generation moves. A client that can change that data can clear a process's caches once per change and repeated call.
  - It needs that kind of function, and each round costs one cache clear in one pool. So it's minor, but it reopens a little of what EL1 closed.
- **Fix, a design choice for the owner, since the expiry is this fix's own choice:**
  - **(a) Recommended.** A pending entry also expires after 60 seconds, as the memory does, through the same `test-hooks` shortening. `.retry(n)` and an app's own retry loop run well within that, so EM3's healing is kept, and the window for a data change is a minute.
  - **(b)** Keep no expiry, and R-1246 names the case and its cost.

## What I couldn't run

- **morm itself.** There is no runtime yet. EN1's to EN5's outcomes under the rules come from their text. The error shapes, and what other values, the same text and an unnamed run do behind PgBouncer, come from Postgres and PgBouncer themselves.
- **Postgres 15 and 16.** I ran 18.6 and 17.11. Proof 14 already pins the cached statement's `XX000` on 15 to 18, straight.
- **PgBouncer 1.20.1 and 1.22.** I ran 1.26.1 only. Proof 16 records the shared stale server statement on 1.20.1 and 1.22.1.
- **Two client processes.** I ran one client process with several connections. One PgBouncer server statement per text holds whatever the process, as proof 16 and the ninth review recorded.

## Verdict

**Not approved yet.**

What holds:

- every changed ID is in INDEX's EM entry at `a463237`, and the PR body's lists match;
- the checker reports 0 problems, and CI is green on the head;
- EM1 is fixed as the owner decided: behind a pooler, a cached statement's `XX000` after a type is dropped by hand now heals after one failed call per process outside a transaction, and after one same-values retry inside one, while plpgsql's `XX000` moves no marker;
- EM2, EM4, EM5 and EM6 are fixed as the ninth review recommended;
- straight to Postgres the rule set stays sound;
- behind a pooler, no client or stored text clears a cache through EL1's path, nothing passes `max` or the budget, every set is bounded at 64 per pool, and every stale server session heals through `RECONNECT` or `server_lifetime`.

What remains:

- **Should fix:**
  - EN1: calls with other values never confirm, so a hand-made change's stale statement inside transactions keeps failing until `server_lifetime`, against R-1253's loud title (a design choice; (a) recommended);
  - EN2: nothing removes a pending entry after its confirmation succeeds, so read as written every later run with the same values clears the caches again; the third outcome and the clearing triggers aren't defined consistently;
  - EN3: the per-run check of the pending set is a hot-path cost the rules don't state or bound.
- **Minor:**
  - EN4: outside a transaction, a failed pending run skips R-0981's retry on another connection;
  - EN5: no expiry lets a data change pass for a stale statement long after the failure (a design choice; (a) recommended).

Nothing blocks approval outright, but the rule set isn't sound behind a pooler until EN2 closes the churn the text allows as written, and EN1's notice tells the truth.
