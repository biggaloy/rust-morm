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
