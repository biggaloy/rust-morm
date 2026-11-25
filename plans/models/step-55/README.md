# Step 55's concurrency model

The Rust model fix agents run to check the stale-session procedure (R-1270, R-1271, T-0319), with its Postgres probe and notes. This copy is from the EZ answers round (PR #3, `a1a6d0b`; work branch commit `488a8d6`); the coordinator replaces it after each later round. It isn't part of the workspace or the product.

- **Fix agents:** start from this copy instead of rebuilding it. Work on it on your `wip/...` branch and report the commit.
- **Reviewers: don't read or run anything here.** A review builds its own model, so that it doesn't repeat this one's mistakes.

`results/` holds that round's runs. `NOTES.md` is its agent's working notes, and its paths are those of its `wip` branch (`plans/wip/`).
