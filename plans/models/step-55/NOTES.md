
## State after the model and probe (before the usage limit)

Model works (plans/wip/model: bash build.sh; model all|faults|<setting> <fault> <seed0> <count> [straight|pooled]; OPT=both,a1s,real,trig,row7,noreset).
Check names: "lit " = 93854c2's sentence, "ez " = review's, "mid " = partial, others = final wording. All 52 no-fault settings pass the
final wording; all 33 switches caught (see `model faults 0 0 256`). Probe: probe.py, probe17.txt, probe18.txt.
Findings to write: I1 (n each was taken under), I2 (adoption), I8 held (XX000 exception first, visible, parameter test), I8 pooled
RECONNECT/server_lifetime (begun since), I8 per-E clauses (no run begun while its entry lasts), clearing clause (another process's
copy), I9 silent time bound (first request, not first confirmation: a marked entry's confirming run requests nothing), I9 loud
(visible; decision on it makes no request...; not "returned as it is ... makes no request" which switch 33 slips through), I11 (40001).
Load: rerun in a new tx; newest label when the statement runs (switch 17: 73 vs 0); search_path/record shapes drawn; long-holds
setting (from-then-on half: 1/256 at the defaults, 215 with long holds); two processes setting.
R-1294 must include the reset: relabel alone leaves constants answering `_morm_retired_a` silently (probe 9).
Open questions: Q1 A1/C1 fresh run's session-class error + row 2/row 7 order (probe 2, 3; OPT=both, trig);
Q2 third silent case (probe 1); Q3 row 2's name test vs the bound value (probe 5); Q4 C2 "another client connection" when the pool
has none free (model deadlocked with 4 calls in C2); Q5 an entry doesn't hold its first error.
