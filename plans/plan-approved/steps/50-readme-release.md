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

**R-0797** One compact, complete file at the repo root. It must be enough to use morm efficiently without other docs, and it must be explicit about doing the right thing. It's included as crate docs (`#![doc = include_str!("../README.md")]`), so CI compiles every example and it can't go stale.

- **R-0798** What it is and why: a few lines, plus benchmark numbers.
- **R-0799** Quick start: install, `morm-admin init`, first model, first query.
- **R-0800** **Pick the right call**, near the top:

| ID | You want to | Use | Not |
|---|---|---|---|
| R-0801 | Send the data on unchanged | `fetch_json`, `stream_json` | `fetch`, then serialize |
| R-0802 | Read a few columns | `fetch_raw` | `fetch` |
| R-0803 | Read and compute, no changes | `fetch_ref` | `fetch` |
| R-0804 | Keep or pass data around | `fetch` | |
| R-0805 | Change known fields | `update` | Fetching the row first |
| R-0806 | Set one value on many rows | `update_q` | A loop of `update` |
| R-0807 | Set different values on many rows | `update_each` | A loop of `update` |
| R-0808 | Do math on a value | `set_expr` | Read, compute, write |
| R-0809 | Fetch related rows | `= ANY` or a JOIN | A query per row |
| R-0810 | Add many rows | `insert_many` | A loop of `insert` |
| R-0811 | Handle huge results | `stream` | `.all()` |

- **R-0812** **Performance rules:** never query in a loop, always limit lists, select through the narrowest view, keep transactions short with no network calls inside them, index what you filter on, release every `db.hold()`, use `tx` and never `db` inside a transaction, no network calls or app `Mutex` waits inside one, build rows inside a retried closure, lock several rows with `tx.lock`, use `.lock_ordered()` on contended bulk updates, put side effects in `after_commit`.
- **R-0813** **Extra calls, one line each.** Every opt-in call, escape hatch and override gets one line saying when to use it. A public option with no line here is a documentation bug, and CI compares this table with the public API.

| ID | Call | Use this when |
|---|---|---|
| R-0814 | `.allow_missing()` | You will not handle a 0-row error |
| R-0815 | `#[col(json_string)]` | JavaScript clients read this integer, and it can pass 2^53 |
| R-0816 | `schema = "..."` | A model's table lives outside the default schema |
| R-0817 | `mgr mode development` | This is a local or staging database, and you want `rollback` |
| R-0818 | `mgr revert ID` | You must undo a migration in production |
| R-0819 | `rollback --production` | `revert` can't work and you accept the risk; a reason is required |
| R-0820 | `mgr squash --to ID` | The chain has grown long, and every database has passed `ID` |
| R-0821 | `tx.emit` | An event must not be lost if the app crashes; use `after_commit` when losing one is acceptable |
| R-0822 | `tx.emit_keyed` | Events about the same thing must arrive in order |
| R-0823 | `fields = "..."` | A view is public, and new columns must stay out until you add them |
| R-0824 | `input = "..."` | A view takes a request body that creates a row |
| R-0825 | `patch = "..."` | A view takes a partial update, such as a PATCH body |
| R-0826 | `.allow_empty()` | An empty update is fine, and you will not handle `EmptyPatch` |
| R-0827 | `span_sample` | You want more or fewer query spans in your tracing platform |
| R-0828 | `morm::sample_scope` | A traced request must keep all of its query spans |
| R-0829 | `slow_query` | You want slow queries reported from the app, including time waiting for a connection |
| R-0830 | `metrics_interval` | Your graphs need finer detail, or your platform bill needs to come down |
| R-0831 | `.explicit_ids()` | You insert rows with ids you already have, such as an import |
| R-0832 | `morm::OrDefault` | A nullable column has a default, and you sometimes need a real NULL there |
| R-0833 | `.for_key_change()` | You will delete locked rows or change their keys, and other requests add child rows |
| R-0834 | `blocking` | A `clean` function is slow, such as password hashing |
| R-0835 | `morm::allow_repeats` | Your code is meant to repeat one query, and the loop detector fails it |
| R-0836 | `pooler = "transaction"` | You connect through PgBouncer in transaction mode |
| R-0837 | `.tag()` | You need to trace a query in the server logs back to the code that ran it |
| R-0838 | `morm-admin size` | You change pool or Postgres memory settings and want to check they fit the server |
| R-0839 | `size --here` | You need a starting config for the machine you're on |
| R-0840 | `size --measure` | You want estimates based on your app's real statements |
| R-0841 | `[server]` | The database runs on another machine, so morm can't read its size |
| R-0842 | `shape_cache` | Logs show churn in runtime-built queries, or your app has many builder query shapes |
| R-0843 | `#[col(secret)]` | A column must never reach JSON or logs, such as a password hash |
| R-0844 | `#[model(deserialize)]` | You load a full model from trusted data, such as a cache; never from a request |
| R-0845 | `keep_case` | You're matching an existing mixed-case table or column |
| R-0846 | `tx.get_for_update` | Your logic must see the current row before changing it, and `set_expr` can't express it |
| R-0847 | `#[col(version)]` | Users edit a row over time, and a stale save must fail, not overwrite |
| R-0848 | `.in_batches(n)` | You change so many rows that one transaction would hold locks too long, and it needn't be atomic |
| R-0849 | `.version(v)` | You update a `version` column and hold the value the client read |
| R-0850 | `.ignore_version()` | A server-side update may overwrite whatever version is there |
| R-0851 | `tx.set_local(name, &value)` | A setting must last one transaction, such as the tenant for RLS |
| R-0852 | `db.insert_stream` | Your rows are too many to hold in memory, such as an import from a file |
| R-0853 | `db.load_related` | You need the child rows of a whole list |
| R-0854 | `.keyset(...)` | You page through a list that can go deep |
| R-0855 | `.order_by_allowed(...)` | A client chooses the sort column |
| R-0856 | `morm::quote_ident` | You use a dynamic table or column name in `q_unchecked` or `sql_unchecked` |
| R-0857 | `.custom_plan()` | A query is slow only for some values, because your data is skewed |
| R-0858 | `nulls_not_distinct` | A unique column may hold only one NULL |
| R-0859 | `#[derive(PgEnum)]` | You're matching an existing Postgres enum; use `TextEnum` otherwise |
| R-0860 | `storage = "fillfactor=90"` | A table gets many updates to columns with no index |
| R-0861 | `.returning()` | You need the changed rows back |
| R-0862 | `.all()` | You want every row, and you know the result is small |
| R-0863 | `set_expr` | The new value depends on the current one, such as `stock - 1` |
| R-0864 | `.retry(n)` | You want the transaction to run again after a deadlock, and you build its rows inside the closure |
| R-0865 | `.isolation(...)` | You read rows and write based on them, and nothing may change them in between |
| R-0866 | `.lock_timeout(...)` | A transaction will wait on locks for longer than 5s, such as a large batch |
| R-0867 | `.transaction_timeout(...)` | A transaction will run for longer than 30s, such as a large batch |
| R-0868 | `tx.lock(&ids)` | You will change several rows that other requests also change, such as a money transfer |
| R-0869 | `.lock_ordered()` | You run bulk updates or deletes that can touch the same rows at the same time |
| R-0870 | `.skip_locked()` | Your workers take jobs from a queue and shouldn't wait for each other |
| R-0871 | `tx.advisory_lock(&keys)` | Only one process may do this work at a time, such as a billing run |
| R-0872 | `db.extra_conn()` | You need a write to survive a rollback, such as an audit row, or a query beside an open stream |
| R-0873 | `.update_only(...)` | An upsert must change only certain columns on conflict |
| R-0874 | `db_set` | Postgres always fills a column on insert, and your field's value should be ignored |
| R-0875 | `deferrable` | Rows in one transaction may break a constraint until commit, such as swapping two unique values |
| R-0876 | `#[morm::hooks]` | You write `impl Hooks` for a model |
| R-0877 | `db.tx()` | A hook needs `tx.emit` or another `tx`-only call |
| R-0878 | `.pause_ms(n)` | Batches must leave room for other work, such as replicas catching up |
| R-0879 | `.limit_each(n)` | You load related rows and each parent needs only a few |
| R-0880 | `db.check_schema()` | A binary should refuse to boot against a schema it can't use |
| R-0881 | `#[col(deprecated)]` | You're removing a column, and old binaries still run |
| R-0883 | `Product::UK.<name>` | An upsert targets an expression unique index, or two partial indexes share columns |
| R-0884 | `db.hold()` | You need `LISTEN`, temp tables, session advisory locks or `SET` |
| R-0885 | `db.pool("replica")` | You run heavy reads, such as reports, that can be slightly out of date |
| R-0886 | `db.pool("jobs")` | You run heavy jobs in the web app's process, and they must not take its connections |
| R-0887 | `db.groups(mask)`, `db.sudo()` | Your callers may see or change different fields |
| R-0888 | `Numeric` | Your values can exceed 28 digits, or be `NaN` or `Infinity` |
| R-0889 | `q_unchecked`, `sql_unchecked` | You must build SQL at runtime, and you will handle injection safety yourself |
| R-0890 | `recycle_when` | You need your own rule for replacing connections |
| R-0891 | `no_default` | You want a compile error when a required field is missed |
| R-0892 | `--fast` | The table is small, or you're in a maintenance window |
| R-0893 | `--accept-data-loss` | You've read the plan, and you accept losing that data |
| R-0894 | `--skip-fingerprint` | Your role can't create a temporary database, and you accept a weaker check |

- **R-0895** Models: bases, every `#[model]` and `#[col]` option in tables.
- **R-0896** `get`, `insert`, `update`, `delete`, lists, `filter_cols`, related rows, pagination, batches, lost updates, raw SQL with `db.sql()`, transactions, and which of `Decimal`, `Numeric` or `bigint` to use.
- **R-0897** Views, including public input and output with `fields`, `input` and `secret`, groups, validation.
- **R-0898** Hooks, with a table of what each receives and this line: "update and delete hooks never see a row unless you fetch it." Then `after_commit` versus the outbox.
- **R-0899** JSON output and schemas.
- **R-0900** Pool and `morm.toml`: every setting, sizing, and the rule for long-running work: "use `db` for everything, including jobs and crons, and use `db.hold()` only when you need session state." It states plainly that service jobs, crons and background workers running as their own process must not spawn a pool with `min` above 1, and shows `connect_as`, `MORM_POOL` and `MORM_POOL_MIN`. It includes the full environment variable table. It also shows the jobs table, the `db.hold()` lifecycle, and when a websocket needs a listener: "release when you're done, and rely on Drop as the safety net, never as the plan."
- **R-0901** Migrations: workflow, every command in one table, safety rules, production being forward only with `revert`, running as a deploy step, triggers and functions.
- **R-0902** Security: TLS settings, encrypting disks and backups (outside morm's job, but expected in production), and password hashing with `clean`.

### Risks, open items and later work / Open items

- **R-0919** `morm`, `morm-macros`, `morm-migrate`, `morm-admin` and `morm-mgr` were free on crates.io on 2026-11-05. Reserve them at the first publish.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0115 | suite | **Doc tests:** every README and doc example compiles and runs. |
