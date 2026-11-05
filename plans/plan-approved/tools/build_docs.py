#!/usr/bin/env python3
"""One-time: writes rules.md, contracts.md and INDEX.md."""
from pathlib import Path
from steps_data import STEPS
HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
info_c = (HERE / "_contracts_info.md").read_text(encoding="utf-8")
info_i = (HERE / "_index_info.md").read_text(encoding="utf-8")

rules = """# Rules for every step

These apply to every step, with no exceptions. A step session reads this file, `contracts.md` and its own step file, and nothing else unless the step file says so.

## Starting a step

1. Check `INDEX.md`: every dependency must be `done`.
2. Work on a branch named `step-NN-name`, in its own worktree when another step is running.
3. Commit as biggodev: repo-local `user.name` is `Biggo Dev`, `user.email` is `biggodev@gmail.com`, and the remote is `git@github.com-biggodev:biggaloy/rust-morm.git`. Never use the neurobin account.
4. Set the step's status to `in progress` in `INDEX.md`.

## Doing the work

- **The step file is the specification.** Build all of it. Don't drop, shorten or defer a requirement, and don't add features it doesn't ask for.
- **If the plan is wrong, stop.** When a requirement can't work, contradicts another or needs a design choice the file doesn't settle, stop and ask the owner with evidence and options. Don't improvise, and don't quietly build something different.
- **Nothing is stubbed.** No `todo!()`, no ignored tests, no "handled later" comments. Work that belongs to a later step is left out completely, and the step file's "Out of scope" says which.
- **Unknown behaviour gets tested.** When a requirement depends on how Postgres or a crate behaves, run it. Don't rely on memory.
- **Goals, in order:** raw performance, lowest memory, rock-solid migrations, hard to misuse. Hot paths have no needless allocation, dynamic dispatch or per-row state.
- **Secrets:** never log, print or commit passwords, tokens or parameter values.
- **Dependencies:** a new crate needs a reason in the PR.

## Tests

- Every `code` requirement and every test obligation ends with at least one test tagged in a comment, as `// covers: R-0123, T-0045`. A `trybuild` case carries the tag in its `.rs` file.
- A tag means the test really exercises that requirement. Tagging a test that doesn't is a failed review.
- A requirement that holds several rules needs tests for each of them, not one test for the first.
- `info` and `doc` requirements need no tag.
- Integration tests run against real Postgres, never mocks.
- Performance claims are measured in step 31. A step that reports a cost states what it measured.

## When a step is done

All of these must be true:

1. Every requirement in the step file is built.
2. `tools/check_coverage.py --code crates tests` passes with this step marked done.
3. `ci.yml` passes: `fmt`, `clippy -D warnings`, unit, `trybuild`, doc tests, and integration on Postgres 15–18.
4. The step's end-to-end check passes, and the PR shows the command and result.
5. Any new public call has its one-line "use this when" entry, ready for the README table in step 33. It states the real use case from the reader's side, such as: `.allow_missing()`: you will not handle a 0-row error.
6. An independent review passes (below).
7. `INDEX.md` says `done`, and the PR is merged.

## Independent review

The session that wrote a step never approves it. A fresh agent reads `rules.md`, `contracts.md`, the step file and the diff, and checks, requirement by requirement:

- Is it built as written?
- Does each tagged test really prove it?
- What did the step skip, weaken or add?

It reports each finding with the requirement ID and evidence. The writing session fixes them, and the review repeats until none remain.

## PRs

- The owner approves the PRs of steps 01, 02, 03 and 33.
- Other PRs merge once CI and the independent review both pass.
- The PR body lists the requirements covered, the end-to-end result, and any owner decisions.
- Never publish crates, force-push or change repository settings without the owner's go-ahead.

## Changing the plan

`plan-approved/` is the source of truth, and `plans/rust-morm-plan.md` is frozen as the record. An owner-approved change is edited into the step file it affects, in the same PR, with a note in `INDEX.md` under Decisions. A new requirement gets the next free ID, and `coverage.md` and `tools/manifest.json` are updated with it. A change to a shared signature also updates `contracts.md`, and every in-progress step is told.

## Keeping sessions small

One step is one fresh session, sized to stay under about 30% of the context window. If a step can't fit, stop at a clean commit and ask the coordinator to split it. Don't read the frozen plan or other step files to fill in gaps. If a gap exists, that's a plan problem: stop and ask.
"""
(ROOT / "rules.md").write_text(rules, encoding="utf-8")

contracts = """# Contracts

What parallel steps share. Signatures here are the starting point, taken from the approved plan. Step 02 corrects them from the proofs, and step 03 freezes them as compiling Rust. After step 03, changing anything here follows "Changing the plan" in `rules.md`.

## Crates

| Crate | Depends on | Holds |
|---|---|---|
| `morm` | `morm-macros`, `tokio-postgres`, `deadpool` | Runtime |
| `morm-macros` | nothing in this workspace | Procedural macros |
| `morm-migrate` | `tokio-postgres` | Migration engine. Doesn't depend on `morm` |
| `morm-admin` | `morm-migrate` | The binary. Doesn't depend on the user's models |
| `morm-mgr` | generated per project; `morm` and the user's models | Prints the manifest as JSON |

## Conventions

- Rust 2024, MSRV 1.99.1, Tokio only, PostgreSQL 15 and later only.
- Tool prefix `morm-`. Config `morm.toml`. Environment `MORM_*` and `DATABASE_URL`. Migrations in `morm-migrations/`. Ledger schema `_morm`.
- Exit codes: `0` ok, `1` error or drift, `2` refused by a guard, `3` open decisions.
- Generated SQL always quotes names. Generated code uses static dispatch.
- Macro-generated items are public API: names below don't change without following `rules.md`.

## Shared types

```rust
// Implemented by #[model] and #[view]
pub trait Model: Sized + Send + Sync + 'static {
    const TABLE: &'static str;
    const SELECT: &'static str;
    fn from_row(row: &tokio_postgres::Row) -> Result<Self>;
    fn from_row_named(row: &tokio_postgres::Row) -> Result<Self>;
}
pub trait HasPk: Model { type Pk: Param; }

// Product::F.<field>. Capability markers let a column refuse .set(), .by() or set_expr
pub struct Column<M, T, Cap> { /* name, marker data */ }

// A parameter: ToSql plus its Postgres type, because query_typed needs the type
pub trait Param: tokio_postgres::types::ToSql + Sync { fn pg_type(&self) -> tokio_postgres::types::Type; }

// Product::UK.<name>, and columns that are unique keys
pub trait UniqueKey<M> { const CONFLICT: &'static str; }

pub enum Patch<T> { Missing, Set(T) }
pub enum OrDefault<T> { Default, Set(T) }
pub struct WriteOnly<T>(/* private */);

pub struct Bits<const N: usize>([u64; N]);
pub struct Mask<M> { /* global and scoped bits */ }

// Connection-like: db, a held connection, or tx
pub trait Conn: Send + Sync { /* every query entry point; fn tx(&self) -> Option<&Tx>; */ }

pub trait Hooks: Model {
    fn validate(&self) -> Result<()> { Ok(()) }
    fn pre_insert(rows: &mut [Self], db: &impl Conn) -> impl Future<Output = Result<()>> + Send;
    fn post_insert(rows: &[Self], db: &impl Conn) -> impl Future<Output = Result<()>> + Send;
    fn validate_changes(changes: &Changes<Self>) -> Result<()> { Ok(()) }
    fn pre_update(changes: &mut Changes<Self>, target: &Target<Self>, db: &impl Conn) -> impl Future<Output = Result<()>> + Send;
    fn post_update(changes: &Changes<Self>, target: &Target<Self>, affected: u64, db: &impl Conn) -> impl Future<Output = Result<()>> + Send;
    fn pre_delete(target: &Target<Self>, db: &impl Conn) -> impl Future<Output = Result<()>> + Send;
    fn post_delete(target: &Target<Self>, affected: u64, db: &impl Conn) -> impl Future<Output = Result<()>> + Send;
    fn after_commit(event: Committed<Self>) -> impl Future<Output = Result<()>> + Send;
}
pub enum Target<M: HasPk> { Id(M::Pk), Ids(Vec<M::Pk>), Filter(/* condition */), All }
pub enum Change<'a, T> { Value(&'a T), Expr }
pub enum Committed<M> {
    Inserted(Vec<M>),
    Copied { count: u64 },
    Updated { changes: Changes<M>, target: Target<M> },
    Upserted { inserted: u64, updated: u64 },
    Deleted { target: Target<M> },
}
pub enum Upserted { Inserted, Updated, Unchanged }

pub enum Error {
    Unique { constraint: String, field: Option<&'static str> },
    ForeignKey, NotNull, Check, Serialization, Deadlock, LockTimeout,
    TransactionTimeout, TransactionAborted, CommitUnknown, NestedConnection,
    PoolerMode, NotFound, EmptyPatch, Conflict, Forbidden, Validation,
    PoolBusy, ConnectionLost, ReadOnly, OutOfRange, InvalidText,
    InvalidOrder, InvalidCursor, Decimal,
    // payloads settled in step 03
}
```

## Between `morm-mgr` and `morm-admin`

`morm-mgr` prints one JSON manifest: `format`, then models (table, schema, columns, types, nullability, defaults, flags, indexes, constraints, `was` hints, comments), managed objects, extensions, and registered `sql!` queries. Step 21 defines it. Everything else reads it, and it's versioned by `format`.

## Approved key decisions

These are context for every step. Details live in the step files.

""" + info_c
(ROOT / "contracts.md").write_text(contracts, encoding="utf-8")

rows = []
for k, s in STEPS.items():
    d = ", ".join(f"{STEPS[x]['n']:02d}" for x in s["deps"]) or "none"
    rows.append(f"| {s['n']:02d} | [{s['title']}](steps/{s['n']:02d}-{k}.md) | {s['phase']} | {d} | todo |")

index = """# rust-morm: approved plan

This folder is the source of truth for building rust-morm. `../rust-morm-plan.md` is frozen as the record. `../app_suggestions.md` holds advice for apps, and `../later-defo-build.plan` holds confirmed work after 0.1.

| File | Holds |
|---|---|
| `rules.md` | How every step works, and when it's done |
| `contracts.md` | Shared types and conventions, plus the key decisions |
| `coverage.md` | Every requirement and test obligation, with its step |
| `steps/` | One self-contained file per step |
| `tools/` | The coverage checker, and the scripts that made this split |

The frozen plan was split word for word: 894 blocks became requirements (`R-nnnn`), and the test lists became 125 obligations (`T-nnnn`). `tools/check_coverage.py` fails if any of them isn't in exactly one place, and with `--code` it fails if a finished step has an untagged one.

## Steps

| Step | Title | Phase | Depends on | Status |
|---|---|---|---|---|
""" + "\n".join(rows) + """

Status is one of `todo`, `in progress`, `in review` or `done`.

## Order and parallel work

1. **Foundation, one at a time:** 01, 02, 03. The owner approves each PR.
2. **After 03, these tracks run side by side:**
   - **Models:** 04, then 05, 06, 07 and 08 in parallel.
   - **Runtime start:** 09, beside 04.
3. **After 04:** 10.
4. **After 09 and 10:** 11. After 11 and 06: 12 and 13 in parallel, then 17 after 12.
5. **After 13:** 14, 15 (with 05), then 16.
6. **Migrations, beside the runtime once 05 is done:** 21, 22, 23, then 24, 25 and 26 in parallel.
7. **After their dependencies:** 18, 19, 20, then 27, then 28, 29 and 30 in parallel.
8. **Finish:** 31 and 32 in parallel, then 33. The owner approves the release.

Steps run in parallel only when `contracts.md` covers what they share. Each gets its own worktree and branch. When a parallel step needs something shared that isn't in `contracts.md`, it stops, and the contract is updated first.

## The coordinating session

The coordinator stays thin: it reads this file and never reads code.

1. Pick steps whose dependencies are all `done`.
2. Start a fresh agent for each one, with this prompt: read `plans/plan-approved/rules.md`, `contracts.md` and `steps/NN-name.md`, then do the step.
3. When it opens a PR, start a fresh review agent as `rules.md` describes.
4. Relay findings until the review is clean, then merge or ask the owner, as `rules.md` says.
5. Update the status here. Take any design question straight to the owner.

## Decisions since the split

None yet.

## Record from the frozen plan

Superseded milestones, later work and build decisions, kept for traceability. Nothing here is a step requirement, except as copied into a step file.

""" + info_i
(ROOT / "INDEX.md").write_text(index, encoding="utf-8")
