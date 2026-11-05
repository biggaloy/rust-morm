//! Checks the workspace against the layout and dependency rules in the plan.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// The direct dependencies each published crate may have, from `contracts.md`
/// and the crate's own step. A new crate needs a reason in its PR: add it here
/// in that PR.
const ALLOWED_DEPS: &[(&str, &[&str])] = &[
    (
        "morm",
        &["morm-core", "morm-macros", "tokio-postgres", "deadpool"],
    ),
    ("morm-core", &[]),
    ("morm-macros", &["morm-core"]),
    ("morm-migrate", &["morm-core", "tokio-postgres"]),
    ("morm-admin", &["morm-core", "morm-migrate"]),
];

const CLI_FRAMEWORKS: &[&str] = &[
    "argh",
    "argparse",
    "bpaf",
    "clap",
    "docopt",
    "getopts",
    "gumdrop",
    "lexopt",
    "pico-args",
    "structopt",
];

const ASYNC_AND_DATABASE: &[&str] = &[
    "async-std",
    "deadpool",
    "futures",
    "futures-util",
    "mio",
    "postgres",
    "smol",
    "tokio",
    "tokio-postgres",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned()
}

fn metadata() -> Value {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version=1", "--locked"])
        .current_dir(root())
        .output()
        .expect("cargo metadata should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn package<'a>(meta: &'a Value, name: &str) -> &'a Value {
    let member = |p: &&Value| {
        p["name"] == name
            && meta["workspace_members"]
                .as_array()
                .unwrap()
                .contains(&p["id"])
    };
    meta["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(member)
        .unwrap_or_else(|| panic!("{name} isn't a workspace member"))
}

fn target_kinds(package: &Value) -> BTreeSet<&str> {
    package["targets"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|t| t["kind"].as_array().unwrap())
        .map(|k| k.as_str().unwrap())
        .collect()
}

fn normal_deps(package: &Value) -> Vec<&Value> {
    package["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["kind"].is_null())
        .collect()
}

/// Everything `name` links, directly or not, leaving out build and dev tools.
fn closure(meta: &Value, name: &str) -> BTreeSet<String> {
    let nodes: BTreeMap<&str, &Value> = meta["resolve"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| (n["id"].as_str().unwrap(), n))
        .collect();
    let names: BTreeMap<&str, &str> = meta["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["id"].as_str().unwrap(), p["name"].as_str().unwrap()))
        .collect();
    let mut seen = BTreeSet::new();
    let mut stack = vec![package(meta, name)["id"].as_str().unwrap()];
    while let Some(id) = stack.pop() {
        for dep in nodes[id]["deps"].as_array().unwrap() {
            let normal = dep["dep_kinds"]
                .as_array()
                .unwrap()
                .iter()
                .any(|k| k["kind"].is_null());
            let dep_id = dep["pkg"].as_str().unwrap();
            if normal && seen.insert(names[dep_id].to_owned()) {
                stack.push(dep_id);
            }
        }
    }
    seen
}

// covers: R-0038
#[test]
fn the_layout_matches_the_plan() {
    let root = root();
    for path in [
        "crates/morm",
        "crates/morm-core",
        "crates/morm-macros",
        "crates/morm-migrate",
        "crates/morm-admin",
        "examples/app",
        "tests",
        ".github/workflows",
        "README.md",
    ] {
        assert!(root.join(path).exists(), "{path} is missing");
    }
    let meta = metadata();
    for (name, _) in ALLOWED_DEPS {
        let package = package(&meta, name);
        assert_eq!(package["edition"], "2024", "{name}");
        assert_eq!(package["rust_version"], "1.99.1", "{name}");
    }
    for name in ["morm-example-app", "morm-tests"] {
        let package = package(&meta, name);
        assert_eq!(package["edition"], "2024", "{name}");
        assert_eq!(
            package["publish"],
            serde_json::json!([]),
            "{name} isn't published"
        );
    }
}

// covers: R-0039
#[test]
fn morm_is_the_runtime_library() {
    let meta = metadata();
    assert!(target_kinds(package(&meta, "morm")).contains("lib"));
    let deps = closure(&meta, "morm");
    assert!(deps.contains("morm-macros") && deps.contains("morm-core"));
}

// covers: R-0040
#[test]
fn morm_macros_is_a_proc_macro_crate() {
    let meta = metadata();
    assert_eq!(
        target_kinds(package(&meta, "morm-macros")),
        BTreeSet::from(["proc-macro"])
    );
}

// covers: R-0041
#[test]
fn morm_migrate_is_used_by_morm_admin_and_not_by_morm() {
    let meta = metadata();
    assert!(target_kinds(package(&meta, "morm-migrate")).contains("lib"));
    assert!(closure(&meta, "morm-admin").contains("morm-migrate"));
    assert!(!closure(&meta, "morm").contains("morm-migrate"));
    assert!(!closure(&meta, "morm-migrate").contains("morm"));
}

// covers: R-0042
#[test]
fn morm_admin_is_an_installable_binary() {
    let meta = metadata();
    let package = package(&meta, "morm-admin");
    assert_eq!(target_kinds(package), BTreeSet::from(["bin", "test"]));
    assert!(
        package["publish"].is_null(),
        "cargo install needs it published"
    );
    assert!(!closure(&meta, "morm-admin").contains("morm"));
}

// covers: R-0727
#[test]
fn published_crates_add_no_unlisted_dependency() {
    let meta = metadata();
    for (name, allowed) in ALLOWED_DEPS {
        for dep in normal_deps(package(&meta, name)) {
            let dep = dep["name"].as_str().unwrap();
            assert!(
                allowed.contains(&dep),
                "{name} depends on {dep}: give the reason in the PR and add it to ALLOWED_DEPS"
            );
        }
    }
}

// covers: R-0727
#[test]
fn outside_crates_are_taken_without_their_default_features() {
    let meta = metadata();
    for (name, _) in ALLOWED_DEPS {
        for dep in normal_deps(package(&meta, name)) {
            let dep_name = dep["name"].as_str().unwrap();
            assert!(
                dep_name.starts_with("morm") || dep["uses_default_features"] == false,
                "{name} takes {dep_name} with default features: name the features it needs"
            );
        }
    }
}

// covers: R-0727
#[test]
fn morm_admin_links_no_cli_framework() {
    let deps = closure(&metadata(), "morm-admin");
    for framework in CLI_FRAMEWORKS {
        assert!(!deps.contains(*framework), "morm-admin links {framework}");
    }
}

// covers: R-0727
#[test]
fn morm_core_links_no_async_or_database_crate() {
    let deps = closure(&metadata(), "morm-core");
    for name in ASYNC_AND_DATABASE {
        assert!(!deps.contains(*name), "morm-core links {name}");
    }
}
