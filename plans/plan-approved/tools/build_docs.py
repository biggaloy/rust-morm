#!/usr/bin/env python3
"""One-time: writes rules.md, contracts.md and INDEX.md."""
from pathlib import Path
from steps_data import STEPS, beside, sub, wave
HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
info_c = (HERE / "_contracts_info.md").read_text(encoding="utf-8")
info_i = (HERE / "_index_info.md").read_text(encoding="utf-8")

rules = """# Rules for every step

These apply to every step. A step session reads this file, `contracts.md`, `proofs.md` once it exists, and its own step file, and nothing else unless the step file says so.

## Starting a step

1. Check `INDEX.md`: every dependency must be `done`.
2. Work on a branch named `step-NN-name`, in its own worktree when another step is running.
3. Commit as biggodev: repo-local `user.name` is `Biggo Dev`, `user.email` is `biggodev@gmail.com`, and the remote is `git@github.com-biggodev:biggaloy/rust-morm.git`. Never use the neurobin account.

The coordinator owns `INDEX.md`: only it changes a status, so parallel steps never conflict over that file.

## Doing the work

- **The step file is the specification.** Build all of it. Don't drop, shorten or defer a requirement, and don't add features it doesn't ask for.
- **If the plan is wrong, stop.** When a requirement can't work, contradicts another or needs a design choice the file doesn't settle, stop and ask the owner with evidence and options. Don't improvise, and don't quietly build something different.
- **Nothing is stubbed.** No `todo!()`, no ignored tests, no "handled later" comments. Work that belongs to a later step is left out completely, and the step file's "Out of scope" says which. The one exception is step @contracts-freeze: frozen entry points on `Db`, `Tx` and `Conn` may call `unimplemented!("step NN")`, naming the step that fills them in, and that step removes it. Step @readme-release confirms none remain.
- **Warnings:** code that warns or logs before step @telemetry calls the internal warning sink in `contracts.md`, never `println!` or `tracing` directly.
- **Unknown behaviour gets tested.** When a requirement depends on how Postgres or a crate behaves, run it. Don't rely on memory.
- **Goals, in order:** raw performance, lowest memory, rock-solid migrations, hard to misuse. Hot paths have no needless allocation, dynamic dispatch or per-row state.
- **Secrets:** never log, print or commit passwords, tokens or parameter values.
- **Dependencies:** a new crate needs a reason in the PR.

## Tests

- Every `code` requirement and every test obligation ends with at least one test tagged in a comment, as `// covers: R-0123, T-0045`, in any language. A `trybuild` case carries the tag in its `.rs` file, and a CI requirement carries it in the workflow file.
- A tag means the test really exercises that requirement. Tagging a test that doesn't is a failed review.
- A requirement that holds several rules needs tests for each of them, not one test for the first.
- `info` and `doc` requirements need no tag.
- Integration tests run against real Postgres, never mocks.
- Performance claims are measured in step @benchmarks. A step that reports a cost states what it measured.

## When a step is done

All of these must be true:

1. Every requirement in the step file is built.
2. `plans/plan-approved/tools/check_coverage.py --step NN --code crates tests benches fuzz .github` passes.
3. `ci.yml` passes: `fmt`, `clippy -D warnings`, unit, `trybuild`, doc tests, and integration on Postgres 15–18.
4. The step's end-to-end check passes, and the PR shows the command and result.
5. Any new public call has its one-line "use this when" entry, ready for the README table in step @readme-release, listed in the PR body. It states the real use case from the reader's side, such as: `.allow_missing()`: you will not handle a 0-row error.
6. An independent review passes (below).
7. The PR is merged, and the coordinator marks the step `done`.

## Independent review

The session that wrote a step never approves it. A fresh agent reads `rules.md`, `contracts.md`, `proofs.md`, the step file and the diff, and checks, requirement by requirement:

- Is it built as written?
- Does each tagged test really prove it?
- What did the step skip, weaken or add?

It reports each finding with the requirement ID and evidence. The writing session fixes them, and the review repeats until none remain.

## PRs

- The owner approves the PRs of steps @bootstrap, @proofs, @contracts-freeze and @readme-release.
- Other PRs merge once CI and the independent review both pass.
- The PR body lists the requirements covered, the end-to-end result, and any owner decisions.
- Never publish crates, force-push or change repository settings without the owner's go-ahead.

## Changing the plan

`plan-approved/` is the source of truth, and `plans/rust-morm-plan.md` is frozen as the record. An owner-approved change is edited into the step file it affects, in the same PR, with a note in `INDEX.md` under Decisions. A new or edited requirement is updated in `coverage.md` and `tools/manifest.json` in the same PR, and a new one gets the next free ID. A change to a shared signature also updates `contracts.md`, and every in-progress step is told.

## Keeping sessions small

One step is one fresh session, sized to stay under about 30% of the context window. If a step can't fit, stop at a clean commit and ask the coordinator to split it. Don't read the frozen plan or other step files to fill in gaps. If a gap exists, that's a plan problem: stop and ask.
"""
(ROOT / "rules.md").write_text(sub(rules), encoding="utf-8")

contracts = """# Contracts

What parallel steps share. Signatures here are the starting point, taken from the approved plan. Step 02 corrects them from the proofs, and step 03 freezes them as compiling Rust. After step 03, changing anything here follows "Changing the plan" in `rules.md`.

## Crates

| Crate | Depends on | Holds |
|---|---|---|
| `morm-core` | no async or database crates | Code several crates need: config loading (`morm.toml`, environment), the 63-byte name function, the connection budget function, and the manifest types |
| `morm-macros` | `morm-core` | Procedural macros |
| `morm` | `morm-macros`, `morm-core`, `tokio-postgres`, `deadpool` | Runtime |
| `morm-migrate` | `morm-core`, `tokio-postgres` | Migration engine. Doesn't depend on `morm` |
| `morm-admin` | `morm-migrate`, `morm-core` | The binary. Doesn't depend on the user's models |
| `morm-mgr` | generated per project; `morm` and the user's models | Prints the manifest as JSON, and runs a migration's Rust data hooks |

## Conventions

- Rust 2024, MSRV 1.99.1, Tokio only, PostgreSQL 15 and later only.
- Tool prefix `morm-`. Config `morm.toml`. Environment `MORM_*` and `DATABASE_URL`. Migrations in `morm-migrations/`. Ledger schema `_morm`.
- Exit codes: `0` ok, `1` error or drift, `2` refused by a guard, `3` open decisions.
- Generated SQL always quotes names. Generated code uses static dispatch.
- Macro-generated items are public API: names below don't change without following `rules.md`.

## Shared types

```rust
// Implemented by #[model] and #[view]
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub trait Model: Sized + Send + Sync + 'static {
    type Pk: PkValue;                 // () for a model with no primary key
    type Global;                      // the app's global mask, from morm::groups!
    type Scoped;                      // scoped group mask, () when the model has none
    const TABLE: &'static str;
    const SELECT: &'static str;
    fn from_row(row: &tokio_postgres::Row) -> Result<Self>;
    fn from_row_named(row: &tokio_postgres::Row) -> Result<Self>;
}
pub trait HasPk: Model {}             // absent on models with no primary key
pub trait PkValue: Send + Sync + 'static {}   // one Param, or a tuple for a composite key

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
pub struct Mask<M: Model> { global: M::Global, scoped: M::Scoped }

// Connection-like: db, a held connection, or tx
pub trait Conn: Send + Sync { fn tx(&self) -> Option<&Tx>; /* every query entry point */ }

// Every hook has an empty default body, so an empty `impl Hooks` compiles away
pub trait Hooks: Model {
    fn validate(&self) -> Result<()> { Ok(()) }
    fn pre_insert(rows: &mut [Self], db: &impl Conn) -> impl Future<Output = Result<()>> + Send { async { Ok(()) } }
    fn post_insert(rows: &[Self], db: &impl Conn) -> impl Future<Output = Result<()>> + Send { async { Ok(()) } }
    fn validate_changes(changes: &Changes<Self>) -> Result<()> { Ok(()) }
    fn pre_update(changes: &mut Changes<Self>, target: &Target<Self>, db: &impl Conn) -> impl Future<Output = Result<()>> + Send { async { Ok(()) } }
    fn post_update(changes: &Changes<Self>, target: &Target<Self>, affected: u64, db: &impl Conn) -> impl Future<Output = Result<()>> + Send { async { Ok(()) } }
    fn pre_delete(target: &Target<Self>, db: &impl Conn) -> impl Future<Output = Result<()>> + Send { async { Ok(()) } }
    fn post_delete(target: &Target<Self>, affected: u64, db: &impl Conn) -> impl Future<Output = Result<()>> + Send { async { Ok(()) } }
    fn after_commit(event: Committed<Self>) -> impl Future<Output = Result<()>> + Send { async { Ok(()) } }
}
pub enum Target<M: Model> { Id(M::Pk), Ids(Vec<M::Pk>), Filter(/* condition */), All }
pub enum Change<'a, T> { Value(&'a T), Expr }
pub enum Committed<M: Model> {
    Inserted(Vec<M>),
    Copied { count: u64 },
    Updated { changes: Changes<M>, target: Target<M> },
    Upserted { inserted: u64, updated: u64 },
    Deleted { target: Target<M> },
}
pub enum Upserted { Inserted, Updated, Unchanged }
// db.upsert returns Upserted; db.upsert_many returns counts

pub enum Error {
    Unique { constraint: String, field: Option<&'static str> },
    ForeignKey, NotNull, Check, Serialization, Deadlock, LockTimeout,
    TransactionTimeout, TransactionAborted, CommitUnknown, NestedConnection,
    PoolerMode, NotFound, EmptyPatch, Conflict, Forbidden, Validation,
    PoolBusy, ConnectionLost, ReadOnly, OutOfRange, InvalidText,
    InvalidOrder, InvalidCursor, Decimal,
    Config, Connect, ServerTooOld, Postgres,   // config, connecting, a server below 15, any other database error
    // payloads settled in step @contracts-freeze
}
```

## Defined in step @contracts-freeze

These are shared by steps that can run side by side, so each is written and frozen before any of them starts:

| Item | Lives in | Shared by |
|---|---|---|
| `Db`, `Tx` and every `Conn` entry point | `morm` | All runtime steps |
| `Changes<M>` | `morm` | Updates, hooks |
| The `by`/`any` filter builder | `morm` | Reads, updates |
| The internal query entry point, which runs SQL through the caches | `morm` | Pool, statements, every query step |
| Macro field IR: the parsed form of `#[model]`, `#[col]` and `#[view]` | `morm-macros` | All model steps |
| The conversion layer between a field and its column | `morm` | Models, types, reads, writes |
| `ModelMeta`, the metadata every model emits | `morm-core` | Models, manifest, migrations |
| The name function, with the 63-byte rule | `morm-core` | Macros, migrations |
| The connection budget function, implemented in step @pool | `morm-core` | Pool, `doctor`, `size` |
| `Timestamp`, re-exported `Uuid`, `Range<T>` as types | `morm` | Bases and inference, before `types` adds their encoding |
| The internal transaction with confirmed commit, implemented in step @inserts for `insert_many` and chunked paths, before `db.transaction` exists | `morm` | Inserts, updates, upsert, transactions |
| `PoolStats`, with every counter the plan names | `morm` | Pool, hold-pooler, locks, outbox, telemetry |
| The internal warning sink: stderr now, `tracing` when enabled | `morm-core` | Every step that warns |
| Expansion layout: `#[model]` calls one module per feature, so model steps add a module and don't edit one shared function | `morm-macros` | Model steps, `manifest`, `rolling` |
| The config structs | `morm-core` | `morm`, `morm-migrate`, `morm-admin` |

`Op`, the typed migration operation, is defined in step @schema-diff. Migration steps run one at a time, so it needs no earlier freeze.

## Between `morm-mgr` and `morm-admin`

`morm-mgr` prints one JSON manifest: `format`, then models (table, schema, columns, types, nullability, defaults, flags, indexes, constraints, `was` hints, comments), managed objects, extensions, and registered `sql!` queries. Step @manifest defines it, in `morm-core`. Everything else reads it, and it's versioned by `format`.

## `morm-admin` layout

Each command is its own module, registered in one table, so steps adding commands don't edit the same code. Step @lint creates this dispatcher. It uses no CLI framework.

## Approved key decisions

These are context for every step. Details live in the step files.

""" + info_c
(ROOT / "contracts.md").write_text(sub(contracts), encoding="utf-8")

rows = []
for k, s in STEPS.items():
    d = ", ".join(f"{STEPS[x]['n']:02d}" for x in s["deps"]) or "none"
    rows.append(f"| {s['n']:02d} | [{s['title']}](steps/{s['n']:02d}-{k}.md) | {s['phase']} | {d} | todo |")

waves = {}
for k in STEPS:
    waves.setdefault(wave(k), []).append(f"{STEPS[k]['n']:02d}")
wave_lines = "\n".join(f"{i}. {', '.join(v)}" for i, v in sorted(waves.items()))

index = f"""# rust-morm: approved plan

This folder is the source of truth for building rust-morm. `../rust-morm-plan.md` is frozen as the record. `../app_suggestions.md` holds advice for apps, and `../later-defo-build.plan` holds confirmed work after 0.1.

| File | Holds |
|---|---|
| `rules.md` | How every step works, and when it's done |
| `contracts.md` | Shared types and conventions, plus the key decisions |
| `coverage.md` | Every requirement and test obligation, with its step |
| `steps/` | One self-contained file per step |
| `tools/` | The coverage checker, and the scripts that made this split |

The frozen plan was split word for word into requirements (`R-nnnn`) and test obligations (`T-nnnn`), each in exactly one place. A few long blocks were cut at a sentence so each part sits in the step that can build it, and a few of those parts were reworded to stand alone. `tools/check_coverage.py` fails if an item isn't in exactly one place, its text has changed, or the frozen plan has changed, and with `--code` it fails if a finished step has an untagged one.

## Steps

| Step | Title | Phase | Depends on | Status |
|---|---|---|---|---|
{chr(10).join(rows)}

Status is one of `todo`, `in progress`, `in review` or `done`. Only the coordinator changes it.

## Order

Each wave below needs only earlier waves. Each step file's "Can run beside" line says which steps it can run with: no dependency path between them, and no shared area of the code. Steps that aren't listed there run one after another, even in the same wave.

{wave_lines}

Steps run in parallel only when `contracts.md` covers what they share. Each gets its own worktree and branch. When a parallel step needs something shared that isn't in `contracts.md`, it stops, and the contract is updated first.

## The coordinating session

The coordinator stays thin: it reads this file and never reads code.

1. Pick steps whose dependencies are all `done`, and mark them `in progress`.
2. Start a fresh agent for each one, with this prompt: read `plans/plan-approved/rules.md`, `contracts.md`, `proofs.md` and `steps/NN-name.md`, then do the step.
3. When it opens a PR, mark it `in review` and start a fresh review agent as `rules.md` describes.
4. Relay findings until the review is clean, then merge or ask the owner, as `rules.md` says.
5. Mark the step `done`. Take any design question straight to the owner.

## Decisions since the split

- **`morm-core` added:** a sixth crate for config loading, the name and budget functions and manifest types, because `morm-migrate` and `morm-admin` can't depend on `morm` but need all three. Confirmed by the owner.
- **Rust data hooks run from `morm-mgr`:** owner decision. `migrate --hooks ./morm-mgr` calls a binary built in CI and shipped with the migrations, so `morm-mgr` is no longer print-only. Dropping hooks from 0.1 was the alternative.
- **`db.upsert_many` named:** the plan said "batch upsert" with no call. Confirmed by the owner.
- **`morm::pg_outbox!()` given its own requirement,** because the only text about it was an example.
- **Doctor, `lint` and `import` rules gathered:** sentences about them moved from their topic into those tools' own steps, so no earlier step depends on a later tool.

## Record from the frozen plan

Superseded milestones, later work and build decisions, kept for traceability. Nothing here is a step requirement.

""" + info_i
(ROOT / "INDEX.md").write_text(sub(index), encoding="utf-8")
