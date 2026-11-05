//! Checks the workflow files and repo files against the plan's rules.

use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned()
}

fn read(path: &str) -> String {
    fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn workflow_files() -> Vec<PathBuf> {
    let dir = root().join(".github/workflows");
    fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect()
}

/// The lines of a top-level YAML block, from `name:` to the next top-level key.
fn top_level_block<'a>(text: &'a str, name: &str) -> Vec<&'a str> {
    let header = format!("{name}:");
    let mut lines = text.lines().skip_while(|line| *line != header);
    let first = lines
        .next()
        .unwrap_or_else(|| panic!("no top-level {header}"));
    let is_body = |line: &&str| line.starts_with(' ') || line.is_empty();
    std::iter::once(first)
        .chain(lines.take_while(is_body))
        .collect()
}

/// The lines of a job, from its header to the next job.
fn job<'a>(workflow: &'a str, name: &str) -> Vec<&'a str> {
    let header = format!("  {name}:");
    let mut lines = workflow.lines().skip_while(|line| *line != header);
    let first = lines.next().unwrap_or_else(|| panic!("no job {name}"));
    let is_body = |line: &&str| line.starts_with("    ") || line.is_empty();
    std::iter::once(first)
        .chain(lines.take_while(is_body))
        .collect()
}

/// The job names under the `jobs:` key.
fn job_names(workflow: &str) -> Vec<String> {
    top_level_block(workflow, "jobs")
        .iter()
        .filter(|line| line.starts_with("  ") && !line.starts_with("   ") && line.ends_with(':'))
        .filter(|line| !line.trim_start().starts_with('#'))
        .map(|line| line.trim().trim_end_matches(':').to_owned())
        .collect()
}

fn holds(lines: &[&str], text: &str) -> bool {
    lines.iter().any(|line| line.contains(text))
}

/// `uses:` values in a file, without any trailing comment.
fn used_actions(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            line.trim_start()
                .trim_start_matches("- ")
                .strip_prefix("uses: ")
        })
        .map(|used| used.split_whitespace().next().unwrap().to_owned())
        .collect()
}

fn pinned_to_a_commit(action: &str) -> bool {
    action.split_once('@').is_some_and(|(_, rev)| {
        rev.len() == 40
            && rev
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

// covers: R-0792
#[test]
fn ci_runs_every_check_the_plan_names() {
    let ci = read(".github/workflows/ci.yml");
    for (job_name, command) in [
        (
            "coverage",
            "plans/plan-approved/tools/check_coverage.py --code crates tests benches fuzz .github",
        ),
        ("fmt", "cargo fmt --all --check"),
        ("clippy", "-- -D warnings"),
        (
            "unit",
            "cargo test --locked --workspace --exclude morm-tests --all-targets",
        ),
        ("trybuild", "--features ui --test ui"),
        ("doc", "cargo test --locked --workspace --doc"),
        ("integration", "cargo test --locked -p morm-tests"),
        (
            "integration",
            "cargo test --locked -p morm-migrate --all-features",
        ),
        ("pooler", "--features pooler --test smoke --test pooler"),
        ("pooler", "pgbouncer -d"),
        ("msrv", "cargo build --locked --workspace --all-targets"),
        ("features", "feature_combos.py"),
    ] {
        assert!(
            holds(&job(&ci, job_name), command),
            "{job_name} doesn't run {command}"
        );
    }
    let integration = job(&ci, "integration");
    assert!(holds(&integration, "transport: [tcp, unix]"));
}

// covers: R-0793
#[test]
fn nightly_runs_daily_and_on_demand_with_every_long_check() {
    let nightly = read(".github/workflows/nightly.yml");
    let triggers = top_level_block(&nightly, "on");
    assert!(holds(&triggers, "workflow_dispatch"));
    let cron = triggers
        .iter()
        .find(|line| line.contains("cron:"))
        .expect("no schedule");
    let fields: Vec<&str> = cron.split('"').nth(1).unwrap().split(' ').collect();
    assert_eq!(fields[2..], ["*", "*", "*"], "not daily");
    for (job_name, command) in [
        ("property", "PROPTEST_CASES"),
        ("fuzz", "cargo fuzz run"),
        (
            "full-chain",
            "cargo test --locked -p morm-migrate --all-features",
        ),
        ("audit", "cargo audit"),
        ("deny", "cargo deny --locked check"),
    ] {
        assert!(
            holds(&job(&nightly, job_name), command),
            "{job_name} doesn't run {command}"
        );
    }
}

// covers: R-0794
#[test]
fn bench_runs_weekly_and_on_demand_and_compares_with_the_last_run() {
    let bench = read(".github/workflows/bench.yml");
    let triggers = top_level_block(&bench, "on");
    assert!(holds(&triggers, "workflow_dispatch"));
    let cron = triggers
        .iter()
        .find(|line| line.contains("cron:"))
        .expect("no schedule");
    let fields: Vec<&str> = cron.split('"').nth(1).unwrap().split(' ').collect();
    assert_eq!(fields[2..4], ["*", "*"], "not weekly");
    assert_ne!(fields[4], "*", "not weekly");
    let job = job(&bench, "bench");
    for text in [
        "cargo bench",
        "binary bytes",
        "peak RSS",
        "gh run download",
        "bench_compare.py",
    ] {
        assert!(holds(&job, text), "bench doesn't do {text}");
    }
}

// covers: R-0795
#[test]
fn release_tests_publishes_and_builds_binaries_on_a_tag() {
    let release = read(".github/workflows/release.yml");
    assert!(holds(
        &job(&release, "test"),
        "cargo test --locked --workspace"
    ));
    assert!(holds(
        &job(&release, "publish"),
        "cargo publish --locked --workspace"
    ));
    assert!(holds(&job(&release, "publish"), "needs: [test, binaries]"));
    assert!(holds(
        &job(&release, "binaries"),
        "cargo build --locked --release -p morm-admin"
    ));
}

// covers: R-0796
#[test]
fn the_ci_job_fails_unless_every_other_job_passed() {
    let ci = read(".github/workflows/ci.yml");
    let gate = job(&ci, "ci");
    assert!(
        holds(&gate, "if: always()"),
        "a skipped required check counts as passing"
    );
    assert!(holds(&gate, "[ \"$result\" = success ] || exit 1"));
    let needs = gate
        .iter()
        .find_map(|line| line.trim().strip_prefix("needs: "))
        .expect("no needs");
    let needs: Vec<&str> = needs.trim_matches(['[', ']']).split(", ").collect();
    for name in job_names(&ci) {
        assert!(
            name == "ci" || needs.contains(&name.as_str()),
            "ci doesn't wait for {name}"
        );
    }
}

// covers: R-0797
#[test]
fn the_full_matrix_runs_on_every_push_and_pr_on_standard_runners() {
    let ci = read(".github/workflows/ci.yml");
    assert_eq!(
        top_level_block(&ci, "on")
            .iter()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>(),
        ["on:", "push:", "pull_request:"],
        "no filter may narrow which pushes and PRs run"
    );
    assert!(holds(&job(&ci, "integration"), "pg: [15, 16, 17, 18]"));
    for line in ci
        .lines()
        .filter(|line| line.trim_start().starts_with("runs-on:"))
    {
        assert!(line.contains("ubuntu-"), "not a standard runner: {line}");
    }
}

// covers: R-0957
#[test]
fn every_workflow_starts_with_read_only_permissions() {
    for path in workflow_files() {
        let text = fs::read_to_string(&path).unwrap();
        assert_eq!(
            top_level_block(&text, "permissions")[..2],
            ["permissions:", "  contents: read"],
            "{}",
            path.display()
        );
    }
}

// covers: R-0957
#[test]
fn only_the_jobs_that_need_it_raise_permissions() {
    let raised = |file: &str| {
        let text = read(file);
        let mut raised = job_names(&text);
        raised.retain(|name| holds(&job(&text, name), "permissions:"));
        raised
    };
    assert_eq!(raised(".github/workflows/ci.yml"), [""; 0]);
    assert_eq!(raised(".github/workflows/nightly.yml"), [""; 0]);
    assert_eq!(raised(".github/workflows/bench.yml"), ["bench"]);
    assert_eq!(raised(".github/workflows/release.yml"), ["github-release"]);
}

// covers: R-0957
#[test]
fn every_action_is_pinned_to_a_full_commit() {
    let mut files = workflow_files();
    files.push(root().join(".github/actions/setup-rust/action.yml"));
    let mut seen = 0;
    for path in files {
        for action in used_actions(&fs::read_to_string(&path).unwrap()) {
            seen += 1;
            assert!(
                action.starts_with("./") || pinned_to_a_commit(&action),
                "{}: {action}",
                path.display()
            );
        }
    }
    assert!(seen > 0, "found no actions: the check is broken");
}

// covers: R-0957
#[test]
fn no_workflow_uses_pull_request_target() {
    for path in workflow_files() {
        let text = fs::read_to_string(&path).unwrap();
        assert!(!text.contains("pull_request_target"), "{}", path.display());
    }
}

// covers: R-0957
#[test]
fn only_a_tag_runs_release_and_publishing_uses_the_protected_environment() {
    let release = read(".github/workflows/release.yml");
    assert_eq!(
        top_level_block(&release, "on")
            .iter()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>(),
        ["on:", "push:", "tags:", "- \"v*\""],
    );
    assert!(holds(&job(&release, "publish"), "environment: crates-io"));
    assert!(holds(
        &job(&release, "publish"),
        "if: github.repository == 'biggaloy/rust-morm'"
    ));
    let uses_token = |name: &String| holds(&job(&release, name), "secrets.");
    let with_secrets: Vec<String> = job_names(&release).into_iter().filter(uses_token).collect();
    assert_eq!(with_secrets, ["publish"]);
    for path in workflow_files()
        .iter()
        .filter(|path| !path.ends_with("release.yml"))
    {
        let text = fs::read_to_string(path).unwrap();
        assert!(!text.contains("secrets."), "{}", path.display());
        assert!(!text.contains("cargo publish"), "{}", path.display());
    }
}

// covers: R-0958
#[test]
fn security_md_gives_a_private_way_to_report() {
    let security = read("SECURITY.md");
    assert!(security.contains("don't report a vulnerability in a public issue"));
    assert!(security.contains("@"), "no private address");
    assert!(security.contains("private"));
}

// covers: R-0958
#[test]
fn dependabot_covers_cargo_and_actions() {
    let dependabot = read(".github/dependabot.yml");
    assert!(dependabot.contains("package-ecosystem: cargo"));
    assert!(dependabot.contains("package-ecosystem: github-actions"));
    assert!(dependabot.contains("/.github/actions/setup-rust"));
}
