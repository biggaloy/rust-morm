# Step 57: Managed objects: enums, domains, composite types and sequences

- **Phase:** Migrations
- **Depends on:** 36
- **Can run beside:** 08, 09, 19, 20, 21, 22, 23, 24, 25, 26, 27, 47
- **Crates:** `morm-migrate`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Read managed types and sequences from the catalog, diff them, and apply them with an exact inverse.

## Deliverables

- IR, catalog reading, diff and `Op` with inverse for each kind: enums (create, drop and rename a value), domains, composite types, sequences
- Changing a domain or composite type

## End-to-end check

Each of these kinds goes from a declaration to a real database and back through the diff with no drift, on Postgres 15–18.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> Step 35 declares these types in the manifest. Step 36 builds the IR, catalog reading, diff and `Op` for the other object kinds, in the same shape.
>
> Step 29 reads enum labels from `pg_enum`, ordered by `enumsortorder`. Step 32's with-data proof keeps its synthetic rows in a table keyed by column type.
>

## Out of scope

- Dependency recreation (step 37)
- Adding and reordering enum values (step 51), and removing them (step 58)
- Domains and composite types built on a swapped enum (step 54)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Triggers, functions and other objects

- **R-0966** **Sequences:** a sequence's catalog comparison uses its definition only (type, start, increment, minimum, maximum, cache and cycle) and never `last_value`, so a sequence in use shows no drift. A sequence that belongs to an identity column stays out of this kind, because it comes with its column.
- **R-0967** Dropping a sequence loses its position, because the inverse recreates it from its definition. That op's data effect is `lossy`, and its description says the position is lost.

### Managed enums

- **R-0991** **Enums in this step** are created with their labels in variant order, dropped, and have values renamed. `ALTER TYPE ... RENAME VALUE` is instant, takes no table lock, and its inverse renames back, with data effect `none`. Renames run before any other change to the type, so a new value can reuse an old label. Exchanging two labels goes through a temporary label in one transaction.
- **R-0992** **Rename or not.** A label that's gone and a label that's new on the same type are a rename decision, unless `was` answers it. It's never guessed. Rust variants alone can't show which it was.
- **R-0993** Any other change to an enum's values is refused by the diff with exit code `2`. Step 51 replaces this refusal for adds and reorders, and step 58 for removals.
- **R-1059** **Changing a domain or composite type.** A changed domain `check` drops and adds the constraint `NOT VALID`, then runs `VALIDATE`. A composite attribute is added or dropped with `ALTER TYPE` when no column uses the type. A changed base type, or a change to a composite a column uses, is refused with exit code `2`. The with-data proof leaves out domain columns whose `check` its rows fail.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0145 | integration | **Sequences:** a sequence shows no drift after `nextval` calls, an identity column's sequence isn't read as an object of this kind, and dropping a sequence reports `lossy`. |
| T-0155 | integration | **Enum renames:** `was` and a decision each rename a value with rows holding it, the inverse restores it, two labels exchange, an open decision exits with code `3`, and adding a value exits with code `2` until step 51, which rewrites this case. |
| T-0185 | integration | **Domain and composite changes:** with rows, a changed domain `check` replaces the constraint and validates it, a row that fails it stops at `VALIDATE`, and a composite attribute is added and dropped when no column uses the type, each with its inverse. A changed base type and a change to a composite a column uses each exit with code `2` and change nothing, and the with-data proof leaves out a domain column whose `check` its rows fail. |
