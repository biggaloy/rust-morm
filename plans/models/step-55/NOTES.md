# Step 55's stale-session model, after the twenty-eighth review's fix

Started from `plans/models/step-55/` on `main` (the twenty-seventh review's fix, PR #3 `c458987`). FF1 changes nothing I4 checks: I4 already fails more than `4 × max + 4` stale-driven opens in any span of `W`, plus `2 × max` per change, and the default load already has a client's `42804` with no schema change (statement `H`, a record shape chosen per call, `Cause::Data`) and stored text toggled at the client's pace. FF1 and FF2 are README wording; what's new shows the bound by kind of replacement, and a case the sweep found: stored text changed to a good label between a failure and its confirming run requests *replace* too.

What changed:

- `model/core.rs`, `Pool::repl_t`: each stale-driven replacement, at the time it's counted, and its kind, 0 an epoch's (a replacement, a discard, a held connection's release), 1 a *replace* action's at checkout; `machine.rs` and `calls.rs` push it beside every `ep_repl` and `cg_repl` count.
- `model/machine.rs`: counters "FF1 replace requested: a 42804 or XX000 at its failure", "... a confirming run on the new connection, row A2" and "... a confirming next attempt, row B3".
- `model/world.rs`, `end_checks`, over the span of `W` ending at each replacement, a change counting when it came up to `W` before the span began (a standby's replay or a read can trail its commit): "I4 span replacements, FF1", more than `4 × max + 2 × max` per change, R-1268's new "Shorter:"; "I3 span, an epoch's replacements, R-0981", an epoch's more than `2 × max + max` per change; "lit FF1 R-1268 shorter stale_cooldown, 2 × max replacements per span", the older wording, which doesn't count; counters for each kind's total and the most in a span.
- `model/main.rs`: two settings in both modes, "stored text only, no schema change" (statement `ST`, world events only toggling the stored text) and "record shapes only, no schema change" (statement `H`, no world events); 86 settings, 47 straight to Postgres and 39 behind PgBouncer.
- `model/ff1.sh <binary> [count]` prints the counts straight to Postgres (`results/ff1.txt`); `model/cmp.py` also sets aside FF1's and R-0981's lines and the two new settings.

This round ran `run-all.sh` with the binary in the worktree's `.claude/scratch/`, then `switches.py` and `ff1.sh`.

Results:

- **No check fails with no fault** in the 86 settings over 256, 2,048 and 20,000 sequences (seeds 0 to 255, 1,000 to 3,047, 10,000 to 29,999), but the older wordings' ("lit ", "ez ", "mid "). `cmp.py` against `c458987`'s outputs: the same, once FF1's lines, the two settings, timings and example seeds are set aside. "lit FF1" fails in 37 of the 47 straight-to-Postgres settings at 256 sequences, 40 at 2,048 and 42 at 20,000 (6 to 19,869 of 20,000).
- **Switches:** every one caught exactly as before (`results/switches.txt`, the same counts); the new checks also catch switch 2 (255 and 221 of 256), 3, 9, 14 and 19. **Breaks:** the same counts (`results/breaks.txt`).
- **FF1,** 256 sequences of 40 s straight to Postgres, `max` = 4, `W` = 1 s, the most replacements in a span / of an epoch's / *replace* requests by a `42804`, A2, B3 / "lit FF1": stored text only 11 / 8 / 0, 4,110, 1,027 / 219 of 256; record shapes only 11 / 7 / 39,037, 0, 0 / 255; defaults 19 / 15 / 7,766, 1,217, 521 / 30; transactions without retry 21 / 16 / 12,466, 693, 0 / 156 (`results/ff1.txt`).

Not run: Postgres and PgBouncer (FF1 and FF2 are wording of morm's own procedure), `fe1.sh` and the earlier rounds' scripts, as nothing they count changed and the outputs are the same.

# Step 55's stale-session model, after the twenty-seventh review's fix

Started from `plans/models/step-55/` on `main` (the twenty-sixth review's fix, PR #3 `b98bada`). FE1 changes what the model checks: behind PgBouncer a stale session's cool-down ends at every pool-wide action, adoption or push-out by the cap as well as at its expiry, so R-1249's "once" and R-1258's "for `stale_cooldown`" were wrong, and T-0319's I8 now states the bound the procedure gives. FE1's sweep straight to Postgres is I3's and I4's bounds, already checked; FE2 is I8's stale-statement bound behind PgBouncer, already checked; FE3 is the PR body.

What changed:

- `model/core.rs`, `Pool`: `cool_rm`, each cool-down entry removed before its expiry, with when and why (1 a pool-wide action, 2 an adoption or a moved `<n>`, 3 the cap), and `d2_sc`, each statement's last row D2 `Error::SchemaChanged`.
- `model/machine.rs`: `clear_tables_k` records the removals, `on_moved` with kind 2, every take with kind 1; `session`'s `add` records a push-out by the cap. At each row D2 `Error::SchemaChanged` inside a transaction, against the statement's last one in that pool: "new I8 FE1 two D2 SchemaChanged of a statement within stale_cooldown, no removal between" fails one within `stale_cooldown` of the last with no removal between, counted when it came before that entry's expiry; "lit FE1 R-1249 and R-1258, a stale session's SchemaChanged once per stale_cooldown", the older wording, fails any one within `stale_cooldown`, and doesn't count; counters "FE1 D2 SchemaChanged: the first", "after a pool-wide action", "after an adoption or a moved <n>", "after a push-out by the cap" and "after the expiry".
- Breaks: 125, D2 never putting the statement in the cool-down inside a transaction; 126, D2 putting it there a step after its failure; 127, a cool-down entry lasting half of `stale_cooldown`. `model/breaks.py` runs each behind PgBouncer at "defaults" and "transactions without retry".
- `model/fe1.sh <binary> [count] [faults...]` prints the counts at seven settings behind PgBouncer (`results/fe1.txt`, fault 0); `model/run-all.sh <binary> <results dir>` is `full.sh`'s commands with both given; `model/cmp.py OLD NEW` compares two outputs, setting aside timings, example seeds and FE1's lines.

This round ran `run-all.sh` with the binary in the worktree's `.claude/scratch/`, then `switches.py` and `fe1.sh`; `cost.sh`, `fa2.sh`, `fb1.sh`, `fc.sh` and `fd1.sh` weren't run again, as nothing they count changed.

Results:

- **No check fails with no fault** in the 82 settings over 256, 2,048 and 20,000 sequences (seeds 0 to 255, 1,000 to 3,047, 10,000 to 29,999), but the older wordings' ("lit ", "ez ", "mid "). `cmp.py` against `b98bada`'s outputs: the same, once FE1's lines, timings and example seeds are set aside.
- **Switches:** every one caught exactly as before (`results/switches.txt`, the same counts). **Breaks:** the earlier ones as before; 125 in 256 and 256 of 256, 126 in 173 and 239, 127 in 256 and 256, at "defaults" and "transactions without retry" (`results/breaks.txt`).
- **FE1,** 256 sequences of 40 s behind PgBouncer, `W` = 1 s, row D2's `Error::SchemaChanged`, first / after an action / after an adoption / after a push-out / after the expiry: "defaults" 1,278 / 6,651 / 6,260 / 0 / 1,569; "transactions without retry" 1,280 / 12,091 / 11,286 / 0 / 1,892; "caps of 1" 1,280 / 3,675 / 3,637 / 22,502 / 141; "heavy" 1,280 / 2,915 / 11,413 / 0 / 712; "two processes" 2,554 / 8,047 / 7,601 / 0 / 4,381; "three processes, heavy" 3,810 / 4,615 / 11,763 / 0 / 5,700; "hand relabels without the reset" 1,633 / 8,108 / 3,316 / 0 / 2,049. "lit FE1" fails in 256 of 256 at each, and in 31 of the 37 settings behind PgBouncer in `all256.txt`, 6 to 256 of 256.

Not run: Postgres and PgBouncer (FE1 to FE3 are wording of morm's own procedure), and the 60-second defaults, which the review's own trace ran.

# Step 55's stale-session model, after the twenty-sixth review's fix

Started from `plans/models/step-55/` on `main` (the twenty-fifth review's fix, PR #3 `e217a1c`). FD1 changes what the model counts, not what it checks: R-1253's and R-1272's gap clauses now count the transactions that run the statement before morm marks it pending, which is I8's set ("new I8 a failure begun while its entry exists", unchanged). FD2 and FD3 are wording the model already agreed with: R-1268's and R-1292's rate for a bad enum value is I14's, R-1271's kept function's "once per pending entry, beyond calls that begin before its entry is made" is I14's too, and R-1288's per-connection bound straight to Postgres is I8's ("I8 once per connection") and R-1271's ("R-1271 a stale copy failing twice").

What changed:

- `model/core.rs`, `Pool::path_fx`, and `model/calls.rs`, `check_sc`: each `search_path` `Error::SchemaChanged` with entries on and no `.retry(n)` also records its transaction's start, before its checkout (`machine.rs`, `Cx::tx_t`, set in `call_tx`).
- `model/world.rs`, `end_checks`: FD1's classes. Against the earliest other such failure whose error came before its own and whose entry wasn't made when its run began: "in flight at an earlier one's error" (its run began before that error), "in a transaction started before it" (its transaction began before that error, its run after), "in a transaction started in the gap" (its transaction began after that error); with no such failure, "the first". The last round's two counters stay, and "begun after another's error" is the sum of the middle two.
- `model/world.rs`: "lit FD1 R-1253 and R-1272 gap clause, transactions that start while morm rolls back", the older README wording, fails a sequence with any failure of the first two extra kinds. It's an older wording's check, so it doesn't count.
- `model/main.rs`: one setting in both modes, "search_path only, no retry, up to 100 ms to the run": the `search_path` setting with `co_gap` 100 ms, a random gap of up to 100 ms between a transaction's checkout and its statement's run, as when it runs other statements first. 82 settings, 45 straight to Postgres and 37 behind PgBouncer.
- `model/fd1.sh <binary> [fault] [count]` prints the counts (`results/fd1.txt`).

This round ran `full.sh`'s commands with the binary in the worktree's `.claude/scratch/`, then `switches.py` and `fd1.sh`; `cost.sh`, `fa2.sh`, `fb1.sh` and `fc.sh` weren't run again, as nothing they count changed.

Results:

- **No check fails with no fault** in the 82 settings over 256, 2,048 and 20,000 sequences (seeds 0 to 255, 1,000 to 3,047, 10,000 to 29,999), but the older wordings' ("lit ", "ez ", "mid "). The 80 earlier settings give the same results as at `e217a1c` once "lit FD1" lines, timings and example seeds are set aside.
- **Switches:** every one caught exactly as before (`results/switches.txt`, the same counts). **Breaks:** the same counts (`results/breaks.txt`).
- **FD1,** 256 sequences of 40 s, `E` = `W` = 1 s, 50 transactions a second on a pool of four, per `E`:
  - "search_path only, no retry", straight to Postgres: 2.00 in all, 0.93 the first, 0.04 in flight, 0.00 begun before, 1.03 begun in the gap; "lit FD1" in 194 of 256.
  - The same behind PgBouncer: 2.20, 0.96, 0.04, 0.00, 1.20; 199 of 256.
  - "up to 100 ms to the run", straight to Postgres: 1.92, 0.93, 0.03, 0.57, 0.39; 248 of 256.
  - The same behind PgBouncer: 2.13, 0.96, 0.03, 0.72, 0.41; 256 of 256.
  - So with the run at checkout nearly every extra failure begins in the gap, which is why FC1's counts, by when the run began, matched the older clause; with transactions that reach the statement later, more than half begin before the earlier failure's error. None came from a run begun while its entry existed.

Not run: Postgres and PgBouncer (FD1 to FD3 are wording of morm's own procedure), and the 60-second defaults, which the review's own model ran.

# Step 55's stale-session model, after the twenty-fifth review's fix

Started from `plans/models/step-55/` on `main` (the twenty-fourth review's fix, PR #3 `59286dc`). No finding changes what the model checks, so no check, setting or switch changed, and the switches weren't run again:

- **FC1** moves R-1271's, R-0984's and R-1272's `search_path` bound to the moment the pending entry is made. That's the bound I8 already stated and the model already checked ("new I8 a failure begun while its entry exists"); "lit I8 search_path once per E", an older wording's check kept to show what it fails, excuses only calls begun before the last failure's check.
- **FC2** says that with entries off behind PgBouncer a stale statement fails until the next salt or adoption, which a `0A000` requests at its failure. I8's clearing clause ("I8 clearing": no cached run uses a copy a change left stale once a pool-wide action or an adoption was taken after that change and before the run's read of the generation word) already checks that in every setting, entries off included.
- FC3 and FC4 are wording.

What's new is counting only:

- `model/core.rs`, `Pool::path_fx`, and `model/calls.rs`, `check_sc`: each `search_path` `Error::SchemaChanged` with entries on and no `.retry(n)`, as its run's error time and its call's start. `model/world.rs`, `end_checks`, classes each: "begun after another's error, before its entry", when its call began after another such failure's error and before the first entry made at or after that error, which is FC1's case; or "otherwise".
- `model/calls.rs`, `check_sc`: with entries off behind PgBouncer, "a stale statement's SchemaChanged, no retry", and "one begun after a pool-wide action taken since its change", which should stay at zero, as I8's clearing clause fails it.
- `model/fc.sh <binary> [fault] [count]` prints them (`results/fc.txt`).

Results, 256 sequences of 40 seconds each, fault 0:

- **`all`:** the 80 settings give the same output as `results/all256.txt` at `59286dc`, byte for byte, so the counters change nothing: no check fails but the older wordings' ("lit ", "ez ", "mid ").
- **FC1, "search_path only, no retry"** (`E` = `W` = 1 s, a transaction about every 20 ms on a pool of four): straight to Postgres 20,529 `Error::SchemaChanged` in 10,264 s, 2.00 per `E`, of which 10,610, 1.03 per `E`, began after another's error and before its entry, and 9,919, 0.97 per `E`, otherwise; behind PgBouncer 22,590, 2.20 per `E`, 12,350 (1.20 per `E`) and 10,240 (1.00 per `E`). So about one per `E` plus about one more begun while the entry wasn't made yet, as R-1271 now says. At "transactions without retry" 2,138 of 8,541 and 2,653 of 8,933.
- **FC2,** with entries off behind PgBouncer: 3,104 stale statements' `Error::SchemaChanged` at "entries off" and 20,319 at "transactions without retry, entries off", none begun after a pool-wide action taken since its change; "I8 clearing" fails nowhere.

Not run: the switches (no check changed), 2,048 and 20,000 sequences, and the defaults' `E` = `W` = 60 s, which the review's own model ran (1.57 and 2.46 per `E`).

# Step 55's stale-session model, after the twenty-fourth review's fix

Started from `plans/models/step-55/` on `main` (the twenty-third review's fix, PR #3 `018b3fb`). FB1 changes what the model checks; FB3's outcome while an entry lasts was already checked by I14 and is now counted too; FB2, FB4 to FB7 are wording and Postgres tests the model already agreed with.

- `model/machine.rs`, `run_call`: I8's entries-off clause. With pending entries off and the caches on, no run a call starts, outside `.retry(n)`'s later attempts, is unnamed, so a statement under a per-transaction `search_path` meets the copy its connection, or behind PgBouncer its server session, prepared under whatever path, and fails inside a transaction at every switch from it ("I8 entries off: a run with no entry not cached").
- `model/calls.rs`, `call_tx`: counts "search_path: transactions, no retry", "search_path: SchemaChanged, no retry" and "entries off: search_path SchemaChanged, no retry"; `check_pg`: "a 22P02 the second row takes, as it is in a savepoint while its entry lasts" (FB3's in-between outcome; I14 fails the other one).
- `model/world.rs`, `end_checks`: "ms simulated", to give the rates per second.
- `model/main.rs`: three settings, "search_path only, no retry", the same with entries off by `stale_pending_max = 0`, and by `stale_pending_expiry = "0s"`: one statement under a per-transaction `search_path`, two paths at random, every call a transaction with no `.retry(n)`, no bursts and no world events; 80 settings, 44 straight to Postgres and 36 behind PgBouncer.
- `model/breaks.py`: break 124 (entries off keeping an entry) also at the new entries-off setting, against I8's new clause.
- `model/fb1.sh <binary> [fault] [count]` prints FB1's counts (`results/fb1.txt`).
- `probes/run10.sh <bindir> <scratch> <socketdir>`: FB1's case on a throwaway cluster, `results/probe10-17.txt` (17.11, `/opt/pgsql-17`) and `probe10-18.txt` (18.6, the Manjaro package), the same but the line numbers.
- This round ran `full.sh`'s commands directly with the binary in the worktree's `.claude/scratch/`, and `fb1.sh`; `cost.sh` and `fa2.sh` were not run again (nothing they count changed).

Results: no check fails with no fault in the 80 settings over 256, 2,048 and 20,000 sequences (seeds 0 to 255, 1,000 to 3,047, 10,000 to 29,999); every switch caught exactly as at `018b3fb` (`switches.txt`, the same file); break 124 caught by I8's new clause in 256 of 256 at every entries-off setting, beside I14 (`breaks.txt`). FB1's counts at 256 sequences of 40 s (`fb1.txt`), one statement under a per-transaction `search_path`, 50 transactions a second with no `.retry(n)`, a pool of four: with entries on and `E` = `W` = 1 s, `Error::SchemaChanged` 2.00 a second straight to Postgres and 2.20 behind PgBouncer (4.0% and 4.4% of transactions), the extra over one per `E` being, in the sequences read one by one, calls already running, queued for the pool, when the first failed, which the bound excludes (no failure begins while an entry exists); with entries off, by either setting, 15.6 a second straight to Postgres (31.3%, about a third, as a connection's copy exists only after a success there) and 22.3 behind PgBouncer (44.9%, about half, as the server statement stays).


# Step 55's stale-session model, after the twenty-third review's fix

Started from `plans/models/step-55/` on `main` (the answer to the two answers' question, PR #3 `e417bc8`). FA1, FA3 and FA4 are wording the model already agreed with; FA2 changes what it checks:

- `model/calls.rs`, `check_pg`: I14 with pending entries off and the caches on, no `22P02` the second row takes (`ret: false`) comes back as it is from an attempt before one that returned `Error::SchemaChanged`, in a savepoint or not. `call_tx`: no attempt after one that did returns `Error::SchemaChanged` for bad text or a kept function's error (I14's later half, every setting). Counters: "entries off: SchemaChanged for bad text or a kept function's error" and "entries off: a 22P02 as it is on a later attempt".
- `model/machine.rs`, `entry`: break 124, entries off still keeping an entry (64, `E` = 1 s).
- `model/main.rs`: two settings, "transactions without retry, entries off" and "transactions without retry, E a fifth of W"; 74 settings.
- `model/breaks.py`: 124 at "entries off" and the new entries-off setting, and switch 28 at "entries off".
- `model/fa2.sh <binary>` prints the cost counters at the FA2 settings (`results/fa2.txt`); `model/fa2breaks.sh <binary>` the breaks (`results/fa2breaks.txt`).
- `probes/run9.sh <bindir> <scratch> <socketdir>`: FA3's savepoint case on a throwaway cluster, `results/probe9-17.txt` (17.11, `/opt/pgsql-17`) and `probe9-18.txt` (18.6, the Manjaro package), the same.
- FA4 needs nothing new: I14 already fails a bad text's `Error::SchemaChanged` from a run begun while its statement's entry exists, which is what the corrected tests state.
- `build.sh`, `full.sh` and `cost.sh` find the worktree with `git rev-parse`; this round ran their commands directly with the binary in the worktree's `.claude/scratch/`, as `full.sh` lists them, `cost.txt` not run again (nothing it counts changed).

Results: no check fails with no fault in the 74 settings over 256, 2,048 and 20,000 sequences; the switches as at `e417bc8` but the 28th and 33rd, also caught by I14's later half now (`switches.txt`); break 124 caught in 249 to 256 of 256.

# Step 55's stale-session model, after the owner's answer to the two answers' question

Started from `plans/models/step-55/` on `main` (the two answers' round's, PR #3 `64820fe`). The answer is in by default, with no option for it: R-1245's second row takes every cached statement's `22P02` from `enum_in`, by its SQLSTATE and routine alone, and the name test is gone. What changed:

- `model/machine.rs`, `Sim::class`: a cached statement's `22P02` is the second row's whatever its message names and whatever the statement casts; the seventh row (`_morm_retired_`) still goes first, and choice I is as it was. Nothing reads a message. Break 120: only a statement that casts a parameter takes the second row (as before). Switch 36, reworded: the second row reading the message again, a `22P02` whose message names a type the statement casts to now taken for a client's label. Break 123: the procedure before the answer, the name test reading a statement that casts one parameter, here statement 10's conversion form (odd values), its two-cast form always taken.
- `model/rule.rs` and `model/ruletest.rs` are gone, and with them `core.rs`'s message machinery: the language, the type's name, its printing, `quote_all_identifiers`, how a kept type is renamed or moved (`how`), the labels holding the type's name (`Label::NewName`, now `Newest`), and the settings that served the test: "English, a type named as a word", "a type named as a word" (Spanish), "messages the test can't read" and the `onpath` setting. `OPT=onpath` is gone.
- `model/core.rs`: statement 10 (`TWO`) stands for a parameter's cast beside a second type, by a second parameter's cast or by a conversion of the statement's own SQL, its stale conversion's message naming the type of the first cast, as after an exchange of names or a kept composite or domain. To the procedure as decided the two forms are the same; break 123 tells them apart by the value.
- `model/calls.rs`: the language exceptions of I7 and I9's loud half are gone. I14, which said an error naming the cast's own type is never the second row's, is gone; what was the cost check is I14 now ("I14 the second row's cost"), and a single cast's bad label taking `Error::SchemaChanged` is counted, not failed.
- `model/main.rs`: a new setting, "every call a cast beside a second type", the conversion case the answer decides; switch 36's setting is that load too; settings 108 to 119, 121 and 122 are gone, 123 added.
- `model/breaks.py`: the name test's breaks (110 to 119, 121, 122) and 43 (the answer itself) are gone; 120 and 123 at their settings, at the defaults and, for 123, at the new setting; 107 (pending runs cached) against I14.
- `model/cost.sh`: runs per call and `Error::SchemaChanged` for bad text at the load, beside break 123 and break 120.

Paths: the scripts put the binary, and `run7.sh` its throwaway cluster, in `.claude/scratch/` of the git worktree they're in (`git rev-parse --show-toplevel`), wherever this folder sits. `bash model/build.sh` builds `.claude/scratch/model`; `model <setting|all|faults|setNN> <switch or break> <seed0> <count> [straight|pooled|both]`; `V=1` prints counts, `T=` threads. `python3 model/breaks.py <binary>`; `bash model/full.sh`; `python3 model/switches.py results`.

Check names starting "lit ", "ez " and "mid " are older wordings, kept to show what they failed; only the others count.

Probes: `probes/probe7.py` and `run7.sh` run the decision itself beside asyncpg on Postgres 17.11 (built from ftp.postgresql.org's `postgresql-17.11.tar.bz2`, SHA-256 `dd27f2b3c59e73ed14aa3324901242bf69a032a6347805f274e6260322d42979`, with `--enable-nls`, so it speaks Spanish and German) and 18.6 (the Manjaro package): `results/probe7-17.txt`, `probe7-18.txt`, the same but the version line. `probes/src8.py` reads every C, yacc, lex and header file of 15.20, 16.16, 17.12 and 18.7 (ftp.postgresql.org's tarballs, checked by their published SHA-256) for functions named `enum_in` and their error sites, and `enum.c` in each of the 58 release tags from GitHub: `results/src8.txt`.

Earlier rounds' files, kept as their record: `probe.py` to `probe4.py`, `po.py`, `probes/po2.py`, `probes/probe5.py`, `probes/probe6.py` and `run5.sh`, `run6.sh`, with `results/probe2-*` to `probe6-*`, `po*.txt`, `msgstr.json`, `rule*.txt` and `cost-q.txt`; the `rule*` results and `msgstr.json` served the name test, which is gone, and `rule.rs` that read them is gone with it. This round's runs are `all*.txt`, `faults*.txt`, `breaks.txt`, `switches.txt`, `cost.txt`, `probe7-*.txt` and `src8.txt`.
