# Step 50: README and 0.1.0

- **Phase:** Finish
- **Depends on:** 48, 49, 44, 46, 43
- **Can run beside:** nothing
- **Crates:** all
- **PR approval:** the owner approves the PR

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

A README that is enough on its own, then the release.

## Deliverables

- Full README with every table
- CI check comparing the Extra calls table with the public API
- Every doc example compiled and run
- `--code` coverage clean
- A check that no `unimplemented!` from step 03 remains
- `0.1.0` published, after the owner says so

## End-to-end check

A new project built only from the README works, `check_coverage.py --code` passes for every step, and CI is green on `main`.

## Out of scope

- Features in `later-defo-build.plan`

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### README.md

**R-0798** One compact, complete file at the repo root. It must be enough to use morm efficiently without other docs, and it must be explicit about doing the right thing. It's included as crate docs (`#![doc = include_str!("../README.md")]`), so CI compiles every example and it can't go stale.

- **R-0799** What it is and why: a few lines, plus benchmark numbers.
- **R-0800** Quick start: install, `morm-admin init`, first model, first query.
- **R-0801** **Pick the right call**, near the top:

| ID | You want to | Use | Not |
|---|---|---|---|
| R-0802 | Send the data on unchanged | `fetch_json`, `stream_json` | `fetch`, then serialize |
| R-0803 | Read a few columns | `fetch_raw` | `fetch` |
| R-0804 | Read and compute, no changes | `fetch_ref` | `fetch` |
| R-0805 | Keep or pass data around | `fetch` | |
| R-0806 | Change known fields | `update` | Fetching the row first |
| R-0807 | Set one value on many rows | `update_q` | A loop of `update` |
| R-0808 | Set different values on many rows | `update_each` | A loop of `update` |
| R-0809 | Do math on a value | `set_expr` | Read, compute, write |
| R-0810 | Fetch related rows | `= ANY` or a JOIN | A query per row |
| R-0811 | Add many rows | `insert_many` | A loop of `insert` |
| R-0812 | Handle huge results | `stream` | `.all()` |

- **R-0813** **Performance rules:** never query in a loop, always limit lists, select through the narrowest view, keep transactions short with no network calls inside them, index what you filter on, release every `db.hold()`, use `tx` and never `db` inside a transaction, no network calls or app `Mutex` waits inside one, build rows inside a retried closure, lock several rows with `tx.lock`, use `.lock_ordered()` on contended bulk updates, put side effects in `after_commit`.
- **R-0814** **Extra calls, one line each.** Every opt-in call, escape hatch and override gets one line saying when to use it. A public option with no line here is a documentation bug, and CI compares this table with the public API.

| ID | Call | Use this when |
|---|---|---|
| R-0815 | `.allow_missing()` | You will not handle a 0-row error |
| R-0816 | `#[col(json_string)]` | JavaScript clients read this integer, and it can pass 2^53 |
| R-0817 | `schema = "..."` | A model's table lives outside the default schema |
| R-0818 | `mgr mode development` | This is a local or staging database, and you want `rollback` |
| R-0819 | `mgr revert ID` | You must undo a migration in production |
| R-0820 | `rollback --production` | `revert` can't work and you accept the risk; a reason is required |
| R-0821 | `mgr squash --to ID` | The chain has grown long, and every database has passed `ID` |
| R-0822 | `tx.emit` | An event must not be lost if the app crashes; use `after_commit` when losing one is acceptable |
| R-0823 | `tx.emit_keyed` | Events about the same thing must arrive in order |
| R-0824 | `fields = "..."` | A view is public, and new columns must stay out until you add them |
| R-0825 | `input = "..."` | A view takes a request body that creates a row |
| R-0826 | `patch = "..."` | A view takes a partial update, such as a PATCH body |
| R-0827 | `.allow_empty()` | An empty update is fine, and you will not handle `EmptyPatch` |
| R-0828 | `span_sample` | You want more or fewer query spans in your tracing platform |
| R-0829 | `morm::sample_scope` | A traced request must keep all of its query spans |
| R-0830 | `slow_query` | You want slow queries reported from the app, including time waiting for a connection |
| R-0831 | `metrics_interval` | Your graphs need finer detail, or your platform bill needs to come down |
| R-0832 | `.explicit_ids()` | You insert rows with ids you already have, such as an import |
| R-0833 | `morm::OrDefault` | A nullable column has a default, and you sometimes need a real NULL there |
| R-0834 | `.for_key_change()` | You will delete locked rows or change their keys, and other requests add child rows |
| R-0835 | `blocking` | A `clean` function is slow, such as password hashing |
| R-0836 | `morm::allow_repeats` | Your code is meant to repeat one query, and the loop detector fails it |
| R-0837 | `pooler = "transaction"` | You connect through PgBouncer in transaction mode |
| R-0838 | `.tag()` | You need to trace a query in the server logs back to the code that ran it |
| R-0839 | `morm-admin size` | You change pool or Postgres memory settings and want to check they fit the server |
| R-0840 | `size --here` | You need a starting config for the machine you're on |
| R-0841 | `size --measure` | You want estimates based on your app's real statements |
| R-0842 | `[server]` | The database runs on another machine, so morm can't read its size |
| R-0843 | `shape_cache` | Logs show churn in runtime-built queries, or your app has many builder query shapes |
| R-0844 | `#[col(secret)]` | A column must never reach JSON or logs, such as a password hash |
| R-0845 | `#[model(deserialize)]` | You load a full model from trusted data, such as a cache; never from a request |
| R-0846 | `keep_case` | You're matching an existing mixed-case table or column |
| R-0847 | `tx.get_for_update` | Your logic must see the current row before changing it, and `set_expr` can't express it |
| R-0848 | `#[col(version)]` | Users edit a row over time, and a stale save must fail, not overwrite |
| R-0849 | `.in_batches(n)` | You change so many rows that one transaction would hold locks too long, and it needn't be atomic |
| R-0850 | `.version(v)` | You update a `version` column and hold the value the client read |
| R-0851 | `.ignore_version()` | A server-side update may overwrite whatever version is there |
| R-0852 | `tx.set_local(name, &value)` | A setting must last one transaction, such as the tenant for RLS |
| R-0853 | `db.insert_stream` | Your rows are too many to hold in memory, such as an import from a file |
| R-0854 | `db.load_related` | You need the child rows of a whole list |
| R-0855 | `.keyset(...)` | You page through a list that can go deep |
| R-0856 | `.order_by_allowed(...)` | A client chooses the sort column |
| R-0857 | `morm::quote_ident` | You use a dynamic table or column name in `q_unchecked` or `sql_unchecked` |
| R-0858 | `.custom_plan()` | A query is slow only for some values, because your data is skewed |
| R-0859 | `nulls_not_distinct` | A unique column may hold only one NULL |
| R-0860 | `#[derive(PgEnum)]` | You're matching a Postgres enum that morm must never change; use `TextEnum` otherwise |
| R-0969 | `#[derive(PgDomain)]` | A column uses a Postgres domain that already exists |
| R-0970 | `#[derive(PgComposite)]` | A column uses a Postgres composite type that already exists |
| R-0971 | `#[pg_domain(...)]` | You want migrations to create and track a domain, so one rule on a value lives in the database |
| R-0972 | `#[pg_composite(...)]` | You want migrations to create and track a composite type, for fields stored together in one column |
| R-0973 | `#[pg_enum(...)]` | You need a real Postgres enum and want migrations to manage it, including adding, renaming, removing and reordering values; use `TextEnum` otherwise, because removing or reordering replaces the type |
| R-1002 | `#[pg_type(name = "...")]` | A bare derive must bind to a type whose name isn't the snake_case of your Rust name |
| R-1003 | `#[pg_enum(value = "...")]` | A variant's Postgres label isn't the snake_case of its name |
| R-1004 | `#[pg_enum(was = "...")]` | You renamed a value, and the migration must rename it and not remove and add it |
| R-1005 | `#[pg_enum(unknown)]` | Old binaries must keep reading rows during a rollout that adds a value |
| R-1058 | `#[derive(TextEnum)]` | A column holds one of a fixed set of values; prefer this to a Postgres enum |
| R-1057 | `#[text_enum(unknown)]` | Old binaries must keep reading a `TextEnum` column during a rollout that adds a value |
| R-1006 | `#[pg_enum(deprecated)]` | You're removing a value in a rolling deploy, and `contract` must wait until this release has shipped |
| R-1007 | `max_rewrite_mb` | You want a different size limit on rewriting a table under a full lock |
| R-0974 | `morm::pg_sequence!` | You need numbers from a sequence that no identity column owns, such as invoice numbers |
| R-1068 | `proof = "..."` | A managed function or procedure takes arguments, or a migration can only prove it through a call you choose; repeat it for each path that matters, and a body using `EXECUTE` or a trigger function with no trigger must have one |
| R-1086 | `PERFORM 'PRAGMA:TABLE: ...'`, `PERFORM 'PRAGMA:TYPE: ...'` | `plpgsql_check` reports a false error because it can't see a table or a record's fields, such as a temp table the function creates; every other line is still checked, and the notice below says what this can hide |
| R-1087 | `PERFORM 'PRAGMA:DISABLE:CHECK'` | A false error can't be described with `TABLE` or `TYPE`, such as a constant `1/0`; it hides real breaks until its block ends, so read the notice below first |
| R-1088 | Accepting a `plpgsql_check` finding | A finding is false on this one migration and you won't change the function; the next migration that reaches the function checks it again, and the notice below says what this can hide |
| R-1089 | Accepting a narrowing | A variable is narrower than what it reads, and you know the values will always fit |
| R-1094 | Accepting a changed column | A column's type, length or precision changes under a managed plpgsql function, and you've read the function and its limits still fit |
| R-1090 | Accepting a missing `plpgsql_check` | Your scratch server can't have the extension, and the function's `proof` lines cover the paths you care about; every other path is unproven |
| R-1091 | `morm_plpgsql_check` template | Your scratch user isn't a superuser, so a superuser makes this template once, and morm clones it |
| R-0861 | `storage = "fillfactor=90"` | A table gets many updates to columns with no index |
| R-0862 | `.returning()` | You need the changed rows back |
| R-0863 | `.all()` | You want every row, and you know the result is small |
| R-0864 | `set_expr` | The new value depends on the current one, such as `stock - 1` |
| R-0865 | `.retry(n)` | You want the transaction to run again after a deadlock, and you build its rows inside the closure |
| R-0866 | `.isolation(...)` | You read rows and write based on them, and nothing may change them in between |
| R-0867 | `.lock_timeout(...)` | A transaction will wait on locks for longer than 5s, such as a large batch |
| R-0868 | `.transaction_timeout(...)` | A transaction will run for longer than 30s, such as a large batch |
| R-0869 | `tx.lock(&ids)` | You will change several rows that other requests also change, such as a money transfer |
| R-0870 | `.lock_ordered()` | You run bulk updates or deletes that can touch the same rows at the same time |
| R-0871 | `.skip_locked()` | Your workers take jobs from a queue and shouldn't wait for each other |
| R-0872 | `tx.advisory_lock(&keys)` | Only one process may do this work at a time, such as a billing run |
| R-0873 | `db.extra_conn()` | You need a write to survive a rollback, such as an audit row, or a query beside an open stream |
| R-0874 | `.update_only(...)` | An upsert must change only certain columns on conflict |
| R-0875 | `db_set` | Postgres always fills a column on insert, and your field's value should be ignored |
| R-0876 | `deferrable` | Rows in one transaction may break a constraint until commit, such as swapping two unique values |
| R-0877 | `#[morm::hooks]` | You write `impl Hooks` for a model |
| R-0878 | `db.tx()` | A hook needs `tx.emit` or another `tx`-only call |
| R-0879 | `.pause_ms(n)` | Batches must leave room for other work, such as replicas catching up |
| R-0880 | `.limit_each(n)` | You load related rows and each parent needs only a few |
| R-0881 | `db.check_schema()` | A binary should refuse to boot against a schema it can't use |
| R-0882 | `#[col(deprecated)]` | You're removing a column, and old binaries still run |
| R-0884 | `Product::UK.<name>` | An upsert targets an expression unique index, or two partial indexes share columns |
| R-0885 | `db.hold()` | You need `LISTEN`, temp tables, session advisory locks or `SET` |
| R-0886 | `db.pool("replica")` | You run heavy reads, such as reports, that can be slightly out of date |
| R-0887 | `db.pool("jobs")` | You run heavy jobs in the web app's process, and they must not take its connections |
| R-0888 | `db.groups(mask)`, `db.sudo()` | Your callers may see or change different fields |
| R-0889 | `Numeric` | Your values can exceed 28 digits, or be `NaN` or `Infinity` |
| R-0890 | `q_unchecked`, `sql_unchecked` | You must build SQL at runtime, and you will handle injection safety yourself |
| R-0891 | `recycle_when` | You need your own rule for replacing connections |
| R-0892 | `no_default` | You want a compile error when a required field is missed |
| R-0893 | `--fast` | The table is small, or you're in a maintenance window |
| R-0894 | `--accept-data-loss` | You've read the plan, and you accept losing that data |
| R-0895 | `--skip-fingerprint` | Your role can't create a temporary database, and you accept a weaker check |
| R-1073 | `--terminate-blockers` | A cutover keeps giving way, and you'd rather end the transactions in its way than wait; those transactions die and their apps lose the connection |
| R-1074 | `cutover_lock_timeout`, `cutover_attempts`, `cutover_pause` | A cutover gives up on a busy table, or its retries stall your traffic too much |

- **R-0896** Models: bases, every `#[model]` and `#[col]` option in tables.
- **R-0897** `get`, `insert`, `update`, `delete`, lists, `filter_cols`, related rows, pagination, batches, lost updates, raw SQL with `db.sql()`, transactions, and which of `Decimal`, `Numeric` or `bigint` to use.
- **R-0898** Views, including public input and output with `fields`, `input` and `secret`, groups, validation.
- **R-0899** Hooks, with a table of what each receives and this line: "update and delete hooks never see a row unless you fetch it." Then `after_commit` versus the outbox.
- **R-0900** JSON output and schemas.
- **R-0901** Pool and `morm.toml`: every setting, sizing, and the rule for long-running work: "use `db` for everything, including jobs and crons, and use `db.hold()` only when you need session state." It states plainly that service jobs, crons and background workers running as their own process must not spawn a pool with `min` above 1, and shows `connect_as`, `MORM_POOL` and `MORM_POOL_MIN`. It includes the full environment variable table. It also shows the jobs table, the `db.hold()` lifecycle, and when a websocket needs a listener: "release when you're done, and rely on Drop as the safety net, never as the plan."
- **R-0902** Migrations: workflow, every command in one table, safety rules, production being forward only with `revert`, running as a deploy step, triggers and functions.
- **R-0903** Security: TLS settings, encrypting disks and backups (outside morm's job, but expected in production), and password hashing with `clean`.

**R-1008** The README shows this notice loudly, beside the enum rows:

> **Use `TextEnum` unless you have a reason not to.** Changing a `TextEnum` is an ordinary migration. Postgres can't remove or reorder an enum value, so morm has to replace the whole type, including everything that depends on it. On a big table that means a long online copy or a table rewrite under a full lock. Adding a value is cheap, but its down is not. morm does all of this safely, but `TextEnum` never needs it.

**R-1069** The README says this plainly, beside the managed function rows. A migration can stop and ask you to add a `proof` line to a managed function or procedure before it goes through, because a change under a function can break it in ways Postgres only shows when it's called, and morm won't ship one it can't call. morm really runs that call, on a scratch database, in a transaction it rolls back, with a 30-second timeout, so the call has to be safe to run, and it must not reach outside Postgres. With no `proof`, morm's own call runs as a role that can't use extension functions or other languages, so a function that uses `dblink`, an HTTP extension or `COPY ... PROGRAM` stops and asks for a `proof`. With a `proof`, morm runs what you wrote with your scratch user's rights and can't check it, so write a call that takes no outside action. Unmanaged functions can't be proven, because scratch can't see them, so they become a migration decision that names each one and says it may fail until it's rewritten.

**R-1084** The README shows this notice loudly, beside the managed function rows:

> **Silencing a check can hide a real break.** When a migration changes something a managed plpgsql function names, morm runs `plpgsql_check` on that function, and any `error` stops the migration. There are two ways past an error, and neither one fixes anything. If the finding was real, the function ships broken and fails in production the first time that line runs.
>
> **What each one does.** A `PERFORM 'PRAGMA:...'` line goes in the function. It changes what `plpgsql_check` looks at, on this migration and every later one, and when the function runs it's only a string, so it does nothing. A decision leaves the function alone and lets that one finding through on that one migration. It's recorded in the migration's `meta.json`, and the next migration that reaches the function checks it again.
>
> ```sql
> -- Tell the checker what it can't see. It still checks every other line.
> PERFORM 'PRAGMA:TABLE: report_rows (id bigint, total numeric)';
> PERFORM 'PRAGMA:TYPE: r (id bigint, total numeric)';
>
> -- Stop checking until this block ends. Keep the block as small as you can.
> BEGIN
>   PERFORM 'PRAGMA:DISABLE:CHECK';
>   PERFORM 1 / 0;
> END;
> ```
>
> ```
> makemigrations --decide ID=OPT     # morm prints the ID and the options
> ```
>
> **When it's right.** Only when you've read the finding and can say why it's false, such as a temp table the function creates, or a record it fills with `EXECUTE`. Prefer `PRAGMA:TABLE` and `PRAGMA:TYPE`, because the checker keeps reading every other line. Use `PRAGMA:DISABLE:CHECK` only when those can't describe the problem. Use a decision when you won't change the function.
>
> **What you give up.** No line under `PRAGMA:DISABLE:CHECK` is checked, on this migration or any later one, so a column dropped from one of them passes. The checker believes a `TABLE` or `TYPE` line over the database, so one that no longer matches hides a break too. An accepted finding stays unfixed, and morm won't stop that migration for it again. After that, only a `proof` that runs that line can catch a real break, and a call proves only the path it takes.

**R-1085** The README says this plainly, beside that notice. A call proves only the path it takes, so a passing `proof` says nothing about a branch it didn't reach, and a function can have several `proof` lines to cover the paths that matter. `plpgsql_check` reads every branch without running it, but it can only read what's in the source. A query built at run time with `EXECUTE` isn't checked, so a function using `EXECUTE` must have a `proof`. morm finds it with a word search, so the word in a comment or a string asks for a `proof` too. That `proof` proves only the query its one call builds, and any other query the body can build stays unproven until production runs it. A trigger function with no trigger on it has no table to check against, so it must have a `proof`, and that `proof` is all it gets, so any path it doesn't take stays unproven until production runs it. `plpgsql_check` can't see a length or precision limit, so `varchar(3)` reading a longer `text` column, or `numeric(6,2)` reading a wider `numeric`, gets no finding. morm knows the column's old and new definition, so any change to a column's type, length or precision, even a widening, opens a decision naming each managed plpgsql function that names that column, found by searching its source for the table's name and the column's name. Read the function before you accept it, because the decision can't show you where a limit sits. If you accept a decision because `plpgsql_check` isn't installed, your `proof` lines are all that's proven. Neither check shows that a body still means what it did, such as a `s::text = 'old'` comparison that stops matching.

**R-1075** The README says this plainly, beside the online swap. During a cutover, a client that isn't morm can see one `XX000` `could not open relation with OID` when it calls a `BEGIN ATOMIC` function the cutover replaces, or one over a view it replaces. morm apps retry it themselves. Other clients should do the same, because calling again works.

### Risks, open items and later work / Open items

- **R-0920** All six names (`morm`, `morm-core`, `morm-macros`, `morm-migrate`, `morm-admin`, `morm-mgr`) are reserved on crates.io at `0.0.0`. The release publishes over them.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0115 | suite | **Doc tests:** every README and doc example compiles and runs. |
