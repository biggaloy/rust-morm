# Step 55's stale-session model, after the three answers' round

Started from `plans/models/step-55/` on `main` (the EZ-answers round's). What changed:

- `model/rule.rs` is R-1245's second row's test as decided: it reads only what stands before the message's value. The model now writes each `22P02`'s message as the server would (language, the type's printed name, the value) and gives it that function; there are no booleans for it any more. `model/ruletest.rs` runs the function alone: `ruletest` over every language, name and value, `ruletest <messages.tsv>` over the messages Postgres itself gave (`probes/probe5.py`).
- `core.rs`: each sequence draws the language, the type's name, `quote_all_identifiers` and how a kept type is renamed from a generator of its own (`speak`). Statements 8 (`STC`, a column's text converted beside the parameter's cast, in the load) and 9 (`NOC`, no parameter cast, only with `OPT=nocast`).
- New settings: `English, a type named as a word`, `a type named as a word` (Spanish, `entrada`), `messages the test can't read`.
- Deliberate breaks of the test, `model set<nn> <nn> ...`: 110 the rule it replaces (bound texts taken out), 111 the last colon a quote mark follows, 112 colons inside double quotes count, 113 English reads any word, 114 an unreadable message is not the second row's, 115 the schema ignored, 116 names compared as printed, 117 only `"` is a mark, 118 a first colon with no mark is passed over, 119 Chinese and Japanese letters count as letters beside a name.
- `OPT=onpath` and `OPT=nocast` are the two cases the open questions raise; `strict` and `wide` the options recommended for each.

`bash model/build.sh` builds to `.claude/scratch/model`; `model <setting|all|faults|setNN> <switch or break> <seed0> <count> [straight|pooled|both]`; `V=1` prints counts, `T=` threads. `python3 model/breaks.py <binary>`; `bash model/full.sh` (the 20,000s take about 15 minutes each mode).

Check names starting "lit ", "ez " and "mid " are older wordings, kept to show what they failed; only the others count.

Probes: `probes/po2.py` (the message in all 58 releases of 15 to 18), `probes/probe5.py` and `run5.sh` (Postgres 17 and 18). Results in `results/`; `rule-postgres.txt` is the rule over Postgres's own messages.
