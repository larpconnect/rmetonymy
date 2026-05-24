use ipa::IpaString;
use language::config::LanguageConfig;
use std::str::FromStr;

fn get_test_config(illegal_patterns: &[&str]) -> LanguageConfig {
    let patterns_json = serde_json::to_string(&illegal_patterns).expect("valid illegal patterns json");
    let json_str = format!(
        r#"{{
        "id": "018f4a3e-6b9f-7a1a-9b1a-2b3c4d5e6f7a",
        "name": {{ "endonym": "p" }},
        "metadata": {{ "created_at": "2024-05-04T00:12:00Z" }},
        "phonology": {{
            "sound_classes": {{}},
            "illegal_patterns": {patterns_json}
        }}
    }}"#
    );
    serde_json::from_str(&json_str).expect("valid config deserialization")
}

#[test]
fn test_syllabification_cases() {
    let config = get_test_config(&[]);

    // Test cases from prompt:
    let cases = [
        ("ˈfɑɹmɚ", "ˈfɑɹ.mɚ"),
        ("dɑːns", "dɑːns"),
        ("wɔkɪŋ", "wɔ.kɪŋ"),
        ("mankind", "man.kind"),
        ("ˈsliːp", "ˈsliːp"),
        ("ki̯el", "ki̯el"),
        ("kuo̯l", "kuo̯l"),
        ("kiel", "ki.el"),
        ("əmɛɹɪkən", "ə.mɛɹ.ɪ.kən"),
        ("ˈfɑːmə", "ˈfɑː.mə"),
        ("pəˈlɪtɪkəl", "pəˈlɪt.ɪ.kəl"),
        ("ˌæstrəˈnɒmɪkəl", "ˌæs.trəˈnɒm.ɪ.kəl"),
    ];

    for (input, expected) in cases {
        let ipa_str = IpaString::from_str(input).expect("valid ipa");
        let word = config.syllabify(&ipa_str).expect("valid syllabification");
        assert_eq!(word.to_string(), expected, "Failed for input: {input}");
    }
}

#[test]
fn test_illegal_onset_rules() {
    // Disallow cz onset
    let config = get_test_config(&["$cz"]);

    // "acza" should split as ac.za or acz.a but never a.cza.
    let ipa_str = IpaString::from_str("acza").expect("valid ipa");
    let word = config.syllabify(&ipa_str).expect("valid syllabification");
    assert_eq!(word.to_string(), "ac.za");
}

#[test]
fn test_no_vowels_entire_word() {
    let config = get_test_config(&[]);
    let ipa_str = IpaString::from_str("pst").expect("valid ipa");
    let word = config.syllabify(&ipa_str).expect("valid syllabification");
    // treat every individual consonant as its own Root syllable: p.s.t
    assert_eq!(word.to_string(), "p.s.t");
}

#[test]
fn test_arbitrary_syllable_no_vowels() {
    let config = get_test_config(&[]);
    // break sl.ip.lɪs has vowels, but first syllable sl has none.
    let ipa_str = IpaString::from_str("sl.ip.lɪs").expect("valid ipa");
    let word = config.syllabify(&ipa_str).expect("valid syllabification");
    assert_eq!(word.to_string(), "sl.ip.lɪs");
}

#[test]
fn test_invalid_boundary_errors() {
    let config = get_test_config(&[]);

    // Syllable break at start/end
    config
        .syllabify(&IpaString::from_str(".abc").expect("valid ipa"))
        .expect_err("should fail at start");
    config
        .syllabify(&IpaString::from_str("abc.").expect("valid ipa"))
        .expect_err("should fail at end");

    // Double syllable breaks
    let ipa_str = IpaString::from_str("ab..cd").expect("valid ipa");
    config.syllabify(&ipa_str).expect_err("should fail with double breaks");

    // Stress + Syllable break
    let ipa_str = IpaString::from_str("abˈ.cd").expect("valid ipa");
    config.syllabify(&ipa_str).expect_err("should fail with stress + break");

    // Adjacent prosody
    let ipa_str = IpaString::from_str("abˈˌcd").expect("valid ipa");
    config.syllabify(&ipa_str).expect_err("should fail with adjacent prosody");
}
