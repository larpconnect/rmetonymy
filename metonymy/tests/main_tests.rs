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

#[test]
fn test_dictionary_cli_workflow() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let dict_path = temp_dir.path().join("dict.json");
    let lang_path = "tests/features/test_language.json";

    // 1. Init dictionary
    let mut cmd = Command::cargo_bin("metonymy").expect("failed to get cargo bin");
    cmd.args(&[
        "--language",
        lang_path,
        "--dict",
        dict_path.to_str().unwrap(),
        "dictionary",
        "init",
    ])
    .assert()
    .success()
    .stdout(predicates::str::contains("Initialized blank dictionary"));

    // Verify file exists and contains blank dictionary
    assert!(dict_path.exists());
    let content = std::fs::read_to_string(&dict_path).unwrap();
    assert!(content.contains(r#""entries": []"#));

    // 2. Add a word
    let mut cmd = Command::cargo_bin("metonymy").expect("failed to get cargo bin");
    let assert = cmd
        .args(&[
            "--dict",
            dict_path.to_str().unwrap(),
            "dictionary",
            "add",
            "--meaning",
            "rɛd",
            "--definition",
            "pat",
            "--word-type",
            "noun",
            "--word-subtype",
            "masculine",
            "--era",
            "1",
            "--etymology",
            "0:pa,ta",
            "--usage-notes",
            "formal notes",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Added word 'pat'"));

    // Extract ID from output
    // Output format: "Added word 'pat' (meaning: 'rɛd') with ID <id>\n"
    let id_prefix = "with ID ";
    let idx = stdout.find(id_prefix).expect("find ID prefix") + id_prefix.len();
    let word_id = stdout[idx..].trim();
    assert!(!word_id.is_empty());

    // 3. Print dictionary
    let mut cmd = Command::cargo_bin("metonymy").expect("failed to get cargo bin");
    cmd.args(&["--dict", dict_path.to_str().unwrap(), "dictionary", "print"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Total Entries: 1"))
        .stdout(predicates::str::contains("Definition : /pat/"))
        .stdout(predicates::str::contains("Meaning    : /rɛd/"))
        .stdout(predicates::str::contains("Type       : noun (masculine)"))
        .stdout(predicates::str::contains("Era        : 1"))
        .stdout(predicates::str::contains("Era 0: pa, ta"))
        .stdout(predicates::str::contains("Usage Notes: formal notes"));

    // 4. Remove word
    let mut cmd = Command::cargo_bin("metonymy").expect("failed to get cargo bin");
    cmd.args(&[
        "--dict",
        dict_path.to_str().unwrap(),
        "dictionary",
        "remove",
        word_id,
    ])
    .assert()
    .success()
    .stdout(predicates::str::contains(format!(
        "Removed word with ID {}",
        word_id
    )));

    // 5. Print dictionary again (should be empty)
    let mut cmd = Command::cargo_bin("metonymy").expect("failed to get cargo bin");
    cmd.args(&["--dict", dict_path.to_str().unwrap(), "dictionary", "print"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Total Entries: 0"));
}
