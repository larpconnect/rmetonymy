use assert_cmd::Command;

#[test]
fn test_lookup_affricate_and_trill() {
    let mut cmd = Command::cargo_bin("metonymy").expect("failed to get cargo bin");

    // Check an affricate: t͡s
    cmd.arg("lookup")
        .arg("--phoneme")
        .arg("t͡s")
        .assert()
        .success()
        .stdout(predicates::str::contains("Manner: [\"affricate\"]"));

    // Check another affricate: d͡ʒ
    let mut cmd2 = Command::cargo_bin("metonymy").expect("failed to get cargo bin");
    cmd2.arg("lookup")
        .arg("--phoneme")
        .arg("d͡ʒ")
        .assert()
        .success()
        .stdout(predicates::str::contains("Manner: [\"affricate\"]"));

    // Check a trill: ɽr
    let mut cmd3 = Command::cargo_bin("metonymy").expect("failed to get cargo bin");
    cmd3.arg("lookup")
        .arg("--phoneme")
        .arg("ɽr")
        .assert()
        .success()
        .stdout(predicates::str::contains("Manner: [\"trill\"]"));
}
