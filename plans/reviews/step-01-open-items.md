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

## From the edit at `d586edf`, waiting on the owner

- **S. Functions that don't name the column.** After `numeric(6,2)` to `numeric(20,4)`, a `numeric(6,2)` variable reading through another function, or through a query built at run time, fails with `numeric field overflow`, and nothing finds them. Options: repeat the search on the names of hit functions until nothing new appears; open a decision for managed functions using `EXECUTE`; or a written limit. R-1085 claims only functions that name the column until this is settled.
- **T. T-0201 and T-0202** were edited, though "stays" was the instruction: `2` to `3`, and "open none" to "open no narrowing decision".
- **U. R-1015 stays at `2`:** `refuse` is a recorded answer that `migrate` enforces on live rows.
- **V. A run with both a refusal and an open decision:** no rule sets the exit code. T-0192 implies `2`.
- **W. `migrate` has no `--decide`:** R-1040 and R-0572 exit `3` in preflight, and nothing says how to answer there.
- **X. R-1040's `%TYPE` text** names only `integer out of range`. A length or precision change gives `numeric field overflow`.
- **Decisions still to add on `main`:** R-0572, R-0593, R-0608, R-1015, R-1038, R-1040, R-1068, R-1076, R-1078 to R-1080, R-1085, R-1092 to R-1094, T-0168, T-0184, T-0198 to T-0202, T-0206, T-0207.

## From the edit at `db265e1`, waiting on the owner

**W, `--decide` on `migrate` and `rollback`:** not written. Four points, with the agent's recommendations:

1. A recorded answer stands only for the same migration id, checksum, direction and decision ID.
2. A live decision's ID is built from the migration id, kind and function signature, plus a hash of the function's source.
3. The ledger keeps `meta.json` verbatim, so `rollback` can search when the files are gone.
4. Answers go in a new `_morm.decisions` table, plus a history row.

**Other gaps:**

- **Y.** A third stale `%TYPE` error: `22001 value too long` after `varchar(3)` to `varchar(10)`.
- **Z.** An unmanaged `EXECUTE` function that names nothing the search looks for is never found.
- **AA.** Non-function callers: a view with `inner_f()::numeric(6,2)` failed the same way, and defaults and checks that call a hit function aren't searched.
- **AB.** R-0572's search says nothing about case, so retires and renames may miss `SELECT N FROM T`.

Decisions still to add on `main`: R-0608, R-1081, T-0208, T-0209.

## From the edit at `4e6d12a`, waiting on the owner

- **AC.** A caller of an unmanaged `EXECUTE` function isn't found: `v := ex_f(a, b)` into `numeric(6,2)`. The decision lists only `ex_f`.
- **AD.** A function or view that reads a found view with no cast isn't found.
- **AE.** A trigger's `WHEN`, a policy, a rule, and expression and partial indexes that call a found function fail the same way. A plpgsql body using an operator built on a found function has no name to search and no `pg_depend` row.
- **AF.** `rehearse` has no `--decide`, so a stopped run has no way to answer, and a restored backup holds no answers.
- **AG.** A `BEGIN ATOMIC` body's source is deparsed, so a rename in an earlier step changes its hash, and `--resume` asks again. Safe, one extra stop.
- **Filled in by the agent, standing unless the owner objects:** `--yes` answers no decision, `--plan` records nothing, an unused ID is reported, a bad option exits `1`, an answer that refuses exits `2` and a later answer replaces it, preflight runs before each migration, the ID form is `<id>/<kind>/<signature>/<hash>`, and one `makemigrations --decide` answers a decision open in both directions.
- **Decisions still to add on `main`:** R-0597, R-0615, R-0950, R-0951, R-1069, R-1095 to R-1100, T-0122, T-0210 to T-0216.

## From the edit at `da5961a`, waiting on the owner

- **AH. Most column changes now stop once.** "Every dependent object" includes a plain index, the column's default and, on Postgres 18, its `NOT NULL`.
- **AI. A function that only receives the value as an argument.** With `pass_f(x numeric)` holding `v numeric(6,2) := x`, the call `pass_f(inner_f())` in a found view overflows, and `pass_f` isn't found. Following what found objects call would pull in far more objects.
- **AJ. A whole row plus another column.** `SELECT x.id, (to_jsonb(x) ->> 'n')::numeric(6,2) FROM t x` has a `pg_depend` row only against `id`.
- **AK. To confirm:** a `rehearse` rerun carries on from `_morm.history` (R-1101). Other ops use the same walk, each hit with its own decision, and the one list applies to column changes. Preflight leaves out objects `meta.json` already holds accepted. The `--resume` re-ask also covers views, defaults and checks.
- **Decisions still to add on `main`:** R-0952, R-1094, R-1101, T-0206, T-0217.

## From the edit at `6fcce01`, waiting on the owner

- **AL.** `DEFAULT now()` on another column makes every change on that table stop once. Recommended: on other columns, a default or check that only calls built-ins isn't listed.
- **AM.** A widening with nothing to list shows no fixed line. Recommended: print it in `makemigrations` output and as a comment in the migration, without stopping.
- **AN.** R-0612's "one the conversion layer accepts" never lists the pairs. Recommended: the plan lists them, including length and precision.
- **Decisions still to add on `main`:** R-1102, T-0218, T-0219.

## From the edit at `eb0e0d4`, waiting on the owner

- **AO.** A check on another column using `query_to_xml('SELECT max(n)::numeric(6,2) ...')` fails after `n` widens, and AL takes it off the list. Options: keep built-ins that run query text listed (the `query_to_xml` family, `ts_stat`, `ts_rewrite(tsquery, text)`), or leave it under the fixed line.
- **AP.** `integer` to `bigint` and `varchar(3)` to `text` fail `check_schema`, because R-0925 compiles decoding for one type with no branch, so an old binary can't start after such a widening.
- **AQ. To confirm:** generated columns stay listed, `Decimal` has a 28-digit limit, and integer fields stay at scale 0.
- **Decisions still to add on `main`:** R-1085, R-1092, T-0220, alongside the IDs already there.

## From the edit at `acf82a2`, waiting on the owner

- **AR. Narrowing writes can change a value silently.** An old app built against `numeric(20,4)` writes `1.0051` after the column becomes `numeric(20,2)`, and `1.01` is stored with no error. The same happens with `timestamptz` to `timestamptz(3)`, and with `'abc  '` into `varchar(3)`, stored as `'abc'`.
- **AS.** `timestamp` and `timestamptz` stay different kinds, because a change between them converts values through the session's `TimeZone`.
- **AT.** `char(n)` is out of the text family, because Postgres pads it. A `Decimal` field on a column changed from `numeric` to `bigint` fails as a cross-family change.
- **AU.** `schema_to_xml`, `database_to_xml` and their xmlschema variants aren't on the query-text list.
- **AV. To confirm:** `contracts.md` gains `fn from_row_as(row, &Decoders<Self>)` and `pub struct Decoders<M>` on `Model`.
- **The PR body isn't updated.** The agent's `sed` edit was refused and it stopped. The next edit agent should add the two prepared items.
- **Decisions still to add on `main`:** R-0202, R-0925, R-1103 to R-1107, T-0220 to T-0223.

## From the edit at `3d91eab`, waiting on the owner

- **AW.** `rollback` has no gate: rolling back a `numeric(20,2)` to `numeric(20,4)` widening narrows the column under live apps, which then round writes silently. Recommended: `rollback` refuses a silent narrowing unless run with `--phase contract` or `--fast`, as `migrate` does.
- **AX.** An old binary restarted after `contract`, such as an app rolled back to an older build, rounds writes silently, because `check_schema` accepts a field built for `numeric(6,2)` on `numeric(4,1)`. Recommended: `check_schema` refuses a field built for a wider scale, precision or length than the column has, so the app fails loudly rather than corrupting data.
- **AY.** `table_to_xml` and `cursor_to_xml` through a runtime regclass aren't on the query-text list. Recommended: add them.
- **AZ.** `morm::Numeric` doesn't read integer columns. Recommended: extend it, as with `Decimal`.
- **BA.** Lowering a scale rounds rows already stored: `1.0051` becomes `1.01`. Recommended: any silent narrowing of stored data is a `lossy` change, so it opens a decision.
- **Decisions still to add on `main`:** R-0424, R-0893, R-1108, R-1109, T-0224.
- **After these:** design online widening of a referenced key, then the short check, then PR #1.

## From the edit at `124c49c`, waiting on the owner

- **BB.** Rolling back a widening rounds rows written since it ran, so it's lossy. Options: a decision at `makemigrations` for every widening's `down`; `rollback` preflight; or `--accept-data-loss`. Recommended: `rollback` preflight counts the live rows that wouldn't fit. With none, there's no decision. With some, a decision opens showing the count, answered with `rollback --decide`.
- **BC.** `db.check_schema()` is opt-in, so an older build that doesn't call it, brought back after `contract`, still rounds writes. Recommended: run it automatically at pool start, with an explicit opt-out that the README covers loudly.
- **BD.** The `lossy` decision is a plain accept or refuse. Recommended: in `migrate` preflight it also shows how many live rows would change.
- **Decisions still to add on `main`:** R-0612, R-0615, R-0951, R-1103, R-1106, R-1107, R-1110 to R-1112, T-0223, T-0225, T-0226, and arrays in R-1108.

## From the edit at `ae0498f`, waiting on the owner

- **BE. Which families step 60 covers.** The design says "same family", but only `smallint`, `integer` and `bigint` were tested, and other families raise three problems: identity works only on integer types, a narrowing key can never pass the `new = old` check, and fitting the old type isn't defined for non-integers. Recommended: step 60 covers `smallint`, `integer` and `bigint` only in 0.1; other rewriting key changes keep the refusal plus `--fast`.
- **BF. A group table with no primary key.** The copy walks by its own primary key; steps 39 and 52 have the same gap. Recommended: walk by a unique `NOT NULL` key when there is one, otherwise refuse online, with `--fast`.
- **BG. A group table in old-style inheritance.** `ADD COLUMN` spreads to child tables. Recommended: refuse online, as for partitioned tables, with `--fast`.
- **BH. README for the scaled-wait refusal.** When the scaled lock wait falls under 1 ms, preflight refuses with exit 2. Recommended: a loud README notice saying what it does, why, and how to go on (raise `deadlock_timeout` or `cutover_lock_timeout`, or `--fast`).
- **Answered:** BE every same-family change (three gaps to design, see Decisions); BF, BG, BH as recommended.

## From the families design at `c18de68`, waiting on the owner

- **BI to BR:** see `reviews/step-60-families-design.md`, each lettered with a recommendation, and section 6 for the README notices.
- **Answered:** BI to BR all as recommended.

## From the edit at `3c2eb3e`, waiting on the owner

- **BS. BG in steps 39 and 52.** Step 39's online swap refuses neither inheritance nor partitioned tables, and step 52 refuses partitioned tables but not inheritance, so `ADD COLUMN` spreads to children there too. Recommended: refuse both online in 39 and 52, with `--fast`.
- **BT. BQ outside step 60.** Step 39's `--fast` and in-place type changes have the same silent operator-class swap. Recommended: compare indexes with scratch and rebuild those that differ for every `ALTER COLUMN ... TYPE` morm runs.
- **BU. Shadow unique indexes in steps 39 and 52.** A value-changing swap of a unique non-key column can collide on the shadow index alone, and only step 60's is mapped. Recommended: the same `_morm_new_` mapping for every online swap.
- **CI:** `pooler` failed three times at `apt-get install pgbouncer` (runner DNS for `azure.archive.ubuntu.com`); rerun by the coordinator.
- **Answered:** BS to BU as recommended, and the edit's choices approved.

## From the edit at `63775d9`, waiting on the owner

- **BV. `--fast` can't change an inherited column on a child or a partition** (`cannot alter inherited column` on 18.6), and a change on the parent reaches children no model declares. Affects steps 39, 52 and 60 (R-1125, T-0241). Recommended: change an inherited column only through its top parent in one `ALTER`; refuse with exit `2`, naming them, a change to a child's inherited column alone and any change reaching a child or partition no model declares.
- **BW. An index a foreign key depends on can't be dropped without `CASCADE`** when the index check must rebuild it (`2BP01`, the run stops). Recommended: `makemigrations` refuses with exit `2` naming the index and foreign key; `--fast` drops and re-adds the foreign key around the rebuild, as R-1126 does.
- **BX. Values that would merge on a unique non-key column** in step 39's value-changing swap and step 52's map of two labels to one have no preflight refusal, only a failed concurrent index build. Recommended: step 60's `GROUP BY` preflight refusal for every unique index on a swapped column whose values change.
- **BY. Other step 39 safe forms on partitioned tables** (R-0560, R-0562 build concurrently and use `USING INDEX`, both fail there; only from changes outside morm). Recommended: refuse in preflight, with `--fast`.
- **Answered:** BV, BX, BY as recommended; BW: drop and re-add the foreign key in every form, online included.
