# Step 55's concurrency model

The Rust model fix agents run to check the stale-session procedure (R-1270, R-1271, T-0319), with its Postgres probes and notes. This copy is from the round of the twenty-fourth review's fix (PR #3, FB1 to FB7 since `018b3fb`; head `59286dc`; work branch commit `00e0aa7`): I8 checks that with pending entries off no run but `.retry(n)`'s later attempts is unnamed, three settings measure the `search_path` case with entries on and off (`model/fb1.sh`, `results/fb1.txt`), and `probes/run10.sh` runs FB1's case on Postgres. `probe.py` to `probe4.py`, `po.py`, `probes/po2.py`, `probes/probe5.py` to `probe7.py`, `probes/src8.py`, `probes/run9.sh` and their results are earlier rounds'; `probes/run10.sh` is this round's. The coordinator replaces this copy after each later round. It isn't part of the workspace or the product.

- **Fix agents:** start from this copy instead of rebuilding it. Work on it on your `wip/...` branch and report the commit.
- **Reviewers: don't read or run anything here.** A review builds its own model, so that it doesn't repeat this one's mistakes.

`results/` holds this round's runs beside the earlier rounds' probe results, which `NOTES.md` lists. `NOTES.md` is its agent's working notes, and its paths are those of its `wip` branch (`plans/wip/`).
