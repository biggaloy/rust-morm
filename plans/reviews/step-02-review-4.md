# Step 02 independent review, round 4

PR #2 (`step-02-proofs`), head `47be1d6`. This is the fourth review, after the fixes for the third review's DJ1 to DJ11. It's done as `rules.md` "Independent review" says, by an agent that didn't write the step.

I read `rules.md`, `contracts.md`, `proofs.md` and `steps/02-proofs.md` on the PR branch. On `origin/main` (`cd3f7a9`) I read:

- every INDEX "Decisions since the split" entry from "Step 02's proofs, DA to DE" through "Step 02 third review, DJ2" and "DJ3";
- `reviews/step-02-review-3.md`, `reviews/param-typing-design.md` and `reviews/step-02-open-items.md`.

I read the whole round's diff, `git diff 4a3a265 origin/step-02-proofs`. It touches only `plans/`:

- `contracts.md`, `coverage.md`, `proofs.md` and the manifest;
- steps 10, 12, 13, 15, 16, 18, 21, 22, 41, 43, 50 and 56.

To check the new text against the steps it lands in, I also read:

- step 05's integer table (R-0186 to R-0203);
- step 12's hold and pooler rules (R-0669 to R-0700);
- step 22's R-0677;
- step 43's R-0671;
- CI's `pooler` job, `start_pgbouncer.sh` and `pgbouncer.ini` at the head.

## What I checked and ran

- **CI on `47be1d6`:** `ci` passed on both its push run (43372571676) and its pull request run (43372578367). That covers every job: fmt, clippy, unit, trybuild, doc, integration on Postgres 15 to 18 over TCP and Unix sockets, pooler, msrv, features and coverage. Every commit of this round, `329796d` to `47be1d6`, has a green `ci` run.
- **Checker:** `check_coverage.py --step 02 --code crates tests benches fuzz .github` and the plain form, run on an extract of the head, both report **1236 requirements, 299 test obligations, 0 problems**.
- **Manifest:** I ran `git diff origin/main...origin/step-02-proofs -- plans/plan-approved/tools/manifest.json`, which adds 54 IDs. I also compared the manifests by ID with a script, against `4a3a265` and against `main`.
  - This round changed 20 IDs: R-0670, R-1117, R-1119, R-1205, R-1207, R-1208, R-1210, R-1213, R-1223, R-1233, R-1234, R-1235, T-0104, T-0278, T-0280, T-0283, T-0286, T-0290, T-0297 and T-0298.
  - It added R-1236 and T-0299.
  - Nothing was removed, and no ID changed its step or kind.
  - INDEX's "Step 02 third review, DJ3" entry names exactly these 22 IDs, so every one is covered.
  - Against `main`, the whole PR changes 26 IDs and adds 54, R-1205 to R-1236 and T-0278 to T-0299. Every one is in a Decisions entry: the earlier ones as the third review found, and T-0104, R-1236 and T-0299 in the DJ3 entry.
  - R-0060 is the same as on `main`. Step 43's quote of R-0670 is byte for byte the same as step 12's.
- **Stale text (item 2):** I searched `plan-approved/` for the following and found none still live:
  - `Sql<'a>` without `P`, `Sql<'_>`, boxing, "no startup parameter", "isn't sent", "can't send the zone" and "rejects unknown";
  - the role having to be `UTC`, and `timezone = 'UTC'` as a requirement on the role.
  
  What's left is history: DH1's line in `proofs.md` ("was first left out ... since DJ2 it's sent there too"), and `param-typing-design.md`'s `Sql<'a>`, a review record that INDEX's DJ3 entry supersedes. `tools/steps_data.py` still matches `No startup parameters`, but against the frozen plan's text, which is right.
- **PgBouncer, run (item 4).** I built PgBouncer 1.22.1, 1.26.1 and 1.19.1 from pgbouncer.org's tarballs into scratch.
  - **Setup.** They ran in transaction mode with `default_pool_size = 1`, in front of a throwaway Postgres 18.6 whose own `timezone` was `Asia/Dhaka`. Clients logged in as a role with `timezone = 'Europe/Berlin'`, through `tokio-postgres` 0.7.18's `Config::options("-c TimeZone=UTC")`. Each PgBouncer ran four ways: `ignore_startup_parameters` empty, `TimeZone`, `timezone` and `options`.
  - **1.22.1 and 1.26.1, with the list empty, `timezone` or `options`:**
    - client A (`-c TimeZone=UTC`) reported `UTC`, and inside a transaction `'2026-01-01 00:00:00+00'::timestamptz::timestamp` gave `2026-01-01 00:00:00`;
    - client B (no options) reported `Europe/Berlin`, then ran `SET timezone = 'Asia/Tokyo'` on the same backend pid;
    - after that, A still reported `UTC`, a new `-c TimeZone=UTC` client reported `UTC`, and a new client with no options reported `Europe/Berlin`;
    - A's own `SET timezone = 'America/New_York'` stuck for A only.
  - **With `TimeZone` in the list,** 1.26.1 dropped the parameter, and every client reported `Europe/Berlin`. 1.22.1 kept `UTC`.
  - **NEWS.** 1.26.1's `NEWS.md` has "Make `ignore_startup_parameters` take precedence over tracked parameters". Nothing in 1.22.1's NEWS against 1.22.0, which Ubuntu ships, touches startup parameters.
  - **So DJ2's claims in `proofs.md` hold,** on 18.6 at least. I had no Postgres 17 here, so I didn't rerun the 17.11 pairs.
  - **Two things DJ2 didn't cover:** PgBouncer before 1.20 refuses the connection outright (DK1), and `application_name` goes through as its own tracked parameter (DK5).
- **`Sql<'a, P>`, run (item 4).** I built a scratch crate on rustc 1.99.1 with `tokio-postgres` 0.7.18. It holds `Fits<T>: Sync`, field and free carrier wrappers, `Sql<'a, P>` with public fields, `Params` over flat tuples with an array-iterator `bind()`, and a `macro_rules!` writing the nested struct literals. I ran it on the same 18.6 server.
  - **Runs.** These compiled and gave the right count:
    - `let q = ..` with a field-bound `i32` on `int8`, two free values whose expressions make runtime temporaries (`&(x + 1)` and `&(x * 1)`, with `x` from `black_box`), a field-bound `String` and a field-bound `Vec<i64>` on `int8[]`;
    - that `q` run twice by reference and once by value;
    - `fn by_name<'a>(..) -> Sql<'a, impl Params>`;
    - a query built inside `tokio::spawn` borrowing a captured row, awaited twice.
  - **`&[&dyn Fits<i64>]`** holding an `i64` and an `i32`, held across an `.await` inside `tokio::spawn`, compiled and encoded 16 bytes.
  - **Allocations, under a counting global allocator:**
    - building `q`: 0;
    - walking `bind()` and encoding into a reserved buffer: 0;
    - a run through `query_typed_raw(text, bind())`: 16, and the same statement through `query_typed` with a borrowed slice: 16, in five runs each after three warm-ups.
  - **So the zero-allocation claim holds.** Two things the proof didn't test: the evidence for temporaries (DK4), and `Params` taken by value through a `Send` trait method (DK3).
- **README notices and "use this when" lines (item 5):**
  - R-1210 covers the zone refusal, the `timezone` key and, now, "What you give up".
  - R-1119 covers the zone check with `check_schema = "off"`.
  - R-1236 covers DJ6's three surprises.
  - The PR adds no new call. The gaps are DK1, DK5 and DK7.
- **The servers, PgBouncers and the scratch crate** are stopped, and their files deleted.

## The third review's findings

| Finding | Fixed as recommended or decided? | Evidence |
|---|---|---|
| DJ1 | Yes | R-1213: every integer type morm maps, `NonZero*` included, carries every wider integer column and `numeric`, with step 05's fit check deciding the precision. `i8`/`u8` map to `int2`, `u16` to `int4`, `u32` to `int8`, and `u64`, `i128` and `u128` to `numeric`, which matches R-0191 to R-0197. `u128` also carries `uuid`, and R-1207 encodes it most significant byte first. `char` is `text` with no text form. `Vec<T>` carries `T`'s types with `[]`. T-0278 adds the cells. T-0280 adds fields that must compile, among them `u32` on `numeric` and `u128` on `uuid`. |
| DJ2 | Yes, the owner's option A | R-1205, R-0670 and its quote in step 43, R-1117, R-1234 (backstop, both fixes in the error), R-1210, R-1119, T-0297 (Berlin role starts and reports `UTC`; refusal through 1.26.1 with `ignore_startup_parameters = TimeZone`) and T-0104. `contracts.md`'s conversion comment says the zone goes through PgBouncer too. See DK1, DK2 and DK5. |
| DJ3 | Yes, the owner's decision | `contracts.md`: `Sql<'a, P>`, `#[doc(hidden)]` public fields, nested struct literals, `-> Sql<'a, impl Params>`, and `db.sql()`/`db.raw()` generic over `P`. R-1223 and step 16's deliverables say the same. The new T-0299 has stored, passed, returned and spawned queries, and allocation counts. See DK3, DK4 and DK6. |
| DJ4 | Yes | `Fits<T>: Sync` with an object-safe `encode`, and `FitsList<T>: Sync`. T-0283 adds a comp awaited inside `tokio::spawn`. |
| DJ5 | Yes | R-1208's text-form list is `String`, `&str`, `Box<str>` and `Cow<str>`. R-1233 gives none of `PgEnum`, `PgDomain`, `PgComposite` or `TextEnum` a text form. `contracts.md` agrees. |
| DJ6 | Yes | R-1236 (`doc`, step 50) has the concrete-type rule with `E0401`, `Option` through a nullable field, and a non-text type on a hand-made domain. R-1210 has "What you give up". |
| DJ7 | Yes | R-1234: `db.check_schema()` checks the zone whatever R-1117 says, as `Error::Config`, ahead of any finding. R-1119 says the same. R-1235 covers every table, and T-0298 adds `[database.migration]` and `[pool]`. |
| DJ8 | Yes | T-0286 sets `Asia/Dhaka` with `sql_unchecked` on a `db.hold()` connection. R-0698 closes held connections on release, so the zone can't leak into the pool. The field-bound answers are compared against a UTC pool run. |
| DJ9 | Yes | The `Vec<i16>` on `integer[]` row moved to the wrong shapes. The rerun line names `9bd63f7`. The opening paragraph gives both version sets. The decisions paragraph lists DI and DJ. |
| DJ10 | Yes for its five lines | The body has new stale lines (DK9). |
| DJ11 | Yes | T-0290: median of 11 runs of 100,000 checks, at most 10 times the one-stream median, at 100 and 1,000 streams, from one thread and from eight. The figures quoted match proof 7: sharded within 34/9 and 116/15 on CI, one lock 278/12 and 5.4 µs/322 ns locally. |

## Findings

### DK1. With DJ2, morm can't connect through PgBouncer before 1.20, which the plan still supports

- **Severity:** should fix, needs an owner decision
- **IDs:** R-0670, R-1205, R-1210, R-1234, R-0677; T-0104, T-0297; DJ2
- **Files:** `steps/12-hold-pooler.md`, `steps/10-config.md`, `steps/41-rolling.md`, `steps/22-locks-timeouts.md`, `steps/50-readme-release.md`, `steps/43-doctor.md`

**What's wrong.**

- DJ2 sends `options=-c TimeZone=UTC` behind PgBouncer. PgBouncer only accepts `options` from 1.20.0 on. 1.20.0's NEWS has "Add support for the `options` startup parameter".
- Before that, a client sending `options` is refused at login. Before DJ2, morm sent no startup parameter behind a pooler, so it worked with any PgBouncer.
- The plan still supports older PgBouncers. R-0677 says prepared statements "need PgBouncer 1.21+ ... On older versions, set `statement_cache = 0` and `shape_cache = 0`".
- R-0670, R-1234 and R-1210 describe two kinds of pooler: one that passes `TimeZone`, and one that drops it, which R-1234 catches. They don't describe a third kind, a pooler that refuses the connection. On that one, every connection fails with PgBouncer's own error, and morm says nothing about the cause.

**Evidence.** PgBouncer 1.19.1, built from source, in transaction mode in front of 18.6:

- With `ignore_startup_parameters` empty, a client sending `-c TimeZone=UTC` failed with `FATAL: unsupported startup parameter: options`. A client sending nothing connected.
- With `ignore_startup_parameters = options`, the usual workaround for clients that send `PGOPTIONS`, the parameter was dropped, and the session reported the role's `Europe/Berlin`. R-1234 would catch that.

**This needs an owner decision:**

- **A (recommended). PgBouncer 1.20 or later for `pooler = "transaction"`, with a clear error.**
  - R-0670 and R-1210 say so loudly. R-0677's "older versions" becomes 1.20.
  - When a connection fails with `unsupported startup parameter: options`, morm returns `Error::Config`. The error names the cause and the two fixes: upgrade PgBouncer, or add `options` to `ignore_startup_parameters` and set the role's `timezone` to `UTC`, which R-1234 then checks.
  - T-0104 adds that error case, on a PgBouncer built from a pinned 1.19 tarball in the `pooler` job (see DK2).
- **B. Fall back.** On that error, open the pool's connections without `options`, remember that for the pool, and rely on R-1234 and the role's `timezone`.
  - It keeps old PgBouncers working with no setup.
  - But it adds a failed handshake at each pool start and a second code path, and the zone then rests on the role again, quietly.

### DK2. The `pooler` job can't run what T-0104 and T-0297 now ask, and no step is told to change it

- **Severity:** should fix, needs a small owner decision (what happens when apt fails)
- **IDs:** T-0104, T-0297; R-0792; DJ2
- **Files:** `steps/22-locks-timeouts.md`, `steps/41-rolling.md`; `.github/workflows/ci.yml`, `.github/scripts/start_pgbouncer.sh`, `.github/pgbouncer/pgbouncer.ini` (step 01's, `done`)

**What's wrong.**

- T-0104 now asks for a session through PgBouncer to report `UTC` "on PgBouncer 1.22 and on the pinned 1.26.1 image".
- T-0297's refusal cases need "1.26.1 (the pooler job's pinned image) with `ignore_startup_parameters = TimeZone`", and the whole test is "on Postgres 15–18".
- The `pooler` job at the head runs one PgBouncer against `postgres:18` only:
  - `start_pgbouncer.sh` installs Ubuntu's 1.22.0 from apt.
  - It starts the pinned 1.26.1 image only when apt fails twice (INDEX, "the pooler install").
  - Its one `pgbouncer.ini` has no `ignore_startup_parameters`.
- So in a normal run:
  - there's no 1.26.1;
  - there's never a PgBouncer that drops `TimeZone`;
  - there's no Postgres 15 to 17 behind PgBouncer.
- T-0297's refusal and alias cases can't run, and T-0104 can never cover both versions in one job.
- R-0792 and these files belong to step 01, which is `done`. No requirement in step 12, 22 or 41 says to change the job. Under `rules.md` ("don't add features it doesn't ask for"), those sessions would have to stop and ask.
- The PR body's "T-0297's refusal cases run through the pinned PgBouncer 1.26.1 image" assumes a job that doesn't exist.

**Fix.** Add a `code` requirement, in step 12 (the first pooler step) or step 22, with the next free ID, saying how the `pooler` job changes:

- It runs Ubuntu's 1.22 and the pinned 1.26.1 image on every run, not as a fallback. The 1.26.1 instance has a second listener or config with `ignore_startup_parameters = TimeZone`. If DK1 picks A, it also runs a 1.19 built from a pinned tarball.
- It runs over Postgres 15 to 18, or T-0297 says its PgBouncer cases run on 18 only, while its alias read-back on every version runs on direct connections, as it already can.

The owner decides what happens when apt fails:

- Recommended: the 1.22 cases fail the job, as R-0792's "the pooler tests never run without PgBouncer" intends.
- Or: they're skipped with a `::warning::`, which would break the rule that a check never silently doesn't run.

Then T-0104 and T-0297 point at that requirement. Update the PR body's line.

### DK3. `Params: Sync` isn't enough for `db.sql(q)` by value through `&impl Conn`

- **Severity:** should fix (step 03 freezes this trait)
- **IDs:** R-1223, T-0299; DJ3; `contracts.md` `Params`, `Conn`
- **Files:** `contracts.md`, `steps/16-reads-more.md`

**What's wrong.**

- `contracts.md` declares `pub trait Params: Sync`, "so a query held across an .await keeps the future Send". It also says `db.sql()` takes "a Sql or a reference to one".
- `Conn` carries "every query entry point", and hooks call through `db: &impl Conn`, with futures declared `+ Send`. So `Conn::sql` must return a future declared `Send` for every `P`.
- A future that holds `q: Sql<'_, P>` by value across an `.await` is `Send` only if `P: Send`. `Sync` covers only the `&Sql` case.
- Concrete tuples of borrows are `Send`, but a trait method generic over `P: Params` can't see that.

**Evidence.** In the scratch crate:

- I wrote a `Conn`-like trait with `fn sql<P: Params>(&self, q: Sql<'_, P>) -> impl Future<Output = usize> + Send`, whose `async move` block yields before reading `q`. It fails to compile with ``error[E0277]: `P` cannot be sent between threads safely``.
- The `&Sql<'_, P>` form compiles.
- With `Params: Send + Sync`, both compile and run.
- T-0299 and the DJ3 proof ran every query through inherent `async fn`s, where auto traits leak, so neither could see this.

**Fix.**

- Make it `pub trait Params: Send + Sync` in `contracts.md`, and fix the comment.
- Add to T-0299: a function generic over `db: &impl Conn` runs `db.sql(q)` by value and `db.sql(&q)` inside `tokio::spawn`, as a hook does.
- Update the PR body's "a `Params: Sync` trait".

### DK4. DJ3's evidence for temporaries didn't test lifetime extension

- **Severity:** minor
- **IDs:** T-0299; DJ3
- **Files:** `proofs.md` (DJ3), `steps/16-reads-more.md` (T-0299)

**What's wrong.**

- `proofs.md` says the prototype's "free value whose expression `1_i64 + 1` makes a temporary" lived as long as `q`, because a `let`'s struct literal extends it.
- But `&(1_i64 + 1)` is a constant expression, so Rust promotes it to a `'static`, and it needs no extension.
- T-0299 says only "a free value whose expression makes a temporary". A test written with a constant would pass whatever `sql!` expands to.

**Evidence.** In the scratch crate, a query built through a function call, which never extends a temporary:

- `Sql { .., params: (free(&(1_i64 + 1)),), .. }` compiled and ran (it printed `2`);
- the same with `&(x + 1)`, `x` a runtime value, failed with `E0716: temporary value dropped while borrowed`;
- the struct-literal form with `&(x + 1)`, `&(x * 1)` and `&(row.1[0] + 0)` compiled and ran right, so the design itself holds.

**Fix.**

- Correct `proofs.md`'s DJ3 line: the constant was promoted, and this review's run-time temporaries are the evidence.
- Make T-0299's case "a free value computed at run time, such as `n + 1` with `n` a variable, not a constant".
- Optionally, add a `trybuild` case that the same value through a function call fails with `E0716`, so the struct-literal expansion can't be lost.

While there: `proofs.md`'s matrix row reads "`` `= ` the array``", with a stray space inside the backticks.

### DK5. "The one startup parameter" isn't so, and `application_name` goes missing behind PgBouncer with no notice

- **Severity:** minor, needs an owner decision
- **IDs:** R-0670 (and its quote in step 43), R-1205, R-0060; `proofs.md` DJ2
- **Files:** `steps/12-hold-pooler.md`, `steps/43-doctor.md`, `steps/10-config.md`, `proofs.md`

**What's wrong.**

1. **"Only `TimeZone`".**
   - R-0670 now says "PgBouncer refuses a startup parameter it doesn't track, by default, so of morm's settings only `TimeZone = 'UTC'` goes as a startup parameter". R-1205 calls it "the one startup parameter that is".
   - R-0060 sets `application_name = "auto"` so the binary's name "shows in pg_stat_activity".
   - PgBouncer tracks `application_name` by default, in every version, so the reason R-0670 gives doesn't apply to it. Yet the text means it isn't sent behind PgBouncer, which nothing tells the user.
   - This was true before DJ2 too, under "No startup parameters", but DJ2's new wording gives a reason that now argues the other way.
2. **`proofs.md`'s DJ2 line** says `tokio-postgres` 0.7.18's startup message "holds only `user`, `database`, `options` and `application_name`". It also always holds `client_encoding = UTF8` (`connect_raw.rs`, line 138: `let mut params = vec![("client_encoding", "UTF8")];`). PgBouncer tracks that too, so it's harmless, but the sentence is wrong.

**Evidence.** Through 1.22.1 and 1.26.1:

- a client with `application_name=morm_app` and `-c TimeZone=UTC` read back `morm_app`, `UTC` and `UTF8`;
- `-c lock_timeout=5s` in `options` failed with `unsupported startup parameter in options: lock_timeout`, which agrees with R-0670 on the rest.

**This needs an owner decision:**

- **A (recommended).** Behind PgBouncer morm also sends `application_name`, which PgBouncer tracks. R-0670 says "only `TimeZone`, in `options`, and `application_name`". T-0104 checks both.
- **B.** Keep it unsent. R-0060's comment and a README line beside `pooler` say `application_name` doesn't reach the server behind a pooler.

Either way, fix `proofs.md`'s list of startup fields, and change "the one startup parameter" to "the one setting sent in `options`".

### DK6. `contracts.md` doesn't say how many values one `sql!` query can hold

- **Severity:** minor (step 03 freezes `Params`)
- **IDs:** R-1223, T-0299; DJ3
- **Files:** `contracts.md` (`Params`), `steps/16-reads-more.md`

**What's wrong.**

- `Params` is "implemented for tuples of carrier-tagged borrows", and `bind()` returns an `ExactSizeIterator` "from an array iterator".
- Stable Rust has no variadic generics, so `Params` exists only for the tuple sizes a macro writes out. A query with more values fails to compile with a trait error that doesn't mention the limit.
- Nesting tuples to lift the limit doesn't fit the stated `bind()`: `Chain` isn't an `ExactSizeIterator`, so a nested form needs its own indexed iterator.
- A wide `INSERT ... SELECT` or a report query written in `sql!` can have dozens of values. Step 16 would have to pick a limit and an error with no guidance.

**Fix.** State the shape in `contracts.md`:

- flat tuples up to a stated size (such as 64), and `sql!` refusing more at compile time with a message naming the limit and suggesting `sql_unchecked` or a smaller query;
- or nested tuples, with `bind()` returning morm's own indexed `ExactSizeIterator`.

Add a case at the limit to T-0299. If there's a limit, add a README line in R-1236.

### DK7. `-> Sql<'a, impl Params>` has no "use this when" line

- **Severity:** minor
- **IDs:** R-1223; `rules.md` "When a step is done", item 5
- **Files:** `steps/50-readme-release.md`, PR #2's description

**What's wrong.**

- The PR says "The DJ round adds no public call: `Sql<'a, P>` and `Params` are types".
- But users write `Params` in their own signatures, to return a query, and that's an opt-in pattern with a real use case.
- The DH round set the precedent with R-1218, a "use this when" line for `impl morm::SqlType`.

**Fix.** Add a line beside R-1218's, for example:

- "`-> Sql<'a, impl morm::Params>`: a function builds a query that its caller runs, such as a filter shared by several handlers."

List it in the PR body.

### DK8. T-0297's last case proves nothing through the PgBouncer that passes the zone

- **Severity:** minor
- **IDs:** T-0297, R-1117
- **Files:** `steps/41-rolling.md`

**What's wrong.**

- T-0297 now starts with a Berlin role through PgBouncer that starts anyway, because the parameter passes.
- It ends "and with `check_schema = "off"` the Berlin role's pool starts". It doesn't say through which PgBouncer.
- Through the passing one, that pool starts with the check on too, so the case can't fail.

**Fix.** Say "through the PgBouncer that drops the parameter, with `check_schema = "off"` the Berlin role's pool starts, and its sessions report `Europe/Berlin`".

### DK9. The PR body has new stale lines

- **Severity:** minor
- **IDs:** none
- **Files:** PR #2's description

**What's wrong.**

1. "INDEX's Decisions name the DH, A to K and DI IDs; the DJ entries name none yet, so the coordinator may want to add the IDs above to them." INDEX's DJ3 entry on `main` now names all 22 IDs.
2. "a `Params: Sync` trait" changes with DK3.
3. "T-0297's refusal cases run through the pinned PgBouncer 1.26.1 image": the job runs that image only as a fallback (DK2).
4. DJ2's "the one startup setting it sends there" changes with DK5.

**Fix.** Update those lines. The rest matches the branch:

- the CI results and the commit list `329796d` to `47be1d6`;
- the checker totals (1236 and 299, 0 problems);
- the DJ round's table of IDs by step;
- the proof table and the "use this when" lines R-1217 to R-1219.

## Requirement by requirement, for the IDs changed since `4a3a265`

- **Step 10.**
  - R-1205 matches DJ2. Its "the one startup parameter" is wrong (DK5).
  - R-1235 and T-0298 cover every table.
- **Step 12.** R-0670 matches DJ2 and my PgBouncer runs. Missing: the pre-1.20 refusal (DK1) and `application_name` (DK5).
- **Step 13.**
  - R-1207, R-1208 and R-1213 match DJ1 and DJ5 and agree with step 05's R-0190 to R-0203.
  - T-0278 adds every new pairing, and pins a free `u128` against `uuid` as `42883`.
  - T-0280 pins refusals and passes both ways.
  - Nothing is skipped or weakened.
- **Step 15.** T-0283's spawned comp case matches DJ4.
- **Step 16.**
  - R-1223 matches DJ3. T-0299 tests the stored, passed, returned and spawned forms with allocation counts, and the zero-allocation part holds (my run).
  - Missing: by value through `Conn` (DK3), a run-time temporary (DK4) and the size limit (DK6).
- **Step 18.** T-0286 sets the zone in a way R-1205 allows, and R-0698 keeps it from leaking.
- **Step 21.** T-0290's bound is a number the test asserts, and it sits between the measured sharded and single-lock figures.
- **Step 22.** T-0104's new case is right, but the job can't run it as written (DK2).
- **Step 41.**
  - R-1117 and R-1234 match DJ2 and DJ7: backstop, both fixes in the error, `db.check_schema()` checking the zone, and `Error::Config` ahead of findings.
  - T-0297 covers them, with DK2's and DK8's caveats.
- **Step 50.** R-1119, R-1210 and the new R-1236 are loud and say what to do. Missing: the pre-1.20 refusal (DK1) and `application_name` (DK5).
- **Step 56.** R-1233 matches DJ5. `TextEnum` carrying only `text` and `varchar` is the fix's own choice, approved in INDEX.

What the fix added beyond the review is the list INDEX approves through PR #2:

- `char` has no text form;
- `TextEnum` carries `text` and `varchar`;
- a list of a text-form type has a text form;
- `u128` on `uuid` is stored most significant byte first;
- a wrong zone is `Error::Config` ahead of findings;
- T-0104 carries the pooler check, and the refusal cases need 1.26.1;
- `Sql`'s fields are `#[doc(hidden)]` public;
- T-0299's allocation rule;
- T-0290's bound is 10 times;
- R-1236's advice on `None`.

Nothing else was added.

## Summary

- **Blocks approval:** none.
- **Should fix:**
  - DK1, needs an owner decision, options above. DJ2 breaks PgBouncer before 1.20, which R-0677 still supports.
  - DK2, a small owner decision on the apt fallback. The `pooler` job can't run T-0104's and T-0297's new cases, and no step owns the change.
  - DK3. `Params` needs `Send` for `db.sql(q)` by value through `&impl Conn`.
- **Minor:**
  - DK4, the evidence for temporaries;
  - DK5, needs an owner decision: `application_name` and "the one startup parameter";
  - DK6, `Params`'s size limit;
  - DK7, a "use this when" line for `-> Sql<'a, impl Params>`;
  - DK8, T-0297's last case;
  - DK9, the PR body.

DJ1 to DJ11 are fixed as the third review recommended or as the owner decided. DJ2 and DJ3 are written in consistently across `contracts.md`, `proofs.md` and the steps, with no live stale text. Every changed manifest ID has a Decisions entry. DJ2's PgBouncer behaviour and DJ3's zero-allocation claim held when I ran them. CI on `47be1d6` is green, and the checker reports 1236 requirements, 299 test obligations and 0 problems.
