//! Checks the workflow files and repo files against the plan's rules.

use std::fs;
use std::os::unix::fs::PermissionsExt;
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
        if line.trim_start().starts_with('#') {
            continue;
        }
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

/// Every file under `.github`.
fn github_files() -> Vec<PathBuf> {
    fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, found);
            } else {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    walk(&root().join(".github"), &mut found);
    found
}

/// Every `action.yml` under `.github/actions`.
fn local_actions() -> Vec<PathBuf> {
    let mut found = github_files();
    found.retain(|path| {
        path.starts_with(root().join(".github/actions"))
            && matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some("action.yml" | "action.yaml")
            )
    });
    found
}

fn run_script(script: &str, args: &[&Path]) -> Output {
    Command::new("python3")
        .arg(root().join(".github/scripts").join(script))
        .args(args)
        .output()
        .expect("python3 should run")
}

/// Whether one line is exactly `text`, so a comment or a neutered form doesn't count.
fn has_line(lines: &[&str], text: &str) -> bool {
    lines
        .iter()
        .any(|line| line.trim().trim_start_matches("- ") == text)
}

/// Every command in a job or action, in order: each `run:` value, and each line
/// of a `run: |` block.
fn commands(lines: &[&str]) -> Vec<String> {
    let indent = |line: &str| line.len() - line.trim_start().len();
    let mut found = Vec::new();
    let mut block = None;
    for line in lines {
        if let Some(run_indent) = block {
            if line.trim().is_empty() {
                continue;
            }
            if indent(line) > run_indent {
                found.push(line.trim().to_owned());
                continue;
            }
            block = None;
        }
        match line.trim().trim_start_matches("- ").strip_prefix("run: ") {
            Some("|") => block = Some(indent(line)),
            Some(command) => found.push(command.to_owned()),
            None => {}
        }
    }
    found
}

/// Setup and tool-install lines. No list pins them, so a tool's version lives in
/// one place and can be bumped there.
const SETUP: &[&str] = &[
    "cargo install --locked cargo-",
    "mkdir ",
    "pg_isready ",
    "pgbouncer ",
    "sudo ",
];

/// Fails unless the job runs exactly these commands, setup lines aside. A later
/// step that changes a job's commands changes this list in the same PR.
fn assert_commands(workflow: &str, name: &str, expected: &[&str]) {
    let mut found = commands(&job(workflow, name));
    found.retain(|command| !SETUP.iter().any(|setup| command.starts_with(setup)));
    assert_eq!(found, expected, "job {name}");
}

/// Whether a line of shell turns a failure into a pass. After `||` only `exit`
/// with a failing status may follow, and `true`, `:`, `exit 0`, `return 0`, a
/// leading `!` and `set +...` may not start a command.
fn hides_failure(line: &str) -> bool {
    if line.contains("exit(0)") || line.contains("exit()") {
        return true;
    }
    let mut spaced = line.to_owned();
    for operator in ["||", "&&", ";", "(", ")"] {
        spaced = spaced.replace(operator, &format!(" {operator} "));
    }
    let words: Vec<&str> = spaced
        .split_whitespace()
        .map(|word| word.trim_matches(['"', '\'']))
        .collect();
    let zero = |word: Option<&&str>| word.is_some_and(|w| w.parse() == Ok(0_u32));
    let failing = |word: Option<&&str>| word.is_some_and(|w| w.parse::<u32>().is_ok_and(|n| n > 0));
    let starts_command = |i: usize| {
        i == 0
            || matches!(
                words[i - 1],
                ";" | "||" | "&&" | "|" | "(" | "{" | "then" | "else" | "do" | "run:"
            )
    };
    words.iter().enumerate().any(|(i, word)| match *word {
        "||" => !(words.get(i + 1) == Some(&"exit") && failing(words.get(i + 2))),
        "true" | ":" | "/bin/true" | "/usr/bin/true" | "!" => starts_command(i),
        "exit" | "return" => starts_command(i) && zero(words.get(i + 1)),
        "set" => starts_command(i) && words.get(i + 1).is_some_and(|w| w.starts_with('+')),
        _ => false,
    })
}

/// The `minute hour day month weekday` fields of a workflow's one cron line.
fn cron_fields(workflow: &str) -> Vec<String> {
    let triggers = top_level_block(workflow, "on");
    let cron = triggers
        .iter()
        .find(|line| line.trim().starts_with("- cron: "))
        .expect("no schedule");
    let fields: Vec<String> = cron
        .split('"')
        .nth(1)
        .unwrap()
        .split(' ')
        .map(str::to_owned)
        .collect();
    let fixed = |field: &String| field.parse::<u32>().is_ok();
    assert!(
        fixed(&fields[0]) && fixed(&fields[1]),
        "not one fixed time a day: {cron}"
    );
    fields
}

/// Puts an executable stub called `name` in `dir`, with the given `sh` body.
fn stub(dir: &Path, name: &str, body: &str) {
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// A clean scratch directory for one test.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// `PATH` with `dir` first, so a stub replaces the real tool.
fn path_with(dir: &Path) -> String {
    format!("{}:{}", dir.display(), std::env::var("PATH").unwrap())
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
    for (name, expected) in [
        (
            "coverage",
            &[
                "python3 plans/plan-approved/tools/check_coverage.py --code crates tests benches fuzz .github",
            ][..],
        ),
        ("fmt", &["cargo fmt --all --check"]),
        (
            "clippy",
            &["cargo clippy --locked --workspace --all-targets --all-features -- -D warnings"],
        ),
        (
            "unit",
            &[
                "cargo test --locked --workspace --exclude morm-tests --all-targets",
                "cargo install --locked --path crates/morm-admin --root \"$RUNNER_TEMP/install\"",
                "\"$RUNNER_TEMP/install/bin/morm-admin\"",
            ],
        ),
        (
            "trybuild",
            &["cargo test --locked -p morm-tests --features ui --test ui"],
        ),
        ("doc", &["cargo test --locked --workspace --doc"]),
        (
            "integration",
            &[
                "cargo test --locked -p morm-tests",
                "cargo test --locked -p morm-migrate --all-features",
            ],
        ),
        (
            "pooler",
            &["cargo test --locked -p morm-tests --features pooler --test smoke --test pooler"],
        ),
        ("msrv", &["cargo build --locked --workspace --all-targets"]),
        ("features", &["python3 .github/scripts/feature_combos.py"]),
    ] {
        assert_commands(&ci, name, expected);
    }
}

// covers: R-0792
#[test]
fn integration_targets_each_version_and_transport() {
    let ci = read(".github/workflows/ci.yml");
    let integration = job(&ci, "integration");
    for line in [
        "pg: [15, 16, 17, 18]",
        "transport: [tcp, unix]",
        "transport: tcp",
        "host: 127.0.0.1",
        "transport: unix",
        "host: /tmp/morm-pg",
        "image: postgres:${{ matrix.pg }}",
        "/tmp/morm-pg:/var/run/postgresql",
        "MORM_TEST_TRANSPORT: ${{ matrix.transport }}",
        "MORM_TEST_PG_MAJOR: ${{ matrix.pg }}",
        "MORM_TEST_DSN: host=${{ matrix.host }} port=5432 user=postgres dbname=postgres",
    ] {
        assert!(has_line(&integration, line), "integration lacks {line}");
    }
    let pooler = job(&ci, "pooler");
    for line in [
        "image: postgres:18",
        "MORM_TEST_TRANSPORT: pgbouncer",
        "MORM_TEST_PG_MAJOR: 18",
        "MORM_TEST_DSN: host=127.0.0.1 port=6432 user=postgres dbname=postgres",
        "MORM_TEST_PGBOUNCER_ADMIN_DSN: host=127.0.0.1 port=6432 user=postgres dbname=pgbouncer",
    ] {
        assert!(has_line(&pooler, line), "pooler lacks {line}");
    }
}

// covers: R-0792
#[test]
fn trybuild_and_msrv_run_on_the_msrv() {
    let ci = read(".github/workflows/ci.yml");
    let manifest = read("Cargo.toml");
    let rust_version = manifest
        .lines()
        .find_map(|line| line.strip_prefix("rust-version = "))
        .expect("no rust-version in the workspace");
    let msrv = format!("MSRV: {rust_version}");
    assert!(
        has_line(&top_level_block(&ci, "env"), &msrv),
        "ci.yml's MSRV differs from rust-version, {rust_version}"
    );
    for name in ["msrv", "trybuild"] {
        assert!(
            has_line(&job(&ci, name), "toolchain: ${{ env.MSRV }}"),
            "{name} doesn't run on the MSRV"
        );
    }
}

// covers: R-0792
#[test]
fn setup_rust_installs_the_toolchain_it_is_given() {
    let action = read(".github/actions/setup-rust/action.yml");
    let lines: Vec<&str> = action.lines().collect();
    assert!(has_line(&lines, "TOOLCHAIN: ${{ inputs.toolchain }}"));
    let found = commands(&lines);
    assert!(
        found
            .iter()
            .any(|command| command.starts_with("rustup toolchain install \"$TOOLCHAIN\" ")),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|command| command == "rustup default \"$TOOLCHAIN\""),
        "{found:?}"
    );
}

// covers: R-0792
#[test]
fn feature_combos_checks_every_combination_and_stops_on_a_failure() {
    let dir = scratch("feature-combos");
    let metadata = serde_json::json!({"packages": [
        {"name": "morm", "publish": null, "features": {"default": [], "a": [], "b": []}},
        {"name": "morm-tests", "publish": [], "features": {"pooler": []}},
    ]});
    fs::write(dir.join("metadata.json"), metadata.to_string()).unwrap();
    stub(
        &dir,
        "cargo",
        "case \"$1\" in\n\
         metadata) cat \"$STUB_DIR/metadata.json\" ;;\n\
         check) echo \"$*\" >> \"$STUB_DIR/calls\"\n\
         case \"$*\" in *\"$STUB_FAIL\"*) exit 1 ;; esac ;;\n\
         esac\n",
    );
    let run = |fail_on: &str| {
        let _ = fs::remove_file(dir.join("calls"));
        Command::new("python3")
            .arg(root().join(".github/scripts/feature_combos.py"))
            .env("PATH", path_with(&dir))
            .env("STUB_DIR", &dir)
            .env("STUB_FAIL", fail_on)
            .output()
            .expect("python3 should run")
    };

    assert!(run("no such argument").status.success());
    let calls = fs::read_to_string(dir.join("calls")).unwrap();
    let base = "check --locked -p morm --all-targets";
    let none = "--no-default-features --features";
    assert_eq!(
        calls.lines().collect::<Vec<_>>(),
        [
            format!("{base} {none} "),
            format!("{base} {none} a"),
            format!("{base} {none} b"),
            format!("{base} {none} a,b"),
            base.to_owned(),
        ]
    );

    assert!(!run("--features a,b").status.success());
    let calls = fs::read_to_string(dir.join("calls")).unwrap();
    assert_eq!(calls.lines().count(), 4, "it must stop at the failure");
}

// covers: R-0793
#[test]
fn nightly_runs_daily_and_on_demand_with_every_long_check() {
    let nightly = read(".github/workflows/nightly.yml");
    assert!(has_line(
        &top_level_block(&nightly, "on"),
        "workflow_dispatch:"
    ));
    assert_eq!(cron_fields(&nightly)[2..], ["*", "*", "*"], "not daily");
    assert_commands(&nightly, "property", &["cargo test --locked --workspace"]);
    assert_commands(
        &nightly,
        "fuzz",
        &[
            "if [ ! -d fuzz ]; then",
            "echo \"No fuzz targets yet.\"",
            "else",
            "targets=$(cargo fuzz list)",
            "if [ -z \"$targets\" ]; then",
            "echo \"::error::cargo fuzz list found no targets.\"",
            "exit 1",
            "fi",
            "for target in $targets; do",
            "cargo fuzz run \"$target\" -- -max_total_time=300",
            "done",
            "fi",
        ],
    );
    assert_commands(
        &nightly,
        "full-chain",
        &["cargo test --locked -p morm-migrate --all-features"],
    );
    assert_commands(&nightly, "audit", &["cargo audit"]);
    assert_commands(&nightly, "deny", &["cargo deny --locked check"]);
    let property = job(&nightly, "property");
    assert!(has_line(&property, "PROPTEST_CASES: 100000"));
    assert!(has_line(&property, "MORM_TEST_PG_MAJOR: 18"));
    let full_chain = job(&nightly, "full-chain");
    for line in [
        "pg: [15, 16, 17, 18]",
        "image: postgres:${{ matrix.pg }}",
        "MORM_TEST_PG_MAJOR: ${{ matrix.pg }}",
    ] {
        assert!(has_line(&full_chain, line), "full-chain lacks {line}");
    }
}

// The job's own script, run the way Actions runs a step with no `shell:`.
// covers: R-0793
#[test]
fn fuzz_fails_on_a_failed_or_empty_list_and_skips_without_a_fuzz_folder() {
    let nightly = read(".github/workflows/nightly.yml");
    let dir = scratch("fuzz-job");
    let script = dir.join("job.sh");
    fs::write(&script, commands(&job(&nightly, "fuzz")).join("\n")).unwrap();
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    stub(
        &bin,
        "cargo",
        "echo \"$*\" >> \"$STUB_DIR/calls\"\n\
         if [ \"$*\" = 'fuzz list' ]; then\n\
         [ \"$STUB_MODE\" = empty ] || printf 'one\\ntwo\\n'\n\
         [ \"$STUB_MODE\" != list-fails ] || exit 1\n\
         fi\n",
    );
    let run = |mode: &str, fuzz_folder: bool| {
        let work = scratch("fuzz-job/work");
        if fuzz_folder {
            fs::create_dir(work.join("fuzz")).unwrap();
        }
        let _ = fs::remove_file(dir.join("calls"));
        let output = Command::new("bash")
            .arg("-e")
            .arg(&script)
            .current_dir(&work)
            .env("PATH", path_with(&bin))
            .env("STUB_DIR", &dir)
            .env("STUB_MODE", mode)
            .output()
            .expect("bash should run");
        let calls = fs::read_to_string(dir.join("calls")).unwrap_or_default();
        (output, calls)
    };

    let (output, calls) = run("targets", false);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(calls, "", "no fuzz folder means nothing to run");

    let (output, calls) = run("targets", true);
    assert!(output.status.success(), "{output:?}");
    let fuzz_calls: Vec<&str> = calls.lines().filter(|l| l.starts_with("fuzz ")).collect();
    assert_eq!(
        fuzz_calls,
        [
            "fuzz list",
            "fuzz run one -- -max_total_time=300",
            "fuzz run two -- -max_total_time=300",
        ]
    );

    for mode in ["list-fails", "empty"] {
        let (output, calls) = run(mode, true);
        assert!(!output.status.success(), "{mode} must fail the job");
        assert!(!calls.contains("fuzz run"), "{mode}: {calls}");
    }
}

// covers: R-0794
#[test]
fn bench_runs_weekly_and_on_demand_and_compares_with_the_last_run() {
    let bench = read(".github/workflows/bench.yml");
    assert!(has_line(
        &top_level_block(&bench, "on"),
        "workflow_dispatch:"
    ));
    let cron = cron_fields(&bench);
    assert_eq!(cron[2..4], ["*", "*"], "not weekly");
    assert!(cron[4].parse::<u32>().is_ok(), "not weekly");
    assert_commands(
        &bench,
        "bench",
        &[
            "cargo bench --locked --workspace --exclude morm-tests | tee bench-results/cargo-bench.txt",
            "cargo build --locked --release -p morm-admin",
            "/usr/bin/time -f %M -o bench-results/rss.txt target/release/morm-admin --version > /dev/null",
            "python3 .github/scripts/bench_metrics.py target/release/morm-admin bench-results/rss.txt bench-results/cargo-bench.txt bench-results/metrics.json",
            "bash .github/scripts/fetch_last_bench.sh",
            "python3 .github/scripts/bench_compare.py bench-results/metrics.json previous/metrics.json | tee -a \"$GITHUB_STEP_SUMMARY\"",
        ],
    );
    let job = job(&bench, "bench");
    for line in [
        "GH_TOKEN: ${{ github.token }}",
        "name: bench-results",
        "path: bench-results",
        "retention-days: 90",
    ] {
        assert!(has_line(&job, line), "bench lacks {line}");
    }
}

// The stub runs each `--jq` filter with real `jq` on sample JSON, so a wrong filter
// fails here. Real `gh` prints nothing for a null, and the stub does the same.
// covers: R-0794
#[test]
fn bench_fetches_the_last_results_into_the_folder_it_compares() {
    let dir = scratch("fetch-last-bench");
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    stub(
        &bin,
        "gh",
        r#"echo "$*" >> "$STUB_DIR/calls"
for last in "$@"; do :; done
filter() {
  out=$(printf '%s' "$1" | jq -r "$last") || exit 1
  [ "$out" = null ] || printf '%s\n' "$out"
}
case "$1 $2" in
'run list')
  case "$STUB_MODE" in
  list-fails) exit 1 ;;
  no-run) filter '[]' ;;
  *) filter '[{"databaseId": 123}, {"databaseId": 99}]' ;;
  esac ;;
'api repos/owner/repo/actions/runs/123/artifacts')
  case "$STUB_MODE" in
  api-fails) exit 1 ;;
  no-artifact) filter '{"artifacts": []}' ;;
  expired) filter '{"artifacts": [{"name": "other", "expired": false}, {"name": "bench-results", "expired": true}]}' ;;
  *) filter '{"artifacts": [{"name": "other", "expired": true}, {"name": "bench-results", "expired": false}]}' ;;
  esac ;;
'run download')
  [ "$STUB_MODE" != download-fails ] || exit 1
  mkdir -p "$last"
  [ "$STUB_MODE" = no-metrics ] || echo '{}' > "$last/metrics.json" ;;
*) exit 1 ;;
esac
"#,
    );
    let run = |mode: &str| {
        let work = scratch("fetch-last-bench/work");
        let _ = fs::remove_file(dir.join("calls"));
        let output = Command::new("bash")
            .arg(root().join(".github/scripts/fetch_last_bench.sh"))
            .current_dir(&work)
            .env("PATH", path_with(&bin))
            .env("STUB_DIR", &dir)
            .env("STUB_MODE", mode)
            .env("GITHUB_REF_NAME", "the-branch")
            .env("GITHUB_REPOSITORY", "owner/repo")
            .output()
            .expect("bash should run");
        let calls = fs::read_to_string(dir.join("calls")).unwrap_or_default();
        (output, work.join("previous/metrics.json").exists(), calls)
    };

    let (output, fetched, calls) = run("present");
    assert!(output.status.success(), "{output:?}");
    assert!(fetched, "bench compares previous/metrics.json");
    let calls: Vec<&str> = calls.lines().collect();
    assert_eq!(calls.len(), 3, "{calls:?}");
    assert_eq!(
        calls[0],
        "run list --workflow bench.yml --branch the-branch --status success --limit 1 --json databaseId --jq .[0].databaseId"
    );
    assert_eq!(
        calls[2],
        "run download 123 --name bench-results --dir previous"
    );

    for mode in ["expired", "no-artifact"] {
        let (output, fetched, calls) = run(mode);
        assert!(output.status.success(), "{mode} is a first run: {output:?}");
        assert!(!fetched && !calls.contains("run download"), "{calls}");
        let log = String::from_utf8(output.stdout).unwrap();
        assert!(
            log.contains("::warning::") && log.contains("first run"),
            "{log}"
        );
    }

    let (output, fetched, calls) = run("no-run");
    assert!(output.status.success() && !fetched, "{output:?}");
    assert!(!calls.contains("run download"), "{calls}");

    for mode in ["list-fails", "api-fails", "download-fails", "no-metrics"] {
        let (output, _, _) = run(mode);
        assert!(!output.status.success(), "{mode} must fail the step");
    }
    let (output, _, _) = run("no-metrics");
    let log = String::from_utf8(output.stdout).unwrap();
    assert!(
        log.contains("::error::") && log.contains("metrics.json"),
        "{log}"
    );
}

// covers: R-0794
#[test]
fn bench_scripts_compare_benchmarks_with_the_last_run() {
    let dir = scratch("bench-scripts");
    let file = |name: &str, text: &str| {
        let path = dir.join(name);
        fs::write(&path, text).unwrap();
        path
    };
    let binary = file("binary", "12345678");
    let rss = file("rss.txt", "2048\n");
    let output = file(
        "output.txt",
        "running 1 test\n\
         test decode_row ... bench:       1,250 ns/iter (+/- 80)\n\
         a bench name with spaces  time:   [1.0 ns 2.0 ns 3.0 ns]\n\
         in ps  time:   [1.0 ps 2.0 ps 3.0 ps]\n\
         in us  time:   [1.0 us 3.0 us 4.0 us]\n\
         in micro  time:   [1.0 µs 5.0 µs 6.0 µs]\n\
         in ms  time:   [1.0 ms 7.0 ms 8.0 ms]\n\
         in s  time:   [1.0 s 9.0 s 10.0 s]\n",
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
            "bench a bench name with spaces ns": 2.0,
            "bench in ps ns": 0.002,
            "bench in us ns": 3_000.0,
            "bench in micro ns": 5_000.0,
            "bench in ms ns": 7_000_000.0,
            "bench in s ns": 9_000_000_000.0,
        })
    );

    let previous = file(
        "previous.json",
        r#"{"bench decode_row ns": 1000.0, "bench gone ns": 5.0}"#,
    );
    let result = run_script("bench_compare.py", &[&metrics, &previous]);
    assert!(result.status.success(), "{result:?}");
    let table = String::from_utf8(result.stdout).unwrap();
    for row in [
        "| bench decode_row ns | 1000.0 | 1250.0 | +25.0% |",
        "| bench gone ns | 5.0 | removed | n/a |",
        "| morm-admin peak RSS KiB | n/a | 2048 | n/a |",
    ] {
        assert!(table.contains(row), "{row} isn't in\n{table}");
    }

    let first_run = run_script("bench_compare.py", &[&metrics, &dir.join("missing.json")]);
    assert!(first_run.status.success(), "{first_run:?}");

    let unreadable = file("bad.txt", "test decode_row ... bench: fast\n");
    let result = run_script("bench_metrics.py", &[&binary, &rss, &unreadable, &metrics]);
    assert!(!result.status.success(), "a lost result must fail the run");
    let duplicate = file(
        "duplicate.txt",
        "test decode_row ... bench: 1 ns/iter (+/- 0)\n\
         decode_row  time:   [1.0 ns 2.0 ns 3.0 ns]\n",
    );
    let result = run_script("bench_metrics.py", &[&binary, &rss, &duplicate, &metrics]);
    assert!(!result.status.success(), "a lost result must fail the run");
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert!(stderr.contains("share the name: decode_row"), "{stderr}");
    let bad_rss = file("bad-rss.txt", "Hello\n2048\n");
    let result = run_script("bench_metrics.py", &[&binary, &bad_rss, &output, &metrics]);
    assert!(!result.status.success(), "a bad RSS file must fail the run");
}

// The fixture is real output: Criterion 0.8.2's second run of a throwaway project,
// holding `change:` blocks, a wrapped name and a throughput bench.
// covers: R-0794
#[test]
fn bench_metrics_reads_real_criterion_output() {
    let dir = scratch("bench-criterion");
    let binary = dir.join("binary");
    let rss = dir.join("rss.txt");
    fs::write(&binary, "1234").unwrap();
    fs::write(&rss, "2048\n").unwrap();
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/criterion-0.8.2-second-run.txt");
    let metrics = dir.join("metrics.json");
    let result = run_script("bench_metrics.py", &[&binary, &rss, &fixture, &metrics]);
    assert!(result.status.success(), "{result:?}");
    let written: serde_json::Value = serde_json::from_slice(&fs::read(&metrics).unwrap()).unwrap();
    assert_eq!(
        written,
        serde_json::json!({
            "morm-admin binary bytes": 4,
            "morm-admin peak RSS KiB": 2048,
            "bench short ns": 52689.0,
            "bench a_group_of_benches/with_a_really_quite_long_name_that_wraps/1000 ns": 52784.0,
            "bench tp/bytes ns": 52816.0,
        })
    );
}

// covers: R-0794
#[test]
fn rss_is_read_from_its_own_file_and_not_from_program_output() {
    let bench = read(".github/workflows/bench.yml");
    let measure = step(&bench, "/usr/bin/time");
    assert!(
        !holds(&measure, "2>&1"),
        "program output mixes with the RSS"
    );
    assert!(holds(&measure, "-o bench-results/rss.txt"));
    assert!(holds(&measure, "morm-admin --version"));
    assert!(holds(
        &step(&bench, "bench_metrics.py"),
        "bench-results/rss.txt"
    ));
}

// covers: R-0794
#[test]
fn a_failure_on_the_left_of_a_pipe_fails_the_step() {
    let mut piped = 0;
    for path in workflow_files() {
        let text = fs::read_to_string(&path).unwrap();
        for step in steps(&text) {
            if holds(&step, " | ") {
                piped += 1;
                assert!(
                    has_line(&step, "shell: bash"),
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
    assert_commands(&release, "test", &["cargo test --locked --workspace"]);
    assert_commands(&release, "publish", &["cargo publish --locked --workspace"]);
    assert_commands(
        &release,
        "binaries",
        &[
            "cargo build --locked --release -p morm-admin",
            "tar -czf \"morm-admin-$GITHUB_REF_NAME-$TARGET.tar.gz\" -C target/release morm-admin",
        ],
    );
    assert_commands(
        &release,
        "github-release",
        &[
            "gh release create \"$GITHUB_REF_NAME\" --repo \"$GITHUB_REPOSITORY\" --verify-tag binaries/*",
        ],
    );
    let test = job(&release, "test");
    for line in [
        "pg: [15, 16, 17, 18]",
        "image: postgres:${{ matrix.pg }}",
        "MORM_TEST_PG_MAJOR: ${{ matrix.pg }}",
    ] {
        assert!(has_line(&test, line), "test lacks {line}");
    }
    assert!(has_line(
        &job(&release, "publish"),
        "needs: [verify, test, binaries]"
    ));
    assert!(has_line(&job(&release, "binaries"), "needs: test"));
}

// covers: R-0957
#[test]
fn release_runs_the_tag_check_with_full_history_before_publishing() {
    let release = read(".github/workflows/release.yml");
    assert_commands(
        &release,
        "verify",
        &["python3 .github/scripts/check_release_tag.py \"$GITHUB_REF_NAME\""],
    );
    let verify = job(&release, "verify");
    assert!(
        has_line(&verify, "fetch-depth: 0"),
        "the check needs all history"
    );
    assert!(
        !release.contains("continue-on-error"),
        "a failed check would count as passing"
    );

    let publish = job(&release, "publish");
    assert!(has_line(&publish, "needs: [verify, test, binaries]"));
    let github_release = job(&release, "github-release");
    assert!(has_line(&github_release, "needs: publish"));
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
        has_line(&gate, "if: always()"),
        "a skipped required check counts as passing"
    );
    assert!(
        has_line(&gate, "RESULTS: ${{ join(needs.*.result, ' ') }}"),
        "the gate has to read the results of the jobs it needs"
    );
    assert_commands(
        &ci,
        "ci",
        &[
            "echo \"$RESULTS\"",
            "for result in $RESULTS; do",
            "[ \"$result\" = success ] || exit 1",
            "done",
        ],
    );
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
    assert!(has_line(&job(&ci, "integration"), "pg: [15, 16, 17, 18]"));
    for line in ci
        .lines()
        .filter(|line| line.trim_start().starts_with("runs-on:"))
    {
        let runner = line.trim().trim_start_matches("runs-on: ");
        assert!(
            runner.starts_with("ubuntu-"),
            "not a standard runner: {line}"
        );
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
    let publish = job(&release, "publish");
    assert!(has_line(&publish, "environment: crates-io"));
    assert!(has_line(
        &publish,
        "CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}"
    ));
    assert!(has_line(
        &publish,
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
    assert!(
        security.contains("`biggodev@gmail.com`"),
        "no private address"
    );
    assert!(security.contains("private"));
}

/// The directories Dependabot's entry for `ecosystem` covers.
fn dependabot_directories<'a>(dependabot: &'a str, ecosystem: &str) -> Vec<&'a str> {
    let header = format!("  - package-ecosystem: {ecosystem}");
    let mut entry = dependabot.lines().skip_while(|line| *line != header);
    entry
        .next()
        .unwrap_or_else(|| panic!("no {ecosystem} entry"));
    entry
        .take_while(|line| !line.starts_with("  - "))
        .filter_map(|line| {
            let line = line.trim();
            line.strip_prefix("directory: ")
                .or_else(|| line.strip_prefix("- "))
        })
        .collect()
}

// covers: R-0958
#[test]
fn dependabot_covers_cargo_and_actions() {
    let dependabot = read(".github/dependabot.yml");
    let cargo = dependabot_directories(&dependabot, "cargo");
    assert!(cargo.contains(&"/"), "the workspace root isn't covered");
    let actions = dependabot_directories(&dependabot, "github-actions");
    assert!(actions.contains(&"/"), ".github/workflows isn't covered");
    for action in local_actions() {
        let dir = action.parent().unwrap().strip_prefix(root()).unwrap();
        let entry = format!("/{}", dir.display());
        assert!(
            actions.contains(&entry.as_str()),
            "dependabot misses {entry}"
        );
    }
}

// `exclude:` drops matrix entries, and `working-directory` or another shell can
// leave a command green without running it, so all three count as switching off.
// covers: R-0792, R-0793, R-0794, R-0795, R-0797, R-0957
#[test]
fn nothing_switches_a_job_or_step_off() {
    const ALLOWED: &[(&str, &str)] = &[
        ("ci.yml", "if: always()"),
        (
            "release.yml",
            "if: github.repository == 'biggaloy/rust-morm'",
        ),
    ];
    const ALLOWED_SHELLS: &[&str] = &["shell: bash"];
    let mut files = workflow_files();
    files.extend(local_actions());
    for path in files {
        let file = path.file_name().unwrap().to_str().unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(!text.contains("continue-on-error"), "{}", path.display());
        for line in text.lines() {
            let line = line.trim().trim_start_matches("- ");
            if line.starts_with('#') {
                continue;
            }
            if line.starts_with("if:") {
                assert!(ALLOWED.contains(&(file, line)), "{file}: {line}");
            }
            if line.contains("shell:") {
                assert!(ALLOWED_SHELLS.contains(&line), "{file}: {line}");
            }
            assert!(!line.contains("exclude:"), "{file}: {line}");
            assert!(!line.contains("working-directory"), "{file}: {line}");
        }
    }
}

// covers: R-0792, R-0793, R-0794, R-0795
#[test]
fn no_file_under_github_hides_a_failure() {
    for line in [
        "cargo test || true",
        "cargo test ||true",
        "cargo test || :",
        "cargo test || echo skipped",
        "cargo test || exit 0",
        "cargo test ||",
        "cargo test; true",
        "cargo test;true",
        "cargo test ; :",
        "cargo test; exit 0",
        "exit 0",
        "return 0",
        "true",
        "run: true",
        "- run: cargo test || true",
        "set +e",
        "set +o pipefail",
        "! cargo test",
        "sys.exit(0)",
    ] {
        assert!(hides_failure(line), "not refused: {line}");
    }
    for line in [
        "cargo test",
        "[ \"$result\" = success ] || exit 1",
        "if [ ! -d fuzz ]; then",
        "exit 1",
        "cargo bench | tee out.txt",
        "persist-credentials: false",
        "fail-fast: false",
        "merge-multiple: true",
    ] {
        assert!(!hides_failure(line), "refused: {line}");
    }

    let files = github_files();
    let count = |dir: &str| {
        files
            .iter()
            .filter(|p| p.starts_with(root().join(dir)))
            .count()
    };
    assert!(count(".github/workflows") > 0 && count(".github/scripts") > 0);
    for path in &files {
        let text = fs::read_to_string(path).unwrap();
        for line in text.lines() {
            assert!(
                line.trim_start().starts_with(['#', ';']) || !hides_failure(line),
                "{}: {line}",
                path.display()
            );
        }
    }
}
