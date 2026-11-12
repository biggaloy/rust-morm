# Step 33: Migration execution and recovery

- **Phase:** Migrations
- **Depends on:** 32
- **Can run beside:** 08, 09, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 47
- **Crates:** `morm-migrate`, `morm-admin`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Run migrations safely, with a defined way forward from any failure.

## Deliverables

- `_morm` ledger, `_morm.decisions`, `migrate` with `--decide`, `status`, `check`, `sql`, fingerprint gates, advisory lock, lock safety, preflight

## End-to-end check

A chain of migrations runs on a real database, each gate refuses when its condition is forced, and two rival migrators can't both run. A decision only preflight can find stops `migrate` with exit code `3`, and `--decide` lets it through.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> **How it fits together:** `morm-admin` holds the engine. When a command needs the models, it builds and runs `morm-mgr` to get the manifest. Commands that don't need models (`migrate`, `rollback`, `status`, `purge`) run from `morm-migrations/` alone, so a production host needs `morm-admin` and no Rust toolchain. The app binary carries no migration code or schema metadata, apart from the short per-model column list `check_schema` reads.
>
> Step 32 builds a decision's ID, with the same code for `makemigrations` and preflight: the migration id, kind and object signature, plus a hash of its source. `meta.json` records each answer with its direction, `up` or `down`.
>

## Out of scope

- Recovery (step 34)
- Safe forms (step 39)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Files and ledger

- **R-0597** `_morm.migrations`: what's applied, with exact up and down SQL and `meta.json` verbatim, so rollback works when files are gone, including the checks it reads from `meta.json`.
- **R-0598** `_morm.steps`: per-step state (`pending/running/done/failed`) and batch cursors.
- **R-0599** `_morm.history`: append-only log of every action, with user, host, duration and tool version.
- **R-1096** `_morm.decisions`: each answer given to `migrate` or `rollback`, one row per migration id, checksum, direction and decision ID, with the option chosen. A pending migration has no `_morm.migrations` row, so this table doesn't depend on one. Each answer also gets a row in `_morm.history`.

### Migrations / Runtime guarantees

- **R-0600** **Fingerprint gate.** Fingerprints cover only objects the chain manages, so unmanaged objects can't fail a run. Live schema must match `schema_before`, or the run refuses. A single-transaction migration must match `schema_after` before commit, or it rolls back. A multi-step migration can't, because its steps commit separately, so the check runs after the last step, and a mismatch marks it `failed` for `rollback --failed`.
- **R-0606** **One migrator:** session advisory lock on a dedicated connection. If that connection is lost, the run stops.
- **R-0607** **Lock safety:** `lock_timeout` on every step with bounded retry and backoff, plus per-step `statement_timeout`.
- **R-0608** **Preflight:** privileges, long-running transactions, invalid indexes, checksums, table sizes. An op can add preflight checks of its own, which run with these and can refuse the run with exit code `2`, or stop it with code `3` when what they found is an open decision. A run where both happen exits with code `2`.

### Migrations / Commands

**R-0614** All are `morm-admin mgr <command>`:

**R-0615**

```
makemigrations [--plan] [--json] [-n name] [--decide ID=OPT] [--decisions F] [--empty] [--fast]
migrate [--plan] [--json] [--yes] [--to ID] [--resume] [--phase expand|contract] [--decide ID=OPT]
rollback [--last N | --to ID | --orphans | --failed] [--plan] [--yes] [--accept-data-loss] [--production --reason R] [--phase contract] [--fast] [--decide ID=OPT]
revert ID [--plan] [--decide ID=OPT]    mode [development --confirm NAME | production]    squash --to ID
status [--json]    verify [--json]    check    lint    check-sql
rehearse --url URL [--decide ID=OPT]    purge --older-than D | --now ID    baseline    rebase-chain
repair --reason R     refingerprint           sql ID
```

- **R-0616** `check`: offline CI check of chain, checksums, models.

**R-0623** **Run migrations as a deploy step,** before new code goes live, so a failed migration stops the deploy. The app binary has no migration code, so it can't migrate on start.

- **R-0950** `migrate [--plan] [--json] [--yes] [--to ID] [--resume] [--phase expand|contract] [--decide ID=OPT]`, where `--yes` skips the prompt, `--to ID` stops after that migration, and `--decide` answers a decision preflight opens. `status [--json]`. `check`. `sql ID` prints a migration's SQL.

- **R-1097** **Answering a decision at `migrate`.** Some decisions only appear against the live database, in preflight, because scratch can't see what they're about. `migrate` takes `--decide ID=OPT` for them, in the same form as `makemigrations`, once per answer. It never prompts for a decision, `--yes` answers none, and each open one is printed with its ID, evidence and options, in `--json` too, before the run stops with exit code `3`. An answer stands only for the same migration id, checksum, direction and decision ID, so an `up` answer doesn't cover `rollback`, and editing the migration cancels it. A decision is open unless `meta.json` holds an answer to it for this direction, or `_morm.decisions` holds a row matching all four. An answer is committed to `_morm.decisions`, with its history row, as soon as its decision opens, while the run holds the advisory lock. So it survives a failed or killed run, a rerun or step 34's `migrate --resume` doesn't need it again unless the decision's ID has changed, as step 32 says of a deparsed body, and a run that stops on a second decision keeps the first answer. A new answer to the same decision replaces the row and gets its own history row. An answer that refuses stops the run with exit code `2`, because the decision is answered and the guard holds. Preflight runs before each migration, so an answer to a later migration in the run waits for it. An ID no migration in the run opened is reported and recorded nowhere, and an option the decision doesn't offer stops the run with exit code `1`, with nothing recorded. `--plan` shows what's open and records nothing.

**R-1176** **Fingerprints of children and partitions.** Fingerprints are built from the reader's IR, so what a child or a partition holds from its parent counts once, on the parent, and a live tree matches `schema_before` and `schema_after`, built on scratch where every table is plain. When a child's or a partition's model declares an object its live table holds from its parent, such as a CHECK under the parent's name, or a partition's primary key once `ATTACH PARTITION` has taken it over, `migrate` and `rollback` preflight refuse with exit code `2` before the first step, changing nothing, naming the table, the object and the parent's object it's held from, and saying a child's or a partition's model declares only its own, rather than reporting a fingerprint mismatch.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0121 | suite | **Concurrency:** rival migrators, migration under live traffic, lock-timeout retry. |
| T-0124 | suite | **Multi-step:** a fingerprint mismatch after the last step marks the migration `failed`. |
| T-0211 | suite | **`--decide` on `migrate`:** on a real database, through a test-only op whose preflight opens a decision about a function, `migrate` exits with code `3`, printing the ID, evidence and options, `--json` gives the same, `--yes` doesn't answer it, and nothing has run. `--decide` with the printed ID lets it through, with one row in `_morm.decisions` holding the migration id, checksum, direction `up`, decision ID and option, and one row in `_morm.history`. A decision `meta.json` has answered for `up` opens nothing and adds no row. With two decisions open, a run answering one exits with code `3`, and a later run answering only the other goes through. A run whose first step fails after its answer is recorded keeps the answer, and a rerun with no `--decide` opens nothing. Editing the pending migration's `up.sql` or `meta.json` cancels the answer, and changing the function gives a new ID, and each stops with code `3`. An answer that refuses exits with code `2`, and a later accepting answer replaces its row, leaving two history rows. An answer to the second migration in a chain, given on a run that starts at the first, is used when that migration's preflight opens it. An ID no migration opened is reported and recorded nowhere, an option the decision doesn't offer exits with code `1`, and `--plan` records nothing. After `migrate`, `_morm.migrations` holds `meta.json` byte for byte. |
| T-0261 | suite | **Fingerprints of children and partitions, on Postgres 15–18:** on a partitioned table and a three-level inheritance tree, each table made by the chain and attached by hand, a migration adding a CHECK, a foreign key and an index to the top parents, every child and partition declared, passes the gate before and after it, and so does its `rollback`. A partition's model that declares a primary key `ATTACH PARTITION` took over, and a child's model that declares its parent's CHECK under the parent's name, each stop `migrate` and `rollback` with exit code `2` before the first step, naming the table, the object and the parent's object, with nothing changed. |
