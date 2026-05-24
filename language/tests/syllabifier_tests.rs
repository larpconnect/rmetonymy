use ipa::IpaString;
use ipa::sequence::PhonemeSequence;
use language::config::LanguageConfig;
use language::syllabifier::{syllabify_sequence, validate_sequence};
use std::str::FromStr;

fn get_test_config() -> LanguageConfig {
    let json_str = r#"{
        "id": "018f4a3e-6b9f-7a1a-9b1a-2b3c4d5e6f7a",
        "name": { "endonym": "p" },
        "metadata": { "created_at": "2024-05-04T00:12:00Z" },
        "phonology": {
            "sound_classes": {},
            "illegal_patterns": []
        }
    }"#;
    serde_json::from_str(json_str).expect("valid config json")
}

#[test]
fn test_validate_sequence_empty() {
    let seq = PhonemeSequence {
        elements: Vec::new(),
    };
    let res = validate_sequence(&seq);
    assert!(res.ok().is_some());
}

#[test]
fn test_validate_sequence_valid() {
    let system = ipa::DEFAULT_SYSTEM.as_ref().expect("valid ipa system");
    let seq = PhonemeSequence::parse_with_system("ˈfɑɹ.mɚ", system).expect("valid parse");
    let res = validate_sequence(&seq);
    assert!(res.ok().is_some());
}

#[test]
fn test_validate_sequence_boundary_break_start() {
    let system = ipa::DEFAULT_SYSTEM.as_ref().expect("valid ipa system");
    let seq = PhonemeSequence::parse_with_system(".abc", system).expect("valid parse");
    let res = validate_sequence(&seq);
    assert!(res.err().is_some());
}

#[test]
fn test_validate_sequence_boundary_break_end() {
    let system = ipa::DEFAULT_SYSTEM.as_ref().expect("valid ipa system");
    let seq = PhonemeSequence::parse_with_system("abc.", system).expect("valid parse");
    let res = validate_sequence(&seq);
    assert!(res.err().is_some());
}

#[test]
fn test_validate_sequence_double_breaks() {
    let system = ipa::DEFAULT_SYSTEM.as_ref().expect("valid ipa system");
    let seq = PhonemeSequence::parse_with_system("ab..cd", system).expect("valid parse");
    let res = validate_sequence(&seq);
    assert!(res.err().is_some());
}

#[test]
fn test_validate_sequence_break_near_prosody() {
    let system = ipa::DEFAULT_SYSTEM.as_ref().expect("valid ipa system");
    let seq1 = PhonemeSequence::parse_with_system("abˈ.cd", system).expect("valid parse");
    assert!(validate_sequence(&seq1).err().is_some());

    let seq2 = PhonemeSequence::parse_with_system("ab.ˈcd", system).expect("valid parse");
    assert!(validate_sequence(&seq2).err().is_some());
}

#[test]
fn test_validate_sequence_adjacent_prosody() {
    let system = ipa::DEFAULT_SYSTEM.as_ref().expect("valid ipa system");
    let seq = PhonemeSequence::parse_with_system("abˈˌcd", system).expect("valid parse");
    let res = validate_sequence(&seq);
    assert!(res.err().is_some());
}

#[test]
fn test_syllabify_sequence_direct() {
    let config = get_test_config();
    let system = ipa::DEFAULT_SYSTEM.as_ref().expect("valid ipa system");
    let ipa_str = IpaString::from_str("əmɛɹɪkən").expect("valid ipa");
    let seq = PhonemeSequence::parse_with_system(ipa_str.as_str(), system).expect("valid parse");

    let word = syllabify_sequence(&seq, &config).expect("syllabification success");
    assert_eq!(word.to_string(), "ə.mɛɹ.ɪ.kən");
}
