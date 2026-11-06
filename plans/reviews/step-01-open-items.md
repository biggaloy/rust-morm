# Step 01: open items for the owner

From the edit at `cba5193`. Tested on Postgres 18.6 and 17.11.

## Stopped, waiting on the owner

- **A. Cutover lock order (R-1049, R-1061, T-0187).** `LOCK TABLE` on a view takes every table under it at once, so it deadlocks. Up to 50 of 60 cutovers were killed. Taking each view's lock with its own `DROP VIEW`, outside in, then locking the table, had 0 kills in 100 runs at 2 to 4 ms. Recommended: that order, then walk `pg_depend` again. Alternative: a `lock_timeout` under `deadlock_timeout`, which stalls traffic on every retry.
- **B. Name replacement (R-1051, R-1064, T-0190).** It can't carry four query shapes: a second table with a same-named column, `JOIN ... USING (n)`, and the column inside a CTE or subquery. Tested alternative: rename the column on scratch and take `pg_get_viewdef`, which matched for all but `USING`, where it runs but reads differently. Recommended: the deparse route. `USING` needs a decision.

## Raised by the agent

1. A view or `BEGIN ATOMIC` function beneath a materialized view is built beside and retired too. The alternative is refusing it.
2. Committing procedures: `CALL` fails in a transaction, so it runs on a throwaway `TEMPLATE` copy.
3. Trigger proofs confirm firing through `EXPLAIN (ANALYZE)`. `TRUNCATE` and event trigger proofs run without that check.
4. A fixed 30-second timeout, no decision for a `WITH NO DATA` view, and `revert` restoring a retired copy as `rollback` does.
5. A call proves only the path it takes. A branch it doesn't reach stays unproven. Open question.
6. R-0572 retires and renames aren't under the real-call rule. Recommended: include them.
7. A `%TYPE` stale error, `integer out of range`, isn't recovered by R-0981, because `22003` can't be matched safely.
8. A retired copy blocks `ALTER COLUMN TYPE` on any other column it reads until `purge --now`.
9. Postgres 15 and 16 weren't run. T-0188 and T-0193 don't name versions.

## IDs to add to Decisions on `main`

Changed: R-0572, R-0593, R-0981, R-0983, R-1038, R-1040, R-1047, R-1049, R-1050, R-1051, R-1052, R-1061, T-0122, T-0152, T-0168, T-0175, T-0187. New: R-1062 to R-1069, T-0188 to T-0193.
