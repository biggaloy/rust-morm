# Step 28: Registration and `morm-mgr`

- **Phase:** Migrations
- **Depends on:** 06, 05
- **Can run beside:** 10, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 27, 47
- **Crates:** `morm-macros`, `morm-core`, `morm-admin`
- **PR approval:** CI and an independent review

Read `../rules.md`, `../contracts.md` and, once it exists, `../proofs.md` first. Everything needed for this step is in those files and this one.

## Goal

Get model metadata out of a project without touching the app binary.

## Deliverables

- Registration behind `cfg(morm_mgr)`, a minimal `morm-mgr` that prints the JSON manifest, the `morm-admin` code that builds it in its own target directory with that `cfg`, manifest types in `morm-core`

## End-to-end check

`morm-mgr` prints a correct manifest for models in several crates, including one nothing references, and the app binary contains none of it.

## Out of scope

- `morm-mgr` packaging for `init` (step 42)

## Requirements

Copied word for word from the approved plan. Every ID must end with a test tagged with it, as `rules.md` describes. Section references such as "section 8" mean the frozen `plans/rust-morm-plan.md`; everything a step needs is copied here or in `contracts.md`.

### Crates and tools

**R-0045** **How it fits together:** `morm-admin` holds the engine. When a command needs the models, it builds and runs `morm-mgr` to get the manifest. Commands that don't need models (`migrate`, `rollback`, `status`, `purge`) run from `morm-migrations/` alone, so a production host needs `morm-admin` and no Rust toolchain. The app binary carries no migration code or schema metadata, apart from the short per-model column list `check_schema` reads.

**R-0046** **Registration:** models, objects and `sql!` queries register themselves through a distributed static list, with no central list to maintain. Their metadata is behind `cfg(morm_mgr)`, not a cargo feature, because `cargo build --workspace` unifies features and would pull it into the app. `morm-admin` builds `morm-mgr` in its own target directory with that `cfg` set. This needs proof early, including models in several crates, where a linker can drop a crate nothing references (section 16).

## Test obligations

None beyond one tagged test per requirement.
