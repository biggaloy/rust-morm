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

**R-1176** **Fingerprints of children and partitions.** Fingerprints are built from the reader's IR, so what a child or a partition holds from its parent counts once, on the parent, and a live tree matches `schema_before` and `schema_after`, built on scratch where every table is plain. A partition's model may declare `pk` on the key the partition holds, under step 29's rule for a partition's key, and fingerprints cover only what the chain manages, so a partition whose model declares it has that key in its fingerprint, reading the same as the key scratch builds on a plain table, and one whose model leaves it out has none. When a child's or a partition's model declares any other object its live table holds from its parent, such as a CHECK under the parent's name, `migrate` and `rollback` preflight refuse with exit code `2` before the first step, changing nothing, naming the table, the object and the parent's object it's held from, and saying a child's or a partition's model declares only its own, rather than reporting a fingerprint mismatch. A key a partition holds on its parent's key columns under another name, such as `p3_other` or `p5_pkey1`, is in its fingerprint under that name, so it doesn't match a declared `pk`, and the gate finds it as drift, the same difference `verify` reports. The gate's report then names the partition, the key under the name it has, the parent's key and the name `<partition>_pkey` its model gives it, and says to rename the key with `ALTER TABLE <partition> RENAME CONSTRAINT <name> TO <partition>_pkey`, which renames its index too, after renaming the relation that already has that name, which it names, as for `p5_pkey1`, rather than reporting only a fingerprint mismatch.

**R-1185** **A partition's `pk` that can't match its parent's key.** On Postgres 18.6 and 17.11, dropping a partitioned table's key dropped the key every partition held from it, which a partition alone can't drop, failing with `cannot drop inherited constraint`, and dropping it and adding it on other columns, or in another order, built each partition's key again as `<partition>_pkey` on the new columns, a key held as `p3_other` included. `ADD PRIMARY KEY` on a partitioned table whose partition had a key of its own on other columns failed with `multiple primary keys for table "q2" are not allowed`. So a partition's declared `pk` that can't match the key it holds from its parent, under step 29's rule for a partition's key, would fail the gate or a step partway. `migrate` and `rollback` preflight read `pg_inherits`, `pg_class` and `pg_constraint` on the live database for every partition a model declares, and refuse with exit code `2` before the first step, changing nothing, naming the partition, the key its model declares, by its columns, and its parent's key, by name and columns, or saying the parent has none, in each of these cases, in the direction the run goes:

1. The partition's model declares `pk` on columns other than its parent's key's, or in another order, in the schema the run starts from, where the parent's key is the live one, or in the one it ends at, where it's the key the parent's model gives.
2. The migration drops its parent's key and the partition's model declares `pk` after it, on any columns, or changes the columns of its parent's key and the partition's model keeps `pk` on the old ones. Postgres drops the partition's key with the parent's, or builds it again on the new columns.
3. The migration gives its parent a key while the partition has a key of its own on other columns, whether its model declares it or not, and even when the same migration takes it out of the model, because Postgres refuses the parent's `ADD PRIMARY KEY` while it's there.

The message says how to go on: declare `pk` on the parent's key columns, in their order, or leave it out; in the migration that drops or changes the parent's key, take `pk` out of each partition's model, or change it the same way; and take a partition's own key on other columns out in an earlier migration than the one that gives its parent a key. A partition's model that changes `pk` the same way as its parent's, in the same migration, matches before and after it, under step 39's rule for a partition's `pk`. Where the parent has no key, before the migration and after it, a partition's `pk` is its own, compared as on any table. `makemigrations` can't see any of these, because the models don't say a table is a partition and scratch has none, so the first refusal comes when a command reaches the live database. `rehearse` runs this preflight too.

**R-1186** **Identity on a partition's model.** A partition takes its parent's identity, under step 29's rule for a partition's identity, and Postgres refuses identity of its own on a partition: `ATTACH PARTITION` of a table with an identity column, and adding identity to a partition's column, both fail. Scratch would still build one from a partition's model with an identity column, such as one on `base = "common"` or `"pk_only"`, which add `id` as identity, or with a field marked `identity`. So `migrate` and `rollback` preflight refuse with exit code `2` before the first step, changing nothing, when a model declares identity on a column of a table that's a partition on the live database, in the schema the run starts from or the one it ends at, naming the partition, the column, and the parent's column whose identity it holds, or saying the parent's column has none. The message says to declare the partition's model with `base = "bare"`, with `pk` on the parent's key columns where the parent has a key, and no identity, because the partition's column already takes its values from the parent's identity. The models don't say a table is a partition, so the macro can't catch this at compile time, and `makemigrations` can't see it on scratch, which has no partitions. `rehearse` runs this preflight too.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0121 | suite | **Concurrency:** rival migrators, migration under live traffic, lock-timeout retry. |
| T-0124 | suite | **Multi-step:** a fingerprint mismatch after the last step marks the migration `failed`. |
| T-0211 | suite | **`--decide` on `migrate`:** on a real database, through a test-only op whose preflight opens a decision about a function, `migrate` exits with code `3`, printing the ID, evidence and options, `--json` gives the same, `--yes` doesn't answer it, and nothing has run. `--decide` with the printed ID lets it through, with one row in `_morm.decisions` holding the migration id, checksum, direction `up`, decision ID and option, and one row in `_morm.history`. A decision `meta.json` has answered for `up` opens nothing and adds no row. With two decisions open, a run answering one exits with code `3`, and a later run answering only the other goes through. A run whose first step fails after its answer is recorded keeps the answer, and a rerun with no `--decide` opens nothing. Editing the pending migration's `up.sql` or `meta.json` cancels the answer, and changing the function gives a new ID, and each stops with code `3`. An answer that refuses exits with code `2`, and a later accepting answer replaces its row, leaving two history rows. An answer to the second migration in a chain, given on a run that starts at the first, is used when that migration's preflight opens it. An ID no migration opened is reported and recorded nowhere, an option the decision doesn't offer exits with code `1`, and `--plan` records nothing. After `migrate`, `_morm.migrations` holds `meta.json` byte for byte. |
| T-0261 | suite | **Fingerprints of children and partitions, on Postgres 15–18:** on a partitioned table and a three-level inheritance tree, each table made by the chain and attached by hand, a migration adding a CHECK, a foreign key and an index to the top parents, every child and partition declared, passes the gate before and after it, and so does its `rollback`. The same migration passes the gate before and after it, and so does its `rollback`, with `pk` declared on the parent's key columns in the models of a partition made by `PARTITION OF` and of one whose own `<partition>_pkey` `ATTACH PARTITION` took over. A child's model that declares its parent's CHECK under the parent's name stops `migrate` and `rollback` with exit code `2` before the first step, naming the table, the object and the parent's object, with nothing changed. |
| T-0263 | suite | **A partition's `pk` that can't match its parent's key, on Postgres 15–18:** on a partitioned table with a key on two columns and partitions made by `PARTITION OF`, each declared by a model, `migrate` and `rollback` each stop with exit code `2` before the first step, naming the partition, its declared key and the parent's key, with nothing changed, in each of these cases: a partition's model with `pk` on one of the parent's key columns, and one with them in another order; a migration that drops the parent's key while a partition's model keeps `pk`; and one that changes the parent's key's columns while a partition's model keeps `pk` on the old ones. On a partitioned table with no key, a migration written with `makemigrations --fast` that gives it one stops the same way while a partition has a key of its own on other columns, declared by its model, and while the same migration takes that key out of the model. The same migrations with each partition's model leaving `pk` out, or changing it the same way as the parent's, pass the gate before and after them, and so do their `rollback`s. A partition holding its parent's key as `p3_other`, and one holding it as `p5_pkey1` beside a table named `p5_pkey`, each with `pk` in its model, stop `migrate` at the gate before the first step, with nothing changed, with a report naming the partition, the key under its name, the parent's key and `<partition>_pkey`, and giving the rename, with the table `p5_pkey` to rename first. After those renames the run passes the gate. |
| T-0264 | suite | **Identity on a partition's model, on Postgres 15–18:** on a partitioned table whose key column is an identity column, with partitions made by `PARTITION OF`, each declared by a model, a partition's model on `base = "common"`, one on `base = "pk_only"` and one with a field marked `identity` each stop `migrate` and `rollback` with exit code `2` before the first step, with nothing changed, naming the partition, the column and the parent's identity column, and saying to use `base = "bare"` with `pk`. On a partitioned table whose column has no identity, the same stop says the parent's column has none. With each partition's model on `base = "bare"` and `pk` on the parent's key columns, the same migration passes the gate before and after it, and so does its `rollback`. |
