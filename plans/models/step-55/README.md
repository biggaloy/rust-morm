# Step 55's concurrency model

The Rust model fix agents run to check the stale-session procedure (R-1270, R-1271, T-0319), with its Postgres probes and notes. This copy is from the round of the twenty-third review's fix (PR #3, FA1 to FA4 since `e417bc8`; head `018b3fb`; work branch commit `e15dc36`): I14 checks the second row's cost with pending entries off, with two new settings, break 124 and the FA3 savepoint probe (`probes/run9.sh`). `probe.py` to `probe4.py`, `po.py`, `probes/po2.py`, `probes/probe5.py`, `probes/probe6.py`, `probes/probe7.py`, `probes/src8.py` and their results are earlier rounds'; `probes/run9.sh` is this round's. The coordinator replaces this copy after each later round. It isn't part of the workspace or the product.

- **Fix agents:** start from this copy instead of rebuilding it. Work on it on your `wip/...` branch and report the commit.
- **Reviewers: don't read or run anything here.** A review builds its own model, so that it doesn't repeat this one's mistakes.

`results/` holds this round's runs beside the earlier rounds' probe results, which `NOTES.md` lists. `NOTES.md` is its agent's working notes, and its paths are those of its `wip` branch (`plans/wip/`).
