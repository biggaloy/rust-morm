# Online widening of a referenced key: design proposal

For the owner's review. Design only: nothing in `plans/` is edited, committed or pushed. Tested on throwaway Postgres 18.6 and 17.11 servers in this worktree's `.claude/scratch/`. Postgres 15 and 16 aren't installed here, so I couldn't run them (section 6).

## 1. The recommended design

**What it covers.** A type change on a column that a foreign key references, where the table has to be rewritten and the old and new types are in the same R-0612 family. The main case is `smallint`, `integer` or `bigint` widened, such as an `integer` primary key becoming `bigint`. Widening `varchar` or the precision of `numeric(p,0)` needs no rewrite, so it stays R-0568's in-place change. A change across families, such as `integer` to `uuid`, still gets R-0567's refusal on a live table, and `--fast` handles it.

**What changes together.** The group is every column of the changed key, plus every column that references it, followed transitively: a referencing column that is itself referenced brings its own referencing columns in. The group also includes the identity or owned sequence, every index and constraint on those columns, and the views over them, carried as R-1061 and R-1063 already describe. Typed foreign keys (R-0224) already force the models to change every referencing column together, so the diff shows the whole group, and the planner makes it one op.

**The steps.** Each has its own boundary, and each brief lock gives way under `lock_timeout` with retries (R-0607, R-1072):

| # | Step | Kind | Lock and effect on apps |
|---|---|---|---|
| 1 | One per table, key table first: add the shadow column for each changed column (two on a self-referencing table), plus one sync trigger `BEFORE INSERT OR UPDATE` that copies each old column into its shadow | `tx` per table | `ADD COLUMN` takes `ACCESS EXCLUSIVE` and `CREATE TRIGGER` takes `SHARE ROW EXCLUSIVE`, each for under a millisecond |
| 2 | Copy each table in batches by its own primary key, up to a bound read once that table's trigger has committed, with the cursor in `_morm.steps` | `batch` per table | Row locks for one batch only |
| 3 | `CREATE UNIQUE INDEX CONCURRENTLY` on the key's shadow columns, built from every unique index or primary key that holds the key column, with the shadow column swapped in (so composite keys keep their other columns). `CREATE INDEX CONCURRENTLY` for each index on a referencing column, and a unique index for each unique constraint holding one | `no-tx` | None |
| 4 | Add a `CHECK ... NOT VALID` on each shadow column, then `VALIDATE` it. For a `NOT NULL` source the check is `(new IS NOT NULL AND new = old)`. For a nullable one it's `(new IS NOT DISTINCT FROM old)` | `tx`, then a separate `tx` | `VALIDATE` takes `SHARE UPDATE EXCLUSIVE`, so writes carry on |
| 5 | For each referencing foreign key: lock the key table `SHARE ROW EXCLUSIVE`, then add the new key on the shadow columns `REFERENCES key_table (shadow columns)`, with the same actions, `MATCH` type and deferrability, `NOT VALID`. Then `VALIDATE` it | `tx`, then a separate `tx` | `VALIDATE` takes `SHARE UPDATE EXCLUSIVE` on the referencing table and `ROW SHARE` on the key table, so writes carry on |
| 6 | `ANALYZE` the shadow columns | `no-tx` | None |
| 7 | Build each materialized view above the group beside the old one, as late as possible (R-1063) | `tx` each | As R-1063 |
| 8 | The cutover | one short `tx` | Below |

Step 4's check proves two things in one scan: every row was copied, and with the same value. It also lets Postgres skip the scan when the cutover sets `NOT NULL` or adds the primary key. Step 5's new foreign key references the shadow column's unique index before that index becomes the primary key, which Postgres allows. At cutover the index becomes the key with the same OID, so the foreign key stays tied to it. Step 5 has to wait for every copy and step 3: a key that isn't copied yet would fail the new foreign key's checks on app writes.

**The cutover** runs as one transaction, under R-1072's rules for giving way, with `--terminate-blockers` and `--resume`:

1. Take each carried view's lock with its own `DROP VIEW`, from the outside in (R-1061). Then lock the tables `ACCESS EXCLUSIVE`: the key table first, then the referencing tables in name order.
2. Walk the group again under the lock. This includes reading `pg_constraint` for every foreign key that references the key. Anything new stops the cutover with nothing changed.
3. Take the sequence's lock with a same-value `ALTER SEQUENCE ... INCREMENT BY <current>`, because `LOCK TABLE` refuses sequences. Then read `last_value` and `is_called`.
4. Drop the sync triggers, the old foreign keys, and the old indexes on the referencing columns.
5. On the key table: drop the identity (which drops its sequence), drop the old primary key or unique constraint, drop `NOT NULL` on the old column, and rename it to its retired name. Rename the shadow column to the real name, then `SET NOT NULL`, which the validated check proves with no scan. Then `ADD CONSTRAINT <name> PRIMARY KEY USING INDEX`, then `ADD GENERATED ... AS IDENTITY (SEQUENCE NAME <old name>, <old options>)` and `setval` to the values read in point 3. Copy the sequence's owner, grants and comment, then drop the check.
6. On each referencing table: retire the old column the same way, rename the shadow column, and `SET NOT NULL` where the old column had it. Drop the check, and rename the new foreign key and indexes to the real names.
7. Recreate the carried views and swap the materialized views (R-1061, R-1063). Refresh functions (R-0994). Copy column comments and grants. Write the step as `done` in `_morm.steps` in this same transaction.

It holds `ACCESS EXCLUSIVE` on every table in the group, `SHARE ROW EXCLUSIVE` and then `ACCESS EXCLUSIVE` on the sequence, and each carried view's lock. Everything inside it is a catalog change: there's no scan and no rewrite.

**A `serial` key** (an owned sequence, not an identity) is simpler. `ALTER SEQUENCE ... AS bigint` keeps the same sequence, with the same OID and position, and `OWNED BY` and `SET DEFAULT` move it to the new column.

**Afterwards** the old columns are retired under R-0570 and dropped by `purge`. Because they're kept, the down can make them live again at their original column positions (section 2).

**Rollback and `revert`** run the same form in the other direction. The retired columns become the shadow columns again: a sync trigger from new to old, a full recopy (rows changed since the cutover need it too), indexes, checks, foreign keys, then a cutover that renames back and moves the identity back to an `integer` sequence. Preflight refuses with exit code `2` if any key value, or the sequence's position, doesn't fit the old type, naming the table, the column and the value. The cutover checks the sequence again under its lock. Nothing can make that reversal safe once a value doesn't fit, so it's a refusal, not a decision.

## 2. What I tested, and the results

The live runs used a key table `parent` (`integer` identity primary key, 1,000,000 rows), `child_a` (`NOT NULL` reference, `ON DELETE CASCADE`, 1,000,000 rows) and `child_b` (nullable reference, `ON DELETE SET NULL`, 1,000,000 rows, half of them null). Load came from `pgbench` with 8 clients running nonstop, plus 8 more from 3 seconds before the cutover. The mix was:

- an insert of a parent and two children
- an insert followed by a delete of the parent, which cascades and sets null
- moving a child to another parent
- a transaction that updates a parent, then reads its children
- a join read
- a `SELECT *` on `child_b`

Every run checked its write counts against `pgbench`'s per-transaction log.

| What | 18.6 | 17.11 |
|---|---|---|
| Whole sequence under load, key table locked first | Done. 990,920 inserts, 329,506 deletes and 658,363 touches, each matching the table exactly. No failed or aborted transaction, no orphan, no leftover trigger or function | Done (2 runs). Every count matched, with no failed transaction |
| Time the cutover held its locks before commit | 7.9 ms (6.3 and 6.8 in other runs) | 6.8 ms both runs |
| Longest app transaction overlapping the cutover | 51 ms, which is the 50 ms lock wait | 52 ms |
| Cutover attempts under nonstop load, key table first | Got in on attempt 89, 31 s in, with the load still running | Attempt 16, and attempt 25 in a second run |
| Cutover attempts, referencing tables first | 0 of 176 in 60 s. It only got in once the load stopped | Not run |
| Step 5 without locking the key table first | 17 attempts in one run, 327 attempts (103 s) in another | Not run |
| Step 5 locking the key table first | 1 attempt, in every run | 1 attempt |
| Copy walking "until nothing is left" (R-1048's wording) | Chased new inserts for 6,283 batches and finished only when the load stopped. With a bound read after the trigger committed: 104 batches | Bounded: 105 batches |
| Final catalog | Matches a fresh build of the target schema: `parent_pkey`, `child_*_parent_id_fkey`, the index names, `parent_id_seq` as `bigint` with the `bigint` maximum, and on 18 the `*_not_null` constraint names. The foreign keys' `conindid` is `parent_pkey` | Same, apart from 18's `NOT NULL` constraints, which 17 doesn't have |
| Copy's client killed partway, then resumed from its cursor | Resumed at cursor 200,000 and finished, with every count matching | Same |
| Backend killed during `CREATE UNIQUE INDEX CONCURRENTLY` | Left `indisvalid = false`. Dropped and rebuilt as valid | Same |
| Backend killed during `VALIDATE` | Left `convalidated = false`. A rerun validated it | Same |
| Migrator killed inside the cutover, holding every lock (the client killed, its backend terminated) | Nothing changed: old columns and types, 3 triggers, 4 foreign keys, an `integer` sequence. A rerun committed on attempt 2. Every count matched, and no app transaction failed | Same, with the rerun committing on attempt 25 |
| The down, online (retired columns back to live), run with no load | Original columns back at `attnum` 1 and 2 as `integer`, an `integer` identity with its position carried over, and the next insert took `last_value + 1` | Same |

In the kill runs the longest app transaction was 288 ms. That came from the test holding the cutover's locks in an injected sleep until it was killed, not from the design.

Single behaviours, each run on both versions with the same result unless noted:

- **Foreign key on a plain unique index:** `ADD CONSTRAINT ... FOREIGN KEY ... REFERENCES parent (id_new) NOT VALID` works on a unique index that has no constraint, and `conindid` is that index.
- **`ADD PRIMARY KEY USING INDEX` on an index a foreign key depends on:** works. Afterwards, `DROP CONSTRAINT parent_pkey` is refused, because "constraint child_parent_id_fkey depends on index parent_pkey". So the link holds.
- **`NOT NULL` with no scan:** with a validated `CHECK (id_new IS NOT NULL AND id_new = id)`, both `SET NOT NULL` and `ADD PRIMARY KEY USING INDEX` (even with the column still nullable) log "existing constraints ... are sufficient to prove that it does not contain nulls". `CHECK (id_new = id)` alone isn't enough: it logs "verifying table", which is a scan (78 ms on 2M rows). Both parts of the check are needed.
- **Identity:** `ADD GENERATED ... AS IDENTITY (SEQUENCE NAME ..., START 1000 INCREMENT 5 CACHE 20)` on the renamed `bigint` column, then `setval`, carried the options. The type became `bigint` and the maximum the `bigint` default. `GENERATED ALWAYS` works the same way.
- **Sequence lock:** `LOCK TABLE` on a sequence fails with "not supported for sequences". A same-value `ALTER SEQUENCE ... INCREMENT BY 1` takes `ShareRowExclusiveLock` and keeps `last_value` and `is_called`. While another session held it, `nextval` timed out with `55P03`.
- **Sessions open across the swap:** an old session's `currval('parent_id_seq')` and `lastval()` fail with `55000` until its next `nextval`. The sequence's grants and comment don't carry over by themselves, so the cutover copies them.
- **`serial`:** `ALTER SEQUENCE ... AS bigint` kept the OID and position, and raised the maximum to `bigint`'s. An explicit `MAXVALUE 5000` stayed 5000.
- **Composite key:** an FK on `(tenant, pid_new)` to a unique `(tenant, id_new)`, validated, then `ADD PRIMARY KEY USING INDEX` on the composite index: works.
- **Self-reference:** one table, two shadow columns, one trigger. A row referencing itself, and a row referencing one inserted in the same statement, both worked with the new foreign key live. No row was out of step.
- **`ON UPDATE CASCADE` and `ON DELETE SET NULL` with the old and new foreign keys both live:** correct end state whichever foreign key's triggers fire first.
- **Deferrable:** the new foreign key keeps `DEFERRABLE INITIALLY DEFERRED`. A foreign key can't reference a deferrable unique key at all ("cannot use a deferrable unique constraint"), so a referenced key is never deferrable.
- **Lock modes:** `ADD FOREIGN KEY ... NOT VALID` takes `ShareRowExclusiveLock` on both tables. `VALIDATE` takes `ShareUpdateExclusiveLock` on the referencing table and `RowShareLock` on the key table.
- **Partitioned tables:**
  - `CREATE INDEX CONCURRENTLY` on a partitioned table fails on both versions.
  - `ADD CONSTRAINT ... USING INDEX` "is not supported on partitioned tables" on both.
  - A `NOT VALID` foreign key from a partitioned table works on 18, but on 17 it fails with "cannot add NOT VALID foreign key on partitioned table".
- **A foreign key no model declares** (`other.ext`): preflight's `pg_constraint` query lists it with its definition. Even if the cutover's walk missed it, `DROP CONSTRAINT parent_pkey` with no `CASCADE` fails with `2BP01`, naming `ext_pid_fk`.
- **Narrowing back past the old range:** the range check finds a value of `3,000,000,000`. During a narrowing, the sync trigger turns both an insert of that value and an `UPDATE` of another column on that row into `22003`. That's why preflight must refuse.
- **Logical replication (17):**
  - Adding the shadow column on the publisher stops the subscriber with "missing replicated column: id_new".
  - With the shadow column on the subscriber too, the publisher's cutover stops it again, with "missing replicated column: _morm_retired_0001_id".
  - The subscriber's own cutover then let it catch up with the right rows.
  - On 18, a one-transaction `ALTER COLUMN TYPE bigint` on the publisher alone kept replicating to an `integer` subscriber with no error.

## 3. Choices for the owner

**A. Lock order (recommend key table first).** The cutover takes the key table, then the referencing tables in name order, and step 5 takes the key table's lock before the referencing table's.

- Referencing tables first got in 0 times in 176 tries under the same load. Key table first got in on attempts 89, 16 and 25, and step 5 went from 17 and 327 attempts to 1.
- Readers and parent-then-child writers lock the key table first. The opposite shape, a single child write checking its parent, still sometimes makes an attempt give way, and R-1072 already handles that.
- R-1049's enum cutover says "sorted order". I'd define it the same way: the key table first, then name order.

**B. Bounding the copy (recommend yes, for every online copy).** R-1048's "walks by key until nothing is left" never finishes while inserts keep coming, which I saw on 18. The fix: read the upper bound once the table's trigger has committed. `CREATE TRIGGER` takes `SHARE ROW EXCLUSIVE`, which waits for every open writer, so any row above the bound was written through the trigger, and `VALIDATE` is still the safety net. This changes R-1048 (step 52) and the copy in step 39, not just this feature. The alternative is to keep chasing, which can stall a migration indefinitely on a busy table.

**C. Scope (recommend B, both directions within one integer family).**

- *A: widening only.* Narrowing a referenced key stays refused online, so `rollback` and `revert` of a widening need `--fast` and its long lock.
- *B: any rewriting change within one family, in either direction,* with narrowing guarded by the range preflight. The steps are the same, and the down is tested on both versions.

Silent narrowings still wait for `contract` under R-1108. An integer narrowing raises a real error, so it needs no gate.

**D. Writes during a narrowing's window (recommend accept, as stated).** From the down's first step, an app write that the old type can't hold, such as an identity crossing 2³¹ during the window, fails with `22003` from the sync trigger. That's R-0612's rule for narrowings ("the database's error for that write"), applied from the first step instead of from the cutover. Preflight and the cutover's check under its lock stop it if a value already doesn't fit. The alternative is to cap the `bigint` sequence's `MAXVALUE` at `integer`'s maximum for the window. That fails the same writes, with a clearer message.

**E. Logical replication (recommend refuse online).** A table in any publication, or one that's a subscription target, is refused online with exit code `2`, naming the publication or subscription. `--fast` handles it, and the subscriber keeps working, as tested. The alternative is a documented order (the subscriber adds its shadow columns first, then the publisher runs everything, then the subscriber cuts over). It works, but replication stops with errors between the two cutovers, while WAL builds up.

**F. Partitioned tables (recommend refuse online, `--fast` handles it).** Postgres can't swap a partitioned key table's key without a scan, because `USING INDEX` isn't supported on partitioned tables, and on 15 to 17 it can't add a `NOT VALID` foreign key from a partitioned table. A partition-by-partition form that works only on 18 could go to `later-defo-build.plan`. This matches R-1052.

**G. Many referencing tables (recommend scaling the wait automatically).** R-1072 refuses unless `cutover_lock_timeout` times the number of locks is under half of `deadlock_timeout`. With the defaults (50 ms and 1 s), that refuses a group needing 10 or more locks, which is common for a `users` table.

- *Keep it.* The operator lowers `cutover_lock_timeout`.
- *Scale it.* For that cutover, each lock waits `min(cutover_lock_timeout, deadlock_timeout / (2 × locks))`, and the output says so. Step 39's data showed a 20 ms wait got in no less often than 50 ms (7 of 213 attempts against 5 of 192).
- *One budget per attempt,* shared by every lock. I haven't tested it.

I'd scale it, so the promise holds with no setting to change.

**H. A referencing foreign key no model declares (recommend refusing under `--fast` too).** Online, it's refused, with exit code `2` and the evidence below. With `--fast`, Postgres would widen the key and leave that column narrow, so the foreign key would fail once ids pass 2³¹. I'd refuse both forms. The fix is to declare the table in the models, or drop that foreign key first.

**I. `max_rewrite_mb` on this `--fast` (recommend applying it to each table).** R-1025 sets it for enum swaps. The same rule fits here, applied to each table in the group.

## 4. What it refuses, and why

Each refusal exits with code `2` before the first step and changes nothing. `makemigrations` checks scratch, `migrate` and `rollback` preflight check the live database, and the cutover's second walk checks again under its lock.

- **A foreign key on the key from a table no model declares, or one the models don't declare.** The evidence is the referencing table, the constraint's name and `pg_get_constraintdef`, its deferrability and the estimated rows. morm can't widen a column it doesn't manage, and if that column stayed narrow, its foreign key would break once a value passed the old limit. The cutover's `DROP CONSTRAINT` with no `CASCADE` is the last check (`2BP01`, tested). With choice H, `--fast` refuses too.
- **A partitioned key table or referencing table** (choice F), with `--fast` handling it.
- **A table in a publication, or a subscription target** (choice E), with `--fast` handling it.
- **On the down, a key value or sequence position past the old type,** naming the value. No option makes it reversible.
- **As R-1052 lists for the column swap:** exclusion constraints and extended statistics on a group column, stored generated columns that read one, a domain or composite type on the key, and a view that would still read the retired column (`SELECT *`). Each of these needs a rewrite or a definition morm can't move with no lock. `--fast` handles them.
- **Unmanaged views and anything else R-1061 already refuses.**
- **Enum keys** keep R-1052's refusal: an enum swap isn't a widening.
- **A change across families on a referenced key,** such as `integer` to `uuid`, keeps R-0567's refusal, with `--fast`.

## 5. Plan changes

**A new step 60, "Online widening of a referenced key."** It depends on 52, and step 40 depends on 60 instead of 52, so the order becomes 59, 52, 60, 40, 41. It needs 39's swap, cutover and view carrying, 37's dependents, and 59's walk. It edits the same cutover code as 52, so it runs after 52, not beside it. It's about the size of step 52: one op with its steps, a cutover, sequence handling, refusals and an inverse. That fits one session.

New requirements, from the next free ID, `R-1110`:

| ID | What |
|---|---|
| R-1110 | What changes together: the transitive group, composite and self-referencing keys, and the identity or owned sequence |
| R-1111 | The steps and their boundaries: the bounded copy, the two check forms, step 5's lock on the key table first, and the foreign key on the unique index |
| R-1112 | The cutover: what it locks and in what order, the second walk, the order of statements, the step written `done` inside it |
| R-1113 | Identity and `serial` sequences: the same-value `ALTER SEQUENCE` lock, carried options, `setval`, owner, grants and comment, and the `currval` note for the README |
| R-1114 | The refusals in section 4, with their evidence |
| R-1115 | The inverse, for `rollback` and `revert`: online, the range preflight, the check under the lock, D's rule for writes during the window |
| R-1116 | Recovery at each boundary: a rerun of steps 1 and 3 to 5 checks the catalog first, a failed concurrent index is dropped and rebuilt, a constraint left `NOT VALID` is validated again, the copy resumes from its cursor, the cutover is atomic, and `rollback --failed` drops the shadow objects |

New tests, from `T-0225`:

- **Live traffic on Postgres 15–18,** with the mix in section 2: no failed transaction, no lost write, counts exact, and the catalog equal to a fresh build. The same for a self-referencing key, a composite key, `ON UPDATE CASCADE`, `SET NULL` and `SET DEFAULT`, deferrable keys, `GENERATED ALWAYS` and `serial`.
- **Fault injection:** killing the backend at every boundary, inside the copy, the index builds, both `VALIDATE`s and the cutover. Then `--resume` and `rollback --failed` must each reach the right state.
- **Each refusal,** with its evidence and with nothing changed.
- **Up, down and up on the exact catalog,** with the original column positions restored, and the down refused once a value passes 2³¹.
- **The walk's one decision** listing objects on every column in the group.
- **The cutover gives way** with the key-first order, and choice G's scaled wait on a group of 12 tables.

Edits to existing items:

| Item | Change |
|---|---|
| R-0567 (step 39) | Rewritten: a rewriting change within one family runs in step 60's online form. A change across families stays refused online, and `--fast` does it in one rewrite |
| Step 39 | The deliverable "refusal of key type changes" narrows to changes across families. Its copy gains choice B's bound |
| R-1048 (step 52) | Choice B |
| R-1049 (step 52) | "sorted order" defined as in choice A |
| R-1052 (step 52) | "as for key type changes" reworded: integer keys are now online |
| R-1072 (step 39) | Choice G |
| R-1025 | Choice I |
| T-0221 (step 41) | Adds an old binary with `i32` fields reading and writing through a referenced key's widening |
| Step 40 | One new test: `revert` of a key widening runs online, and is refused once a value doesn't fit |
| Step 59 | Its "Context from other steps" notes that step 60's op is a column type change on every column in the group |
| `later-defo-build.plan` | The "Online key type changes across foreign keys" section is removed |
| `INDEX.md` | A Decisions entry. Step 60 joins the table and the order |
| `coverage.md`, `tools/manifest.json` | The new IDs |

The rest stays as it is:

- **`contract`:** a widening never waits for it (R-0612), and neither does an integer narrowing.
- **`--decide`:** the walk's decision covers the whole group, as one list, and an unmanaged foreign key isn't a decision.
- **`--terminate-blockers`:** acts during the cutover only, unchanged.

**No new public call,** so the README needs no new "use this when" line. The README does need a sentence on `currval` and `lastval` failing in sessions open across the cutover.

## 6. What I couldn't settle

- **Postgres 15 and 16 weren't run.** Only 17.11 and 18.6 are installed here. Everything the design relies on has been in Postgres since 12 or earlier: skipping the scan with a validated `CHECK`, `USING INDEX`, `NOT VALID` foreign keys on plain tables, `ADD GENERATED`, and `ALTER SEQUENCE AS`. What I'd expect to differ:
  - The `NOT NULL` constraint names exist only on 18.
  - The partitioned `NOT VALID` foreign key works only on 18, and it's refused anyway.

  The step's tests run on 15–18 in CI, and a different result there means stop and ask.
- **Choice G's scaled wait and the per-attempt budget are untested** at 10 or more locks. The step would prove whichever you choose.
- **Under saturated nonstop traffic,** key-first still needed 16 to 89 attempts, and referencing-first got none until the load stopped. The cutover never killed an app transaction. Getting in relies on gaps in traffic or on `--terminate-blockers`, as R-1072 already says. My `pgbench` clients had no think time, which is harsher than real apps.
- **The down wasn't run under load.** It uses the same steps as the up, which was run under load. The step's test runs it under load.
- **Rows behind a `REPLICA IDENTITY USING INDEX` on the old key** would need it moved to the new index in the cutover. I didn't run this, because tables in a publication are refused under choice E.

Working files are in `.claude/scratch/` (`probe*.sql`, `live/driver.sh`, `live/cutover.sql`, `live/down.sql`, and each run's `driver.log` and `verify.out`).
