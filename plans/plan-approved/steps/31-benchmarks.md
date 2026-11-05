# Step 31: Benchmarks

- **Phase:** Finish
- **Depends on:** 19, 20, 30
- **Can run beside:** nothing
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

## Out of scope

- New features

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Performance and memory / Build and benchmarks

- **R-0693** Release profile for benches and examples, recommended in the docs: `lto = "fat"`, `codegen-units = 1`, `strip = true`, `panic = "unwind"` (so a panicking handler doesn't kill the server). A library's profile doesn't reach its users.
- **R-0694** Criterion benchmarks in CI against raw tokio-postgres, SQLx, SeaORM and Diesel.
- **R-0695** Telemetry overhead on a primary-key read, for each row of the cost table in section 11. Target: under 1% with `tracing` on and the default `span_sample`. A miss is a bug, not a note in the docs.
- **R-0696** Also: `morm::Pool` vs `deadpool-postgres`, pool sizes under mixed fast and slow queries, `fetch_ref` vs `fetch` vs `fetch_json`, morm's `Decimal` decoding vs `rust_decimal`'s, binary size and RSS with `tls` compiled in but unused, and RSS on both app and Postgres.

## Test obligations

None beyond one tagged test per requirement.
