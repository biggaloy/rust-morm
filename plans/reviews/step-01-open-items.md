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

## From the edit at `784f68a`, waiting on the owner

- **C. Cutover against app transactions.** The decided order leaves two deadlock shapes: a transaction that touches the table then reads the view, and R-1049's two-table cutover against a reader joining in the opposite order. Under nonstop traffic an app transaction was killed on all 20 cutovers. A 50 ms lock timeout kills nothing, but the cutover can starve and give up. Question: does an app transaction take the `40P01`, or does the cutover give way?
- **D. `BEGIN ATOMIC` callers.** A caller of a `BEGIN ATOMIC` function over a carried view fails with `XX000 could not open relation with OID` during a cutover, and a retry succeeds. Question: add it to R-0981 and R-0983, or narrow T-0187?
- **E. Rules to approve.** R-1070: the same-meaning check compares stored query trees with `location`, `stmt_location`, `stmt_len`, `isNatural`, `alias` and `eref` removed. R-1071: a `USING` view binds columns by position, so an unmanaged column can return wrong rows, and preflight refuses that table.
- **Decisions still to add on `main`:** R-1070, R-1071, T-0178, T-0184, T-0194, T-0195, the `contracts.md` scanner row, and step 52's crates line.
- **Still to do after these:** the `plpgsql_check` edit.

## From the edit at `8f4c7b5`, waiting on the owner

`plpgsql_check` v2.10.16 catches dropped columns and changed types in unreached branches on Postgres 15 to 18, needs no preload and runs nothing, but needs a superuser. PGDG packages it for 15 to 18. It hasn't run on GitHub's runners yet.

- **F. Findings on healthy functions.** Six shapes give false errors: temp tables, tables created in the body or by the caller, records from `EXECUTE`, a record with different fields by branch, `set_config` of `search_path`, and a constant `1/0`. A `PERFORM 'PRAGMA:...'` line in the body fixes them and also runs without the extension. Comparing before and after isn't safe alone.
- **G. R-1067 contradiction.** With a superuser-installed extension, a non-superuser scratch user can't revoke `EXECUTE`, so `_morm_proof` keeps it and R-1067 stops `makemigrations`. Options: a superuser scratch, an exemption, or a separate database for the check.
- **H. Narrowing casts.** An `int` variable reading a column now `bigint` isn't an error, only a `performance` warning with `all_warnings`.
- **I. `EXECUTE` detection.** The tool gives no marker, so finding `EXECUTE` needs a word search that can over-ask.
- **J. `SET plpgsql_check.mode TO 'disabled'`** silently gives no findings.
- **K. To confirm.** `--terminate-blockers` on `migrate` and `rollback` ends what `pg_blocking_pids` names. Defaults: 50 ms timeout, 250 ms pause, 600 attempts, about 3 minutes. With overlapping 200 ms transactions no attempt got in without the flag. The `BEGIN ATOMIC` error is retried on the same connection and replaces nothing.
- **L. To flag to the owner.** The check blocked `ftp.postgresql.org`, and the agent built Postgres 15 and 16 from the GitHub mirror instead. Some loops ran from scripts after refusals asked for plain separate commands. Scratch holds 1.4 GB in that agent's worktree.
- **Decisions still to add on `main`:** R-1072 to R-1075, T-0196, T-0197.

## From the edit at `3dcaa78`, waiting on the owner

- **M. Template route (R-1077).** A non-superuser can't `CREATE EXTENSION`, so a superuser makes a `morm_plpgsql_check` template once per scratch server, and morm clones it with `CREATEDB`. A non-superuser clone passed on 15 to 18.
- **N. Length and precision limits.** `plpgsql_check` doesn't see them: `numeric(6,2)` reading `numeric(20,4)` gave no finding, and `varchar(3)` reading `text` looks harmless. Only a `proof` that reaches the limit catches one.
- **O. A trigger function with no trigger** can't be checked, so its `proof` is all it gets.
- **P. The narrowing test** uses `pg_cast`: the pair has a `castcontext` other than `i`.
- **Q. Exit codes.** Decisions that default to refuse exit `2` until accepted, following R-1040 and T-0168, though R-0588 lists `3` for open decisions.
- **R. Unproven.** In-comment `@plpgsql_check_options` may be able to hide an `error`, and it's a recording item in proof 18. T-0204 and T-0201 cover two more claims.
