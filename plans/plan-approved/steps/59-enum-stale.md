# Step 59: Managed enums: stale functions and standbys

- **Phase:** Migrations
- **Depends on:** 53, 12, 39
- **Can run beside:** 08, 09, 19, 20, 21, 22, 23, 24, 25, 26, 27, 42, 43, 44, 45, 46, 47, 48
- **Crates:** `morm-migrate`, `.github`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Find the code Postgres doesn't track, keep managed functions working through any op that R-1038 covers, and prove apps recover behind PgBouncer and on a standby.

## Deliverables

- The source search before any op that R-1038 covers, function refresh in that op's transaction, the unmanaged function decision, and the `ci.yml` job for a hot standby

## End-to-end check

On Postgres 15–18, a swap under a plpgsql function that names a removed label proposes the rewritten function, every stale managed function works in open sessions afterwards, and an unmanaged function stops `migrate` until its decision is accepted. The same holds for every other op R-1038 covers, including a composite type recreate, a plain column type change under a trigger, with `--fast` and through the online swap's cutover, and a view's own shape change.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> Step 51 swaps a type on columns. Step 53 walks dependents and refuses anything unmanaged that the catalog shows, and a function that only names the type in its body isn't shown there.
>
> Step 37 re-runs a managed function's `CREATE OR REPLACE` with its definition unchanged, in the same transaction, which makes every session recompile it.
>
> Step 57 changes composite types no column uses. A shape `ALTER TYPE` can't make, such as a middle attribute, a new order or a rename, is a composite type recreate: one transaction drops the type and creates it again from its declaration, keeping its owner, comment and grants. Step 37 carries the managed dependents the catalog shows through that transaction. `ALTER TYPE` keeps the type, so it isn't a recreate.
>
> A recreate differs from a swap. A swap creates a new enum beside the old one, moves columns onto it and rewrites their rows. A composite a column uses is refused, so a recreate has no labels, no rows to move and no affected table or column, and the source search for it is the type's name and the name of every domain, composite type and view carried with it, because each of those gets a new OID too. It's the same for compiled code: the type under a function is replaced, and a function that names it only in its body has no `pg_depend` row, so steps 57 and 37 don't see that function. On Postgres 18, a session that compiled `DECLARE v c[]` failed on every call after a recreate with `cache lookup failed for type`. `DISCARD ALL` didn't clear it, and new sessions worked. Variables of plain `c` kept working in the bodies tried, so only some bodies go stale, which is why every hit is refreshed.
>
> Step 37 drops and recreates managed dependents when a change needs it. Postgres refuses a column type change under a view, and an `ALTER TYPE` that drops or retypes an attribute a view names, or drops one a domain's check names. A view whose own result shape changes is dropped and recreated too. Whatever depends on a dropped object goes with it, such as a domain or composite type over a view.
>
> Step 39's online column swap changes a column's type by adding a new column and renaming it in a short cutover transaction, where it recreates managed views. That cutover is the transaction for that op. With `--fast`, the op is a plain `ALTER COLUMN`.
>
> R-1038 is a rule about what an op does, not a list of ops. Build it as one check that reads what each op reports it changes, so a new op can't miss it. Step 52's cutover comes after this step and uses the same check.
>
> On Postgres 18 and 17, a session that compiled `DECLARE v x[]` and `v := '{}'` failed on every call with `cache lookup failed for type` once `x` was dropped and recreated: a view, a materialized view, a domain, a composite type or an enum, whether by a column type change, an `ALTER TYPE`, its own shape change or by itself. New sessions worked, and a refresh in the same transaction cleared it.
>
> A column type change breaks more, with nothing replaced, the same with a plain `ALTER COLUMN` and the online swap. A trigger function comparing `NEW.n`, a function with a variable of the table's row type comparing a field, and `f(r t)` whose body names no table each failed on every call with `type of parameter ... does not match that when preparing the plan`, and a `t.n%TYPE` variable failed with `integer out of range` once a value needed the new type. `ALTER ATTRIBUTE ... TYPE` did the same to a variable of the composite type. New sessions worked, and a refresh cleared each one. `v t[]`, `RETURNS SETOF t` and `SELECT n INTO` a wider variable kept working.
>
> So a column type change finds functions three ways. `pg_trigger` gives the table's trigger functions, whatever they name. `pg_depend` gives every function with the table's row type or that type's array in its signature, whose body may name neither. The source search finds the rest by the table's name and the column's name, because a row variable names the table but not the column. A changed composite type uses `pg_depend` and the type's name the same way. Preflight repeats all of these on the live database.
>
> A hit can need more than a refresh. A `RETURNS TABLE (a int)` body over the changed column failed in every session, new ones too, and a refresh couldn't clear it, so only a rewrite fixes it. `ROW(1, 2)::c` after `ADD ATTRIBUTE` is the same. That's why a hit is a decision.
>
> `CREATE OR REPLACE VIEW` that adds a column keeps the view's OID, and `v vw[]` kept working, so it isn't a drop and recreate.
>
> Step 38 searches function source for a name, in whole words, in the affected schemas and in every function an affected table's triggers call.
>
> Step 12 builds `pooler = "transaction"` and the `pooler` tests.
>
> Update the earlier tests this changes.
>

## Out of scope

- Dependents the catalog shows (step 53)
- The online form (step 52), which runs this refresh at cutover
- Changing a composite type (step 57) and carrying its dependents (step 37)
- Changing a column's type and carrying what depends on it (steps 37 and 39)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Managed enums / Dependents

- **R-1038** **Code Postgres doesn't track.** Before any op, or its inverse, that changes a column's type, changes a composite type's attributes, drops and recreates a type or view, or renames or removes an enum label, step 38's source search also runs. It looks for the name of every type and view the op changes or replaces, including a domain or composite type, for each label that changes, and for each table and column whose type changes. For such a table, `pg_trigger` also gives its trigger functions and `pg_depend` gives every function with its row type or that type's array in the signature, and `pg_depend` does the same for a changed type. Every view in this rule includes a materialized view. A hit on a managed function is a decision, and `makemigrations` proposes the rewritten function. A `plpgsql` body can name a removed label, and a `s::text = 'old'` comparison never fails, it just stops matching.

- **R-1039** **Refreshing managed functions.** Every managed plpgsql function R-1038 finds, which includes every one an affected table's triggers call, is refreshed in that op's transaction. A trigger function needs this even when it never names the type: on Postgres 18 a trigger comparing `NEW.s` failed on every call in every open session until it was refreshed.

- **R-1040** **Unmanaged functions** R-1038 finds are a decision, repeated in preflight on the live database. The default refuses. Accepting says other services must reconnect, because morm doesn't change unmanaged functions. morm apps recover by themselves from a dropped type, and must reconnect too after any other change.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0168 | integration | **Stale plpgsql on Postgres 15–18:** a session that compiled `DECLARE v the_enum`, `SELECT ... INTO`, `%ROWTYPE` and a trigger function before a swap gets what proof 15 recorded with no refresh, and all four work with one. A session that compiled `DECLARE v the_composite[]` and `v := '{}'` in a function that names the type only in its body, so `pg_depend` has no row for it, fails on every call after a composite type recreate with no refresh, and works with one. So does one that names only a domain carried through that recreate. The same holds for an array of a view, a materialized view, a domain and a composite type that a column type change or an `ALTER TYPE` replaces, and of a view or materialized view that its own shape change drops and recreates, with a refresh in that op's transaction. On a plain column type change with no view, with `--fast` and again through the online swap's cutover, a trigger function comparing `NEW.n`, a function with a `%ROWTYPE` variable of the table comparing a field, and a function with the table only in its signature each fail on every call with no refresh, and work with one. So does a function with a variable of a composite type after `ALTER ATTRIBUTE ... TYPE`. A `RETURNS TABLE` function that no longer matches its column is a decision, because no refresh clears it. A `CREATE OR REPLACE VIEW` that keeps the view's OID runs no search. An unmanaged function like any of these stops `migrate` with exit code `2` until its decision is accepted, including a trigger function that names nothing the change touches. If a result contradicts this design, stop and ask the owner with the evidence and options. |
| T-0171 | integration | **Proof, PgBouncer:** in a `pooler` test, in transaction mode, an app recovers from a swap the way proof 16 recorded, with managed functions refreshed. If replacing the client connection can't clear stale server state and no refresh covers it, stop and ask the owner with the evidence and options. |
| T-0172 | integration | **Proof, hot standby:** a `read_only` pool on a standby recovers from a swap on the primary the way proof 17 recorded. If a result contradicts this design, stop and ask the owner with the evidence and options. Add a `ci.yml` job on Postgres 15–18 with a streaming standby. The test has its own Cargo feature with `required-features`, as `pooler` does. |
| T-0184 | suite | **Unmanaged functions:** a function that only names the type in its body stops `migrate` with exit code `2` and the list, changing nothing. Accepting the unmanaged function decision lets it run. |
