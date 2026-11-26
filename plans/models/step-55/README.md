# Step 55's concurrency model

The Rust model fix agents run to check the stale-session procedure (R-1270, R-1271, T-0319), with its Postgres probe and notes. This copy is from the round of the two answers to the three answers' questions (PR #3, `64820fe`; work branch commit `01386ac`); `probe.py` to `probe4.py`, `po.py`, `probes/po2.py`, `probes/probe5.py` and their results are earlier rounds', `probes/probe5.py` run again with `probes/run6.sh`; the coordinator replaces it after each later round. It isn't part of the workspace or the product.

- **Fix agents:** start from this copy instead of rebuilding it. Work on it on your `wip/...` branch and report the commit.
- **Reviewers: don't read or run anything here.** A review builds its own model, so that it doesn't repeat this one's mistakes.

`results/` holds that round's runs. `NOTES.md` is its agent's working notes, and its paths are those of its `wip` branch (`plans/wip/`).
