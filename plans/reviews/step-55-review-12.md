# Step 55 independent review, twelfth round

PR #3 (`step-55-enum-proofs`), head `0f26ba7`. The review follows `rules.md`'s "Independent review" section. I didn't write the step, the design or any of the fixes. This round covers the fix for the eleventh review's EO1 to EO10, `git diff c4b4ce2 origin/step-55-enum-proofs` (one commit, `0f26ba7`), and checks that nothing earlier regressed.

The owner decided:

- at most one confirmation-driven cache clear per pool per `stale_cooldown`, and a stale signal from freshly salted text counts as plpgsql, with no further clear;
- `stale_cooldown_max` and `stale_first_failure_max` keep a minimum of `1`, with a loud notice;
- every stale-session setting has a ceiling: durations at most 10 minutes, caps at most 4096.

## What I read

- **On the PR branch:** `rules.md`, `contracts.md`, and `proofs.md` (step 55's section and the new "eleventh review" bullets). I read every changed and new ID in full: R-0060, R-0718, R-0982, R-0984, R-1040, R-1245, R-1246, R-1249, R-1250, R-1253, R-1258, R-1262 to R-1269, T-0306, T-0312 and T-0317. For the rule set I also re-read R-0981, R-0442 (`.retry(n)`'s backoff), R-1229 and R-0852 (`tx.set_local`), and step 48's deferred list.
- **On `origin/main`:** INDEX's Decisions entries "Step 55 tenth review, EN1 to EN5" and "Step 55 eleventh review, EO1 to EO10", with the EO IDs and the fix's own choices, and `reviews/step-55-review-11.md`.
- **The PR body.**

## What I checked and ran

- **CI on the head.** `0f26ba7` is green: `ci` on push (run 43708265628) and on the pull request (run 43708270625), 36 of 36 checks passing. The diff touches only `plans/plan-approved/`: `coverage.md`, `proofs.md`, `manifest.json`, and steps 10, 11, 12, 14, 20, 27, 47, 48, 50 and 59. No code, test or workflow changed.
- **Coverage checker,** run on a `git archive` of the head:
  - `check_coverage.py --step 55 --code crates tests benches fuzz .github`: **1269 requirements, 317 test obligations, 0 problems**;
  - the plain form gives the same.
- **Manifest against INDEX.** I compared the manifests at `c4b4ce2` and `0f26ba7` ID by ID:
  - changed: R-0060, R-0718, R-0982, R-0984, R-1040, R-1245, R-1246, R-1249, R-1250, R-1253, R-1258, R-1262 to R-1268, T-0306, T-0312 and T-0317;
  - new: R-1269;
  - none removed, and `frame` and `master_sha256` unchanged.

  That's exactly INDEX's EO entry, "Written in at `0f26ba7`", and the PR body's list.
- **R-0060's copies.** With the quote marks stripped, the `morm.toml` block in steps 11, 12, 27 and 47 is byte for byte step 10's, including the five changed comment lines.
- **Spot checks on throwaway Postgres 18.6,** a cluster under `.claude/scratch/` in my worktree with `TMPDIR` set there and its socket under `~/.claude/scratch/`, through `psql`. PgBouncer wasn't run.

  | Claim | Result on 18.6 |
  |---|---|
  | A `0A000` from `RevalidateCachedQuery` needs a schema change | **No.** Schemas `t1` and `t2` each hold an `items` table, `t2`'s with one more column. With `search_path = t1`, `PREPARE s AS SELECT * FROM items WHERE id = $1`, then in a transaction `SELECT set_config('search_path', 't2', true)` (what `tx.set_local` sends, R-1229) and `EXECUTE s(2)`: `0A000` `cached plan must not change result type`, `LOCATION: RevalidateCachedQuery`. `SET LOCAL search_path` gives the same. No DDL ran between the two. See EP2 |
  | Stored text can make the same bytes fail, then succeed, on R-0981's path | `PREPARE r(int) AS SELECT t::theme FROM prefs WHERE id = $1` with row 1's `t` = `'_morm_retired_x'`: `22P02` `invalid input value for enum theme: "_morm_retired_x"`, `LOCATION: enum_in`, though `theme` was never swapped or retired. After `UPDATE prefs SET t = 'dark'`, the same `EXECUTE r(1)` returns `dark`. See EP1 |

## EO1 to EO10: are they fixed?

| Finding | Verdict and evidence |
|---|---|
| EO1 | **Fixed as the owner decided, for the statement generation and the salt.** R-0982's new paragraph "At most one confirmation-driven clear per window" limits a raise driven by a confirming run to once per pool per `stale_cooldown` while `<n>` is unmoved. In the window, a stale signal from text prepared since the raise "is treated as from compiled code". R-0718, R-1245's `XX000` row, R-1246's first two bullets, R-0984 and R-1258's caches sentence point to it. T-0306 and T-0312 add the `server_round_robin = 1` case with one server session that never compiled the function. Gaps: the connection generation isn't covered (EP1); within the window, R-1246 and R-0982 describe different calls (EP3); and nothing tests that the window reopens (EP5) |
| EO2 | **Fixed for the sixth-row path.** The same window covers it. T-0312 adds the three-call `UPDATE ... conv($2) ... ($3::kind)` cycle ten times within a minute, behind PgBouncer and straight, raising once. **But** the same client trick, a row changed between two runs of the same bytes, still works on R-0981's and R-0984's retired-label path straight to Postgres. There it raises the connection generation, which no window limits (EP1) |
| EO3 | **Fixed as the owner decided.** R-1262 keeps floors of `1` and states their cost. R-1268's "Small values cost connections" gives the two-statement case. T-0317 starts a pool at `1`. One sentence overstates the cost behind PgBouncer (EP8) |
| EO4 | **Fixed as the owner decided.** R-1262 sets `"1s"` to `"10m"`, up to `"10m"`, and `0`/`1` to `4096`. The refusal names "the floor or the ceiling", and R-1262 explains why. R-0060 and its four copies, and R-1263 to R-1267, give the bounds. R-1268 states each setting's cost. T-0317 tests `"11m"` twice, each count at `4097`, and `"10m"` and `4096` starting. R-1268's `stale_cooldown` cost misses the main one the window adds (EP4) |
| EO5 | **Fixed.** R-1246's new sub-bullet "Checks turned off" covers both modes. R-1268 is corrected straight to Postgres. T-0312 asserts the outcome for `stale_pending_expiry = "off"` and `stale_pending_max = 0`, behind PgBouncer and straight. The PR body's settings note now matches R-1268 |
| EO6 | **Fixed.** T-0312 adds `stale_pending_max = 2`, `stale_first_failure_max = 2`, and a same-values run two seconds after a `"1s"` expiry that adds no salt |
| EO7 | **Fixed.** R-1246's "Other values meanwhile" says an other-values run that ends in a plpgsql signal is a failed confirmation, with the outcome inside and outside a transaction. The summary line, R-1253's last sentence and a T-0312 case carry the cost |
| EO8 | **Fixed.** R-1269 is in step 48, with its deferred-list entry, `coverage.md` and the manifest |
| EO9 | **Fixed.** R-0984, R-1040, R-1249, R-1250 and R-1253 name `stale_cooldown` or `stale_pending_expiry`. R-0984 now sends a failed plpgsql `XX000` "inside a transaction into step 14's memory ... and outside one to step 14's retry on another connection". R-1262 says "Every tunable ..., these five" |
| EO10 | **Fixed in the main.** `stale_pending_expiry` takes `"off"`, `stale_cooldown = "off"` is refused, and R-0060 says `"off"`. **But** "`0` also means off" contradicts the new "a duration written as a bare number is refused", and T-0317 asserts both (EP6) |

Nothing earlier regressed in the text: SA to SH, EF1, EG1 to EG3, EH1 to EH8, EI1 to EI9, EJ1 to EJ9, EK1 to EK8, EL1 to EL7, EM1 to EM6 and EN1 to EN5 read consistently with it. EP1 sits in R-0981's and R-0984's confirming retry, which earlier rounds treated as sound because stored text was taken to be the same on both runs. EO2's own finding, that a client can change its row between two runs, is what reopens it, and the owner's "whatever the cause" is the standard this round measures it against.

## The rule set, path by path

Straight to Postgres:

| Path | Heals? Bounded? |
|---|---|
| Hand-made swap, sixth-row `22P02` or R-0718's retried error | First confirmation raises the statement generation, then the window. Heals; a second change within the window waits up to about twice `stale_cooldown` (EP4, EP5) |
| A client's bad label, row change, same bytes (EO2) | One raise per pool per `stale_cooldown`. Bounded |
| Retired-label `22P02` from text a statement reads from a row the client can write, outside a transaction | R-0981's retry on a newly opened connection confirms if the row changed while that connection opened. Then it raises the **connection generation** and the pool is replaced. A failed retry fills the memory, so a toggling client wins only sometimes. Not limited by any window (EP1) |
| The same inside `.retry(n).transaction(..)` | First failure: record, discard, backoff, attempt on a new connection. A row changed during the jittered backoff (R-0442) makes the attempt confirm. The connection generation is raised, which clears the records, so the next round is a first failure again. **Every round replaces the whole pool, with no bound** (EP1) |
| `0A000` from a per-transaction `search_path` over tables of different shapes | Each switch raises the statement generation, and inside a transaction returns `SchemaChanged`. Not limited (EP2) |
| Plpgsql stale after an unmanaged change; real cutover | As in earlier rounds: a new connection confirms and the pool is replaced. Heals |

Behind a pooler:

| Path | Heals? Bounded? |
|---|---|
| Plpgsql stale in one of several server sessions (EO1) | One salt per process per `stale_cooldown` to `2 × stale_cooldown`, the statement's calls that reach the stale session failing in between. Heals at `server_lifetime` or `RECONNECT`. Bounded |
| Client's bad label cycle (EO2) | One salt per pool per window. Bounded |
| Hand-made swap, first change | One `SchemaChanged` or one unnamed run per process, then a salt. Heals |
| A second hand-made change within the window | The salted text's stale signal counts as plpgsql. The calls fail until the window and then the memory run out, up to about twice `stale_cooldown`, unstated (EP4). What the call returns meanwhile reads two ways (EP3) |
| `0A000` | Not limited, though R-1246's first bullet says it is (EP3). A per-transaction `search_path` makes it client-paced (EP2) |
| Retired-label `22P02`, `42804` | No record, no discard, no raise. Sound |

Across all these paths, nothing grows past `max` or the budget, and every set is bounded by its setting and now by a ceiling. No path gives a silent wrong answer beyond what R-1253 and R-1268 state.

**The rule set isn't sound yet.** Straight to Postgres, a client that can write text a statement casts to an enum can make every `.retry(n)` round replace the whole pool (EP1). A per-transaction `search_path` makes each switch clear the caches (EP2). Behind PgBouncer, the window bounds churn as decided. But its cost to a real second change isn't stated (EP4) or tested (EP5), and its in-window call reads two ways (EP3).

## Findings

### EP1. Blocks approval. R-0981, R-0984, R-0982, T-0312: straight to Postgres, the confirming retry on a newly opened connection raises the connection generation outside the one-clear window, so a client that changes a row between the two runs replaces the whole pool, inside `.retry(n)` with no bound

- **Where:**
  - R-0981: the retry on a newly opened connection "confirms a stale session when it succeeds, with or without rows ...", and then "morm raises the connection generation".
  - R-0984: `.retry(n)`'s attempt "confirms only if it runs the statement that failed with the same text and the same parameter bytes ... then morm raises the connection generation, the pool's other connections are replaced at checkout". Also: "The records are ... all cleared when the connection generation moves."
  - R-0982's window covers "a raise driven by a confirming run, an unnamed or re-prepared run ... as for a `22P02`, for an `XX000` from a cached statement behind a pooler, or for any error R-0718 retries". That's the statement generation and the salt. R-0981's and R-0984's connection-generation raise isn't in it.
  - R-0442: "Between attempts the transaction rolls back and the connection is released during the jittered backoff."
  - INDEX's EO entry: the window "bounds churn whatever the cause, behind a pooler or straight".
- **What's wrong:**
  - The `22P02` R-0981 handles needs no retired type. The spot check gets it from `enum_in` on a plain enum, because the message holds the input text. The plan names the source itself: "a function converting text it read from a table" (R-0981), "a value read from a table" (R-1249). The parameter test sees only the call's own bound parameters.
  - **Inside a transaction, straight to Postgres, with `.retry(2)`:**
    1. A client stores `_morm_retired_x` in a text column a statement converts, then calls it.
    2. Attempt 1 fails. It's a first failure: morm records it, discards the connection and returns `SchemaChanged`, and `.retry(n)` backs off.
    3. During the backoff, the client's next request sets the text to `dark`.
    4. Attempt 2, on a newly opened connection with the same bytes, succeeds. That confirms, and the connection generation is raised, so every connection in the pool is replaced.
    5. Raising the connection generation clears the first-failure records, so the next round is a first failure again.

    Each round costs one discarded connection and a full pool replacement, at the client's request rate, with no bound. The timing needs only sequential requests, because the backoff is jittered and the attempt opens a new connection.
  - **Outside a transaction,** R-0981's retry waits only for a new connection to open, so the client must race it, and a lost race fills the memory for `stale_cooldown`. That's bounded in practice, but not by any rule.
  - **The window makes it worse.** Inside the window, R-0982 sends a stale signal from freshly prepared text to compiled-code handling, "outside a transaction after R-0981's retry on another connection". Straight to Postgres, that retry is on a newly opened connection. It succeeds whenever the signal came from a statement and not from plpgsql, and R-0981 then raises the connection generation. R-0982's "no further salt or raise" doesn't say whether it means that raise too. If it doesn't, the window turns a limited statement-generation raise into an unlimited pool replacement. If it does, no rule says what the confirming connection does instead.
  - T-0312 tests the window only through the statement generation. Its retired-label cases change no row between attempts.
- **Evidence:** the spot check's second row; the R-0981, R-0984, R-0442 and R-0982 text above.
- **Fix, a design choice for the owner, since the decision named cache clears, and this one replaces connections:**
  - **(a) Recommended. Apply the owner's own "fresh text" rule to fresh connections.**
    - Within `stale_cooldown` of a confirmation that raised the connection generation, with the database's statement generation unmoved, a retired-label `22P02` on a connection opened since that raise can't come from code compiled before the change. It's treated as stored or client text: R-0981's memory, the error as it is, no new connection and no raise.
    - R-0984 already reads the database's generation. R-0981 reads it once on the rare path, from the `_morm` comment.
    - R-0981's confirming retry inside the window, and R-0982's compiled-code path straight to Postgres, may give the new connection the first one's slot and the caller its result. They raise nothing pool-wide.
    - A real second cutover is a migration, which moves the generation, so it still confirms at once. A hand-made one heals as each stale connection fails once, at most the pool's size.
    - R-0981, R-0982 and R-0984 say so, and R-1040, R-1249 and R-1250 state the rate.
  - **(b)** Keep the raise. A first-failure record a confirmation cleared comes back, so a second round on a connection opened after the confirmation counts as a failed confirmation. That bounds R-0984 to one raise per migration, but not R-0981 outside a transaction.

  Either way, T-0312 adds the cycle: stored text set to `_morm_retired_x`, the call in `.retry(2).transaction(..)`, the text changed during the backoff. Repeated ten times within `stale_cooldown`, straight, it raises the connection generation at most once. T-0311 or T-0312 adds the outside-a-transaction case, with the text changed by a volatile expression so the timing is deterministic, such as a `CASE` on `nextval`.

### EP2. Should fix. R-0982, R-1246, R-0718, PR body choice 27: the window leaves out `0A000` because "only a real change raises" it, but a per-transaction `search_path` raises it with no change, so an app's per-request schema choice clears the caches on every switch

- **Where:**
  - R-0982: "A `0A000` from `RevalidateCachedQuery`, which only a real change raises, and a migration, which moves `<n>`, aren't limited."
  - PR body choice 27, and INDEX's EO entry: "the one-clear window limits only confirmation-driven raises, not a `0A000`".
  - `tx.set_local(name, &value)`, whose README line is "A setting must last one transaction, such as the tenant for RLS" (R-0852), sends `set_config(name, value, true)` (R-1229).
- **What's wrong:**
  - The spot check gets `0A000` from `RevalidateCachedQuery` with no DDL at all. A cached statement re-analyses when the `search_path` it was planned under differs, and fails if the table it now finds has another row shape.
  - A schema-per-tenant app sets `search_path` per transaction, through `tx.set_local("search_path", ..)` or `SET LOCAL`. Its tenant schemas differ in shape while a per-tenant rollout is under way, or for good if one tenant has an extra column. Then each request for a tenant whose shape differs from the cached plan's gets `0A000`.
  - Each `0A000` is a certain signal (R-0982), so it raises the statement generation straight, or adds a salt and raises it behind a pooler. Every connection then drops its caches. Inside a transaction, the call also returns `SchemaChanged`.
  - The rate is the rate at which requests switch tenants, which clients set, and no window limits it. So the exclusion leaves the client-paced cache clearing that EO1 and EO2 closed, on another signal.
- **Evidence:** the spot check's first row; R-0852 and R-1229.
- **Fix, a design choice for the owner, since the fix's choice rests on the premise:**
  - **(a) Recommended.** Bring `0A000`'s pool-wide raise under the same window, keeping its local recovery:
    - straight to Postgres, the statement is still evicted and prepared again on that connection, and the call retried once;
    - behind a pooler, the unnamed run still answers the call outside a transaction;
    - only the statement-generation raise and the salt wait for the window.

    A second hand-made column change inside the window then costs one unnamed run per call, outside a transaction, until the window ends, with no wrong answer. R-0982 drops "which only a real change raises", and R-1253 states the rate.
  - **(b)** Keep `0A000` unlimited. R-0982 states the `search_path` case, and the README says loudly, beside `tx.set_local`, that a per-transaction `search_path` over tables of different shapes costs a `SchemaChanged` and a cache clear on every switch.

  Either way, T-0312 or T-0306 adds two `search_path`s over tables of different shapes, alternated ten times within `stale_cooldown`, and asserts what the chosen rule gives.

### EP3. Should fix. R-1246, R-0982, R-1245: inside the window, the two rules describe different calls; a builder can't tell whether the unnamed run still runs and returns its answer, or whether `0A000` behind a pooler is limited

- **Where:**
  - R-1246's first bullet covers every stale-statement signal, `0A000` included. It runs the call unnamed, and on success raises the generation "at most once per `stale_cooldown` while `<n>` is unmoved, under R-0982's window".
  - R-0982: within the window, "a stale signal from a cached statement prepared since it ... is treated as from compiled code: no further salt or raise, and the statement enters R-0981's memory and its error returns as that memory says, outside a transaction after R-0981's retry on another connection". And: "A `0A000` ... aren't limited."
  - R-1245's `XX000` row: "a stale session from plpgsql once it fails or inside that window". There's no "prepared since" condition, while R-0982 says a confirmation from a statement "prepared before the raise still returns its run's own result".
- **What's wrong:**
  - **Behind a pooler, outside a transaction, in the window,** `SELECT (($1::text)::st)::text` hits a second hand-made drop, under the salt.
    - Under R-1246's first bullet, the call runs unnamed, succeeds, and the caller gets the right answer with no salt.
    - Under R-0982, the error returns after R-0981's retry, which reaches the same salted server statement, and the statement enters the memory, so every call fails for up to `stale_cooldown`.
  - **`0A000` behind a pooler.** R-1246 limits it, and R-0982 doesn't. If it's not limited, a second `0A000` needs a new salt, but R-0982's "that raise carries R-1246's one salt" and R-1246's "kept until `<n>` next moves" don't say a second salt replaces the first.
  - **R-1245** classifies an in-flight confirmation from before the raise as plpgsql, which R-0982 says isn't so.
- **Fix:**
  - R-1246's first bullet states, for text prepared since the raise within the window, the outcome the owner chose: either the unnamed run still runs and returns its result with nothing raised, or the call goes straight to R-0981 with no unnamed run. It also says whether `0A000` is excepted, which follows EP2's choice.
  - If `0A000` stays unlimited behind a pooler, R-1246 says each one replaces the salt with a new one.
  - R-1245's row adds "from text prepared since R-0982's raise".
  - T-0306 asserts the chosen in-window outcome for the second-drop case.

### EP4. Should fix. R-0982, R-1253, R-1268, R-1263: the window's cost to a real stale statement, a second change made by hand within `stale_cooldown` of a confirmation, isn't stated, and R-1253 says the opposite

- **Where:**
  - R-0982 lists "several changes made by hand" among the causes it bounds.
  - R-1253: "each process's first call to a statement the change left stale fails, is retried once unnamed and succeeds, and from then on the process uses fresh text."
  - R-1268's `stale_cooldown` bullet: "Longer ..., and the caches may be cleared that much less often."
  - R-1263's use-when names only retired labels and refused connections.
- **What's wrong:**
  - Say a confirmation at `t0` added a salt, and a second hand-made change, such as the next statement of a DBA's swap, makes the salted text stale within `stale_cooldown`.
    - Its signals count as plpgsql: no salt, and the statement enters the memory at `t1`, which returns its error at once until `t1 + stale_cooldown`.
    - Only the next call after that, now outside the window, confirms and heals.
  - So behind PgBouncer, every call of that statement fails, inside a transaction and out, for up to about twice `stale_cooldown`. That's two minutes by default and twenty at the ceiling, in every process.
  - Straight to Postgres, the same holds for the sixth-row `22P02` path and R-0718's retried errors, unless EP1's open question routes them to a new connection.
  - That's the main cost of the window and of a longer `stale_cooldown`. The owner's rule is that a setting's costs get a loud notice.
- **Fix:**
  - R-0982 states the bound: a stale statement whose first stale signal comes within `stale_cooldown` of an earlier confirmation heals within about twice `stale_cooldown`, and fails meanwhile.
  - R-1253 adds: "A second change made by hand within `stale_cooldown` of the first one's confirmation, 60 seconds by default, fails that statement's calls for up to twice that, before the process moves to fresh text: make hand-made changes in one transaction, or run them through morm."
  - R-1268's `stale_cooldown` bullet and R-1263 say the same in their terms.

### EP5. Should fix. T-0306, T-0312: no test shows that the window reopens, so a build that allows one confirmation per `<n>` forever passes, and real healing after a second change is never shown

- **Where:**
  - T-0306: "calls of it outside a transaction raise the statement generation at most once per `stale_cooldown`".
  - T-0312: "raise the statement generation at most once per `stale_cooldown` ... and nothing is cleared again within the window", and the three-call cycle "repeated ten times within a minute ... raises the statement generation once".
- **What's wrong:**
  - Every new assertion is an upper bound inside one window. A build that never re-allows a confirmation while `<n>` is unmoved passes them all, yet leaves every later hand-made change stale until a migration or `server_lifetime`.
  - The question the window has to answer, whether a real stale session still heals in bounded time, has no test.
  - `rules.md`: "A requirement that holds several rules needs tests for each."
- **Fix:** with `stale_cooldown = "1s"`, T-0312 and T-0306 add, behind PgBouncer and straight:
  - a hand-made swap confirmed;
  - a second hand-made change within the second, whose statement fails as EP4's bound says and then, after the window and the memory have run out, confirms once more, raising the generation a second time and healing;
  - one in-flight confirmation from text prepared before the raise, which returns its run's result and moves nothing.

### EP6. Should fix. R-1262, R-1268, T-0317, PR body: `stale_pending_expiry = 0` is both "off" and "a duration written as a bare number", which is refused

- **Where:**
  - R-1262's row: "up to `"10m"`, or `"off"`, which `0` and `"0s"` also mean". Its next paragraph: "A value outside these bounds, or a duration written as a bare number, stops the pool at start with `Error::Config`."
  - R-1268: "`stale_pending_expiry = "off"`, or `0`".
  - T-0317 asserts both "a duration written as a bare number ... stop the pool" and "`stale_pending_expiry = "off"`, `0` and `"0s"` ... each start the pool".
  - The PR body's table: "(also `0`, `"0s"`)".
- **What's wrong:**
  - In TOML, `0` is an integer, a bare number, so T-0317 asserts two outcomes for the same input, and no build passes both.
  - The env variable has the same problem: `MORM_POOL_STALE_PENDING_EXPIRY=0` is the string `"0"`, a bare number too.
- **Fix:** pick one and write it in R-1262, R-1268, T-0317 and the PR body:
  - **(a) Recommended:** only `"off"` and `"0s"` turn checks off, and `0` and `"0"` are refused as bare numbers, consistent with the fix's own choice;
  - **(b)** the integer `0` and `"0"` are the one bare number accepted, for this key only, meaning off.

### EP7. Minor. R-1253, PR body, R-0982, R-0060, R-1263: the window is per pool but read as per process; "while `<n>` is unmoved" has no meaning straight to Postgres; the setting's comment and use-when line don't name the window

- **Where and what:**
  - **Per pool or per process.** R-0982 and R-1262 say "per pool". R-1253 says "at most once per `stale_cooldown` per process", and the PR body says "one clear per process per cool-down". A process with several pools on one database, such as `[pools.jobs]`, clears up to once per pool. R-1253 should say "per pool".
  - **`<n>` straight to Postgres.** R-0982's window holds "while `<n>` is unmoved". Straight to Postgres there's no marker (R-1246: "Straight to Postgres, or with the caches off, there's no marker") and no `<n>` is read outside R-0984's rollback path. R-0982 should say what reopens the window straight: only its end, or the database's statement generation read on that path.
  - **The setting's text.** R-0060's `stale_cooldown` comment ("how long a stale-enum error whose fresh retry failed returns at once") and its four copies don't mention that it also sets the one-clear window, which R-1262's "Governs" column does. R-1263's use-when doesn't mention it either.

### EP8. Minor. R-1268: "Small values cost connections" says a discarded connection inside a transaction, which happens only straight to Postgres

- **Where:** R-1268: "with `1`, two statements failing in turn each cost a new connection outside a transaction, or a discarded connection inside one, on every call."
- **What's wrong:** behind PgBouncer, morm keeps no first-failure record and discards nothing (R-0984). There, two statements pushing each other out of the cool-down memory cost a `SchemaChanged` inside a transaction and a retry on another connection outside one, per call.
- **Fix:** "straight to Postgres, a new connection outside a transaction or a discarded connection inside one; behind PgBouncer, a retry on another connection outside a transaction or a `SchemaChanged` inside one, on every call."

## What I couldn't run

- **PgBouncer.** I didn't run it this round. EP3 and EP4 come from the plan's text and transaction-mode routing, which the eleventh review and R-1238 record.
- **Postgres 15 to 17.** I ran 18.6 only. `search_path` re-analysis in `RevalidateCachedQuery`, and `enum_in` printing the input text, are long-standing.
- **morm itself.** There's no runtime yet. Every outcome under the rules comes from their text.

## Verdict

**Not approved yet.**

What holds:

- every changed and new ID is in INDEX's EO entry at `0f26ba7`, and the PR body's lists match;
- the checker reports 1269 requirements, 317 test obligations and 0 problems both ways, and CI is green on the head, 36 of 36;
- EO1 to EO10 are written in as decided or recommended, apart from the gaps above;
- the floors, ceilings and `"off"` handling match across R-0060 and its copies, R-1262 to R-1268 and T-0317, apart from EP6;
- behind PgBouncer, the window bounds the churn EO1 and EO2 found;
- nothing passes `max` or the budget, and no path gives a silent wrong answer beyond what the README states.

What remains:

- **Blocks approval:**
  - EP1: straight to Postgres, a client changing a row between the two runs makes R-0984's confirming attempt replace the whole pool on every `.retry(n)` round, outside any window (a design choice; (a) recommended).
- **Should fix:**
  - EP2: `0A000` isn't only from a real change; a per-transaction `search_path` makes it client-paced and unlimited (a design choice; (a) recommended);
  - EP3: in the window, R-1246 and R-0982 describe different calls, and `0A000` behind a pooler reads both ways;
  - EP4: a second hand-made change within the window fails for up to twice `stale_cooldown`, unstated, and R-1253 says otherwise;
  - EP5: no test shows the window reopening or a second change healing;
  - EP6: `stale_pending_expiry = 0` is both off and refused, and T-0317 asserts both.
- **Minor:**
  - EP7: per pool against per process, `<n>` straight to Postgres, and `stale_cooldown`'s comment and use-when line;
  - EP8: R-1268's small-values line overstates the cost behind PgBouncer.
