# Step 55's concurrency model

The Rust model fix agents run to check the stale-session procedure (R-1270, R-1271, T-0319), with its Postgres probes and notes. This copy is from the round of the owner's answer to the two answers' question (PR #3, `e417bc8`; work branch commit `89cb9cb`; work branch `wip/step-55-answers-5`): R-1245's second row takes every cached statement's `22P02` from `enum_in`, and the name test is gone from the model too. `probe.py` to `probe4.py`, `po.py`, `probes/po2.py`, `probes/probe5.py`, `probes/probe6.py` and their results are earlier rounds'; `probes/probe7.py` and `probes/src8.py` are this round's. The coordinator replaces this copy after each later round. It isn't part of the workspace or the product.

- **Fix agents:** start from this copy instead of rebuilding it. Work on it on your `wip/...` branch and report the commit.
- **Reviewers: don't read or run anything here.** A review builds its own model, so that it doesn't repeat this one's mistakes.

`results/` holds this round's runs beside the earlier rounds' probe results, which `NOTES.md` lists. `NOTES.md` is its agent's working notes, and its paths are those of its `wip` branch (`plans/wip/`).
