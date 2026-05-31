// qual:allow(srp) - Integration test module containing multiple distinct CLI test scenarios
use assert_cmd::Command;

// qual:allow(test_quality) — CLI integration test running binary as subprocess has no static SUT references
#[test]
fn test_cli_modes() {
    // 1. Normal mode
    let mut cmd = Command::cargo_bin("metonymy").expect("failed to get cargo bin");
    let assert_res = cmd.assert().success();
    assert!(assert_res.get_output().status.success());
    assert_res.stdout(predicates::str::contains("Metonymy is running..."));

    // 2. Verbose mode
    let mut cmd2 = Command::cargo_bin("metonymy").expect("failed to get cargo bin");
    let assert_res2 = cmd2.arg("--verbose").assert().success();
    assert!(assert_res2.get_output().status.success());
    assert_res2.stdout(predicates::str::contains(
        "Metonymy is running in verbose mode...",
    ));
}

macro_rules! init_dict {
    ($dict_path:expr, $lang_path:expr) => {
        Command::cargo_bin("metonymy")
            .expect("failed to get cargo bin")
            .args([
                "--language",
                $lang_path,
                "--dict",
                $dict_path,
                "dictionary",
                "init",
            ])
            .assert()
            .success();
    };
}

macro_rules! add_word_to_dict {
    ($args:expr) => {{
        let assert_res = Command::cargo_bin("metonymy")
            .expect("failed to get cargo bin")
            .args($args)
            .assert()
            .success();
        String::from_utf8(assert_res.get_output().stdout.clone()).expect("invalid utf8 stdout")
    }};
}

macro_rules! print_dict {
    ($dict_path:expr) => {{
        let mut cmd = Command::cargo_bin("metonymy").expect("failed to get cargo bin");
        cmd.args(["--dict", $dict_path, "dictionary", "print"]);
        cmd
    }};
}

macro_rules! remove_from_dict {
    ($dict_path:expr, $word_id:expr) => {
        Command::cargo_bin("metonymy")
            .expect("failed to get cargo bin")
            .args(["--dict", $dict_path, "dictionary", "remove", $word_id])
            .assert()
            .success();
    };
}

// qual:allow(test_quality) — CLI integration test running binary as subprocess has no static SUT references
#[test]
fn test_dictionary_cli_lifecycle() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let dict_path = temp_dir.path().join("dict.json");
    let dict_path_str = dict_path.to_str().expect("valid dict path string");
    let lang_path = "tests/features/test_language.json";

    init_dict!(dict_path_str, lang_path);

    let add_args = [
        "--dict",
        dict_path_str,
        "dictionary",
        "add",
        "--meaning",
        "red",
        "--type",
        "adjective",
        "--definition",
        "pat",
        "--era",
        "1",
        "--etymology",
        "0:pa,ta",
        "--usage-notes",
        "notes",
    ];
    let stdout = add_word_to_dict!(&add_args);
    let word_id = stdout
        .split("with ID ")
        .nth(1)
        .map(str::trim)
        .expect("no ID");

    print_dict!(dict_path_str)
        .assert()
        .success()
        .stdout(predicates::str::contains("Total Entries: 1"))
        .stdout(predicates::str::contains("Definition : /pat/"));

    remove_from_dict!(dict_path_str, word_id);

    print_dict!(dict_path_str)
        .assert()
        .success()
        .stdout(predicates::str::contains("Total Entries: 0"));
}

// qual:allow(test_quality) — CLI integration test running binary as subprocess has no static SUT references
#[test]
fn test_dictionary_cli_generate() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let dict_path = temp_dir.path().join("dict.json");
    let dict_path_str = dict_path.to_str().expect("valid dict path string");
    let lang_path = "tests/features/test_language.json";

    init_dict!(dict_path_str, lang_path);

    let _ = add_word_to_dict!(&[
        "--language",
        lang_path,
        "--dict",
        dict_path_str,
        "dictionary",
        "add",
        "--meaning",
        "red",
        "--type",
        "noun.masculine",
        "--generate",
    ]);

    print_dict!(dict_path_str)
        .assert()
        .success()
        .stdout(predicates::str::contains("Total Entries: 1"))
        .stdout(predicates::str::contains("Era        : 0"));
}

// qual:allow(test_quality) — CLI integration test running binary as subprocess has no static SUT references
#[test]
fn test_dictionary_cli_custom_era() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let dict_path = temp_dir.path().join("dict.json");
    let dict_path_str = dict_path.to_str().expect("valid dict path string");
    let lang_path = "tests/features/test_language.json";

    init_dict!(dict_path_str, lang_path);

    let _ = add_word_to_dict!(&[
        "--dict",
        dict_path_str,
        "dictionary",
        "add",
        "--meaning",
        "blue",
        "--definition",
        "tap",
        "--type",
        "noun.feminine",
        "--era",
        "4",
    ]);

    print_dict!(dict_path_str)
        .assert()
        .success()
        .stdout(predicates::str::contains("Total Entries: 1"))
        .stdout(predicates::str::contains("Era        : 4"));
}
