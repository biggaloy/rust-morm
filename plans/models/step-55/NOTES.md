# Step 55's stale-session model, after the EZ-answers round

- `model/`: Rust, no crate. `bash model/build.sh` builds it to `.claude/scratch/model` (change the path at will). `model <setting|all|faults|setNN> <switch or break> <seed0> <count> [straight|pooled|both]`; `V=1` prints counts, `T=` threads, `OPT=noreset,real,notrig,noboth,pooled2`.
- `model all 0 0 256`: every setting, no fault. `model faults 0 0 256`: every switch at its T-0319 setting (34 to 39 are the six answers', 40 is labels with no random part, 132 the 32nd with held connections kept longer). `python3 model/breaks.py <binary>`: the other deliberate breaks (41 to 46, 101 to 107). `bash model/full.sh`: all of it, 256, 2,048 and 20,000, about 45 minutes on 11 threads.
- Check names starting "lit ", "ez " and "mid " are older wordings, kept to show what they failed; only the others count.
- Outputs of this round: `all256.txt`, `all2048.txt`, `all20000.txt`, `faults256.txt`, `faults2048.txt`, `breaks.txt`, `pooled2-2048.txt` (the second row first behind PgBouncer, open question 2).
- Probes: `probe2.py` (17 and 18 outputs beside it), `probe3.py` (row C2 through PgBouncer), `probe4.py` (the reset run later), `po.py` (the message in every language).
- `edit_ans.py`, `proofs_add.md`, `edit_pr2.py`: how this round's text was written. `ez/`: the EZ round's files.
