# Step 24: Hooks and write ordering

- **Phase:** Runtime
- **Depends on:** 22, 09
- **Can run beside:** 08, 23, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 51, 52, 53, 54, 57, 58, 59, 60
- **Crates:** `morm`, `morm-macros`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Hooks with exact inputs and order on `insert`, `update` and `delete`.

## Deliverables

- `Hooks`, `#[morm::hooks]`, `Changes`, `Target`, `Committed`, `validate`, `validate_changes`, lazy `db`, `hooks(tx)`, `after_commit`, `db.tx()`, `blocking`, rule ordering, pipelined `BEGIN` and `COMMIT`

## End-to-end check

A recording model proves the exact order and inputs on `insert`, `insert_many`, `update` and `delete`, including no connection during `clean`, and `after_commit` firing only on a confirmed commit.

## Out of scope

- Upsert, batches and shutdown (step 25)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Models / Struct options (`#[model(...)]`)

| ID | Option | Meaning |
|---|---|---|
| R-0153 | `hooks` | the model has its own `impl Hooks`; `hooks(tx)` also runs its statements in a transaction |

### Models / Generated per model

- **R-0222** an empty `impl Hooks` (unless `hooks` is set)

### Views and groups / Views: restricted models

- **R-0237** Only the definition is migrated. `clean` and declarative rules apply to every view. `impl Hooks` is per type, so a write through a view doesn't run the full model's hooks. A rule that must hold for every write belongs in `clean`, a CHECK or a trigger.

### Views and groups / Public input and output

- **R-0253** `.apply()` goes through hooks, `clean`, rules, `Changes<Self>` and group masks exactly as `.set()` does, and `clean` runs only on fields that were sent.

### Validation

- **R-0293** `Hooks::validate(&self)` handles rules across fields on insert, and `Hooks::validate_changes(&Changes<Self>)` on update.
- **R-0294** A rule that needs the whole row on update must be a `CHECK` constraint, or its hook fetches the row visibly.
- **R-0296** **Runs only on write,** and on updates only for values passed to `.set()`. Reads are never validated. `clean`, declarative rules and `validate` run per row on bulk inserts, and they do no I/O.
- **R-0297** **Runs once,** in order: `clean` and declarative rules, then pre hooks, then `validate`, then SQL. `clean` comes first so that slow work never holds a connection: it runs before one is checked out.
- **R-0298** **Trade-off:** `clean` runs even if a pre hook then rejects the request, so a slow hash can be spent on it. Do cheap rejections, such as checking whether an email is taken, in the handler before `insert`. The unique constraint is still the real guarantee, and endpoints running a slow `clean` need rate limits.
- **R-0299** A value a pre hook sets through `changes.set` runs through that field's `clean` and rules as it's set. On a `blocking` field `changes.set` is a compile error, and `changes.set_blocking(...).await` is the form to use. Fields `pre_insert` changes on a row are the hook's responsibility, and `validate` still sees them.
- **R-0300** **`clean` changes the row you passed.** After a failed `insert`, `p.password` already holds the hash, so inserting `p` again would hash it twice. Build the row again, as `.retry()` closures do.
- **R-0301** **`blocking`** runs a slow `clean`, such as password hashing, through `spawn_blocking`, because 50–100ms of CPU on an async thread stalls other requests. Inside `db.transaction` the connection is already held, so hash before opening the transaction.

### Reading and writing / Transactions

- **R-0440** `after_commit` fires only after a confirmed commit. If the connection is lost during `COMMIT`, the outcome is unknown, so the result is `CommitUnknown`, with no retry and no `after_commit`.

**R-0443** Each attempt starts with no queued `after_commit` events.

### Reading and writing / Deadlocks

- **R-0451** `after_commit` runs after the connection is released, so it can't cause this.

**R-0462** Retry covers only the SQL, so hooks don't run again and `RETURNING` fills the row only on success.

**R-0463** `hooks(tx)` statements are transactions, so they follow `.retry(n)`.

### Reading and writing / JSON output and schemas

- **R-0482** **Input:** a view with `input` or `patch` is the request body (section 5). `insert` and `update` run `clean` and the rules. No hand-written input type is needed.

### Hooks

**R-0488** Hook signatures show exactly what's available. **Update and delete hooks take no `self`, so they never see a row unless they fetch it.** Nothing is ever fetched automatically.

**R-0489**

```rust
#[morm::hooks]
impl Hooks for Product {
    // Insert: you built the rows, so hooks get them. One row for `insert`, all of them for `insert_many`
    fn validate(&self) -> Result<()>;                                 // per row
    async fn pre_insert(rows: &mut [Self], db: &impl Conn) -> Result<()>;
    async fn post_insert(rows: &[Self], db: &impl Conn) -> Result<()>;   // includes id and defaults

    // Update: only what was set, and what was targeted
    fn validate_changes(changes: &Changes<Self>) -> Result<()>;
    async fn pre_update(changes: &mut Changes<Self>, target: &Target<Self>, db: &impl Conn) -> Result<()>;
    async fn post_update(changes: &Changes<Self>, target: &Target<Self>, affected: u64, db: &impl Conn) -> Result<()>;

    // Delete: only what was targeted
    async fn pre_delete(target: &Target<Self>, db: &impl Conn) -> Result<()>;
    async fn post_delete(target: &Target<Self>, affected: u64, db: &impl Conn) -> Result<()>;

    async fn after_commit(event: Committed<Self>) -> Result<()>;
}
```

**R-0490** `Changes<Self>` holds only the columns passed to `.set()`:

**R-0491**

```rust
match changes.get(F.price) {
    Some(Change::Value(price)) => { /* set in this update */ }
    Some(Change::Expr)         => { /* set_expr: the value exists only in SQL */ }
    None                       => { /* not in this update; the row's value is unknown */ }
}
changes.set(F.slug, slug);   // a pre hook can add or change a value
```

**R-0492** `Target<Self>` says which rows are affected, and every variant must be handled: `Id(id)`, `Ids(ids)`, `Filter(f)` (unknown rows, only the condition is known), or `All`.

### Hooks / Order

| ID | Operation | Order |
|---|---|---|
| R-0493 | `insert` | `clean` and rules → `pre_insert` → `validate` → **INSERT** → `post_insert` → *(commit)* → `after_commit` |
| R-0494 | `update` | `clean` and rules → `pre_update` → `validate_changes` → **UPDATE** → `post_update` → *(commit)* → `after_commit` |
| R-0496 | `delete` | `pre_delete` → **DELETE** → `post_delete` → *(commit)* → `after_commit` |

### Hooks / Rules

- **R-0497** **A hook needing the full row** fetches it itself, so the query is visible in the code.
- **R-0498** **Hooks run once per statement,** never per row, so a bulk write can't become thousands of hook calls. `pre_insert` and `post_insert` take a slice for that reason: `insert` passes one row with no allocation, and `insert_many` passes all of them. `insert_stream` calls them once per chunk.
- **R-0499** **`#[morm::hooks]`** on the `impl` records which hooks are overridden, because stable Rust can't detect that. It's what lets a model skip an unused transaction, boxing or pipelining check at compile time.
- **R-0501** **The `db` a hook receives is lazy.** A connection is checked out on its first query, and with `hooks(tx)` the transaction begins there too, so a hook that doesn't query holds nothing.
- **R-0502** **SQL is built after pre hooks,** so changes made there apply. A pre hook can cancel the operation by returning an error.
- **R-0503** **Post hooks** run on the same connection or transaction. Without `hooks(tx)` or a surrounding transaction, the statement has already committed, so a post hook's error can't undo it. Use `hooks(tx)` when it must.
- **R-0504** A hook that needs `tx.emit` calls `db.tx()`, which returns the transaction when there is one, since `emit` isn't on `Conn`.
- **R-0505** **`after_commit`** is for side effects (emails, queues, cache invalidation). It gets an owned enum: `Inserted(rows)`, `Copied { count }` for `insert_stream`, which can't keep its rows, `Updated { changes, target }`, `Upserted { inserted, updated }` or `Deleted { target }`. It's boxed only for models overriding it, runs only after commit, and is skipped on rollback. Errors there can't undo the commit. It runs in the app process, so a crash right after commit loses it. Events that must not be lost use `tx.emit` (below).
- **R-0506** **Automatic transaction:** a model with `hooks(tx)` has each statement wrapped in a transaction. Others stay at one round trip.
- **R-0507** **Raw SQL doesn't run hooks;** this is documented.
- **R-0508** **Zero cost:** static dispatch, and empty default hooks compile away.
- **R-0509** **Axum-safe:** signatures use `-> impl Future<Output = Result<()>> + Send` underneath, and users can still write `async fn`.

### Performance and memory / Query path

- **R-0724** `BEGIN`, statement and `COMMIT` pipelined to cut auto-transaction round trips. This applies only to models without post hooks, because `post_insert` and `post_update` must run before the `COMMIT`.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0784 | Hashing on an async thread, or while holding a connection | `clean` runs before checkout, and `blocking` moves it off the runtime |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0013 | trybuild | `changes.set` on a `blocking` field |
| T-0017 | trybuild | `self` absent in update and delete hooks |
| T-0038 | integration | hook inputs (`Changes`, `Target`) |
| T-0039 | integration | hooks order |
| T-0041 | integration | `after_commit` on commit and skip on rollback |
| T-0080 | integration | hook slices on `insert` and `insert_many` |
| T-0101 | integration | `clean` and lazy connections |
| T-0102 | integration | `blocking` |
| T-0291 | trybuild | **`#[morm::hooks]` refuses a macro call** among the impl's items, with the message that hooks a macro writes would never run, and refuses an `OVERRIDES` written inside the impl |
| T-0292 | trybuild | **`Overrides` can't be written by hand:** an `impl Hooks` without `#[morm::hooks]` fails with `missing 'OVERRIDES'`, `Overrides` has no public constructor or constant, and another model's `OVERRIDES` doesn't fit, each a pinned compile error |
