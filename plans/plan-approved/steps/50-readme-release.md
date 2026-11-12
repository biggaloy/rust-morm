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
| R-0832 | `.explicit_ids()` | You insert, stream or upsert rows with ids you already have, such as an import |
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
| R-1094 | Accepting a changed column | A column's type, length or precision changes, and you've read what morm lists and its limits still fit; being listed doesn't mean something is broken |
| R-1146 | `makemigrations --decide ID=<zone>` | A column changes between `timestamp` and `timestamptz`; give the zone its stored values were written in, after reading the notice |
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
| R-0881 | `db.check_schema()` | You want the startup check's answer again later, such as in a readiness probe |
| R-1120 | `check_schema = "off"` | A pool's database isn't meant to hold this binary's models, such as a pool used only for raw SQL on another database; read the notice first |
| R-0882 | `#[col(deprecated)]` | You're removing a column, and old binaries still run |
| R-1109 | `migrate --phase expand`, then `--phase contract` | You deploy without stopping apps, and a migration removes a column or a value, or narrows a column where Postgres would round or cut a write silently |
| R-1112 | `rollback --phase contract`, `rollback --fast` | You roll back a migration that widened a scale, a timestamp precision or a length, so the rollback narrows it where Postgres would round or cut silently: `--phase contract` once every app runs the release built for the narrower shape, `--fast` with apps stopped |
| R-1193 | `rollback --fast` | You roll back with apps stopped and want each step in its one-transaction form, or a rollback is refused because a step's form for live tables can't run, such as adding back a foreign key from a partitioned table on Postgres 15 to 17 |
| R-0884 | `Product::UK.<name>` | An upsert targets an expression unique index, or two partial indexes share columns |
| R-0885 | `db.hold()` | You need `LISTEN`, temp tables, session advisory locks or `SET` |
| R-0886 | `db.pool("replica")` | You run heavy reads, such as reports, that can be slightly out of date |
| R-0887 | `db.pool("jobs")` | You run heavy jobs in the web app's process, and they must not take its connections |
| R-0888 | `db.groups(mask)`, `db.sudo()` | Your callers may see or change different fields |
| R-0889 | `Numeric` | Your values can exceed 28 digits, or be `NaN` or `Infinity` |
| R-0890 | `q_unchecked`, `sql_unchecked` | You must build SQL at runtime, and you will handle injection safety yourself |
| R-0891 | `recycle_when` | You need your own rule for replacing connections |
| R-0892 | `no_default` | You want a compile error when a required field is missed |
| R-0893 | `--fast` | The table is small, or you're in a maintenance window; for a narrowing Postgres would round or cut silently, only with apps stopped |
| R-0894 | `--accept-data-loss` | You've read the plan, and you accept losing that data |
| R-0895 | `--skip-fingerprint` | Your role can't create a temporary database, and you accept a weaker check |
| R-1073 | `--terminate-blockers` | A cutover keeps giving way, and you'd rather end the transactions in its way than wait; those transactions die and their apps lose the connection |
| R-1074 | `cutover_lock_timeout`, `cutover_attempts`, `cutover_pause` | A cutover gives up on a busy table, or its retries stall your traffic too much |
| R-1100 | `--decide ID=OPT` on `migrate`, `rollback` and `rehearse` | A run stopped with exit code `3` on something only the live database shows, such as an unmanaged function, and you've read what it names; the answer covers only that migration, its current version and that direction |

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

**R-1069** The README says this plainly, beside the managed function rows. A migration can stop and ask you to add a `proof` line to a managed function or procedure before it goes through, because a change under a function can break it in ways Postgres only shows when it's called, and morm won't ship one it can't call. morm really runs that call, on a scratch database, in a transaction it rolls back, with a 30-second timeout, so the call has to be safe to run, and it must not reach outside Postgres. With no `proof`, morm's own call runs as a role that can't use extension functions or other languages, so a function that uses `dblink`, an HTTP extension or `COPY ... PROGRAM` stops and asks for a `proof`. With a `proof`, morm runs what you wrote with your scratch user's rights and can't check it, so write a call that takes no outside action. Unmanaged functions can't be proven, because scratch can't see them, so they become a migration decision that names each one and says it may fail until it's rewritten. morm can only see them on the live database, so `migrate`, `rollback` or `rehearse` stops with exit code `3` and prints an ID, and you answer with `--decide ID=OPT` on that same command. morm keeps the answer in `_morm.decisions` on that database, and it covers only that migration, that version of it, that direction and that ID. So an answer on `migrate` doesn't cover `rollback`, an answer on `rehearse` doesn't reach production, and editing the migration or changing the function asks again.

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

**R-1085** The README says this plainly, beside that notice. A call proves only the path it takes, so a passing `proof` says nothing about a branch it didn't reach, and a function can have several `proof` lines to cover the paths that matter. `plpgsql_check` reads every branch without running it, but it can only read what's in the source. A query built at run time with `EXECUTE` isn't checked, so a function using `EXECUTE` must have a `proof`. morm finds it with a word search, so the word in a comment or a string asks for a `proof` too. That `proof` proves only the query its one call builds, and any other query the body can build stays unproven until production runs it. A trigger function with no trigger on it has no table to check against, so it must have a `proof`, and that `proof` is all it gets, so any path it doesn't take stays unproven until production runs it. `plpgsql_check` can't see a length or precision limit, so `varchar(3)` reading a longer `text` column, or `numeric(6,2)` reading a wider `numeric`, gets no finding. So after any change to a column's type, length or precision, even a widening, morm walks everything that reaches that column, of any kind, and repeats until nothing new turns up. It follows `pg_depend` to every object that depends on what it has found, and it searches function source for the name of everything it has found, including operator and aggregate symbols. Everything it finds goes on one list, as one decision per migration. A function using `EXECUTE` starts the walk too, even one that doesn't name the column, because no tool can see what a built query reads, and whatever calls it is found by name. Being on the list doesn't mean something is broken. The search reads text, so a common name can bring in something that has nothing to do with the change. The list holds only what carries code or an expression: functions, procedures, views, triggers, policies, rules, expression and partial indexes, and non-constant defaults and checks. Any such object on the changed table is listed, even one on another column, because a whole row such as `to_jsonb(x)` carries the changed column without naming it. A default or a check on another column that only calls built-ins, such as `DEFAULT now()`, isn't listed, because it can't depend on the changed column. A built-in that runs query text it's given, such as `query_to_xml`, `ts_stat` or `ts_rewrite` with a query, or reads every table in a schema or the database, such as `schema_to_xml` or `database_to_xml`, or a table or a cursor it's given by name at run time, such as `table_to_xml` or `cursor_to_xml`, counts as code, so a default or a check that calls one is listed, on any column. A plain index, a `NOT NULL` or a constant default isn't listed, because it can't hold a stale type. morm walks only what it can see. On scratch that's your managed objects, so `makemigrations` lists those. Scratch can't see an unmanaged object, so that part of the walk only works in the preflight of `migrate`, `rollback` and `rehearse`, on the live database, and that's where the first warning about one comes, as a list of its own. The walk follows objects, not values, and that's its stated limit. So every widening's decision carries this line, and a widening with nothing to list still prints it in `makemigrations` output and as a comment in the migration, without stopping: `Values can now exceed the old limit anywhere that reads them, including code morm can't see.` `pass_f(max(n))` shows why. A function that only receives the column's value as an argument, and names nothing, isn't found, though what calls it is, because a value can flow through arguments, other tables, application code and other services, further than any tool can follow. So read what a listed object calls, and treat everything that reads that column as able to see a value the old limit kept out. A query built from data in a table leaves nothing in the catalog or in any source to find, so that shows up when production runs it. Your Rust code is on that side of the line too, because the walk reads nothing but the database. The compiler checks a model field only against the SQL type declared for it: `i64` on `integer` is a compile error, and `i32` on `bigint` compiles. That pair gets a range CHECK, so the column can't store a value the field can't read, and if one still arrives, because the CHECK was removed outside morm, decoding returns `OutOfRange`, never a wrapped value. `db.check_schema()` runs at startup and refuses a column of a kind the field can't decode, such as `text` for an integer field, which is a question of compatibility, so it doesn't show that a field or the code using it is ready for a wider value. A decision proves nothing. It says what's listed may break and leaves the reading to you, and it can't show you where a limit sits or which queries a built string can become. Read each object and what it calls before you accept it, and after that only production shows whether a value fits. If you accept a decision because `plpgsql_check` isn't installed, your `proof` lines are all that's proven. Neither check shows that a body still means what it did, such as a `s::text = 'old'` comparison that stops matching.

**R-1075** The README says this plainly, beside the online swap. During a cutover, a client that isn't morm can see one `XX000` `could not open relation with OID` when it calls a `BEGIN ATOMIC` function the cutover replaces, or one over a view it replaces. morm apps retry it themselves. Other clients should do the same, because calling again works.

**R-1129** The README shows this notice loudly, beside the online swap:

> **Changing a key other tables reference can be refused.** A type change on a key that foreign keys reference, such as an `integer` primary key becoming `bigint`, or any other change within one kind of type that rewrites the table, such as `numeric(20,4)` becoming `numeric(20,2)`, runs online with apps up. morm changes every column that references the key, and the identity sequence, with it. It adds a new column beside each one, copies the rows in batches, builds the indexes and foreign keys with no long lock, and swaps them all in one cutover of milliseconds. `rollback` and `revert` go back the same way.
>
> **What it refuses, and how to go on.** In these cases morm stops the migration with exit code `2`, naming the cause, before it changes anything:
>
> - **A foreign key your models don't declare,** on a table they don't declare or on one they do. morm can't change a column it doesn't manage, and if that column stayed narrow, its foreign key would fail once an id passed the old limit. `--fast` refuses it too. Declare the table in your models, or drop that foreign key first.
> - **A partitioned table, a partition, a table in old-style inheritance, or a table in a publication or a subscription.** Postgres can't swap a partitioned table's key without a scan, a new column on an inheritance parent lands on every child, and the new columns would stop replication. Run `makemigrations --fast` instead. On a partitioned or inherited table, `--fast` needs every partition and child declared in your models with the same change, as the notice on inherited columns says.
> - **A table with no primary key and no unique key on `NOT NULL` columns.** The copy has no key to walk by. Add one first, or run `makemigrations --fast` instead.
> - **An exclusion constraint, extended statistics, a stored generated column, a domain or composite type, or a view such as `SELECT *`, on a column in the change, or a view your models don't declare.** morm can't move these with no lock. Run `makemigrations --fast` instead.
> - **A change to another kind of type,** such as `integer` to `uuid`, or `timestamp` to `timestamptz`. Run `makemigrations --fast` instead.
> - **A generated key leaving the integer types, a reference of another kind of number, keys that would merge, and some collation changes.** The notices below say why, and how to go on.
> - **Going back down,** such as a `rollback` from `bigint` to `integer`, **once a value or the sequence has passed the old type's limit.** Nothing makes that safe, so nothing forces it. Leave the column wide, or remove the values that don't fit first.
>
> **When `--fast` is right.** Only with apps stopped, in a maintenance window, or on tables small enough that a full lock for their rewrite doesn't matter.
>
> **What you give up.** `--fast` rewrites each table under `ACCESS EXCLUSIVE`, so every read and write on any of them waits until it's done, and `max_rewrite_mb` refuses a table bigger than its limit. Going back down online, an app write the old type can't hold, such as a new id past `2^31 - 1`, fails with `22003` from the migration's first step, not only after the cutover.

**R-1130** The README says this plainly, beside that notice. When an identity key's type changes online, the cutover replaces its sequence with a new one under the same name, at the same position. A session that was open across the cutover gets `55000` from `currval` and `lastval` on that sequence until it next calls `nextval`, such as with its next insert. So read a new id from `RETURNING`, not from `currval` or `lastval`. A `serial` key keeps its sequence, so it isn't affected.

**R-1137** The README shows this notice loudly, beside the cutover settings:

> **A cutover that takes many locks can be refused before it starts.** A cutover gives way to your app: each lock waits at most `cutover_lock_timeout`, and then the attempt changes nothing and tries again, so Postgres never has to kill one of your transactions to break a deadlock. Postgres starts looking for a deadlock once a lock has waited `deadlock_timeout`, and an attempt holds every lock it has while it waits for the next, so morm shrinks each lock's wait to `deadlock_timeout / (2 × locks)` when that's shorter, and says so. Postgres counts `lock_timeout` in whole milliseconds and reads 0 as no limit, so when that wait comes to under 1 ms, `migrate` and `rollback` stop with exit code `2` before the change's first step, naming `cutover_lock_timeout`, `deadlock_timeout` and the number of locks, and change nothing.
>
> **How to go on.** Raise `deadlock_timeout` on the server, so each lock gets at least a millisecond: its default of `1s` covers a cutover of up to 500 locks. If `cutover_lock_timeout` is the one set under 1 ms, raise that instead. Or write the migration with `makemigrations --fast` and run it in a maintenance window.
>
> **What you give up.** With a higher `deadlock_timeout`, a real deadlock between your own transactions takes that long to be broken. `--fast` changes the tables in one transaction under `ACCESS EXCLUSIVE`, so every read and write on them waits until it's done.

**R-1138** The README shows this notice loudly, beside the online swap:

> **An online change needs a key to walk the table.** When morm changes a column's type online, swaps an enum online, or changes a key other tables reference, it copies the rows in batches, walking the table by its primary key, so the copy stops at a fixed point and carries on after a crash. A table with no primary key is walked by a unique key whose columns are all `NOT NULL`, with no `WHERE` and no expression. A table with neither is refused before anything changes, with exit code `2`, naming the table.
>
> **How to go on.** Add a primary key, or a unique key on `NOT NULL` columns, in an earlier migration, or write this one with `makemigrations --fast` and run it in a maintenance window.
>
> **What you give up.** `--fast` rewrites the table in one transaction under `ACCESS EXCLUSIVE`, so every read and write on it waits until it's done.

**R-1139** The README shows this notice loudly, beside the notice on changing a key other tables reference:

> **A generated key stays an integer.** Postgres allows identity columns, and sequences, only on `smallint`, `integer` and `bigint`, and no sequence can count past `bigint`'s maximum, so a generated key gains nothing as `numeric`. So `identity` on any other type is a compile error, and a migration that takes a key out of the integer types while dropping its identity, or a `serial` key's sequence, is refused with exit code `2` when it's made.
>
> **When it's right to go there anyway.** When your app supplies its own ids, such as `u64` ids from another system, stored as `numeric(20,0)`.
>
> **How to go on.** Use two migrations. The first drops the identity, or the default, which changes no rows. The second changes the type, with no sequence involved.
>
> **What you give up.** The database no longer makes the ids, so every insert must supply one, and a build that still leaves the id out of its inserts is refused when its pool starts. If you add identity back later, morm sets its sequence past the highest key, so the next insert doesn't collide.

**R-1140** The README shows this notice loudly, beside the notice on changing a key other tables reference:

> **Changing a key's values changes them everywhere.** A key change that lowers a scale, a timestamp precision or a length, such as `numeric(20,4)` to `numeric(20,2)`, changes the stored keys: `1.0051` becomes `1.01`. morm runs it online and changes every column that references the key the same way, so no reference is lost. Like every change Postgres would round or cut silently, it waits for `migrate --phase contract`, or for `--fast` with apps stopped. `makemigrations` asks you to accept it as a `lossy` decision, naming every referencing column, and `migrate` asks again with the number of rows that would change.
>
> **What it refuses.** Two keys that would become one, such as `1.0051` and `1.0149`, both `1.01`, stop `migrate` or `rollback` with exit code `2` before anything changes, naming them, because no answer can make two keys one. A referencing column that doesn't change to exactly the key's new type is refused when the migration is made, because it would no longer match its key.
>
> **How to go on.** Change or remove the keys that would merge, then run it again.
>
> **What you give up.** Copies of the key held outside the database, such as in caches, URLs, other services, logs and exports, keep the old values and won't match any more. While it runs, a write whose key matches another only once rounded fails with `Error::Unique` on the key's field.

**R-1141** The README shows this notice loudly, beside the notice on changing a key other tables reference:

> **A narrower key is refused while a value doesn't fit.** Going down to a narrower key, by `rollback`, `revert` or a narrowing migration, such as `bigint` to `integer` or `varchar(10)` to `varchar(3)`, stops with exit code `2` before anything changes when a live value doesn't fit the narrower type, naming the table, the column, one such value and how many there are. A sequence past the narrower limit is refused the same way. Nothing makes that safe, so nothing forces it.
>
> From the migration's first step until it's done, a write the narrower type can't hold fails with the database's error for that write, such as `22003` for an id past `2^31 - 1` or `22001` for a text too long. A row that doesn't fit and arrives before the migration's trigger is in place stops the copy with the same message, and `rollback --failed` cleans up.
>
> **How to go on.** Leave the key wide, or change or remove the values that don't fit first.

**R-1142** The README shows this notice loudly, beside the notice on changing a key other tables reference:

> **A reference must stay the key's kind of number.** When a key changes, every column that references it must end up the same kind of type as the key. Postgres accepts a `bigint` column referencing a `numeric(20,0)` key, but compares the two through a cast, so the foreign key can't use the referencing column's index, and every delete of a key reads the whole referencing table: deleting 10 keys took 683 ms, against 2 ms with matching types. So `makemigrations` refuses it with exit code `2`, naming both columns, with `--fast` too.
>
> **How to go on.** Give the referencing column the key's new type.

**R-1143** The README shows this notice loudly, beside the notice on changing a key other tables reference:

> **Some collation changes on a key are refused online.** morm names a column's collation in every type change it writes, because Postgres would otherwise reset it to the default, and a collation change on its own runs through the online swap, because it rebuilds the column's indexes. A key whose collation changes between two deterministic collations, where equal means the same bytes, changes online like any other key change. A change to or from a nondeterministic collation, such as a case-insensitive one, is refused online with exit code `2`, naming both collations, because it can turn two keys into one, such as `'abc'` and `'ABC'`, and Postgres won't keep a foreign key between the two collations while the change runs.
>
> **How to go on.** Write the migration with `makemigrations --fast` and run it with apps stopped. If two keys would become one, it fails on the unique index and changes nothing.
>
> **What you give up.** `--fast` rewrites every table in the change in one transaction under `ACCESS EXCLUSIVE`, so every read and write on them waits until it's done.

**R-1144** The README shows this notice loudly, beside the rolling deploy rows:

> **A change between `timestamp` and `timestamptz` asks which zone your values are in.** Postgres converts each value through the time zone of the session that runs the change, with no error, so the same stored `12:00` becomes `12:00` UTC on one host and `17:00` UTC on another. So `makemigrations` stops with exit code `3` and asks which zone the stored values are in, then writes that zone into the migration, so it gives the same data wherever it runs, and its down converts back through the same zone.
>
> **How to answer.** `makemigrations --decide ID=<zone>`, such as `ID=UTC`, with the zone your app wrote the values in.
>
> **What you give up.** One zone covers every row of the column, so rows written in different zones need fixing first. On a key other tables reference, the change can't run online, so it needs `makemigrations --fast`.

**R-1145** The README shows this notice loudly, beside the notice on changing a key other tables reference:

> **`--fast` on a key rebuilds its foreign keys and indexes.** With `--fast`, a key change that changes stored values, moves to another kind of number, such as `integer` to `numeric(20,0)`, or changes to or from a nondeterministic collation can't run with the foreign keys in place. So morm drops the foreign keys on the key, changes every column, and adds them back with the same names and rules, checking every row, all in the one transaction. Then it rebuilds any index on those columns that Postgres changed on its own, such as one whose operator class it replaced, so every index ends as your models declare it.
>
> **What you give up.** All of this runs under the one full lock, so checking the foreign keys and rebuilding the indexes adds to the time every read and write on those tables waits.

**R-1155** The README shows this notice loudly, beside the online swap:

> **An online change refuses partitioned and inherited tables.** When morm changes a column's type or collation online, swaps an enum online, or changes a key other tables reference, it adds a new column beside the old one, copies the rows, builds the indexes with no long lock, and renames them at cutover. Postgres can't do that on a partitioned table or a partition: it can't build an index concurrently on a partitioned table, or add a column to a partition. On a table in old-style inheritance, a new column on a parent lands on every child, and an inherited column's type can't change on a child alone. So `migrate` and `rollback` stop with exit code `2` before anything changes, naming the table and its partitions, parent or children.
>
> **How to go on.** Write the migration with `makemigrations --fast` and run it in a maintenance window. On a partitioned table or an inheritance parent, Postgres changes the column on every partition and child with it, so morm runs it only when your models declare every one of them with the same change, as the notice on inherited columns says.
>
> **When `--fast` is right.** Only with apps stopped, or on tables small enough that a full lock for their rewrite doesn't matter.
>
> **What you give up.** `--fast` changes the column in one transaction under `ACCESS EXCLUSIVE`, on every partition and child too, so every read and write on them waits until it's done.

**R-1156** The README shows this notice loudly, beside the online swap:

> **A type change rebuilds an index Postgres changed on its own, and stops on one that can't fit.** `ALTER COLUMN ... TYPE` rebuilds a column's indexes from their old definitions without checking them against your models: it silently replaced an explicit `int4_ops` with `bigint`'s default class, and kept `varchar_pattern_ops` on a column changed to `text`. So after every type change, in place, under `--fast` or in an enum swap, morm compares each index on the column with what your models build on scratch and rebuilds any that differs: in place on a live table with `CREATE INDEX CONCURRENTLY`, and under `--fast` in the same transaction. An online swap builds its new indexes from your models in the first place. An index whose `ops` doesn't accept the new type stops `makemigrations`, naming the index and its `ops`, such as `operator class "int4_ops" does not accept data type bigint`.
>
> **When it's right.** Always: an index whose class doesn't accept the new type can't exist on it, so there's no override.
>
> **How to go on.** Change that index's `ops` in your model to one for the new type, or remove it to use the type's default class, then run `makemigrations` again.
>
> **What you give up.** Under `--fast`, a rebuild adds to the time every read and write on that table waits. In place, the new index is built beside the old one, so it takes the time and space of a fresh index. While it builds, and while an online swap runs, a write whose value matches another row's only once converted, such as `1.0051` and `1.0149` both becoming `1.01`, fails with `Error::Unique` on the column's field.

**R-1163** The README shows this notice loudly, beside the online swap:

> **An inherited column changes only through its top parent.** On a partitioned table, or a table in old-style inheritance, a column the children inherit can only be changed on the table at the top. Postgres refuses to change it on a child or a partition alone, with `cannot alter inherited column`, and a change on the top parent changes it on every child and partition with it. So when a migration changes such a column's type or collation, `migrate` and `rollback` run one `ALTER` on the top parent, and only when your models declare every child and partition it reaches, each with the same change. Otherwise they stop with exit code `2` before anything changes, naming the column, its top parent and every child and partition: a change to a child's or a partition's column without its top parent, a change that would reach a child or a partition your models don't declare, or declare with the old type, and a change to a column a child inherits from two parents, which no one `ALTER` can make.
>
> **When it's right.** Always: morm never changes a table your models don't declare, and Postgres can't change a child's inherited column on its own.
>
> **How to go on.** Declare every child and partition in your models with the same change, make it on the top parent too, and run `makemigrations` again. A column a child inherits from two parents can't be changed by morm, so change it by hand outside morm, then let your models follow.
>
> **What you give up.** A child or a partition you'd rather leave out of your models has to be declared before its inherited column can change.

**R-1164** The README shows this notice loudly, beside the online swap:

> **A type change drops and adds back the foreign keys on an index it rebuilds.** When morm rebuilds an index after a type change, as the notice above says, and a foreign key references that index, Postgres won't drop the index while the foreign key is there, and has no way to move the foreign key to a new index. So morm drops the foreign key and adds it back with the same name and rules. In place on a live table, it builds the new index first with no long lock, swaps the index and the foreign keys in one short transaction that gives way to your app, as a cutover does, and then checks the existing rows with `VALIDATE`, which doesn't block writes. Every write is checked by the old foreign key or the new one, so no row that breaks it can get in. Under `--fast`, all of it runs in the one transaction. A foreign key your models don't declare on that index stops the migration with exit code `2` before anything changes, naming it.
>
> **When it's right.** Always: the index can't be rebuilt any other way.
>
> **How to go on.** For a foreign key your models don't declare, declare its table in your models, or drop that foreign key first.
>
> **What you give up.** The short transaction takes a full lock on the key table and every table that references it, for a few milliseconds. Until `VALIDATE` finishes, Postgres marks the foreign key `NOT VALID`, although it checks every write, so the planner can't use it to leave out a join.

**R-1165** The README shows this notice loudly, beside the online swap:

> **An online change refuses values that would become one on a unique index.** A type change, a collation change or an enum's map can make two different values equal: `1.0051` and `1.0149` both become `1.01` in `numeric(10,2)`, `'01'` and `'1'` both become `1` as an `integer`, `'abc'` and `'ABC'` become equal under a case-insensitive collation, and mapping `pending` to `active` puts two rows on `UNIQUE (uid) WHERE status = 'active'`. Before an online change touches a table, morm looks for such values on every unique index that holds the column, and stops `migrate` or `rollback` with exit code `2`, naming the table, the index, the values and how many rows hold each. When the new type comes from the same migration, as an enum swap's does, it creates that type in a transaction of its own, looks, and rolls the transaction back, so nothing is left behind and the migration isn't marked `failed`. The check only reads, so your app's reads and writes go on while it runs.
>
> **When it's right.** Always: no answer can make two rows one on a unique index.
>
> **How to go on.** Change or remove the rows that would merge, or drop the unique index first if equal values are what you want, then run it again.
>
> **What you give up.** The check reads the whole table once. A row that would merge, written after the check, stops the migration as `failed` when the new unique index is built, and `rollback --failed` cleans up. While the change runs, such a write fails with `Error::Unique` on the column's field.

**R-1166** The README shows this notice loudly, beside the safe forms table:

> **Adding an index or a unique key to a partitioned table needs `--fast`.** On an existing table, morm builds an index with `CREATE INDEX CONCURRENTLY` and turns a unique index into a unique key or a primary key with `USING INDEX`, so writes never wait. Postgres can do neither on a partitioned table. So adding an index, a unique key or a primary key to one, or rebuilding an index on one after a type change, stops `migrate` and `rollback` with exit code `2` before anything changes, naming the table. Your models can't declare a partitioned table, so this only happens on one made outside morm. A single partition is a plain table, so these run there with no long lock.
>
> **How to go on.** Write the migration with `makemigrations --fast` and run it in a maintenance window. A unique key or a primary key on a partitioned table must include every column of its partition key, so morm refuses one that doesn't, under `--fast` too, naming those columns. Under `--fast` the index reaches every partition, so each must be declared in your models, as the notice on changes that reach a child or a partition says.
>
> **When `--fast` is right.** Only with apps stopped, or on tables small enough that building the index under a lock doesn't matter.
>
> **What you give up.** `--fast` builds the index on every partition under a lock that blocks writes to the table until it's done.

**R-1170** The README shows this notice loudly, beside the safe forms table:

> **A foreign key from a partitioned table needs `--fast` on Postgres 15 to 17.** On an existing table, morm adds a foreign key `NOT VALID`, which checks no rows, then checks the rows already there with `VALIDATE` in a separate step, which doesn't block writes. Postgres 15 to 17 can't add a foreign key `NOT VALID` on a partitioned table, and Postgres 18 can. So on 15 to 17, adding a foreign key from a partitioned table, adding one back after an index rebuild, or restoring one in a `rollback` stops `migrate` or `rollback` with exit code `2` before anything changes, naming the table, the foreign key and the server's version. A foreign key from a single partition, or from a plain table to a partitioned one, runs as usual.
>
> **When it's right.** Always on those versions: otherwise Postgres would refuse the statement partway through the migration.
>
> **How to go on.** Write the migration with `makemigrations --fast`, or roll back with `rollback --fast`, and run it in a maintenance window, or upgrade the server to Postgres 18, where the usual form runs.
>
> **What you give up.** `--fast` adds the foreign key in one statement that checks every row of every partition while writes to those tables, and to the table it references, wait. A row that breaks it stops the statement, naming the partition, with nothing changed. In a `rollback` of a retire, keeping the foreign key `NOT VALID` isn't offered for such rows.

**R-1171** The README shows this notice loudly, beside the safe forms table:

> **A change that reaches a child or a partition needs every one declared.** On a partitioned table, or a table in old-style inheritance, Postgres carries most changes from the table to every child and partition: adding, dropping or renaming a column, `NOT NULL`, a default, a CHECK, and on a partitioned table also an index, a unique key, a foreign key, a trigger and identity. Dropping a partitioned table drops every partition. And Postgres refuses most of these on a child or a partition alone when it holds the column or the constraint from its parent. So `migrate`, `rollback` and `purge` run such a change once, on the top parent, and only when your models declare every child and partition it reaches, each making the same change to its columns. Otherwise they stop with exit code `2` before anything changes, naming the change, the table, its top parent and every child and partition it reaches: a change that would reach a child or a partition your models don't declare, or declare without that change, and a change to what a child or a partition holds from its parent, made without its top parent. `--fast` doesn't change this. Retiring a table with children or partitions retires every one of them in the same migration, or none.
>
> **When it's right.** Always: morm never changes a table your models don't declare, and Postgres would change such a child with no model asking for it.
>
> **How to go on.** Declare every child and partition in your models with the same change, make the change on the top parent too, and run `makemigrations` again.
>
> **What you give up.** A child or a partition you'd rather leave out of your models has to be declared before its parent can change. A child created by hand after a column is retired stops that column's `purge` until it's declared.

**R-1173** The README shows this notice loudly, beside the safe forms table:

> **Retiring a partition or a child table needs its parent retired too, or the table detached.** morm retires a table by moving it to the `_morm_retired` schema, so it keeps its rows for `rollback`. A partition moved there stays attached to its partitioned table, and a child in old-style inheritance stays under its parent: the parent keeps reading its rows, and a partition keeps receiving new ones. So `migrate` and `rollback` stop with exit code `2` before anything changes when a migration retires a partition or a child while a parent of it stays, naming the table and each parent. `drop now` and `purge --now` on such a table stop the same way, because dropping a partition takes its rows out of the parent with no error and leaves its range with no partition.
>
> **When it's right.** Always: a retired table must stop serving the app's reads and writes.
>
> **How to go on.** Detach the table first, outside morm, with `ALTER TABLE parent DETACH PARTITION child` or `ALTER TABLE child NO INHERIT parent`, then run the command again. Or retire the parent too, with every child and partition, in the same migration.
>
> **What you give up.** Dropping an old partition takes one statement by hand before the migration that retires it.

**R-1182** The README says this plainly, beside the notice on changes that reach a child or a partition. A child's or a partition's model declares all its columns, inherited or not, and only the constraints, indexes and triggers of its own. What Postgres puts on each child and partition from a parent, a CHECK, and on a partitioned table its foreign keys, primary key, unique keys, indexes and row triggers, is declared once, in the parent's model, and morm reads it as the parent's wherever it reads the database: in `verify`, in the checks before and after each migration, and in `import`. A partition's primary key is the one exception: Postgres names a partition's copy of its parent's key `<partition>_pkey`, which is morm's own name for it, so a partition's model may mark `pk` on the parent's key columns, in their order, with `base = "bare"` or a base of your own, and morm reads it as the parent's key, not a second one. Leaving `pk` out works too. It matches only under that name: while a partition's model marks `pk`, a key Postgres kept under another name, such as one `ATTACH PARTITION` took over, is drift, and `verify`, `migrate` and `import` give the `ALTER TABLE ... RENAME CONSTRAINT` that fixes it. `import` marks `pk` on a partition whose key has that name, and leaves it out, saying why, on one whose key doesn't. A child in old-style inheritance holds no key from its parent, so its `pk` is its own. A model that declares anything else of these on a child or a partition stops `migrate` and `rollback` with exit code `2`, naming it and the parent's object it's held from, and `verify` reports it.

**R-1188** The README shows this notice loudly, beside the safe forms table:

> **A partition's `pk` must stay on its parent's key.** A partition's model may mark `pk` on the key the partition holds from its partitioned table, on the parent's key columns, in their order. Postgres keeps that key in step with the parent's: dropping the parent's key drops every partition's, changing its columns builds every partition's again on the new ones, and adding a key to the parent fails while a partition has a key of its own on other columns. So `migrate` and `rollback` stop with exit code `2` before anything changes, naming the partition, the key its model marks and its parent's key, when a partition's model marks `pk` on other columns or in another order, when a migration drops or changes the parent's key and a partition's model keeps `pk` as it was, and when a migration gives the parent a key while a partition has its own on other columns. `makemigrations` can't see this, because your models don't say a table is a partition, so it shows up when a command reaches the database.
>
> **When it's right.** Always: otherwise the migration would fail partway, or leave a partition's key different from its model.
>
> **How to go on.** Mark `pk` on the parent's key columns, in their order, or leave it out. In the migration that drops or changes the parent's key, take `pk` out of each partition's model or change it the same way. Take a partition's own key on other columns out in an earlier migration than the one that gives the parent a key.
>
> **What you give up.** A partition can't keep a key of its own on other columns once its parent has one, and giving the parent a key can take one more migration.

**R-1189** The README shows this notice loudly, beside the safe forms table:

> **A partition's model can't have identity, and the column its parent's identity fills is `db_set`.** A partition takes its values from its parent's identity, with no sequence of its own, and Postgres refuses identity of its own on a partition. `base = "common"` and `base = "pk_only"` add `id` as identity, and so does a field marked `identity`. So `migrate` and `rollback` stop with exit code `2` before anything changes when a partition's model has identity, naming the partition, the column and the parent's identity column. A partition's model whose inserts send the column its parent's identity fills is refused too, when a pool starts: under `GENERATED ALWAYS`, Postgres refuses every such insert with `cannot insert a non-DEFAULT value into column`, and under `BY DEFAULT`, the keys you send can collide with the ones the parent's sequence gives later. So `Db::connect()` returns `Error::SchemaMismatch`, naming the partition, the column and the parent's identity column, and saying to mark the field `db_set`. The compiler and `makemigrations` can't see either, because your models don't say a table is a partition, so they show up when a command or a pool reaches the database.
>
> **When it's right.** Always: Postgres would refuse the identity partway through the migration, and would refuse every insert that sends a `GENERATED ALWAYS` column.
>
> **How to go on.** Declare the partition's model with `base = "bare"`, mark `pk` on the parent's key columns, with no identity, and mark the column the parent's identity fills `db_set`, so inserts leave it out and Postgres gives each row the parent's next value, which the insert fills in on the row. When a migration adds identity to the parent's column, the partitions' models leave identity out and mark that column `db_set`, in the same release that marks the parent's column `identity`, deployed with the builds that insert into the table stopped, as the next notice says.
>
> **What you give up.** Inserts through a partition's model can't choose the key: each row gets the parent's next value. To write keys you choose, insert through the parent's model with `.explicit_ids()`, and Postgres puts each row in its partition.

**R-1196** The README shows this notice loudly, beside the rolling deploy rows:

> **A build whose inserts send an identity column won't start.** Postgres refuses every insert that sends a value into a `GENERATED ALWAYS` identity column, even `NULL`, with `cannot insert a non-DEFAULT value into column`. Into a `GENERATED BY DEFAULT` one it takes the key you send, but the sequence doesn't move past it, so it hands that key out again later, and that insert fails with `duplicate key`. A field your model marks `identity` or `db_set` is left out of every insert, and `.explicit_ids()`, on `insert`, `insert_many`, `insert_stream`, `upsert` and `upsert_many`, lets your keys in under either kind and moves the sequence past them. Any other field is sent, an `Option` one whenever it holds a value. So when a pool starts, `Db::connect()` returns `Error::SchemaMismatch` for a model whose inserts send a column with identity of either kind, naming the table, the column and the kind, and saying to mark the field `identity`, or `db_set`, and to write keys you choose with `.explicit_ids()`, before any of your queries run. On a partition, the notice above says what to do.
>
> **When it's right.** Always, for a build you mean to run. This is how a build made before a migration gave that column identity shows up, such as an older release brought back: it's refused at startup instead of failing each insert in production, or storing keys that collide later. It's worse with `insert_stream`: `COPY` stores the keys it sends with no error under either kind, and the sequence hands the same keys out again later, so a later insert fails with `duplicate key`. It also catches a newer build that took `identity` off a field before the migration that drops the identity has run, and a model that writes its own keys into a `BY DEFAULT` column with plain inserts, which works only until the sequence reaches them.
>
> **How to go on.** Mark the field `identity`, as the build made with the migration does, or `db_set` in a partition's model. In a model of a table morm migrates, mark it `identity` rather than only `db_set`, so `makemigrations` keeps the identity. To write keys you choose, under either kind, use `.explicit_ids()`, which keeps the sequence ahead of them. Plan the deploy that adds identity as the next paragraph says.
>
> **What you give up.** Keys you choose go in only through `.explicit_ids()`, never through a plain insert. The role that writes them needs `UPDATE` on the table as well as `SELECT` and `UPDATE` on its sequence, and moving the sequence holds the table's other writes back for a moment. A build made before such a migration can't start again after it, so rolling the app back past it needs the migration rolled back too. A running one isn't stopped: morm checks once, when a pool starts, so its inserts into that table keep failing, or keep sending keys that collide later, until you stop it. `check_schema = "off"` lets such a build start, with those inserts.

**R-1197** The README says this plainly, beside that notice. Adding identity to a column that already exists and has no default has no deploy order that keeps every running build's inserts into that table working. A build made before the migration sends the column, and a build made with it leaves it out. Run the migration first, and the older build's inserts fail under `GENERATED ALWAYS`, or, under `BY DEFAULT`, go in with keys the sequence hands out again later, so a later insert fails with `duplicate key`. Deploy first, and the new build can't start, because the column has nothing to fill it yet. So stop the builds that insert into that table, or stop their inserts, run `migrate`, and then start the build made with the migration. Reads, deletes and updates that don't set the column keep working throughout, and so does the rest of the app. `makemigrations`, `migrate` and `rollback` print a warning naming the table and column for such a migration, and rolling it back needs the same stop, in reverse. On a partitioned table, the same release marks the parent's column `identity` and the column in each partition's model `db_set`: neither an earlier release nor a later one works. Adding identity to a column that has a default, such as `serial`, is gentler. morm replaces the default with the identity in one transaction, keeps the old sequence's name, and starts the identity past every key the old sequence gave, so builds whose inserts leave the column out, with the field marked `db_set` or an `Option` holding `None`, keep working through the migration in either order, and so does the build made with it. Only builds that send the column need stopping before `migrate`, and the warning says so. Rolling it back puts the old sequence and default back the same way. Adding a new column with identity, creating a table with it, and changing a column between `BY DEFAULT` and `GENERATED ALWAYS` deploy in the usual order, `migrate` and then the new build, because no build made from your models sends those columns.

**R-1201** The README shows this notice loudly, beside the safe forms table:

> **A sequence something else uses can't become a column's identity.** When a migration gives identity to a column whose default calls a sequence the column owns, such as `serial`'s, morm drops that sequence and gives the identity's sequence its name and position, in one transaction. Postgres won't drop a sequence anything else depends on, such as another table's default, a view or a function. An inheritance child's default on it would be dropped with the parent's, and Postgres gives the parent's identity to partitions but not to inheritance children. So `migrate`, and `makemigrations` when it can see it, stop with exit code `2` before anything changes, naming the sequence, the column and each thing that uses it. `rollback` stops the same way when something has come to depend on the identity's sequence since.
>
> **When it's right.** Always: Postgres would refuse the drop partway through the migration, and an inheritance child would be left with no default and no identity, so each of its inserts would fail with `null value in column`.
>
> **How to go on.** Keep the default on that column, or in an earlier migration give each other user a sequence or default of its own, then add the identity.
>
> **What you give up.** A sequence that several columns or objects share stays a plain sequence and default, for as long as it's shared.

**R-1106** The README says this plainly, beside the `Decimal` and `Numeric` rows. `Decimal` holds up to 28 significant digits. The limit comes from `rust_decimal`, which keeps every value in a fixed 16 bytes, as a 96-bit whole number and a scale, so it needs no heap and is `Copy`. A `Decimal` on a `numeric(p,s)` with `p` or `s` above 28 is a compile error. A `numeric` with no precision compiles with `Decimal`, because existing tables have it, and a value that doesn't fit fails at read with `Error::Decimal`. So does a value past 28 digits in a column a migration has widened. A `Decimal` field also reads `smallint`, `integer` and `bigint`, because every one of their values fits in 28 digits, so a column changed from `numeric` to an integer type doesn't stop a running app built for `numeric`. When that change drops a scale, Postgres would round what such an app writes, so `migrate` holds it for `contract`, and an app built for that scale that starts after it fails the schema check morm runs when a pool starts, rather than rounding what it writes. `morm::Numeric` reads `smallint`, `integer` and `bigint` the same way. morm never rounds a `Decimal`, on read or on write. Use `morm::Numeric` when values can exceed 28 digits, or be `NaN` or `Infinity`, or when you don't know a column's range.

**R-1107** The README says this plainly, beside the rolling deploy rows. A type change never stops a running app. An app built before a migration changes a column's type, length or precision keeps working through it, and still starts after it, apart from the one case below, while the column stays in the same family: integers; `numeric`, where `Decimal` and `Numeric` fields also read `smallint`, `integer` and `bigint`; `text` and `varchar`; `char`; or timestamps. `timestamp` and `timestamptz` stay apart, because a change between them shifts every value by the time zone of the session that runs it, with no error. It reads each value through the decoder for the column's real type, chosen once per statement, so it costs nothing when the types match. After a widening, every value the old column could hold reads as before, and a value only the new column can hold fails on its own: an integer field returns `Error::OutOfRange` naming the column, and `Decimal` returns `Error::Decimal` past its 28 digits. `Numeric` and `String` read every value whole, apart from `char`: Postgres pads a `char` value with spaces to its length and treats them as insignificant, so a field built for `char(3)` reads `char(4)`'s `'ab  '` as `'ab '`, as it did before, and `'abcd'` whole. After a narrowing, every value fits the old field, so every read works, and a write the column can no longer hold, such as a number with too many digits before the point or a text too long, gets the database's error for that write. Some narrowings would change a write with no error instead: a lower scale stores `1.0051` as `1.01`, and `numeric(12,2)` to `integer` stores `1.25` as `1`, a lower timestamp precision stores `.123456` as `.123`, and a shorter `varchar` or `char` stores `'abc  '` as `'abc'`. morm never runs one of those while an app built for the old shape may be running, and in an ordinary online `migrate` it always assumes one is, without trying to find out. So `migrate` refuses it, naming the column, what would change, and the two ways to run it: `migrate --phase contract` once the new release is out everywhere, or `makemigrations --fast` for a maintenance window with apps stopped. An old app never sees one: through the rollout its column keeps the old shape, and every value it writes is stored as written. So an old app keeps running through the deploy, because the narrowing waits until it's gone. Only a build brought back after `contract` has run, such as a rollback of the app to an older release, would write to the narrower column, so it's refused at startup: morm runs `check_schema` when each pool starts, and `Db::connect()` returns `Error::SchemaMismatch`, naming the column and what would change, so the app fails loudly rather than storing values other than the ones it writes, unless that pool is set to `check_schema = "off"`. A narrowing that raises a real error on a write that doesn't fit still lets such a build start. `rollback` holds the same narrowings when one is the down of a widening, and runs it with `--phase contract` once every app runs the release built for the narrower shape, or `--fast` with apps stopped. Such a narrowing also changes the rows already stored, so a `1.0051` already in the column becomes `1.01`, and `makemigrations` asks you to accept that as a `lossy` decision before it writes the migration. `migrate` and `rollback` count the rows in the live table it would change, and when there are any they stop and ask, showing the count, answered with `--decide`. When there are none they run with no question, so rolling back a widening asks only when rows were written since that need the wider column. Nothing read is ever rounded. A change to an unrelated kind, such as `text` to `integer`, still fails the startup check.

**R-1119** The README shows this notice loudly, beside the rolling deploy rows:

> **Turning off the startup check lets a build run against a schema it can't use.** morm runs `check_schema` when each pool starts, so a build that doesn't match the database refuses to start. `Db::connect()` returns `Error::SchemaMismatch`, naming each table and column, the definition the build expects, the one the database has, and what's wrong, before any of your queries run. A pool added with `db.pool(name)` returns the same error from every call until a check passes.
>
> **What it does.** One query over the catalog, on a pool's first connection, covering every model in the binary, and nothing after: no work on later connections or queries. 200 tables took about 3 ms on a local connection. Every pool runs it, replicas included, so a replica that hasn't replayed a migration yet fails it until it has. It works behind PgBouncer in transaction mode, checking the database PgBouncer connects you to.
>
> ```toml
> [pools.legacy]
> check_schema = "off"     # this database holds none of this binary's models
> ```
>
> **When it's right.** Only for a pool whose database isn't meant to hold the binary's models, such as a pool used only for raw SQL on another database. `MORM_POOL_CHECK_SCHEMA=off` does the same for the pool a process opens at startup. Never turn it off to get a failing build to start: the check failed because that build can't use this schema.
>
> **What you give up.** A build brought back after a migration's `contract`, such as a rollback of the app to an older release, starts anyway, and Postgres changes what it writes with no error: a field built for a wider scale, timestamp precision or length than its column stores `1.0051` as `1.01` or `'abc  '` as `'abc'`. A missing column or an unrelated type fails each query that reaches it, in production, instead of failing at startup. `db.check_schema()` still runs the check whenever you call it.

### Risks, open items and later work / Open items

- **R-0920** All six names (`morm`, `morm-core`, `morm-macros`, `morm-migrate`, `morm-admin`, `morm-mgr`) are reserved on crates.io at `0.0.0`. The release publishes over them.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0115 | suite | **Doc tests:** every README and doc example compiles and runs. |
