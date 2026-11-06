# Step 59: Managed enums: stale functions and standbys

- **Phase:** Migrations
- **Depends on:** 53, 12
- **Can run beside:** 08, 09, 19, 20, 21, 22, 23, 24, 25, 26, 27, 42, 43, 44, 45, 46, 47, 48
- **Crates:** `morm-migrate`, `.github`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Find the code Postgres doesn't track, keep managed functions working through a swap, a composite type recreate, or a column type change or `ALTER TYPE` that replaces a carried object, and prove apps recover behind PgBouncer and on a standby.

## Deliverables

- The source search before a swap, a rename, a removal or a composite type recreate, or a column type change or `ALTER TYPE` that replaces a carried object, function refresh in that transaction, the unmanaged function decision, and the `ci.yml` job for a hot standby

## End-to-end check

On Postgres 15–18, a swap under a plpgsql function that names a removed label proposes the rewritten function, every stale managed function works in open sessions afterwards, and an unmanaged function stops `migrate` until its decision is accepted. The same holds for a function that names a recreated composite type in its body, and for one that names a view carried through a column type change or an `ALTER TYPE`.

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
> Two more changes replace objects without being a swap or a recreate, and both come from step 37. Postgres refuses a column type change under a view, so step 37 drops every view above the column, changes the column and creates them again. `ALTER TYPE` keeps the type, but step 37 does the same for a view that names an attribute being dropped or retyped, and for a domain whose check names an attribute being dropped. Whatever depends on a dropped view goes with it, such as a domain or composite type over it.
>
> Each object created again gets a new OID, as in a recreate, so a function that names it only in its body goes stale the same way. On Postgres 18 and 17, a session that compiled `DECLARE v vw[]` and `v := '{}'` failed on every call with `cache lookup failed for type` after either change, for a view, a materialized view, a domain and a composite type. New sessions worked, and a refresh in the same transaction cleared it.
>
> So what triggers the search here is step 37's `pg_depend` walk for that change. If it drops and recreates a domain, composite type or view, the search runs for the name of each one and for no other name, because these changes have no labels and the table and the altered type keep their names and OIDs. Preflight takes the names from the same walk on the live database. R-1039 and R-1040 then apply unchanged, with the refresh in the transaction that creates the objects again. If the walk drops none of them, as with `ADD ATTRIBUTE` or retyping an attribute no view names, nothing here runs.
>
> The table under such a column type change is an affected table for R-1039. On Postgres 18 and 17, a trigger function comparing `NEW.n` failed on every call with `type of parameter ... does not match that when preparing the plan` until it was refreshed.
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
- Changing a column's type and carrying what depends on it (step 37)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Managed enums / Dependents

- **R-1038** **Code Postgres doesn't track.** Before a swap, a rename, a removal or a composite type recreate, step 38's source search also runs for the type's name, for every domain, composite type and view replaced along with it, for each label that changes, and for each affected table and column. Before a column type change or an `ALTER TYPE` that drops and recreates a domain, composite type or view, it runs for the names of those objects alone. Every view in this rule includes a materialized view. A hit on a managed function is a decision, and `makemigrations` proposes the rewritten function. A `plpgsql` body can name a removed label, and a `s::text = 'old'` comparison never fails, it just stops matching.

- **R-1039** **Refreshing managed functions.** Every managed plpgsql function that search finds, and every one an affected table's triggers call, is refreshed in that transaction. A trigger function needs this even when it never names the type: on Postgres 18 a trigger comparing `NEW.s` failed on every call in every open session until it was refreshed.

- **R-1040** **Unmanaged functions** that would go stale are a decision, repeated in preflight on the live database. The default refuses. Accepting says other services must reconnect, because morm doesn't change unmanaged functions. morm apps recover by themselves.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0168 | integration | **Stale plpgsql on Postgres 15–18:** a session that compiled `DECLARE v the_enum`, `SELECT ... INTO`, `%ROWTYPE` and a trigger function before a swap gets what proof 15 recorded with no refresh, and all four work with one. A session that compiled `DECLARE v the_composite[]` and `v := '{}'` in a function that names the type only in its body, so `pg_depend` has no row for it, fails on every call after a composite type recreate with no refresh, and works with one. So does one that names only a domain carried through that recreate. After a column type change and after an `ALTER TYPE` that drop and recreate a view, a materialized view, a domain and a composite type, a session that compiled the same body for any one of them fails the same way, and works with a refresh in that transaction. A trigger function comparing `NEW.n` on the table under that column type change fails on every call with no refresh, and works with one. An `ALTER TYPE` that drops nothing, such as `ADD ATTRIBUTE`, runs no search. An unmanaged function like any of these stops `migrate` with exit code `2` until its decision is accepted. If a result contradicts this design, stop and ask the owner with the evidence and options. |
| T-0171 | integration | **Proof, PgBouncer:** in a `pooler` test, in transaction mode, an app recovers from a swap the way proof 16 recorded, with managed functions refreshed. If replacing the client connection can't clear stale server state and no refresh covers it, stop and ask the owner with the evidence and options. |
| T-0172 | integration | **Proof, hot standby:** a `read_only` pool on a standby recovers from a swap on the primary the way proof 17 recorded. If a result contradicts this design, stop and ask the owner with the evidence and options. Add a `ci.yml` job on Postgres 15–18 with a streaming standby. The test has its own Cargo feature with `required-features`, as `pooler` does. |
| T-0184 | suite | **Unmanaged functions:** a function that only names the type in its body stops `migrate` with exit code `2` and the list, changing nothing. Accepting the unmanaged function decision lets it run. |
