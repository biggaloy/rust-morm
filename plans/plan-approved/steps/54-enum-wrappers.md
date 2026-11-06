# Step 54: Managed enums: domains and composite types

- **Phase:** Migrations
- **Depends on:** 53
- **Can run beside:** 08, 09, 19, 20, 21, 22, 23, 24, 25, 26, 27, 42, 43, 44, 45, 46, 47, 48
- **Crates:** `morm-migrate`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Carry a swap through managed domains and composite types built on the enum.

## Deliverables

- Swapping domains, composite types and arrays of them with the enum, maps through them, and their inverses

## End-to-end check

On Postgres 15–18, a swap with a map on an enum under a domain, a composite type and an array of each runs and reverses exactly, with populated columns.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> Step 51 swaps the enum, keeps old values and copies the owner, comment and grants. Step 53 walks dependents and refuses anything unmanaged, including every unmanaged domain or composite type.
>
> On a composite type no column uses, `ALTER TYPE ... ALTER ATTRIBUTE ... TYPE` works. If a column uses it, Postgres refuses, and `CASCADE` doesn't help.
>

## Out of scope

- Online swaps through these types: step 52 refuses them

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Managed enums / Domains and composite types

- **R-1041** **A managed domain over the enum gets a new type too.** Postgres has no command to change a domain's base type. In the swap's transaction the domain is renamed, created again over the new enum from its declaration, with its `check`, and its columns change to it. It's dropped before the old enum.

- **R-1042** **A managed composite type holding the enum** changes with `ALTER ATTRIBUTE` when no column uses it. Otherwise it's renamed and created again like a domain, and its columns change to it through a text cast, which preserves every attribute.

- **R-1043** **Order.** New types are created from the enum outward, and old types are dropped from the outside in. This holds at any depth, such as a domain over a domain, or a composite holding an array of the enum.

- **R-1044** **Maps pass through.** A map applies inside a domain value, a composite attribute and each array element, leaving every other attribute unchanged, and the retired copy of a mapped column holds the whole old value as text.

- **R-1045** These types keep their owner, comment and grants the way the enum does, and their inverses and data effects are the enum op's own.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0173 | suite | **Domains and composites:** with rows, on Postgres 15–18: a domain, a composite with and without a using column, arrays of each, and a nested chain, each with no map, with a map and through down. A map inside a composite wasn't run in the design's tests. If a result contradicts this design, stop and ask the owner with the evidence and options. |
| T-0174 | suite | **Fault injection:** kill the process during a swap through a domain and a composite, and every type and column is as it was. These tests run in CI's `Migration suite` step, and the PR links that step's log. |
