// qual:allow(srp) - Integration test module containing multiple distinct CLI test scenarios
use anyhow::Context;
use assert_cmd::Command;

// qual:allow(test_quality) — CLI integration test running binary as subprocess has no static SUT references
#[test]
fn test_cli_modes() -> anyhow::Result<()> {
    // 1. Normal mode
    let mut cmd = Command::cargo_bin("metonymy").context("failed to get cargo bin")?;
    let assert_res = cmd.assert().success();
    assert!(assert_res.get_output().status.success());
    assert_res.stdout(predicates::str::contains("Metonymy is running..."));

    // 2. Verbose mode
    let mut cmd2 = Command::cargo_bin("metonymy").context("failed to get cargo bin")?;
    let assert_res2 = cmd2.arg("--verbose").assert().success();
    assert!(assert_res2.get_output().status.success());
    assert_res2.stdout(predicates::str::contains(
        "Metonymy is running in verbose mode...",
    ));
    Ok(())
}

// qual:allow(test_quality) — CLI integration test running binary as subprocess has no static SUT references
#[test]
fn test_dictionary_cli_lifecycle() -> anyhow::Result<()> {
    let temp_dir = tempfile::tempdir().context("create temp dir")?;
    let dict_path = temp_dir.path().join("dict.json");
    let dict_path_str = dict_path.to_str().context("valid dict path string")?;
    let lang_path = "tests/features/test_language.json";

    Command::cargo_bin("metonymy")?
        .args(["--language", lang_path, "--dict", dict_path_str, "dictionary", "init"])
        .assert().success();

    let assert_add = Command::cargo_bin("metonymy")?
        .args([
            "--dict", dict_path_str, "dictionary", "add",
            "--meaning", "red", "--type", "adjective", "--definition", "pat",
            "--era", "1", "--etymology", "0:pa,ta", "--usage-notes", "notes",
        ])
        .assert().success();

    let stdout = String::from_utf8(assert_add.get_output().stdout.clone())?;
    let word_id = stdout.split("with ID ").nth(1).map(str::trim).context("no ID")?;

    Command::cargo_bin("metonymy")?
        .args(["--dict", dict_path_str, "dictionary", "print"])
        .assert().success()
        .stdout(predicates::str::contains("Total Entries: 1"))
        .stdout(predicates::str::contains("Definition : /pat/"));

    Command::cargo_bin("metonymy")?
        .args(["--dict", dict_path_str, "dictionary", "remove", word_id])
        .assert().success();

    Command::cargo_bin("metonymy")?
        .args(["--dict", dict_path_str, "dictionary", "print"])
        .assert().success()
        .stdout(predicates::str::contains("Total Entries: 0"));

    Ok(())
}

// qual:allow(test_quality) — CLI integration test running binary as subprocess has no static SUT references
#[test]
fn test_dictionary_cli_generate_and_default_era() -> anyhow::Result<()> {
    let temp_dir = tempfile::tempdir().context("create temp dir")?;
    let dict_path = temp_dir.path().join("dict.json");
    let dict_path_str = dict_path.to_str().context("valid dict path string")?;
    let lang_path = "tests/features/test_language.json";

    Command::cargo_bin("metonymy")?
        .args(["--language", lang_path, "--dict", dict_path_str, "dictionary", "init"])
        .assert().success();

    // 2. Add a generated word with no era and no etymology (defaults to 0)
    Command::cargo_bin("metonymy")?
        .args([
            "--language", lang_path, "--dict", dict_path_str, "dictionary", "add",
            "--meaning", "red", "--type", "noun.masculine", "--generate",
        ])
        .assert().success();

    // 3. Add another word with explicit era 4
    Command::cargo_bin("metonymy")?
        .args([
            "--dict", dict_path_str, "dictionary", "add",
            "--meaning", "blue", "--definition", "tap", "--type", "noun.feminine",
            "--era", "4",
        ])
        .assert().success();

    // 4. Add a generated word with no era (defaults to 4)
    Command::cargo_bin("metonymy")?
        .args([
            "--language", lang_path, "--dict", dict_path_str, "dictionary", "add",
            "--meaning", "green", "--type", "noun.masculine", "--generate",
        ])
        .assert().success();

    // 5. Print dictionary and assert values
    Command::cargo_bin("metonymy")?
        .args(["--dict", dict_path_str, "dictionary", "print"])
        .assert().success()
        .stdout(predicates::str::contains("Total Entries: 3"))
        .stdout(predicates::str::contains("Era        : 0"))
        .stdout(predicates::str::contains("Era        : 4"));

    Ok(())
}
