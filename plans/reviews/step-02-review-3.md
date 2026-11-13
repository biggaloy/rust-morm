# Step 02 independent review, round 3

PR #2 (`step-02-proofs`), head `4a3a265`. This is the third review, after two rounds of fixes, done as `rules.md` "Independent review" says, by an agent that didn't write the step.

I read `rules.md`, `contracts.md`, `proofs.md` and `steps/02-proofs.md` on the PR branch. On `origin/main` (`beeffad`) I read:

- every INDEX "Decisions since the split" entry from "Step 02's proofs, DA to DE" onward: DA to DE, DF1 to DF10, DG1 to DG4, DH1 and wall-clock time, DH2/DH3/DH5, parameter typing A to K, and DI1 to DI4;
- `reviews/step-02-review.md`, `reviews/step-02-review-2.md`, `reviews/param-typing-design.md` and `reviews/step-02-open-items.md`;
- `later-defo-build.plan`, "Wall-clock time".

I also read the whole diff, `git diff origin/main...origin/step-02-proofs`. It touches only `plans/`: `contracts.md`, `coverage.md`, `proofs.md`, the manifest, and steps 10, 12 to 22, 24, 41, 43, 45, 50, 55 and 56. To check the new text against the steps it lands in, I also read step 05 (R-0186 to R-0203) and the quoted copies of R-0060 in steps 11, 12, 27 and 47.

## What I checked and ran

- **CI on `4a3a265`:** `ci` passed on both its push and pull request runs (43331782016, 43331790916). That covers every job: fmt, clippy, unit, trybuild, doc, integration on Postgres 15 to 18 over TCP and Unix sockets, pooler, msrv, features and coverage. There's no `spikes` workflow, as DE says.
- **Checker:** `check_coverage.py --step 02 --code crates tests benches fuzz .github` and the plain form both report **1235 requirements, 298 test obligations, 0 problems**.
- **Manifest:** `git diff origin/main...origin/step-02-proofs -- plans/plan-approved/tools/manifest.json`. I also compared the two manifests by ID with a script.
  - Changed: R-0350, R-0387, R-0412, R-0415, R-0431, R-0433, R-0435, R-0436, R-0442, R-0453, R-0670, R-0716, R-0718, R-0767, R-0865, R-0961, R-0978, R-0984, R-1104, R-1117, R-1119, T-0021, T-0148, T-0151 and T-0222.
  - Added: R-1205 to R-1235, and T-0278 to T-0298.
  - Nothing was removed, and no ID changed its step or kind.
  - INDEX's Decisions covers every one. The DF entry names R-0435, R-0442, R-0453, R-0767 and R-0865. The A to K entry names the rest up to R-1233 and T-0296. The DI1/DI2 entry names R-0670, R-1210, R-1234, R-1235, T-0297 and T-0298. The DI3 note names R-1117, R-1119, R-1210, R-1234 and T-0297.
  - R-0060 is the same as on `main`: DH1 took back DG2's `timezone` key, so the copies of R-0060 in steps 11, 12, 27 and 47 still match. R-0670's copy in step 43 matches step 12.
- **Spikes (DE, item 4):**
  - `origin/step-02-proofs` has no `spikes/` and no `spikes.yml`.
  - `9bd63f7` deletes them. It changes 72 files: everything under `spikes/`, `spikes.yml` and `proofs.md`. Outside `plans/`, the head is the same as `main`.
  - `proofs.md` names `ca3747b38fe66e28b579d9bae5e5e9632946049b` as the last commit with the spikes. Its `spikes` runs passed on Postgres 15, 16, 17 and 18, push (43320908197) and pull request (43320920564), and so did its `ci` runs.
  - The pull request log shows PostgreSQL 15.20, 16.16, 17.12 and 18.7, as proof 9 says.
  - I extracted `spikes/` from `ca3747b` into scratch and read `p09_matrix.rs` and `p09_matrix.expected`. The pinned table agrees with `proofs.md`'s matrix, except one row (DJ9).
- **The zone claims behind DI3 and DI4,** checked on throwaway Postgres 18.6 and 17.11 servers whose own `timezone` was `Asia/Dhaka`, with each name sent as a startup option. Both versions gave the same results:
  - `UTC` and `utc` read back `UTC`, `etc/utc` reads `Etc/UTC`, and `ZULU` reads `Zulu`.
  - `UTC0` and `utc0` read back `UTC0`. So does a role set to `utc0`.
  - `UTC+0`, `GMT`, `Greenwich`, `Etc/GMT-0` and `UCT` read back as written.
  - `+00` reads `<+00>-00`.
  - Every one of them showed offset 0 in 1850, with abbreviation `UTC` or `GMT`, or `+00` for `+00`.

  This matches R-1234 and proofs.md's DI3 and DI4. The servers are stopped and their data deleted.
- **PgBouncer's handling of `TimeZone`:** I read PgBouncer's documentation, `config.html` (`ignore_startup_parameters`, `track_extra_parameters`) and `features.html` (DJ2). PgBouncer isn't installed here, so I didn't run it.
- **Stale text (item 2):** I searched `plan-approved/` for `UNKNOWN`, `ParamType`, `pg_type()`, `has_text`, `encode_text`, text format, `MORM_DB_TIMEZONE`, `timezone`, "from `Param`", postfix `.retry`/`.isolation`, `|tx| async move`, `db.sql("..", &[..])`, `db.raw::<T>("..")`, `sql!` with `$n`, and wall-clock time.
  - **None is live.** What's left is history in `proofs.md`: "What came before, superseded", and the DF1, DG and DH lines of "Owner decisions on step 02", each of which says what superseded it.
  - R-1206 names `MORM_DB_TIMEZONE` and `timezone` only to say they don't exist.
  - R-0350's "text format" means array literals sent as text values, which is still how it works.
  - R-0917 and step 02's proof list are word-for-word copies of the frozen plan.

## The second review's findings

| Finding | Fixed as recommended or decided? | Evidence |
|---|---|---|
| DH1 | Yes, the owner's option C | R-1205 (every connection sends `TimeZone = 'UTC'`, nothing overrides it). R-1206 (`info`, no setting or variable). The conversion comment in `contracts.md`, without "so raw SQL converts times the same way" as a claim about a configured zone. R-1207 ("No answer depends on the session's `TimeZone`"). R-1210, a loud notice. R-0670 and its copy in step 43 (role `timezone = 'UTC'`). Wall-clock time is in `later-defo-build.plan` only. DI1 to DI4 add R-1234, R-1235, T-0297 and T-0298. See DJ2 for the PgBouncer premise. |
| DH2 | Yes, superseded by the parameter-typing design | DG3's untyped sending is gone everywhere. A list where one value is expected now fails with `42883` (R-1209, T-0278, pinned at `ca3747b`). |
| DH3 | Yes, option A | R-1208 (through `SqlType`, a const assertion, aliases resolve, an unknown `sql` only on a type with a text form, an error naming the field), R-1230 (checked again at startup), T-0280. See DJ1 and DJ5. |
| DH4 | Yes | T-0278 runs straight on `tokio_postgres` in step 13 and says so (step 13's "Out of scope" too). T-0286 in step 18 covers `.by`, `.any`, `.qc`, `get_where`, `insert`, `insert_many`'s `UNNEST`, `update().set()`, `update_each`, `delete_where` and `sql!` through `db.sql()`, each at least twice, with the array elements asked for. T-0296 in step 45 covers check-sql and `#[morm::test]`. The format wrapper is gone, so no step needs to own it. See DJ8. |
| DH5 | Yes, option A | R-0436, `contracts.md`'s `impl Tx`, T-0287, T-0288, R-1216 and R-1219. |
| DH6 | Yes | `Param`'s comment is now `SqlType`'s and `Param`'s, saying every type with no fixed OID, arrays included, travels as text or its base type's carrier, cast to its name. |
| DH7 | Yes | T-0289 (step 20), T-0290 (step 21), T-0291 and T-0292 (step 24), in `coverage.md` and the manifest. See DJ11. |
| DH8 | Yes | R-1210 and R-1211 are loud notices, and R-1217 is `.statement_timeout(...)`'s line. R-1215, R-1216, R-1218 and R-1219 are added too. See DJ6. |
| DH9 | Yes for its two lines | Both stale lines are gone. The body has new stale lines (DJ10). |

## The parameter-typing design, A to K, and DI1 to DI4

Each choice is written in, and agrees across `contracts.md`, `proofs.md` and the steps:

- **A, values inline:** R-1223 and R-0412's examples in step 16, and step 15's `db.raw` example.
- **B, `db.sql()` and `db.raw()` take only `sql!`:** R-0415 and T-0021.
- **C, carriers:** `contracts.md`, R-1220, R-0978, R-0961 and R-1233.
- **D, a free value beside an `F` column:** R-1224 and T-0284.
- **E, the parse-tree check:** R-0431, R-1231 and T-0294.
- **F, casts in fragments:** R-0433, R-1221 and T-0281.
- **G, `comp` through its field:** R-1222, R-1227 and T-0283.
- **H, `#[morm::test]`:** R-1232 and T-0295.
- **I, the retry:** R-0718, R-0984, T-0151 and T-0288.
- **J, the rerun:** proof 9 at `ca3747b`.
- **K, `Option` fits only `Option`:** R-1214 and T-0280.

The design's section 10 table is covered row by row, including the step 41 `to_regtype` check (R-1230, T-0293) and `set_local`'s cast (R-1229).

- **DI1:** R-1234 and T-0297.
- **DI2:** R-1235 and T-0298 (see DJ7).
- **DI3:** R-1234's list, with R-1117 and R-1119 saying the zone isn't checked when `check_schema = "off"`.
- **DI4:** `UTC0` alone among POSIX strings, in R-1234, T-0297 and R-1210.

Two places where the design isn't fully written in:

- The design's sentence "the README says so", for a free parameter's type having to be concrete (DJ6).
- The shape of `Sql`, which the design's prototype never built without boxing (DJ3).

## Findings

### DJ1. R-1213's `SqlType` table refuses fields that step 05 says compile

- **Severity:** should fix
- **IDs:** R-1213, R-1208; R-0201, R-0203, R-0198, R-0199 in step 05; T-0278, T-0286
- **Files:** `steps/13-types.md` (R-1213)

**What's wrong.**

- R-1208 makes `#[model]` refuse any field whose column type its Rust type doesn't list in `SqlType`'s `CARRIES`.
- R-1213 fills `CARRIES`:
  - "`i16`, `i32` and `i64` ... each also carrying every wider integer column and `numeric`";
  - "every other integer type morm maps as the column it maps to".
- So under R-1208:
  - **`u32` with `sql = "numeric"` is a compile error.** R-0201 gives exactly that field as its example of a widening that compiles. The same goes for any widening of `i8`, `u8`, `u16` or `u32`, which R-0201 allows.
  - **`u128` with `sql = "uuid"` is a compile error.** R-0203 recommends it as "a compact alternative for ids and hashes". No conversion step encodes or decodes a `u128` as `uuid` either: it's not in R-1207's list or T-0278.
  - **`char` (R-0199, `text` with a length CHECK) and `NonZero*` (R-0198, as the base type) aren't in R-1213 at all.** R-1208 says a type without `SqlType` "can't be a field".
  - **R-1213 doesn't say what `Vec<T>` carries.** It gives `Vec<T>`'s cast and carrier, but not its `CARRIES`. T-0278, T-0286 and proof 9 bind `Vec<String>` on `citext[]`, `char(5)[]` and `varchar(5)[]`, and `Vec<i16>` on `integer[]`. If `Vec<String>` carries only `text[]`, R-1208 refuses those fields.
- The PR body's "Choices to review" says integer fields carry `numeric` "because step 05's R-0201 lets `u32` widen to `numeric`". The text gives `numeric` only to `i16`, `i32` and `i64`, so it doesn't do what that choice says.

**Fix.** Reword R-1213:

- Every integer type morm maps, signed or unsigned, `NonZero*` included, carries every wider integer column and `numeric`, as R-0201 allows. Step 05's fit checks still decide which precisions hold the whole type.
- `u128` also carries `uuid`. Add that encoding to R-1207's list and T-0278, in step 13, or say which step builds it.
- `char` is `text`, carrying what `String` carries.
- `Vec<T>` and `&[T]` carry each of `T`'s column types with `[]`.

Add `u32` on `numeric` and `u128` on `uuid` to T-0278's cells, so the table can't drift from step 05 again.

### DJ2. PgBouncer accepts `TimeZone` as a startup parameter, so "morm can't send the zone" isn't true

- **Severity:** should fix, needs an owner decision
- **IDs:** R-1205, R-1210, R-0670 (and its copy in step 43), R-1234, T-0297; DH1, DI1
- **Files:** `steps/10-config.md`, `steps/12-hold-pooler.md`, `steps/41-rolling.md`, `steps/43-doctor.md`, `steps/50-readme-release.md`, `proofs.md` (DH1, DI1)

**What's wrong.**

- R-1210 tells users: "Behind PgBouncer in transaction mode morm can't send the zone, so set it on the role."
- R-1205 says that with `pooler = "transaction"` the parameter isn't sent.
- R-0670 gives the reason for sending no startup parameters: "PgBouncer rejects unknown ones by default".
- But `TimeZone` isn't unknown to PgBouncer. It's one of the parameters PgBouncer tracks per client and restores on every server connection the client gets, in transaction pooling too:
  - `config.html`, `ignore_startup_parameters`: "By default, PgBouncer allows only parameters it can keep track of in startup packets ... All other parameters will raise an error."
  - `config.html`, `track_extra_parameters`: the tracked set is `application_name`, `client_encoding`, `DateStyle`, `default_transaction_read_only`, `IntervalStyle`, `scram_iterations` (16 and later), `search_path` (18 and later), `session_authorization`, `standard_conforming_strings` and `TimeZone`. A client's value is "maintained in the client variable cache and restored in the server whenever the client becomes active".
  - `features.html`: "Startup parameters" are supported under transaction pooling, and footnote 1 lists `Timezone`.
- So morm could send `TimeZone = 'UTC'` through PgBouncer.
  - Then neither the role setup in R-0670 nor R-1234's refusal would usually come into play.
  - T-0297's first case, a Berlin role through PgBouncer, would then start.
- I couldn't run PgBouncer here, so this rests on its documentation. CI's `pooler` job, with PgBouncer 1.22 from Ubuntu or the pinned 1.26.1 image, can prove it either way.

**This needs an owner decision:**

- **A (recommended).**
  - With `pooler = "transaction"`, morm still sends `TimeZone = 'UTC'` (only that parameter), because PgBouncer tracks it.
  - R-1234 stays as the backstop, for poolers that don't track it, or a PgBouncer whose `ignore_startup_parameters` lists `timezone`.
  - R-0670 keeps the role setting as advice for other poolers.
  - R-1210 says the role setting is needed only when the pooler drops the parameter, and the pool refuses to start if it does.
  - T-0297 becomes: through PgBouncer with a Berlin role, a session reports `UTC` even after another client's `SET timezone` on the same server connection; with `ignore_startup_parameters = timezone`, the pool refuses to start, naming `Europe/Berlin`.
  - Step 12's `pooler` job proves both on PgBouncer 1.22 and 1.26.1.
- **B.** Keep sending no startup parameters through any pooler. Reword R-1205, R-1210 and R-0670 so they don't claim PgBouncer rejects it: morm sends none through a pooler so the same setup works with poolers that don't track it, and the role carries the zone. T-0297 stays as written.

### DJ3. `Sql<'a>` can't hold a per-query tuple, and `let q = sql!(..); db.sql(&q)` isn't proven without boxing

- **Severity:** should fix (step 03 freezes this signature)
- **IDs:** R-1223, R-0412, R-0415, T-0285; choice A
- **Files:** `contracts.md` (`Sql`), `steps/16-reads-more.md`

**What's wrong.**

- `contracts.md` declares `pub struct Sql<'a> { /* private */ }`, holding "the borrowed values, each as its carrier, in a tuple with no boxing". R-1223 says the same.
- A tuple's type differs from query to query. So a struct with no type parameter can hold it only by erasing it, as boxes or as a borrowed slice of trait objects.
- The design's prototype "boxed them for brevity" (`param-typing-design.md`, section 3). No spike and no prototype built the unboxed form.
- R-0412's example binds the query first and runs it twice: `let q = sql!(..);`, then `db.sql(&q)`. With a borrowed slice, the values must outlive the `let`.
  - Rust extends a temporary's life only through a struct, tuple or array literal or a `&` operand. A function call doesn't extend it.
  - So the shape of `sql!`'s expansion decides whether that example compiles.
- Each field-bound value is also "as its carrier", such as an `i32` sent as `int8`. That needs a wrapper value per parameter, which has to live somewhere.

**Fix.** Settle the shape in `contracts.md` before step 03 freezes it, with one of:

- `Sql<'a, P>`, where `P` is the tuple of carrier-tagged borrows and `db.sql()` and `db.raw()` are generic over it;
- an erased `Sql<'a>` holding `&'a [Bound<'a>]`, a concrete struct of `&'a dyn Fits..` plus its carrier, built by a struct literal so `let` extends its temporaries.

Then add to T-0285, or a trybuild case in step 16, that `let q = sql!(..)` followed by two `db.sql(&q)` calls compiles and runs with nothing boxed. Choosing between the two forms is a design choice. I recommend the generic `Sql<'a, P>`, because it keeps every value's encoding static, as R-1213 and the goals in `rules.md` ask.

### DJ4. `Fits<T>` must be object safe and `Sync`, and the contract doesn't say so

- **Severity:** minor
- **IDs:** R-1222, R-1227, R-1214; choice G
- **Files:** `contracts.md` (`Fits`)

**What's wrong.**

- R-1222 and R-1227 type `comp`'s values as `&[&dyn Fits<T>]`.
- So `Fits<T>` must be object safe. It must also be `Sync`, because the slice is held across an `.await` in a future that must be `Send` under Axum, the same class of failure proof 4 hit.
- `contracts.md` gives `pub trait Fits<T> { /* encode as the column's carrier */ }`, with no supertraits. `Param` gets `ToSql + Sync` spelled out.

**Fix.** Give `Fits<T>` its supertraits in `contracts.md`, such as `Fits<T>: Sync`, with object-safe methods. Add a line to T-0283, or a compile test, that a `get_where` with a comp value awaited inside `tokio::spawn` compiles.

### DJ5. R-1208 and R-1233 disagree on where `PgEnum`, `PgDomain` and `PgComposite` may sit

- **Severity:** minor
- **IDs:** R-1208, R-1233; DH3
- **Files:** `steps/13-types.md` (R-1208), `steps/56-enum-types.md` (R-1233), `contracts.md` (`SqlType`)

**What's wrong.**

- R-1208 accepts a `sql` naming a type morm doesn't know "only on a type with a text form (`String`, `&str`, and morm's enum, domain and composite types)".
- `contracts.md` says a type with `TEXT_FORM` "may also sit on a SQL type morm doesn't know, carried as text".
- R-1233 gives `PgEnum` and `PgComposite` "a text form", yet says they "carry only that type". It gives `PgDomain` "only the domain", and says nothing about its text form.
- Read together, a `PgEnum` field with `sql = "other_enum"`, or with `sql = "ltree"`, is accepted by R-1208 and refused by R-1233. Steps 13 and 56 would build different checks.
- A `PgDomain` over `i32` with a text form would be accepted on `ltree`, and sent as an `int4` carrier cast to `ltree`.

**Fix.** Say once which rule wins:

- R-1233 sets `TEXT_FORM` to false for `PgEnum`, `PgDomain` and `PgComposite`, because they carry only their own type.
- R-1208's parenthesis becomes "(`String`, `&str`, `Box<str>`, `Cow<str>`)", with morm's enum, domain and composite types sitting on their own type through `CARRIES`.

That matches DH3's option A, where the named types were meant to sit on their own types.

### DJ6. Some surprises the design hands users have no README line

- **Severity:** minor
- **IDs:** R-1223, R-1214, R-1208, R-1210; R-1211
- **Files:** `steps/50-readme-release.md`

**What's wrong.** These have no loud notice or README requirement:

- **A free value's type must be concrete.** `param-typing-design.md`, section 3, says "A free parameter's type must be concrete, and the README says so". R-1223 has the `E0401` failure, but no README requirement carries it. rustc's `can't use generic parameters from outer item` doesn't tell a user that the fix is to name a concrete type or bind through a field.
- **An `Option` value binds only through a nullable field (K).** Comparing an `Option<i64>` with a `NOT NULL` key is a compile error, with ``the trait bound `Option<i64>: Fits<i64>` is not satisfied``. Nothing says why, or what to do: unwrap the value, or use `IS NOT DISTINCT FROM` with a free value.
- **A plain integer field on a domain made by hand is refused at compile time** under R-1208, so such a column needs `PgDomain`. That's what T-0142 asks for text domains, but nothing says it for others.
- **R-1210's notice doesn't name what a user will actually see.** It states the rule, then says what UTC changes: `current_date`, `now()::date`, `localtimestamp` and `date_trunc('day', now())` give UTC days, and `timestamptz` text, in `fetch_raw` and `psql` alike, renders as `+00`. It has no "What you give up" part as R-1119 has: no session-local zone, and with it every SQL default that leans on one.

**Fix.**

- Add the first three to R-1211 or to a new `doc` requirement beside `sql!`, each with how to go on.
- Add a "What you give up" line to R-1210 naming those functions and the `+00` output.

### DJ7. Two DI gaps: `db.check_schema()` and the zone, and the tables R-1235 covers

- **Severity:** minor
- **IDs:** R-1234, R-1116, R-1119, R-1235; DI1, DI2
- **Files:** `steps/41-rolling.md`, `steps/50-readme-release.md`, `steps/10-config.md`

**What's wrong.**

1. **`db.check_schema()`.**
   - R-1116 says `db.check_schema()` "runs the same check whenever it's called, whatever R-1117's setting says".
   - R-1119's notice, under "What you give up", says the zone isn't checked with the check off, then "`db.check_schema()` still runs the check whenever you call it".
   - R-1234 doesn't say whether `db.check_schema()` reads the zone too. If it does, it doesn't say whether a non-UTC zone comes back as `Error::Config` or as a finding in `Error::SchemaMismatch`.
2. **R-1235's tables.**
   - R-1235 refuses `timezone` in `[database]` and `[pools.<name>]`.
   - A `timezone` in `[database.migration]`, whose connection R-1205 also covers, or in `[pool]`, isn't covered.
   - The plan doesn't say how unknown keys are treated in general, so such a key would be read silently or not, depending on the config loader.

**Fix.**

- R-1234 says `db.check_schema()` also checks the zone and returns `Error::Config` for it (my recommendation, matching the pool start), or that it doesn't, and R-1119 says the same.
- R-1235 refuses the key in every table of `morm.toml`, and T-0298 adds `[database.migration]` and `[pool]`.

### DJ8. T-0286 needs a non-UTC session through morm's own paths, which R-1205 rules out

- **Severity:** minor
- **IDs:** T-0286; R-1205
- **Files:** `steps/18-updates.md`

**What's wrong.**

- T-0286 runs "with the session's `TimeZone` not UTC" through `.by`, `insert`, `sql!` and the rest, so a free time value's wrong answer shows.
- But R-1205 says every morm connection sends `TimeZone = 'UTC'` and "Nothing overrides it". A role's `timezone` loses to the startup parameter.
- So the step-18 session must change the zone by hand, and the test doesn't say how. T-0278 runs on `tokio_postgres` directly, so it has no such problem.

**Fix.** Say how T-0286 gets the zone, for example with `SET TimeZone = 'Asia/Dhaka'` on a `db.hold()` connection, or with `SET LOCAL` inside a transaction through `sql_unchecked`. Add that the field-bound answers are the same under UTC and under that zone.

### DJ9. Small inaccuracies in `proofs.md`

- **Severity:** minor
- **IDs:** proof 9; DE
- **Files:** `proofs.md`

**What's wrong.**

1. **Row 125 of the matrix, the widenings.** It says free `Vec<i16>` on `integer[]` is "right". `p09_matrix.expected` at `ca3747b` has `Vec<i16> | integer[] | = {1,2} | ok | - | error 42883`. A free `int2[]` compared with an `integer[]` column has no operator. That's a wrong shape and an error, not a right answer.
2. **The rerun line.** It says the spikes "were deleted by PR #2's last commit (DE)". `9bd63f7` deleted them. Four commits that change only the plan have followed it, and the head is `4a3a265`.
3. **The opening paragraph.** It gives the CI versions as 15.19, 16.15, 17.11 and 18.6. The last run with the spikes, at `ca3747b`, ran 15.20, 16.16, 17.12 and 18.7, as proof 9 says.
4. **The paragraph on decisions** lists DA to DH and A to K, but not DI1 to DI4, which have entries below.

**Fix.**

1. Move `Vec<i16>` on `integer[]` with `=` to the wrong-shape rows: field right, free `42883`.
2. Name `9bd63f7` as the deleting commit.
3. Say which versions each run used.
4. Add DI1 to DI4 to that paragraph.

### DJ10. The PR body has new stale lines

- **Severity:** minor
- **IDs:** none
- **Files:** PR #2's description

**What's wrong.**

1. "INDEX's DH and DI entries name no IDs yet, so the coordinator may want to add these." INDEX on `main` now names them in the A to K entry and in the DI1/DI2, DI3 and DI4 notes.
2. "the last commit deletes `spikes/` and `spikes.yml` again". `9bd63f7` does.
3. "on `9bd63f7` and on the final head `e78b970`". The head is `4a3a265`, as a later sentence says.
4. Under DI1, "See the open question below". "Questions for the owner" says "None open".
5. "Integer fields also carry `numeric` ... because step 05's R-0201 lets `u32` widen to `numeric`". R-1213 doesn't give `u32` `numeric` (DJ1).

**Fix.** Update those five lines. The rest matches the branch: the CI results, the checker totals (1235 and 298, 0 problems), the changed and new IDs by step, `ca3747b`, the proof table and the "use this when" lines R-1217 to R-1219.

### DJ11. T-0290's pass condition isn't a number

- **Severity:** minor
- **IDs:** T-0290; DF4, DH7
- **Files:** `steps/21-pool-deadlocks.md`

**What's wrong.**

- T-0290 passes when the check "stays within a few times its cost with one stream open". "A few times" can't fail a test.
- Proof 7 gives the measured basis. Sharded, the check took 8 to 16 ns from one thread and 11 to 30 ns from eight, from one open stream to 1,000, locally. On the CI runners it took 9 to 34 ns and 15 to 116 ns.

**Fix.** Give a bound the test asserts: for example, at 1,000 open streams the median check is at most 4 times the median with one open, from one thread and from eight. The PR states the figures.

## Requirement by requirement

Step 02's only requirement, R-0917 (`info`), is met: every item it names has a passing proof in the shape the owner chose, recorded in `proofs.md`, and the twelve rows agree with the spikes' last green run at `ca3747b`.

For each changed and new ID, I checked the text against its decision and its neighbours. None is skipped or weakened against what the owner decided. The additions beyond the design are the fix's own choices, each listed in INDEX as approved through PR #2:

- `TextEnum`'s `SqlType` in step 56;
- R-1230 in step 41;
- integers carrying `numeric`;
- R-0350's per-row cast;
- R-0984's two errors;
- `tx.transaction_timeout` on 15 and 16;
- the spike's smaller typing pass;
- all 17 aliases;
- `Error::Config` for the zone.

The problems are the ones above:

- DJ1 (R-1213 against step 05) and DJ5 (R-1208 against R-1233), where requirements contradict each other;
- DJ3 and DJ4, where `contracts.md` leaves out shapes that step 03 must freeze;
- DJ2, a premise about PgBouncer that its documentation contradicts;
- DJ6 to DJ11, gaps in wording and tests.

Each new test obligation names what proves its requirement, apart from T-0286's missing setup (DJ8) and T-0290's bound (DJ11).

## Summary

- **Blocks approval:** none.
- **Should fix:** DJ1, DJ2 (needs an owner decision, options above), DJ3 (a design choice, options above).
- **Minor:** DJ4, DJ5, DJ6, DJ7, DJ8, DJ9, DJ10, DJ11.

DH1 to DH9 are fixed as recommended or as the owner decided. A to K and DI1 to DI4 are written in, with no live stale text. Every changed manifest ID has a Decisions entry. The spikes are deleted, and their last commit, `ca3747b`, ran green on Postgres 15 to 18. CI on `4a3a265` is green, and the checker reports 1235 requirements, 298 test obligations and 0 problems.
