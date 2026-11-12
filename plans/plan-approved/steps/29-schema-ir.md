# Step 29: Schema IR, catalog and scratch

- **Phase:** Migrations
- **Depends on:** 28, 10
- **Can run beside:** 07, 08, 09, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 47, 56
- **Crates:** `morm-migrate`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Read any schema, from models or a live database, into one typed form.

## Deliverables

- `Schema` IR for everything the plan manages, the catalog reader, reading what a child or a partition holds from its parent as the parent's, a partition's declared `pk` as the key it holds, scratch database handling, creating models in a temporary database, replaying a chain

## End-to-end check

A model set created in a scratch database reads back into an IR equal to the one built from a replayed chain, on Postgres 15–18.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> **How it fits together:** `morm-admin` holds the engine. When a command needs the models, it builds and runs `morm-mgr` to get the manifest. Commands that don't need models (`migrate`, `rollback`, `status`, `purge`) run from `morm-migrations/` alone, so a production host needs `morm-admin` and no Rust toolchain. The app binary carries no migration code or schema metadata, apart from the short per-model column list `check_schema` reads.
>

## Out of scope

- The diff (step 30)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Migrations / What's managed

**R-0528** **Everything an application's schema depends on, from v1:** tables, columns (type, collation, nullability, default, identity, generated, comment), constraints (including deferrable and validated state), indexes, enums, domains, composite types, sequences, extensions, **database views, materialized views, functions, procedures, triggers, RLS policies, and grants** on managed objects.

### Migrations / Source of truth

**R-0543** No snapshots. `makemigrations` uses a scratch Postgres (`MORM_SCRATCH_URL`) and never touches staging or production:

- **R-0544** Replay the migration chain into temporary database A.
- **R-0545** Create the models and objects in temporary database B.
- **R-0546** Read both through the same catalog queries (`pg_get_viewdef`, `pg_get_functiondef`, `pg_get_triggerdef` and so on) into a typed `Schema` IR, so Postgres normalizes everything.

### Risks, open items and later work / Risks

- **R-0918** **Scratch databases:** `makemigrations` needs a Postgres where it can `CREATE DATABASE`.

### Managed enums

- **R-0986** **Enum catalog reads.** Labels come from `pg_enum` directly, ordered by `enumsortorder`. The reader never calls `enum_range` or casts a label to its type, because both fail inside a transaction after `ADD VALUE`, and the fingerprint is read there. The fingerprint holds labels in order, and never an OID or a sort position, so a swap to the same labels shows no drift.

**R-1174** **What a child or a partition holds from its parent.** Postgres puts some of a parent's objects on each child and partition, and they belong to the parent. On Postgres 18.6 and 17.11, a CHECK on an inheritance parent or a partitioned table sat on every child, grandchild and partition under the parent's name, with `conislocal = false` and `coninhcount` counting the parents it came from, and a child of two parents held each one's. A foreign key, a primary key and a unique key on a partitioned table sat on every partition with `conparentid` pointing at the parent's constraint, `conislocal = false` and `coninhcount = 1`: the foreign key under the parent's name, the primary key as `<partition>_pkey`, the unique key under a name Postgres made, such as `q1_v_id_key`, and each under the partition's own name when `ATTACH PARTITION` took over a matching one the partition already had. An index on a partitioned table sat on every partition under a name Postgres made, such as `p1_a_idx`, or under the name of a matching index the partition had before `ATTACH PARTITION`, linked to the parent's index by a `pg_inherits` row whose `inhrelid` is the partition's index. A foreign key that references a partitioned table added a row on the referencing table for each partition, with `conparentid` set, named `r_x_fkey1` on 17.11 and `r_x_fkey_1` on 18.6. On 18.6 a `NOT NULL` is a `pg_constraint` row, and a child held its parent's under the parent's name, such as `parent_id_not_null`. An inheritance parent's foreign keys, primary key, unique keys and indexes weren't copied, and a CHECK marked `NO INHERIT` stayed on the parent. A child or a partition couldn't drop what it held, failing with `cannot drop inherited constraint` and `cannot drop index ... because index ... requires it`. `indisclustered` follows no tree: `ALTER TABLE ... CLUSTER ON` a partitioned table's index failed with `cannot mark index clustered in partitioned table`, and on a partition's index or an inheritance parent's index it marked that index alone. After `DETACH PARTITION`, everything the partition held became its own, with `conislocal = true`, `coninhcount = 0` and no `pg_inherits` row for its indexes, and after `NO INHERIT` a child's CHECKs became its own the same way. So the catalog reader reads each constraint with `coninhcount > 0` or `conparentid <> 0`, and each index with a parent index in `pg_inherits`, as part of the object it was copied from, which it reads once, on the table it was created on, and leaves it out of the child's, the partition's and the referencing table's own objects. A child's or a partition's model declares only its own constraints and indexes, the ones with `coninhcount = 0`, no `conparentid` and no parent index, and the parent's model declares the rest, once, apart from a partition's primary key, which R-1183 covers. Nullability stays a column's, read from `attnotnull` on every version, so Postgres 18's `NOT NULL` rows are never objects of their own. Postgres prints every index on a partitioned table with `ON ONLY`, as `CREATE INDEX p_a_ix ON ONLY x.p USING btree (a)` for one created without it, so the reader reads that definition as the same index on a plain table, which is how scratch builds it. A CHECK an inheritance child declares itself and also inherits, with `conislocal = true` and `coninhcount > 0`, is the parent's while it's inherited. When the parent's is dropped, Postgres keeps it on the child, with `coninhcount = 0`, as an object of the child's own that no model declares. A child's model still declares every column its table has, inherited or not. Scratch has no inheritance, so this matters wherever the reader reads a live database.

**R-1183** **A partition's primary key.** Postgres names the key a partition holds from its partitioned table `<partition>_pkey`, which is morm's own name for a table's key. On Postgres 18.6 and 17.11, `PARTITION OF`, and `ATTACH PARTITION` of a table with no key, built it as `<partition>_pkey` on the parent's key columns, in their order, with `conparentid` pointing at the parent's key and its index linked to the parent's in `pg_inherits`, and a partition of a partition held `<partition>_pkey` from its own parent's key. `ATTACH PARTITION` of a table that already had a key on those columns took it over under the name it had, such as `p2_pkey` or `p3_other`, and so did `ADD PRIMARY KEY` on a partitioned table whose partition already had one. When another relation already had the name, Postgres named the key `p5_pkey1`. A table can't hold two keys: `ADD PRIMARY KEY` on a partition that held its parent's failed with `multiple primary keys for table "p1" are not allowed`, and so did `ATTACH PARTITION` of a table whose key was on other columns. So a partition's model may declare `pk`, as the one exception to R-1174's rule that a child's or a partition's model declares only its own. The reader still reads the key as the parent's, one object, on the parent, and also records on each partition the key it holds, by name and columns. A partition's declared `pk` on the same columns, in the same order, under the name morm gives that table's key, is that key, not a second object, and a partition's model with no `pk` matches it too, as R-1174 says. A declared `pk` that differs from the key the partition holds, in its columns or its name, doesn't match it, as a key that differs on a plain table doesn't. So a key a partition holds on its parent's key columns under another name, such as `p3_other` or `p5_pkey1`, doesn't match a declared `pk`, and is drift on that partition until it's renamed `<partition>_pkey`. On Postgres 18.6 and 17.11, `ALTER TABLE p3 RENAME CONSTRAINT p3_other TO p3_pkey` renamed a key a partition held, and its index, keeping its `conparentid` and its index's `pg_inherits` row, under a brief `ACCESS EXCLUSIVE` lock on the partition, and renaming `p5_pkey1` failed with `relation "p5_pkey" already exists` until the relation holding that name was renamed. An inheritance child holds no key from its parent, so its `pk` is its own, as on any table.

**R-1187** **A partition's identity.** Postgres gives a partition its parent's identity, with no sequence of its own. On Postgres 18.6 and 17.11, a partition made by `PARTITION OF`, one attached by `ATTACH PARTITION`, a partition of a partition, and every partition of a table whose column got identity by `ADD GENERATED ... AS IDENTITY` read that column with the parent's `attidentity`, `pg_get_serial_sequence` gave no sequence on the partition, and a row inserted into the partition directly took its value from the parent's sequence. `ATTACH PARTITION` of a table with an identity column failed with `table "p2" being attached contains an identity column "id"`, whether the parent's column had identity or not, adding or dropping identity on a partition's column alone failed with `cannot add identity to a column of a partition` and `cannot drop identity from a column of a partition`, and dropping it on the parent took it off every partition. After `DETACH PARTITION` the column had no identity. So the catalog reader reads identity on a partition's column as the parent's, once, on the parent's column, and reads the partition's column with none, whatever the catalog shows on the partition, on every version. A partition's model never declares identity, which step 33's preflight refuses. An inheritance child holds no identity from its parent, so its identity is its own, as on any table.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0259 | integration | **What a child or a partition holds, on Postgres 15–18:** on a three-level inheritance tree with a child of two parents, and on a partitioned table with one partition made by `PARTITION OF` and one attached by `ATTACH PARTITION` with a matching CHECK, foreign key, primary key and index of its own, where the parents have a CHECK, a `NO INHERIT` CHECK, a foreign key, a primary key, a unique key and indexes, and a plain table has a foreign key to the partitioned table, the reader puts each CHECK, foreign key, primary key, unique key and index once, on the table it was created on, and none on a child or a partition. The `NO INHERIT` CHECK is on the parent alone, each child's and partition's own CHECK and index are on that table, the referencing table has one foreign key, Postgres 18 shows no `NOT NULL` object anywhere, and nullability reads the same on every version. Each index on the partitioned table reads the same as the index scratch builds from its model on a plain table. Each partition records the key it holds, with the parent's key columns, under `<partition>_pkey`, or under the name its own key had when `ATTACH PARTITION` took it over, and a partition of a partition records its own parent's key. A partition's model that declares `pk` on those columns under `<partition>_pkey` matches what the reader reads, and so does one with no `pk`, while one whose `pk` names other columns doesn't, and an inheritance child's key reads as its own. After `DETACH PARTITION` and `NO INHERIT`, what each table held reads as its own. |
| T-0265 | integration | **A partition's identity, on Postgres 15–18:** on a partitioned table whose key column is an identity column, with a partition made by `PARTITION OF`, one attached by `ATTACH PARTITION` of a table with no identity, and a partition of a partition, and on a partitioned table whose column gets identity by `ADD GENERATED ... AS IDENTITY` after its partitions exist, the reader puts the identity once, on the parent's column, and reads each partition's column with none, on every version. A partition's model with `base = "bare"` and `pk` on the parent's key columns matches what the reader reads. After `DETACH PARTITION`, the detached table's column reads with no identity. |
