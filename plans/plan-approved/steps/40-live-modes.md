# Step 40: Production mode, revert and squash

- **Phase:** Migrations
- **Depends on:** 60
- **Can run beside:** 08, 09, 19, 20, 21, 22, 23, 24, 25, 26, 27, 43, 44, 45, 46, 47, 48
- **Crates:** `morm-migrate`, `morm-admin`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Forward-only production, with safe ways to undo and tidy up.

## Deliverables

- `mode` with `--confirm`, `rollback` guards and the emergency override, `revert`, `rehearse` with `--decide`, `squash`

## End-to-end check

A new database refuses `rollback`, `revert` restores retired data through a forward migration, and a squashed chain gives the same schema as the full one.

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
> Step 33 owns `_morm.decisions` and the rules for `--decide`. An answer stands only for the same migration id, checksum, direction and decision ID. It's committed with a history row as soon as its decision opens, an answer that refuses exits with code `2`, an unused ID is reported and recorded nowhere, and an option the decision doesn't offer exits with code `1`. Preflight runs before each migration, an op can add its own checks to it, and step 33's tests use a test-only op whose preflight opens a decision about a function. Step 59 comes before this step: a change to a column's type, length or precision opens one decision in preflight, listing everything its walk finds on that database, including unmanaged functions using `EXECUTE`. `rehearse` gets that by running the same preflight.
>
> Step 60 comes before this step. A type change within one family on a key that foreign keys reference runs online in either direction, with every referencing column and the identity sequence, and its inverse is refused with exit code `2` once a value doesn't fit the old type.
>
> Update the earlier tests this changes: a test that runs `rollback` or `rehearse` marks its database with `mode development --confirm NAME` first, including the `full-chain` test `nightly.yml` runs, which CI doesn't.
>

## Out of scope

- Rolling deploys (step 41)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / Production is forward only

**R-0578** Rolling back after new data has been written can fail or lose that data, so production moves forward only. `down.sql` is still proven for every migration, because `revert` and development depend on it.

- **R-0579** **`mode development` must name the database,** as in `mgr mode development --confirm app_staging`, so a command aimed at the wrong URL can't remove the protection.
- **R-0580** **Every database is production until marked otherwise.** The mode is stored in `_morm`, not in an environment variable, so a wrong setting on a host can't turn it off, and a backup restores with it. Forgetting to mark a database leaves it safe.
- **R-0581** `morm-admin mgr mode development` marks a local or staging database, and `mode` shows the current value. Every change goes in `_morm.history`.
- **R-0582** **On a production database,** `rollback` and `rehearse` exit with code `2`. To rehearse on a clone of production, mark the clone first.
- **R-0583** **`morm-admin mgr revert ID`** is the way to undo in production. It creates a new forward migration from that migration's down, runs the full proof on scratch, and needs the same decisions, such as `down_fill`. Retired columns and tables come back with their data, and the chain stays linear, so every database has the same history. The file names the reverted migration's retired objects unsuffixed, and each database maps them, when the file runs, to the names its applied run of that migration gave them, as step 52 says, so a migration run twice there is reverted from its second run's retired objects.
- **R-0585** **Emergency override:** `rollback --production --reason "..."` works but is logged to `_morm.history`, and it still needs `--accept-data-loss` where that applies.

### Migrations / Commands

- **R-0618** `rehearse`: pending up → down → up on a clone or restored backup, with **real data**. The recommended last gate. Each pass maps retired names as step 52 says, so the last `up`, a re-run before `purge` whose retired objects would collide with the first pass's, gives its own retired objects its run's suffix.
- **R-0622** `squash --to ID`: replaces the chain up to `ID` with one baseline generated by replaying it. A database that has applied all of them just continues, and one partway through is refused until it catches up. Run it only when every database has passed `ID`. It refuses while retired objects from those migrations haven't been purged, because their rollback path would be gone.
- **R-0952** `revert ID [--plan] [--decide ID=OPT]`. `mode [development --confirm NAME | production]`. `squash --to ID`. `rehearse --url URL [--decide ID=OPT]`.
- **R-1101** **Answering a decision at `rehearse`.** `rehearse` runs step 33's preflight, with each op's own checks, before each migration in each of its three passes, in direction `up`, then `down`, then `up`. A decision it opens stops it with exit code `3`, printing the ID, evidence and options, and `rehearse --decide ID=OPT` answers it, once per answer, in the same form as `migrate` and `rollback` and under step 33's rules for `--decide`, recorded in that database's `_morm.decisions` with the pass's direction and a history row. An ID open in more than one pass is answered for each pass of that run that opens it, with a row for each direction. A stop leaves the database where it stopped, so a stop in the `down` pass has its `up` pass already applied. So `rehearse` records each pass and migration it finishes in `_morm.history`, and a rerun on that database carries on from where it stopped, runs nothing twice, and can't report a pass because nothing is pending. Answers stay on the database they were given on. They don't answer `migrate` anywhere else, so production is asked for its own, and a restored backup holds none given after it was taken, so `rehearse` asks for those there. A production database still exits with code `2` first, with nothing recorded.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0762 | Rolling back a production database | Refused, exit code `2`, unless marked `development`; `revert` is the forward path |

### Managed enums

- **R-0996** **Reverting an enum change.** `revert` plans its forward migration like any other change, so reverting an add removes the value through the online swap by default, and needs the same decisions. Retired values come back with their rows, read from the retired column of the run the ledger records as applied, through step 52's mapping of retired names. `rollback` uses `down.sql`, which is always the one-transaction form.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0125 | suite | **Forward only:** `rollback` and `rehearse` refused on production mode, the default on a new database, and `revert` restoring retired data. |
| T-0217 | suite | **`--decide` on `rehearse`:** on a restored backup marked `development`, with no row in `_morm.decisions` for the pending migration, through step 33's test-only op, `rehearse` exits with code `3`, printing the ID, evidence and options, with nothing run, and `rehearse --decide` with that ID lets it through, with a row for direction `up` and a history row. A decision that opens only in the `down` pass stops it there with exit code `3` and the `up` pass applied. A rerun with no answer stops the same way and doesn't report a pass, and a rerun with the answer carries on from that pass, runs nothing twice and finishes the last pass, with a row for direction `down`. An ID open in both directions, given once, is answered for both, with a row for each. An answer that refuses exits with code `2`, an ID no migration opened is reported and recorded nowhere, and an option the decision doesn't offer exits with code `1`. A killed `rehearse` keeps its answers. With an unmanaged function using `EXECUTE` on the database, a column type change stops `rehearse` on step 59's one list until it's answered, for each direction. `migrate` of the same migration on another database still stops with code `3`, so an answer stays where it was given. On a database in production mode, `rehearse --decide` exits with code `2` and records nothing. |
| T-0126 | suite | **Squash:** a squashed chain gives the same schema as the full chain, and a database partway through is refused. |
| T-0157 | suite | **Enum revert:** `revert` of an add runs the online swap and stops on rows holding the value unless a map is decided, `revert` of a remove restores retired rows, and `rollback` on a production database is refused. `revert` of an online enum swap that was run, rolled back and run again, with rows changed and inserted between the runs, restores each row from the second run's retired column; and `rehearse` over an online enum swap passes, its last `up` giving its retired objects its run's suffix while the migration's files keep their checksums. |
| T-0236 | suite | **Reverting a key widening, on Postgres 15–18:** on a database in production mode, `revert` of a migration that widened an `integer` identity primary key referenced by two tables to `bigint` writes a forward migration that runs online in step 60's form under live writers, with no failed app transaction and no lost write. It ends on the catalog the widening started from, apart from retired columns `purge` drops, with the original columns back at their positions and the identity's position carried. Once a key value or the sequence's position passes `2^31 - 1`, `migrate` of that forward migration is refused with exit code `2` before its first step, naming the table, the column and the value, with nothing changed. |
