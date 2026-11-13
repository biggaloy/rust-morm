# Step 55 independent review

PR #3 (`step-55-enum-proofs`), head `7a7285b`, last commit with the spikes `d8bd613`. Done as `rules.md` "Independent review" says, by an agent that didn't write the step.

## What I read

- On the PR branch: `rules.md`, `contracts.md`, `proofs.md` and `steps/55-enum-proofs.md`.
- On `origin/main` (`c483f83`): INDEX's "Decisions since the split", `reviews/step-55-open-items.md`, and the requirements the proofs inform: R-0718, R-0981, R-0983 and T-0151 (step 14), R-0975 (step 11), R-0984 (step 20), R-0994 (step 37), R-1050 (step 52), R-1038 to R-1040, R-1066, R-1076 to R-1083, R-1093, T-0168, T-0171, T-0172 and T-0204 (step 59).
- The diff, `git diff origin/main...origin/step-55-enum-proofs`: `proofs.md` (+128) and `contracts.md` (+12) only.
- At `d8bd613`: every spike (`spikes/src/lib.rs`, `stale.rs`, `p13.rs` to `p18.rs`), `start_pgbouncers.sh`, `start_standby.sh` and `spikes.yml`.

## What I checked and ran

- **CI.** Head `7a7285b`: `ci` passed on push (43422363651) and pull request (43422372872). `d8bd613`: `ci` and `spikes` passed on push (43422318284, 43422317939) and pull request (43422324857, 43422324896). The runs `proofs.md` links, at `757361d`, passed too, and `757361d..d8bd613` changes only `proofs.md` and `contracts.md`.
- **Spike logs.** I downloaded the `spikes` logs of `757361d` and `d8bd613` and split them by version and proof. Every proof printed "every check passed" on 15.20, 16.16, 17.12 and 18.7, and on both PgBouncers. The tables in `proofs.md` match the printed rows I compared: proof 13's lookups and parameter types, proof 14's codes, routines and German messages, proof 15's four functions and cures, proof 16's `prepare:` errors and cures, proof 17's replay timings and proof 18's findings. Proof 13 has 111 checks and proof 18 has 66.
- **Reruns.** I rebuilt the spikes from `d8bd613` and ran `p13`, `p14` and `p15` on throwaway Postgres 18.6 and 17.11. All passed, with the same rows apart from the German messages, since the local builds have no translations.
- **My own checks.** I ran these on the same servers, and through PgBouncer 1.20.1 and 1.22.0, which I built from pgbouncer.org's tarballs (SHA-256 `24992cf5…2088` for 1.20.1). The results are under each finding.
- **Workflow rules.** `spikes.yml` at `d8bd613` has top-level `permissions: contents: read`, the checkout action pinned to a full commit, and every cargo step uses `--manifest-path spikes/Cargo.toml` with no `working-directory`. It has no pipe and no `if:`, `continue-on-error`, `exclude:`, `secrets.` or `|| true`. `spikes/Cargo.toml` has its own `[workspace]`. `ci` passed with `repo_policy.rs` reading it. Both the workflow and `spikes/` are gone at the head.
- **Checker.** On the head, `check_coverage.py --step 55 --code crates tests benches fuzz .github` and the plain form both report **1240 requirements, 300 test obligations, 0 problems**.
- **Manifest.** `git diff origin/main -- plans/plan-approved/tools/manifest.json` is empty. No ID changed, so no INDEX Decisions entry is needed for this PR as it stands.

## Proof by proof

- **13. Built as the step says.** The carriers are proven: `pg_prepared_statements` shows only `text`, `text[]` and `integer`. The enum decoder never reads the variant list, and the driver's stale copy after `ADD VALUE` proves it. Composites with `"`, `,` and `\` round-trip. Two problems:
  - The driver's own lookup is a failure behind PgBouncer, not just a cost (EE2).
  - Statements cached before `ADD VALUE` weren't run after it (EE8).
- **14. The recorded codes are right on 15 to 18.** The statement "R-0718 and T-0151 hold as written" doesn't hold with the old type kept, because a shared label runs on the old type silently (EE1). The `lc_messages` part is right as far as it goes, but EB is framed too narrowly (EE3).
- **15. The recorded results are right.** The `DISCARD PLANS` deviation contradicts R-0994's text, which `proofs.md` doesn't name (EE7). The kept-type case is the online form's normal state, not an edge (EE5).
- **16. The recorded results are right**, and the contradiction with R-0718 and R-0981 is real. `default_pool_size = 1` makes it deterministic, but it also hides EE2. EC's recommendation is unsafe as written (EE4).
- **17. Recorded correctly.** Every recovery works on the standby with the refresh run on the primary. Replay cancels a conflicting query with `40001` after the delay. I found nothing wrong here.
- **18. Pass, and built as the step says.** Install with no restart, the template and clone as a `CREATEDB` user, `42501` from `CREATE EXTENSION`, the false shapes and their pragma lines, the disabled-mode forms and every trigger form all match R-1076 to R-1082, R-1093 and T-0204. `@plpgsql_check_options` hiding an `error` confirms R-1093. One pragma that can hide R-1080's narrowing wasn't tested (EE6).

## Findings

### EE1. Blocks approval. Proofs 14 and 15, with R-0718, T-0151, R-0984, R-1040 and R-1050: a stale statement silently uses the old type while it's kept

- **Where:** `proofs.md`, proof 14's table and its "R-0718 and T-0151 hold as written" line; proof 15's table.
- **What's wrong:** with the old type kept, a statement that touches no table of the type still runs on the old type. So does a function compiled before the swap. Both only fail for a label the old type lacks. `proofs.md` calls the shared label "right", because the spike only round-trips it as text. A swap exists to reorder or remove labels, and a reorder changes what every comparison means. So a stale statement gives wrong answers, and can write wrong rows, with no error. R-0718, R-0981 and R-0984 all act only on an error, so nothing recovers. The kept type isn't an edge case: R-1050 keeps the retired type alive until `purge`, which is the state after every online swap, and online is the default.
- **Evidence:** run on 18.6 and 17.11 with the same results.
  - Before the swap, the session prepared these, with `st` as `('a','b','c')`:
    - `PREPARE b_lt(text,text) AS SELECT ($1::st) < ($2::st)`
    - `PREPARE b_audit(text) AS INSERT INTO audit (note) SELECT 'a<' || ($1::text) WHERE 'a'::st < ($1::st) RETURNING note`
    - a plpgsql `DECLARE v st; v := p; RETURN (v < 'c'::st)::text`
  - The swap renamed `st` to `st_old`, created `st` as `('c','b','a','d')` and moved the column, keeping `st_old`.
  - Afterwards:
    - `EXECUTE b_lt('a','c')` returned `t`, where a fresh `SELECT 'a'::st < 'c'::st` returns `f`.
    - `EXECUTE b_audit('c')` inserted a row that the new order excludes.
    - The function returned `true`.
  - The same held with a retired column of `st_old` left on the table, as R-1050 leaves it. The cause is that Postgres records no plan dependency on a plain enum type, so the cached plan keeps the old type's OID.
- **This needs an owner decision.** Options:
  - **(a) Recommended. Rename every label of the kept type.** At cutover, and in a `--fast` swap that keeps the old type, rename each label to a reserved form, such as `ALTER TYPE st_old RENAME VALUE 'a' TO '_morm_retired_a'`. A later rollback or down renames them back before it reads the retired column.
    - Rows are stored by enum OID, so the retired column keeps every value. I checked: its rows read `_morm_retired_a`, and so on.
    - Any stale use then fails loudly with `22P02` naming the retired type, on 18.6 and 17.11, both for `EXECUTE b_lt('a','c')` and for the function. R-0718 already retries that, and R-0984 already maps it to `SchemaChanged`.
    - It's catalog-only and takes no table lock.
  - **(b) Drop the old type at cutover**, keeping retired values as `text`. This rewrites the table, which works against the online form.
  - **(c) Document it loudly and rely on reconnects.** Silent wrong writes remain, which conflicts with the project's goals.

  Whichever is chosen:
  - T-0151 and T-0168 gain an order-sensitive statement and function, run with the old type kept.
  - `proofs.md` says "runs on the old type" where it now says "right".

### EE2. Blocks approval. Proofs 13 and 16, EA (Q1): the driver's own lookup fails behind a PgBouncer with no prepared statements

- **Where:** `proofs.md`, proof 13's Finding (Q1) and Q1; `contracts.md`'s proof 13 comment; EA in `step-55-open-items.md`.
- **What's wrong:** EA treats the lookup as a cost, and recommends testing it later, in step 12. The lookup runs on named statements (`s0`, `s1` and so on) that `tokio-postgres` prepares and then executes in separate round trips. In transaction mode, with no prepared-statement support, those round trips can reach different server connections. That covers PgBouncer 1.20, which DK1 supports, and 1.21 or later with `max_prepared_statements = 0`, which DL3's caches-off mode is for. Decoding any enum or composite column then fails with `26000`. Proof 16 couldn't show this, because:
  - it ran only PgBouncers that keep prepared statements;
  - it ran with `default_pool_size = 1`, so every round trip reaches the one server anyway.
- **Evidence:** eight clients, five rounds, three `query_typed` decodes of an enum column each, run as 120 decodes per setup:

  | PgBouncer | Pool size | `max_prepared_statements` | Result |
  |---|---|---|---|
  | 1.20.1 | 4 | n/a | 83 of 120 failed with `26000 prepared statement "sN" does not exist [FetchPreparedStatement]` |
  | 1.20.1 | 1 | n/a | 120 right |
  | 1.22.0 | 4 | 0 | 86 of 120 failed the same way |
  | 1.22.0 | 4 | 200 | 120 right |

  The only named statements on these connections are the driver's lookups.
- **What this means for EA:** recommendation (a) is unsafe. The failure is already known. Step 12 runs after the contract freeze in step 03. A pooler test with a pool of one would pass and hide it.
- **This needs an owner decision.** Options:
  - **(a) Recommended, which is EA's option (b).** morm writes enum and composite result columns, and arrays of them, as `::text` or `::text[]`, and its decoders read text, with composites parsed from their text form. User `sql!` queries that return such a column are refused by the macro or by check-sql, and must cast too.
  - **(b) Raise the PgBouncer floor** to 1.21 or later with `max_prepared_statements` above 0, refused at pool start. This reverses part of DK1 and DL3.

  Either way, the pooler test runs with a pool size above 1, and the `contracts.md` comment is corrected.

### EE3. Should fix. Proofs 14 to 16, EB (Q2): the framing misses options and requirements, and its recommendation is fragile

- **Where:** `proofs.md`, proof 14's `lc_messages` bullet and Q2; `contracts.md`'s `SchemaChanged` comment; EB.
- **What's wrong:**
  1. **"No client can change" `lc_messages` is overstated.** A superuser can run `ALTER ROLE app SET lc_messages = 'C'`. On 15 or later it can also run `GRANT SET ON PARAMETER lc_messages TO app`, after which the role can `SET` it or send it as a startup option. I tested all three on 18.6 and 17.11. That opens an option EB lacks: the startup statement already reads `TimeZone` (DI1), and it could read `lc_messages` too and refuse a non-English setting, naming the `ALTER ROLE` fix. A role setting works behind PgBouncer too, because it applies on the server.
  2. **EB leaves out R-0984 and R-0983.** R-0984 turns a `22P02` into `SchemaChanged` only when it names a different enum from the one the statement casts to. That name comes from the message, which is translated, as in proof 14's German row (`ungültiger Eingabewert für Enum ….st_old: »d«`). The routine, `enum_in`, can't tell one enum from another. R-0983's "a `42804` with any other message" is also a message match.
  3. **The routine allow-list is fragile.** `pl_exec.c` raises "type of parameter … does not match …" from three routines on both 15 and 18: `plpgsql_param_eval_recfield`, `plpgsql_param_eval_generic` and `plpgsql_param_eval_generic_ro`. The proof saw only the first. Routine names are internal, and the proof itself shows two renamed between 16 and 17.
  4. **A small gap in the recorded table.** Proof 15's table gives no routine for the trigger's `42804`. The log shows `plpgsql_param_eval_recfield`.
- **Fix:** add these points and the startup-check option to EB before the owner answers. My recommendation is English messages, checked at pool start in the existing statement with the `ALTER ROLE` fix named, and message matching kept, with the routine as a second signal only.

### EE4. Should fix. Proof 16, EC (Q3): a per-pool generation in the statement text collides across processes

- **Where:** Q3 in `proofs.md`; EC's recommendation (a).
- **What's wrong:**
  - **The marker collides across processes.** R-0975's generation is an atomic counter inside each pool, so every process starts at the same value. PgBouncer shares one server statement per text across clients, as proof 16's "new client, same text" rows show. So a process started after a swap sends the same marked texts that older processes prepared before it, and gets their stale server statements. Each later swap repeats this for any process that's behind. Each hit is a `SchemaChanged` the app sees inside a transaction.
  - **These errors come at prepare time.** Proof 16 shows `prepare: 0A000` and `prepare: XX000`. R-0718 retries only a cached statement, so it doesn't cover them.
- **This needs an owner decision.** Options:
  - **(a) Recommended. Use a marker every process shares** that changes with each swap, such as the ledger's last applied migration id, read at pool start in the existing startup statement and again whenever the generation rises. Also add a rule that a prepare-time stale error retries once unnamed.
  - **(b) Use a random marker per process.** Nothing collides, but PgBouncer no longer shares statements across clients, so each server connection holds more of them.

### EE5. Should fix. Proof 15, ED (Q4): the kept type is the default path, and two options conflict with the plan

- **Where:** Q4 in `proofs.md`; ED.
- **What's wrong:**
  1. **Q4 reads like an edge case.** Under R-1050 the old type stays alive until `purge` after every online swap, which is the default form.
  2. **Option (c), always drop the old type, contradicts R-1050**, and ED doesn't say so.
  3. **Option (a), the refresh plus the decision, leaves R-1040's decision text wrong.** That text says morm apps recover by themselves from a dropped type. Here the type isn't dropped: unmanaged `DECLARE` and `INTO` give `22P02`, which nothing recovers from, and shared labels run silently on the old type, as in EE1's function.
- **Fix:** decide ED together with EE1. With EE1's option (a), the retired type's labels no longer match, so every stale unmanaged function fails loudly with `22P02` naming a type morm retired, and ED's option (b) becomes precise. In any option, R-1040's text changes to cover the kept type.

### EE6. Should fix. Proof 18, R-1079, R-1080 and R-1093: one pragma that can hide a narrowing wasn't tested

- **Where:** proof 18 in `proofs.md`.
- **What's wrong:** plpgsql_check 2.10.16 documents a `disable:performance_warnings` pragma, under "Supported pragmas" in its README. It comes in the short form `PERFORM 'PRAGMA:DISABLE:PERFORMANCE_WARNINGS'` and through `plpgsql_check_pragma(...)`. Placed in the top block's declarations, it applies to the whole function.
  - R-1080's narrowing is a `performance` finding, so this pragma would likely silence a decision that R-1080 says must stop `makemigrations`.
  - R-1079 allows pragma lines, and R-1093 refuses only the in-comment form.
  - The proof showed that the in-comment `performance_warnings = false` hides a narrowing, but it didn't test the pragma line.
  - I couldn't run it: building the extension was refused in my environment.
- **Fix:**
  - Add the case to proof 18, restoring the spike temporarily, as step 02 did under DE.
  - If the pragma hides the narrowing, it needs an owner decision. Options:
    - **(a) Recommended.** Step 59 refuses `disable:performance_warnings` in either form, as R-1093 does.
    - **(b)** List it with the README's loud pragma notices.

### EE7. Minor. Proof 15 against R-0994: `DISCARD PLANS` does clear some shapes

- **Where:** proof 15's "What clears it" bullet; R-0994 in step 37.
- **What's wrong:** R-0994 says `DISCARD PLANS` doesn't clear a stale compiled function. On 15 to 18 it clears `%ROWTYPE` and the trigger, though not `DECLARE` or `INTO`. Proof 14's `%ROWTYPE` column-change case is cleared by it too. `proofs.md` notes the difference from the step's text but doesn't name R-0994. Nothing in the design relies on `DISCARD PLANS`, so this only corrects text.
- **Fix:** name R-0994 in `proofs.md`. My recommendation is to reword R-0994 to say it doesn't clear every shape, such as a `DECLARE` variable, with the INDEX entry and manifest update. That needs the owner's approval.

### EE8. Minor. Proof 13: statements cached before `ADD VALUE` weren't run after it

- **Where:** `p13.rs` runs the statements it kept only after the swap. Proof 13 in `proofs.md`.
- **What's wrong:** `ADD VALUE` is the most common change and the path step 56 builds on. The spike never runs a statement prepared before `ADD VALUE` after it, with the new label.
- **Evidence:** I ran three statements prepared before `ADD VALUE 'z'` with the new label, on 18.6 and 17.11. All returned `z` with no error:
  - a free `($1::st)::text`;
  - an `INSERT` binding `z`;
  - a `SELECT s FROM t` returning the column.
- **Fix:** record this in proof 13. Nothing in the design changes.

### EE9. Minor. Proof 13: the composite decoder's shape leaves out NULL fields

- **Where:** proof 13's "Composite, `FromSql`" bullet in `proofs.md`, and `contracts.md`'s comment.
- **What's wrong:** the recorded shape is a field count, then each field's OID, length and bytes. It doesn't say that a length of `-1` is a NULL field, or that each length must be checked against the buffer. The spike's decoder would panic on a NULL `street` (`raw[12..12 + len]` with `len` as `-1`). The test data had no NULL field, so the proof didn't show this.
- **Fix:** add both points to the recorded shape, so step 56 builds them.

### EE10. Blocks approval. The step's end-to-end check isn't met yet

- **Where:** `steps/55-enum-proofs.md`, "End-to-end check": every item needs "a pass or an owner-approved change".
- **What's wrong:** proofs 14 to 16 are "recorded", and proof 16 contradicts R-0718 and R-0981. Proof 15 differs from the step's text, and Q4 is open. None has an owner-approved change yet. This is waiting on the owner, not an error by the step session.
- **Fix:** once the owner answers EA to ED and EE1 to EE6, do these in this PR, as "Changing the plan" requires:
  - write the answers into `proofs.md`;
  - edit the affected step files: R-0718, R-0981, R-0983, R-0984, R-0975, R-0994, R-1040 and T-0151, T-0168 and T-0171, with others as the answers need;
  - update `coverage.md`, `manifest.json`, `contracts.md` and an INDEX Decisions entry;
  - review the result again.

## Verdict

Not approved yet. The spikes do what `proofs.md` says, the recorded codes, messages and routines are accurate on 15 to 18, and proof 18 passes as written. But four things block approval:

- **EE1:** a silent wrong-answer case on the default online path, recorded as "right".
- **EE2:** EA's recommendation would accept a lookup that fails behind a PgBouncer morm supports.
- **EE3 to EE6:** each changes what the owner is choosing between in EB to ED, or adds a case to proof 18.
- **EE10:** the step's end-to-end check isn't met.
