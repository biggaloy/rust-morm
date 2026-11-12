# Step 34: Rollback and crash recovery

- **Phase:** Migrations
- **Depends on:** 33
- **Can run beside:** 08, 09, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 47
- **Crates:** `morm-migrate`, `morm-admin`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

A defined way forward from any failure.

## Deliverables

- `rollback` with `--failed` and `--orphans` and its preflight, `--decide` on `rollback`, `--resume`, batch cursors, `repair`, `verify`, `refingerprint`, Postgres version handling

## End-to-end check

Killing the process at every step boundary, `--resume` and `rollback` both reach the right state, and a tampered migration is refused. A decision `rollback` opens stops it with exit code `3`, with the migration's files gone too, and an answer from `migrate` doesn't cover it.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> ```
> makemigrations [--plan] [--json] [-n name] [--decide ID=OPT] [--decisions F] [--empty] [--fast]
> migrate [--plan] [--json] [--yes] [--to ID] [--resume] [--phase expand|contract] [--decide ID=OPT]
> rollback [--last N | --to ID | --orphans | --failed] [--plan] [--yes] [--accept-data-loss] [--production --reason R] [--phase contract] [--fast] [--decide ID=OPT]
> revert ID [--plan] [--decide ID=OPT]    mode [development --confirm NAME | production]    squash --to ID
> status [--json]    verify [--json]    check    lint    check-sql
> rehearse --url URL [--decide ID=OPT]    purge --older-than D | --now ID    baseline    rebase-chain
> repair --reason R     refingerprint           sql ID
> ```
>
> Step 33 owns `_morm.decisions` and the rules for `--decide`. An answer stands only for the same migration id, checksum, direction and decision ID. It's committed with a history row as soon as its decision opens, an answer that refuses exits with code `2`, an unused ID is reported and recorded nowhere, an option the decision doesn't offer exits with code `1`, and `--plan` records nothing. Preflight runs before each migration, and an op can add its own checks to it. The ledger keeps `meta.json` verbatim.
>

## Out of scope

- Production guards (step 40)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Production is forward only

- **R-0584** **A failed migration can always be cleaned up.** `migrate --resume` is the first choice. `rollback --failed` undoes the completed steps of a migration that never finished, because no release depends on it yet.

### Migrations / Runtime guarantees

- **R-0601** **Fingerprint gate:** rollback checks the fingerprints in reverse.
- **R-0605** **Postgres versions.** Catalog text can differ between majors, so `pg_major` is recorded. On a mismatch, the expected state is rebuilt by replaying the chain on the target's version in a temporary database. Without `CREATEDB` it refuses unless `--skip-fingerprint` is given. `refingerprint` handles upgrades.
- **R-0609** **Crash recovery:** preconditions make steps safe to re-run. `status` reports the exact step, `migrate --resume` continues, `rollback` undoes only completed steps, failed concurrent indexes are rebuilt, and batches resume from their cursor.
- **R-0610** **Tamper refusal:** edited or missing applied migrations are refused. `repair --reason "..."` re-signs and logs it.
- **R-0611** **LIFO rollback,** plus `rollback --orphans` for another branch's migrations, on development databases.

### Migrations / Commands

- **R-0617** `verify`: live schema vs models and ledger; read-only and safe in production. Unmanaged objects are listed as information, not as drift.
- **R-0951** `rollback [--last N | --to ID | --orphans | --failed] [--plan] [--yes] [--accept-data-loss] [--production --reason R] [--phase contract] [--fast] [--decide ID=OPT]`, where `--decide` answers a decision preflight opens, and `--phase contract` and `--fast` change nothing `rollback` runs: they only let through the silent narrowings step 41 holds. `verify [--json]`. `repair --reason R`. `refingerprint`.
- **R-1098** **Answering a decision at `rollback`.** `rollback` runs step 33's preflight, with each op's own checks, before each migration it undoes, in direction `down`, in every form: `--last`, `--to`, `--orphans` and `--failed`. It reads `meta.json` from the ledger, so the search and the checks work when the files are gone. A decision it opens stops it with exit code `3`, and `rollback --decide ID=OPT` answers it, under step 33's rules for `--decide`, recorded in `_morm.decisions` with direction `down` and a history row. So an `up` answer from `migrate` doesn't cover `rollback`, and a `down` answer doesn't cover `migrate`, even with the same decision ID. An answer given to `migrate` survives `migrate --resume`, and one given to `rollback` survives a rerun of a `rollback` that was killed.
- **R-0960** **`full-chain` job.** This step sets what `nightly.yml`'s `full-chain` job runs and changes the job to run it: all up, all down, all up, across the migrations this step can run, on Postgres 15–18. Until now the job ran the same command as CI's migration suite, so the result must go beyond that suite: a job that runs only what that suite runs doesn't satisfy this. Step 49 widens it to the whole chain later. Gate the test with a Cargo feature on `morm-tests` and `required-features`, as `pooler` is. If the migration suite's command uses `--all-features` on `morm-tests`, change it so this feature stays off. Tag the test the job runs and the job itself, update the line in `tests/tests/repo_policy.rs` that pins the old command, and link the job's log from a `workflow_dispatch` run in the PR.

**R-1177** **`verify` on children and partitions.** `verify` reads the live schema through the reader's IR, so what a child or a partition holds from its parent isn't drift on that table and is checked once, on the parent. A child's or a partition's model that declares an object its table holds from its parent is reported as drift, naming the parent's object and saying the child's model declares only its own. A partition's `pk` that matches the key it holds, under step 29's rule for a partition's key, isn't drift, and neither is a partition's model with no `pk`, while a declared `pk` that differs from that key is drift on that table, named with the key the partition holds and the parent's key. A key the partition holds on its parent's key columns under another name, such as `p3_other` or `p5_pkey1`, is drift the same way when its model declares `pk`, and the report names the partition, the key under the name it has, the parent's key and `<partition>_pkey`, and says to rename it with `ALTER TABLE <partition> RENAME CONSTRAINT <name> TO <partition>_pkey`, which renames its index too, after renaming the relation that already has that name, which it names, as for `p5_pkey1`, the same report the fingerprint gate gives. Identity a partition's column holds from its parent, under step 29's rule for a partition's identity, isn't drift on the partition, and a partition's model that declares identity is drift on that table, naming the parent's column whose identity it holds and saying to use `base = "bare"` with `pk`. A CHECK Postgres kept on an inheritance child when the parent's was dropped is listed as unmanaged, like any object no model declares.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0117 | suite | **Fault injection:** kill the process at every step boundary, then prove `--resume` and `rollback` both reach the right state. |
| T-0136 | suite | **Fault injection runs in CI:** the `Migration suite` step of `ci.yml`'s `integration` job runs the T-0117 tests, and the PR links that step's log showing them pass. A run of zero tests doesn't count. At step 01 it ran `cargo test --locked -p morm-migrate --all-features`, which doesn't reach `tests/` (package `morm-tests`). If its command doesn't reach these tests, change it and update the line in `tests/tests/repo_policy.rs` that pins it. |
| T-0212 | suite | **`--decide` on `rollback`:** through step 33's test-only op, after `migrate --decide` with an accepting answer, `rollback` exits with code `3`, printing the same decision ID, with nothing undone, and `rollback --decide` lets it through, with a row in `_morm.decisions` for direction `down` and a history row. That row doesn't cover `migrate` of a second migration with a new ID, and the earlier `up` row still covers the same migration on the way back up. All of this holds with the migration's files deleted, reading `meta.json` from the ledger, and for `rollback --failed` and `--to`. Fault injection: killed at each step boundary after an answer, `migrate --resume` and a rerun of `rollback` each go on with no `--decide`, and with the function changed in between, each stops with code `3` on a new ID. |
| T-0266 | integration | **`verify` on a partition's key and identity, on Postgres 15–18:** on a partitioned table with a key on an identity column, and partitions declared by models on `base = "bare"` with `pk` on the parent's key columns, one made by `PARTITION OF`, one whose own key `ATTACH PARTITION` took over as `p3_other`, and one holding it as `p5_pkey1` beside a table named `p5_pkey`, `verify` reports no drift on the first and drift on the others, naming the partition, the key under its name, the parent's key and `<partition>_pkey`, and giving the rename, with the table `p5_pkey` to rename first. After those renames, `verify` reports none. A partition's model with `pk` on other columns, and one on `base = "common"`, are each drift on that table, naming the parent's key or its identity column. |
