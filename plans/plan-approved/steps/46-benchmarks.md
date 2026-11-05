# Step 46: Benchmarks

- **Phase:** Finish
- **Depends on:** 25, 26, 45, 44
- **Can run beside:** 37, 38, 39, 41, 42, 43, 47
- **Crates:** `benches/`
- **PR approval:** CI and an independent review

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Measure every performance claim the plan makes, and hold it to its targets.

## Deliverables

- Criterion suite against raw tokio-postgres, SQLx, SeaORM and Diesel, every comparison listed below, `bench.yml` comparing with the last run

## End-to-end check

Every target in the requirements is met or reported to the owner with numbers. A missed target is a bug.

Other steps defer these measurements here: `.lock_ordered()` cost, statement cache split against a single cache, prepared against `query_typed` patches, `Bits<N>` word cost, `fetch_json` against `fetch_ref`, conversion-layer overhead, the `COPY` threshold, and the array literal path.

## Context from other steps

Owned elsewhere, and quoted so this step needs no other file. Nothing here needs a test in this step.

> | Features compiled out | None |
>
> | `tracing` compiled in, no subscriber | One relaxed atomic load |
>
> | `tracing` on, query not sampled | That load, a thread-local counter, and two clock reads for `slow_query` |
>
> | `tracing` on, query sampled | One span, with fields recorded only then |
>
> | `metrics` on | Two clock reads and one uncontended bucket increment |
>

## Out of scope

- New features

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Performance and memory / Statements

- **R-0712** The first benchmark run measures this split against a single cache, and a patch through `query_typed` against a prepared one.

### Performance and memory / Build and benchmarks

- **R-0739** Release profile for benches and examples, recommended in the docs: `lto = "fat"`, `codegen-units = 1`, `strip = true`, `panic = "unwind"` (so a panicking handler doesn't kill the server). A library's profile doesn't reach its users.
- **R-0740** Criterion benchmarks in CI against raw tokio-postgres, SQLx, SeaORM and Diesel.
- **R-0741** Telemetry overhead on a primary-key read, for each row of the cost table in section 11. Target: under 1% with `tracing` on and the default `span_sample`. A miss is a bug, not a note in the docs.
- **R-0742** Also: `morm::Pool` vs `deadpool-postgres`, pool sizes under mixed fast and slow queries, `fetch_ref` vs `fetch` vs `fetch_json`, morm's `Decimal` decoding vs `rust_decimal`'s, binary size and RSS with `tls` compiled in but unused, and RSS on both app and Postgres.

## Test obligations

None beyond one tagged test per requirement.
