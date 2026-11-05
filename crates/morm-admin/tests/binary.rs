// covers: R-0042
#[test]
fn the_binary_runs() {
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_morm-admin"))
        .status()
        .expect("morm-admin should start");
    assert!(status.success(), "morm-admin exited with {status}");
}

// bench.yml measures this command, so it has to keep exiting 0.
// covers: R-0794
#[test]
fn the_command_bench_measures_exits_zero() {
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_morm-admin"))
        .arg("--version")
        .status()
        .expect("morm-admin should start");
    assert!(
        status.success(),
        "morm-admin --version exited with {status}"
    );
}
