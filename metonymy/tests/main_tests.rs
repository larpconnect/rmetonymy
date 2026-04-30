use assert_cmd::Command;

#[test]
fn test_cli_verbose() {
    let mut cmd = Command::cargo_bin("metonymy").expect("failed to get cargo bin");
    cmd.arg("--verbose")
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "Metonymy is running in verbose mode...",
        ));
}

#[test]
fn test_cli_normal() {
    let mut cmd = Command::cargo_bin("metonymy").expect("failed to get cargo bin");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Metonymy is running..."));
}
