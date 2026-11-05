# Step 03: Freeze the contracts

- **Phase:** Foundation
- **Depends on:** 02
- **Can run beside:** nothing
- **Crates:** `morm`, `morm-core`, `morm-macros`: traits and types only
- **PR approval:** the owner approves the PR

Read `../rules.md` and `../contracts.md` first. Everything needed for this step is in those two files and this one.

## Goal

Turn `contracts.md` into compiling Rust: the shared traits, types and error enum that every later step builds on, so parallel steps can't drift apart.

## Deliverables

- Everything listed in `contracts.md`, compiled, with signatures only where a later step fills in behaviour, including the items marked "defined in this step"
- `contracts.md` matching that code exactly
- The parallel groups in `INDEX.md` confirmed against what `contracts.md` covers

## End-to-end check

`cargo doc` and `cargo test` pass, and a small test crate implements each trait by hand to prove the signatures are usable, including a model with no primary key and one with a composite key.

## Out of scope

- Behaviour of any feature

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

None beyond the deliverables above.

## Test obligations

None beyond one tagged test per requirement.
