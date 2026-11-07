# Step 41: Rolling deploys

- **Phase:** Migrations
- **Depends on:** 40, 15
- **Can run beside:** 19, 20, 21, 22, 23, 24, 25, 27, 43, 44, 46, 47, 48
- **Crates:** `morm-migrate`, `morm-admin`, `morm`, `morm-macros`, `morm-core`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Keep old and new binaries working through a deploy.

## Deliverables

- `phase`, expand and contract, rename sync triggers, `deprecated` steps, `db.check_schema()` with its per-model column list, data migration hooks run through `morm-mgr` with `--hooks`

## End-to-end check

An old binary boots and works during an expand phase, a migration with a Rust data hook runs through `--hooks ./morm-mgr`, and `check_schema` reports a dropped default.

## Out of scope

- New features

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools

- **R-0044** `morm-mgr` also runs a migration's Rust data hooks, and nothing else. Build it in CI and ship the binary with the migrations. `migrate --hooks ./morm-mgr` and `rollback --hooks ./morm-mgr` call it as `morm-mgr hook ID up` or `down`, and a migration with hooks refuses to run without it, or if that binary doesn't contain the hook, because `morm-admin` holds no project code. A hook connects with the migration role and runs as a `no-tx` step, so it must be safe to run again on `--resume`.

### Migrations / Rolling deploys

**R-0612** Static column lists mean a dropped column breaks the old binary mid-deploy. Migrations carry `phase: expand | contract`, and `--phase` splits them. `contract` refuses while a required `#[col(deprecated)]` hasn't shipped. Marking a `NOT NULL` column with no default as `deprecated` generates an expand step dropping that `NOT NULL`, because the new binary stops writing it. A rename decision offers an expand/contract form for live tables: add the new column, keep both in sync with a generated trigger, backfill in batches, and drop the old column and trigger in `contract`. `db.check_schema()` checks compatibility at startup, not equality, so an old binary restarting during an expand phase still boots. Every column the binary reads or writes must exist, with a type the field can decode in the same family as the type it was built against, so only an unrelated kind fails. These are the families, and there are no others: for a field of a Rust integer type, `smallint`, `integer`, `bigint` and `numeric(p,0)` with any `p`, and never a `numeric` with a scale or with no precision, because an integer field has no way to read a fraction; for `Decimal` and `morm::Numeric`, `numeric` with any precision and scale, or with none; for `String`, `text`, and `varchar` with any length or none; for `DateTime<Utc>` and `Timestamp`, `timestamptz` with any precision, and for `NaiveDateTime`, `timestamp` with any precision; and for an array of any of these, an array of its element's family. Every other field needs the same type, with the same length or precision, as `format_type` prints it from `atttypid` and `atttypmod`. `char(n)` isn't in the text family and keeps its length, because Postgres pads a value with spaces to the length, so a change in either direction changes what a `String` reads. `timestamp` and `timestamptz` are different kinds here, because a change between them converts each value through the session's `TimeZone`. Any other difference fails the check, such as `text` for an integer field, `numeric(25,2)` for an integer field or `bigint` for a `Decimal` field. Within a family, the field reads through the decoder for the column's real type, chosen once per statement when it's prepared, and chosen again when a cached statement is re-prepared because Postgres reports that its result type changed, as it does after a cutover. When the types match, the path is the one the binary always ran. A field wider than the column reads every value losslessly. A field narrower than the column checks each value's range, and only a value that can't fit errors: an integer field returns `Error::OutOfRange` naming the column, never a wrapped value, and `Decimal` returns `Error::Decimal` for a value it can't hold exactly, past its 28 significant digits, as it does on unbounded `numeric`. `morm::Numeric` and `String` read every value whole. Nothing read is rounded. A write goes as the column type the binary was built against, and Postgres converts it to the column's real type, so a value the column can't hold, such as a number with more digits before the point than it allows or a text longer than its length, gets the database's error for that write. So a type change never stops a running binary, and one restarting after it still boots. A type change has no expand and contract of its own and never waits for `contract`, which still guards dropped columns and removed enum values. Extra columns and objects are fine. Two cases that would fail that binary's inserts are reported: an extra `NOT NULL` column with no default, and a column the binary may leave out that is no longer nullable and has lost its default.

### Migrations / Data migrations

**R-0613** SQL steps first, including `batch` steps, covered by checksums. Custom Rust data hooks are registered by migration id and use raw SQL only, since model types change. A hook needs a matching down hook, or the migration is declared irreversible.

### Managed enums

- **R-0997** **Enums in `check_schema`.** The per-model list also holds each enum's labels. Every labelled variant's label must exist, or its `was` label, and a `deprecated` variant may be missing. Extra labels in the database are fine, because a binary only fails on a row that holds one, and `Error::UnknownEnumValue` names it.
- **R-0998** **Enum phases.** Adds, renames and the online swap's steps before its cutover are `expand`. A cutover that removes a label is `contract`, because binaries that still write it would fail. A cutover that only reorders stays in `expand`. In one migration that adds and removes, the add goes in `expand`, so a value and its replacement exist together through the rollout. A `--fast` swap takes the phase its cutover would.
- **R-0999** **Removing a value in a rolling deploy.** `#[pg_enum(deprecated)]` on a variant says that value is on its way out. `contract` refuses a swap that removes a label until a release with that variant marked `deprecated` has shipped, the same gate `#[col(deprecated)]` uses.
- **R-1000** **Renaming a value in a rolling deploy.** A rename runs instantly by default, so old binaries fail on that value during the rollout: reads of rows holding it return `Error::UnknownEnumValue`, and writes of the old label return `22P02`. The rename decision also offers expand/contract: add the new label in `expand`, copy rows across in batches, and remove the old label by swap in `contract`. With `was`, a new binary reads both labels through either path. `was` answers only whether it's a rename. The path stays a decision with the instant rename as its default.

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0103 | integration | `check_schema` during expand phases and after a dropped default |
| T-0158 | integration | **Enums in a rolling deploy:** `check_schema` passes during an add and reports a missing label, an old binary reads rows on a plain add and gets `Error::UnknownEnumValue` only on rows holding the new value, `contract` refuses a removal until the `deprecated` variant has shipped, an add and a remove in one migration split across phases, and the expand/contract rename ends with the right labels and rows. |
| T-0220 | integration | **`check_schema`'s type families, on Postgres 15–18:** each of these passes: an `i32` field built against `integer` on `smallint`, `bigint` and `numeric(12,0)`, a `u64` field on `numeric(20,0)` on `numeric(25,0)` and `bigint`, a `Decimal` field built against `numeric(6,2)` on `numeric(20,4)`, `numeric(4,1)`, `numeric(40,2)` and `numeric`, one built against `numeric` on `numeric(6,2)`, a `String` field built against `varchar(3)` on `varchar(10)`, `varchar(2)`, `varchar` and `text`, a `DateTime<Utc>` field on `timestamptz(3)`, a `NaiveDateTime` field on `timestamp(0)`, and a `Vec<Decimal>` field built against `numeric(6,2)[]` on `numeric(20,4)[]`. Each of these fails: an `i32` field on `text` and on `real`, a `u64` field on `numeric(25,2)` and on `numeric`, a `Decimal` field on `bigint`, a `String` field on `integer` and on `char(3)`, a `String` field built against `char(3)` on `char(4)`, a `DateTime<Utc>` field on `timestamp` and on `date`, an `f32` field on `double precision`, and a `Vec<Decimal>` field on `numeric(6,2)`. After a pass, a `Decimal` field built against `numeric(12,2)` on `numeric` reads a 30-digit value as `Error::Decimal`, the `u64` field on `numeric(25,0)` reads `10^21` as `Error::OutOfRange` naming the column, and the `String` field on `varchar(10)` reads a 10-character value whole. |
| T-0221 | integration | **An old binary through a type change, on Postgres 15–18:** a binary built against `integer`, `numeric(6,2)` and `varchar(3)` columns keeps reading and writing them through a pool, outside transactions, while a migration widens them to `bigint`, `numeric` and `text`, run once as a plain type change and once through step 39's online column swap. No error reaches it apart from those below. Its cached statements re-prepare after the cutover and read through the new type's decoder, and it restarts and passes `check_schema`. A row holding `2^40` then reads as `Error::OutOfRange` naming the column, one holding a 30-digit value as `Error::Decimal`, and every other row as before, a 10-character text included. A binary built against `bigint`, `numeric(20,4)` and `text` does the same while they narrow to `integer`, `numeric(12,4)` and `varchar(3)`: it reads every value exactly and restarts, and a write of `2^40`, of `10^9` and of a 10-character text each gets the database's error for that write, while every write that fits succeeds. After a column changes by hand from `text` to `integer`, a binary with a `String` field on it fails `check_schema` at restart, naming the column. |
