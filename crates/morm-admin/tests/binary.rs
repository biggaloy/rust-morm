// covers: R-0042
#[test]
fn the_binary_runs() {
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_morm-admin"))
        .status()
        .expect("morm-admin should start");
    assert!(status.success(), "morm-admin exited with {status}");
}
