//! Checks the workflow files and repo files against the plan's rules.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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

/// The permissions a job sets, as trimmed lines. Empty if it sets none.
fn job_permissions(job: &[&str]) -> Vec<String> {
    let Some(start) = job.iter().position(|l| l.starts_with("    permissions:")) else {
        return Vec::new();
    };
    let deeper = |line: &&&str| line.starts_with("      ");
    let scopes = job[start + 1..].iter().take_while(deeper);
    std::iter::once(&job[start])
        .chain(scopes)
        .map(|line| line.trim().to_owned())
        .collect()
}

/// Every workflow step, as its lines.
fn steps(workflow: &str) -> Vec<Vec<&str>> {
    let mut steps: Vec<Vec<&str>> = Vec::new();
    let mut in_steps = false;
    for line in workflow.lines() {
        if line.starts_with("      - ") && in_steps {
            steps.push(vec![line]);
        } else if line == "    steps:" {
            in_steps = true;
        } else if line.starts_with("        ") && in_steps {
            steps.last_mut().unwrap().push(line);
        } else if !line.is_empty() {
            in_steps = false;
        }
    }
    steps
}

/// The one step in `workflow` holding `text`.
fn step<'a>(workflow: &'a str, text: &str) -> Vec<&'a str> {
    let mut found = steps(workflow);
    found.retain(|lines| holds(lines, text));
    assert_eq!(found.len(), 1, "expected one step with {text}");
    found.remove(0)
}

/// Every `action.yml` under `.github/actions`.
fn local_actions() -> Vec<PathBuf> {
    fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, found);
            } else if matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some("action.yml" | "action.yaml")
            ) {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    walk(&root().join(".github/actions"), &mut found);
    found
}

fn run_script(script: &str, args: &[&Path]) -> Output {
    Command::new("python3")
        .arg(root().join(".github/scripts").join(script))
        .args(args)
        .output()
        .expect("python3 should run")
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
}

// covers: R-0794
#[test]
fn bench_results_reach_the_comparison() {
    let bench = read(".github/workflows/bench.yml");
    let output = "bench-results/cargo-bench.txt";
    let metrics = "bench-results/metrics.json";
    assert!(holds(
        &step(&bench, "cargo bench --locked"),
        &format!("| tee {output}")
    ));
    let collect = step(&bench, "bench_metrics.py");
    assert!(holds(&collect, output), "cargo bench's output isn't read");
    assert!(holds(&collect, metrics), "the measurements aren't written");
    assert!(holds(&step(&bench, "gh run download"), "bench-results"));
    let compare = step(&bench, "bench_compare.py");
    assert!(holds(&compare, &format!("{metrics} previous/metrics.json")));
    assert!(holds(
        &step(&bench, "actions/upload-artifact"),
        "path: bench-results"
    ));
}

// covers: R-0794
#[test]
fn bench_scripts_compare_benchmarks_with_the_last_run() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("bench-scripts");
    fs::create_dir_all(&dir).unwrap();
    let file = |name: &str, text: &str| {
        let path = dir.join(name);
        fs::write(&path, text).unwrap();
        path
    };
    let binary = file("binary", "12345678");
    let rss = file("rss.txt", "2048\n");
    let output = file(
        "output.txt",
        "running 2 tests\n\
         test decode_row ... bench:       1,250 ns/iter (+/- 80)\n\
         encode_row              time:   [2.00 µs 2.50 µs 3.00 µs]\n\
         a_group/with_a_very_long_name\n\
         \x20                       time:   [1.0 ms 1.5 ms 2.0 ms]\n",
    );
    let metrics = dir.join("metrics.json");
    let result = run_script("bench_metrics.py", &[&binary, &rss, &output, &metrics]);
    assert!(result.status.success(), "{result:?}");
    let written: serde_json::Value = serde_json::from_slice(&fs::read(&metrics).unwrap()).unwrap();
    assert_eq!(
        written,
        serde_json::json!({
            "morm-admin binary bytes": 8,
            "morm-admin peak RSS KiB": 2048,
            "bench decode_row ns": 1250.0,
            "bench encode_row ns": 2500.0,
            "bench a_group/with_a_very_long_name ns": 1500000.0,
        })
    );

    let previous = file("previous.json", r#"{"bench decode_row ns": 1000.0}"#);
    let result = run_script("bench_compare.py", &[&metrics, &previous]);
    assert!(result.status.success(), "{result:?}");
    let table = String::from_utf8(result.stdout).unwrap();
    assert!(
        table.contains("| bench decode_row ns | 1000.0 | 1250.0 | +25.0% |"),
        "{table}"
    );

    let unreadable = file("bad.txt", "test decode_row ... bench: fast\n");
    let result = run_script("bench_metrics.py", &[&binary, &rss, &unreadable, &metrics]);
    assert!(!result.status.success(), "a lost result must fail the run");
}

// covers: R-0794
#[test]
fn rss_is_read_from_its_own_file_and_not_from_program_output() {
    let bench = read(".github/workflows/bench.yml");
    let measure = step(&bench, "/usr/bin/time");
    assert!(holds(&measure, "-o bench-results/rss.txt"));
    assert!(holds(&measure, "morm-admin --version"));
    assert!(
        !holds(&measure, "2>&1"),
        "program output mixes with the RSS"
    );
    assert!(holds(
        &step(&bench, "bench_metrics.py"),
        "bench-results/rss.txt"
    ));
}

// covers: R-0794, R-0957
#[test]
fn a_failure_on_the_left_of_a_pipe_fails_the_step() {
    let mut piped = 0;
    for path in workflow_files() {
        let text = fs::read_to_string(&path).unwrap();
        for step in steps(&text) {
            if holds(&step, " | ") {
                piped += 1;
                assert!(
                    holds(&step, "shell: bash"),
                    "{}: without `shell: bash` there's no pipefail: {step:?}",
                    path.display()
                );
            }
        }
    }
    assert!(piped > 0, "found no pipes: the check is broken");
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
    assert!(holds(
        &job(&release, "publish"),
        "needs: [verify, test, binaries]"
    ));
    assert!(holds(
        &job(&release, "binaries"),
        "cargo build --locked --release -p morm-admin"
    ));
}

// covers: R-0957
#[test]
fn release_runs_the_tag_check_with_full_history_before_publishing() {
    let release = read(".github/workflows/release.yml");
    let verify = job(&release, "verify");
    assert!(holds(&verify, "check_release_tag.py \"$GITHUB_REF_NAME\""));
    assert!(
        holds(&verify, "fetch-depth: 0"),
        "the check needs all history"
    );
    assert!(
        !release.contains("continue-on-error"),
        "a failed check would count as passing"
    );

    let publish = job(&release, "publish");
    assert!(holds(&publish, "needs: [verify, test, binaries]"));
    let github_release = job(&release, "github-release");
    assert!(holds(&github_release, "needs: publish"));
    for lines in [&publish, &github_release] {
        for status_check in ["always()", "failure()", "cancelled()"] {
            assert!(
                !holds(lines, status_check),
                "{status_check} would run the job after a failed check"
            );
        }
    }
}

/// Runs the release check in `repo`, returning whether it passed and its stderr.
fn check_tag(repo: &Path, tag: &str) -> (bool, String) {
    let script = root().join(".github/scripts/check_release_tag.py");
    let output = Command::new("python3")
        .arg(script)
        .arg(tag)
        .current_dir(repo)
        .output()
        .expect("python3 should run");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (output.status.success(), stderr)
}

fn git(repo: &Path, args: &[&str]) {
    let config = ["-c", "user.name=t", "-c", "user.email=t@example.com"];
    let output = Command::new("git")
        .args(config)
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(repo)
        .output()
        .expect("git should run");
    assert!(output.status.success(), "git {args:?}: {output:?}");
}

// covers: R-0957
#[test]
fn the_tag_check_refuses_anything_but_a_matching_tag_on_main() {
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("release-check");
    let _ = fs::remove_dir_all(&base);
    let repo = base.join("repo");
    fs::create_dir_all(&repo).unwrap();
    fs::write(
        repo.join("Cargo.toml"),
        "[workspace.package]\nversion = \"1.2.3\"\n",
    )
    .unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["add", "Cargo.toml"]);
    git(&repo, &["commit", "-q", "-m", "first"]);
    git(&repo, &["commit", "-q", "--allow-empty", "-m", "second"]);

    let (passed, stderr) = check_tag(&repo, "v1.2.3");
    assert!(!passed, "origin/main is missing, so the check can't pass");
    assert!(stderr.contains("missing"), "{stderr}");

    git(&repo, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    let (passed, stderr) = check_tag(&repo, "v1.2.3");
    assert!(passed, "a matching tag on main must pass: {stderr}");

    git(&repo, &["checkout", "-q", "HEAD~1"]);
    let (passed, stderr) = check_tag(&repo, "v1.2.3");
    assert!(passed, "an older commit on main must pass: {stderr}");

    for tag in ["v1.2.4", "1.2.3", "v1.2.3-rc1"] {
        let (passed, stderr) = check_tag(&repo, tag);
        assert!(!passed, "{tag} doesn't match the version");
        assert!(stderr.contains("doesn't match"), "{stderr}");
    }

    git(&repo, &["checkout", "-q", "--detach", "main"]);
    git(&repo, &["commit", "-q", "--allow-empty", "-m", "off main"]);
    let (passed, stderr) = check_tag(&repo, "v1.2.3");
    assert!(!passed, "a commit off main must be refused");
    assert!(stderr.contains("isn't on main"), "{stderr}");

    git(&repo, &["checkout", "-q", "main"]);
    let url = format!("file://{}", repo.display());
    git(&base, &["clone", "-q", "--depth=1", &url, "shallow"]);
    let (passed, stderr) = check_tag(&base.join("shallow"), "v1.2.3");
    assert!(!passed, "a shallow checkout must be refused");
    assert!(stderr.contains("shallow"), "{stderr}");
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
    let mut files = local_actions();
    files.push(root().join(".github/workflows/ci.yml"));
    for path in files {
        let text = fs::read_to_string(&path).unwrap();
        assert!(
            !text.contains("continue-on-error"),
            "{}: a job that's allowed to fail counts as passing",
            path.display()
        );
    }
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
        let mut block = top_level_block(&text, "permissions");
        block.retain(|line| !line.is_empty());
        assert_eq!(
            block,
            ["permissions:", "  contents: read"],
            "{}",
            path.display()
        );
    }
}

// covers: R-0957
#[test]
fn only_the_jobs_that_need_it_raise_permissions() {
    let mut jobs_seen = 0;
    for path in workflow_files() {
        let text = fs::read_to_string(&path).unwrap();
        let file = path.file_name().unwrap().to_str().unwrap();
        for name in job_names(&text) {
            jobs_seen += 1;
            let expected: &[&str] = match (file, name.as_str()) {
                ("bench.yml", "bench") => &["permissions:", "contents: read", "actions: read"],
                ("release.yml", "github-release") => &["permissions:", "contents: write"],
                _ => &[],
            };
            assert_eq!(
                job_permissions(&job(&text, &name)),
                expected,
                "{file}: {name}"
            );
        }
    }
    assert!(jobs_seen > 0, "found no jobs: the check is broken");
}

// covers: R-0957
#[test]
fn every_action_is_pinned_to_a_full_commit() {
    let mut files = workflow_files();
    files.extend(local_actions());
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
    for action in local_actions() {
        let dir = action.parent().unwrap().strip_prefix(root()).unwrap();
        let entry = format!("- /{}\n", dir.display());
        assert!(dependabot.contains(&entry), "dependabot misses {entry}");
    }
}
