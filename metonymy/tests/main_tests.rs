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
    cmd.args([
        "--language",
        lang_path,
        "--dict",
        dict_path,
        "dictionary",
        "init",
    ])
    .assert()
}

#[derive(Copy, Clone)]
struct DictionaryAddArgs<'a> {
    lang_path: Option<&'a str>,
    dict_path: &'a str,
    meaning: &'a str,
    definition: Option<&'a str>,
    r#type: &'a str,
    era: Option<&'a str>,
    etymology: &'a [&'a str],
    usage_notes: &'a str,
}

fn run_dictionary_add(args: DictionaryAddArgs<'_>) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("metonymy").expect("failed to get cargo bin");
    let mut cli_args = Vec::new();
    if let Some(lang) = args.lang_path {
        cli_args.push("--language".to_string());
        cli_args.push(lang.to_string());
    }
    cli_args.extend(vec![
        "--dict".to_string(),
        args.dict_path.to_string(),
        "dictionary".to_string(),
        "add".to_string(),
        "--meaning".to_string(),
        args.meaning.to_string(),
        "--type".to_string(),
        args.r#type.to_string(),
    ]);
    if let Some(def) = args.definition {
        cli_args.push("--definition".to_string());
        cli_args.push(def.to_string());
    } else {
        cli_args.push("--generate".to_string());
    }
    if let Some(e) = args.era {
        cli_args.push("--era".to_string());
        cli_args.push(e.to_string());
    }
    for ety in args.etymology {
        cli_args.push("--etymology".to_string());
        cli_args.push(ety.to_string());
    }
    if !args.usage_notes.is_empty() {
        cli_args.push("--usage-notes".to_string());
        cli_args.push(args.usage_notes.to_string());
    }
    cmd.args(cli_args).assert()
}

fn run_dictionary_print(dict_path: &str) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("metonymy").expect("failed to get cargo bin");
    cmd.args(["--dict", dict_path, "dictionary", "print"])
        .assert()
}

fn run_dictionary_remove(dict_path: &str, word_id: &str) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("metonymy").expect("failed to get cargo bin");
    cmd.args(["--dict", dict_path, "dictionary", "remove", word_id])
        .assert()
}

fn add_test_word(dict_path_str: &str) -> String {
    let assert_add = run_dictionary_add(DictionaryAddArgs {
        lang_path: None,
        dict_path: dict_path_str,
        meaning: "rɛd",
        definition: Some("pat"),
        r#type: "noun.masculine",
        era: Some("1"),
        etymology: &["0:pa,ta"],
        usage_notes: "formal notes",
    });
    let assert_add = assert_add.success();

    let stdout = String::from_utf8(assert_add.get_output().stdout.clone()).expect("valid UTF-8 stdout");
    assert!(stdout.contains("Added word 'pat'"));

    let id_prefix = "with ID ";
    stdout
        .split(id_prefix)
        .nth(1)
        .map(str::trim)
        .map(String::from)
        .expect("should find ID in output")
}

#[test]
fn test_dictionary_cli_workflow() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let dict_path = temp_dir.path().join("dict.json");
    let dict_path_str = dict_path.to_str().expect("valid dict path string");
    let lang_path = "tests/features/test_language.json";

    run_dictionary_init(lang_path, dict_path_str)
        .success()
        .stdout(predicates::str::contains("Initialized blank dictionary"));

    assert!(dict_path.exists());
    let content = std::fs::read_to_string(&dict_path).expect("read dict file successfully");
    assert!(content.contains(r#""entries": []"#));

    let word_id = add_test_word(dict_path_str);

    run_dictionary_print(dict_path_str)
        .success()
        .stdout(predicates::str::contains("Total Entries: 1"))
        .stdout(predicates::str::contains("Definition : /pat/"))
        .stdout(predicates::str::contains("Meaning    : /rɛd/"))
        .stdout(predicates::str::contains("Type       : noun (masculine)"))
        .stdout(predicates::str::contains("Era        : 1"))
        .stdout(predicates::str::contains("Era 0: pa, ta"))
        .stdout(predicates::str::contains("Usage Notes: formal notes"));

    run_dictionary_remove(dict_path_str, &word_id)
        .success()
        .stdout(predicates::str::contains(format!(
            "Removed word with ID {word_id}"
        )));

    run_dictionary_print(dict_path_str)
        .success()
        .stdout(predicates::str::contains("Total Entries: 0"));
}

fn add_generated_word(dict_path_str: &str, lang_path: &str, meaning: &str) {
    run_dictionary_add(DictionaryAddArgs {
        lang_path: Some(lang_path),
        dict_path: dict_path_str,
        meaning,
        definition: None,
        r#type: "noun.masculine",
        era: None,
        etymology: &[],
        usage_notes: "",
    })
    .success();
}

#[test]
fn test_dictionary_cli_generate_and_default_era() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let dict_path = temp_dir.path().join("dict.json");
    let dict_path_str = dict_path.to_str().expect("valid dict path string");
    let lang_path = "tests/features/test_language.json";

    run_dictionary_init(lang_path, dict_path_str).success();

    // 2. Add a generated word with no era and no etymology (defaults to 0)
    run_dictionary_add(DictionaryAddArgs {
        lang_path: Some(lang_path),
        dict_path: dict_path_str,
        meaning: "rɛd",
        definition: None,
        r#type: "noun.masculine",
        era: None,
        etymology: &[],
        usage_notes: "generated",
    })
    .success()
    .stdout(predicates::str::contains("Added word"));

    // 3. Add another word with explicit era 4
    run_dictionary_add(DictionaryAddArgs {
        lang_path: None,
        dict_path: dict_path_str,
        meaning: "blue",
        definition: Some("tap"),
        r#type: "noun.feminine",
        era: Some("4"),
        etymology: &[],
        usage_notes: "",
    })
    .success();

    // 4. Add a generated word with no era (defaults to 4)
    add_generated_word(dict_path_str, lang_path, "green");

    // 5. Print dictionary and assert values
    run_dictionary_print(dict_path_str)
        .success()
        .stdout(predicates::str::contains("Total Entries: 3"))
        .stdout(predicates::str::contains("Era        : 0"))
        .stdout(predicates::str::contains("Era        : 4"));
}
