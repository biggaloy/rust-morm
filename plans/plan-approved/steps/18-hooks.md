# Step 18: Hooks and write ordering

- **Phase:** Runtime
- **Depends on:** 13, 14, 15, 08
- **Can run beside:** nothing
- **Crates:** `morm`, `morm-macros`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Hooks with exact inputs and order, on every write path.

## Deliverables

- `Hooks`, `#[morm::hooks]`, `Changes`, `Target`, `Committed`, `validate`, `validate_changes`, lazy `db`, `hooks(tx)`, `after_commit`, `db.tx()`, `blocking`, rule ordering

## End-to-end check

A recording model proves the exact order and inputs on every write path, including no connection during `clean`, and `after_commit` firing only on a confirmed commit.

## Out of scope

- Outbox (step 20)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Models / Struct options (`#[model(...)]`)

| ID | Option | Meaning |
|---|---|---|
| R-0146 | `hooks` | the model has its own `impl Hooks`; `hooks(tx)` also runs its statements in a transaction |

### Validation

- **R-0275** `Hooks::validate(&self)` handles rules across fields on insert, and `Hooks::validate_changes(&Changes<Self>)` on update.
- **R-0276** A rule that needs the whole row on update must be a `CHECK` constraint, or its hook fetches the row visibly.
- **R-0279** **Runs once,** in order: `clean` and declarative rules, then pre hooks, then `validate`, then SQL. `clean` comes first so that slow work never holds a connection: it runs before one is checked out.
- **R-0280** **Trade-off:** `clean` runs even if a pre hook then rejects the request, so a slow hash can be spent on it. Do cheap rejections, such as checking whether an email is taken, in the handler before `insert`. The unique constraint is still the real guarantee, and endpoints running a slow `clean` need rate limits.
- **R-0281** A value a pre hook sets through `changes.set` runs through that field's `clean` and rules as it's set. On a `blocking` field `changes.set` is a compile error, and `changes.set_blocking(...).await` is the form to use. Fields `pre_insert` changes on a row are the hook's responsibility, and `validate` still sees them.
- **R-0282** **`clean` changes the row you passed.** After a failed `insert`, `p.password` already holds the hash, so inserting `p` again would hash it twice. Build the row again, as `.retry()` closures do.
- **R-0283** **`blocking`** runs a slow `clean`, such as password hashing, through `spawn_blocking`, because 50–100ms of CPU on an async thread stalls other requests. Inside `db.transaction` the connection is already held, so hash before opening the transaction.

### Hooks

**R-0452** Hook signatures show exactly what's available. **Update and delete hooks take no `self`, so they never see a row unless they fetch it.** Nothing is ever fetched automatically.

**R-0453**

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

**R-0454** `Changes<Self>` holds only the columns passed to `.set()`:

**R-0455**

```rust
match changes.get(F.price) {
    Some(Change::Value(price)) => { /* set in this update */ }
    Some(Change::Expr)         => { /* set_expr: the value exists only in SQL */ }
    None                       => { /* not in this update; the row's value is unknown */ }
}
changes.set(F.slug, slug);   // a pre hook can add or change a value
```

**R-0456** `Target<Self>` says which rows are affected, and every variant must be handled: `Id(id)`, `Ids(ids)`, `Filter(f)` (unknown rows, only the condition is known), or `All`.

### Hooks / Order

| ID | Operation | Order |
|---|---|---|
| R-0457 | `insert` | `clean` and rules → `pre_insert` → `validate` → **INSERT** → `post_insert` → *(commit)* → `after_commit` |
| R-0458 | `update` | `clean` and rules → `pre_update` → `validate_changes` → **UPDATE** → `post_update` → *(commit)* → `after_commit` |
| R-0459 | `upsert` | `clean` and rules → `pre_insert` → `validate` → **UPSERT** → `post_insert` or `post_update`, or neither if unchanged → *(commit)* → `after_commit` |
| R-0460 | `delete` | `pre_delete` → **DELETE** → `post_delete` → *(commit)* → `after_commit` |

### Hooks / Rules

- **R-0461** **A hook needing the full row** fetches it itself, so the query is visible in the code.
- **R-0462** **Hooks run once per statement,** never per row, so a bulk write can't become thousands of hook calls. `pre_insert` and `post_insert` take a slice for that reason: `insert` passes one row with no allocation, and `insert_many` passes all of them. `insert_stream` calls them once per chunk.
- **R-0463** **`#[morm::hooks]`** on the `impl` records which hooks are overridden, because stable Rust can't detect that. It's what lets a model skip an unused transaction, boxing or pipelining check at compile time.
- **R-0464** **`update_each`:** `pre_update` runs once with `Target::Ids` and only the column list. Per-row values go through `clean` and the declarative rules.
- **R-0465** **The `db` a hook receives is lazy.** A connection is checked out on its first query, and with `hooks(tx)` the transaction begins there too, so a hook that doesn't query holds nothing.
- **R-0466** **SQL is built after pre hooks,** so changes made there apply. A pre hook can cancel the operation by returning an error.
- **R-0467** **Post hooks** run on the same connection or transaction. Without `hooks(tx)` or a surrounding transaction, the statement has already committed, so a post hook's error can't undo it. Use `hooks(tx)` when it must.
- **R-0468** A hook that needs `tx.emit` calls `db.tx()`, which returns the transaction when there is one, since `emit` isn't on `Conn`.
- **R-0469** **`after_commit`** is for side effects (emails, queues, cache invalidation). It gets an owned enum: `Inserted(rows)`, `Copied { count }` for `insert_stream`, which can't keep its rows, `Updated { changes, target }`, `Upserted { inserted, updated }` or `Deleted { target }`. It's boxed only for models overriding it, runs only after commit, and is skipped on rollback. Errors there can't undo the commit. It runs in the app process, so a crash right after commit loses it. Events that must not be lost use `tx.emit` (below).
- **R-0470** **Automatic transaction:** a model with `hooks(tx)` has each statement wrapped in a transaction. Others stay at one round trip.
- **R-0471** **Raw SQL doesn't run hooks;** this is documented.
- **R-0472** **Zero cost:** static dispatch, and empty default hooks compile away.
- **R-0473** **Axum-safe:** signatures use `-> impl Future<Output = Result<()>> + Send` underneath, and users can still write `async fn`.

### Hard to misuse

| ID | Mistake | Caught by |
|---|---|---|
| R-0733 | Hashing on an async thread, or while holding a connection | `clean` runs before checkout, and `blocking` moves it off the runtime |

## Test obligations

| ID | Type | Obligation |
|---|---|---|
| T-0010 | trybuild | `changes.set` on a `blocking` field |
| T-0014 | trybuild | `self` absent in update and delete hooks |
| T-0033 | integration | hook inputs (`Changes`, `Target`) |
| T-0034 | integration | hooks order |
| T-0036 | integration | `after_commit` on commit and skip on rollback |
| T-0075 | integration | hook slices on `insert` and `insert_many` |
| T-0088 | integration | `upsert` outcomes, hooks and `after_commit` on each Postgres version |
| T-0096 | integration | `clean` and lazy connections |
| T-0097 | integration | `blocking` |
