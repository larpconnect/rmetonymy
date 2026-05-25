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

fn run_dictionary_init(lang_path: &str, dict_path: &str) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("metonymy").expect("failed to get cargo bin");
    cmd.args(&[
        "--language",
        lang_path,
        "--dict",
        dict_path,
        "dictionary",
        "init",
    ])
    .assert()
}

fn run_dictionary_add(
    dict_path: &str,
    meaning: &str,
    definition: &str,
    word_type: &str,
    word_subtype: &str,
    era: &str,
    etymology: &str,
    usage_notes: &str,
) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("metonymy").expect("failed to get cargo bin");
    cmd.args(&[
        "--dict",
        dict_path,
        "dictionary",
        "add",
        "--meaning",
        meaning,
        "--definition",
        definition,
        "--word-type",
        word_type,
        "--word-subtype",
        word_subtype,
        "--era",
        era,
        "--etymology",
        etymology,
        "--usage-notes",
        usage_notes,
    ])
    .assert()
}

fn run_dictionary_print(dict_path: &str) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("metonymy").expect("failed to get cargo bin");
    cmd.args(&["--dict", dict_path, "dictionary", "print"])
        .assert()
}

fn run_dictionary_remove(dict_path: &str, word_id: &str) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("metonymy").expect("failed to get cargo bin");
    cmd.args(&["--dict", dict_path, "dictionary", "remove", word_id])
        .assert()
}

#[test]
fn test_dictionary_cli_workflow() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let dict_path = temp_dir.path().join("dict.json");
    let dict_path_str = dict_path.to_str().unwrap();
    let lang_path = "tests/features/test_language.json";

    // 1. Init dictionary
    let assert_init = run_dictionary_init(lang_path, dict_path_str);
    assert_init
        .success()
        .stdout(predicates::str::contains("Initialized blank dictionary"));

    // Verify file exists and contains blank dictionary
    assert!(dict_path.exists());
    let content = std::fs::read_to_string(&dict_path).unwrap();
    assert!(content.contains(r#""entries": []"#));

    // 2. Add a word
    let assert_add = run_dictionary_add(
        dict_path_str,
        "rɛd",
        "pat",
        "noun",
        "masculine",
        "1",
        "0:pa,ta",
        "formal notes",
    );
    let assert_add = assert_add.success();

    let stdout = String::from_utf8(assert_add.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Added word 'pat'"));

    // Extract ID from output
    // Output format: "Added word 'pat' (meaning: 'rɛd') with ID <id>\n"
    let id_prefix = "with ID ";
    let idx = stdout.find(id_prefix).expect("find ID prefix") + id_prefix.len();
    let word_id = stdout[idx..].trim();
    assert!(!word_id.is_empty());

    // 3. Print dictionary
    let assert_print = run_dictionary_print(dict_path_str);
    assert_print
        .success()
        .stdout(predicates::str::contains("Total Entries: 1"))
        .stdout(predicates::str::contains("Definition : /pat/"))
        .stdout(predicates::str::contains("Meaning    : /rɛd/"))
        .stdout(predicates::str::contains("Type       : noun (masculine)"))
        .stdout(predicates::str::contains("Era        : 1"))
        .stdout(predicates::str::contains("Era 0: pa, ta"))
        .stdout(predicates::str::contains("Usage Notes: formal notes"));

    // 4. Remove word
    let assert_remove = run_dictionary_remove(dict_path_str, word_id);
    assert_remove
        .success()
        .stdout(predicates::str::contains(format!(
            "Removed word with ID {word_id}"
        )));

    // 5. Print dictionary again (should be empty)
    let assert_print_empty = run_dictionary_print(dict_path_str);
    assert_print_empty
        .success()
        .stdout(predicates::str::contains("Total Entries: 0"));
}
