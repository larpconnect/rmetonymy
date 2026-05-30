// qual:allow(srp) - Sequence test module
#![expect(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::panic,
    reason = "Standard test assertions and indexing"
)]

use ipa::IpaString;
use ipa::sequence::{IpaSequence, PhonemeSequence, ProsodyMarker, SequenceElement, is_modifier};
use std::str::FromStr;

#[test]
fn test_is_modifier() {
    assert!(is_modifier('ʰ'));
    assert!(is_modifier('̃'));
    assert!(is_modifier('ː'));
    assert!(!is_modifier('a'));
    assert!(!is_modifier('p'));
}

#[test]
fn test_parse_simple_word() {
    let seq = PhonemeSequence::from_str("talk").unwrap();
    assert_eq!(seq.elements.len(), 4);
    assert_eq!(seq.to_string(), "talk");

    let phonemes = seq.phonemes();
    assert_eq!(phonemes.len(), 4);
    assert_eq!(phonemes[0].base, "t");
    assert_eq!(phonemes[1].base, "a");
    assert_eq!(phonemes[2].base, "l");
    assert_eq!(phonemes[3].base, "k");
}

#[test]
fn test_parse_modifiers() {
    let seq = PhonemeSequence::from_str("kʰɑʰp").unwrap();
    assert_eq!(seq.elements.len(), 3);
    assert_eq!(seq.to_string(), "kʰɑʰp");

    let SequenceElement::Phoneme(p0) = &seq.elements[0] else {
        panic!("expected Phoneme at index 0");
    };
    assert_eq!(p0.base, "k");
    assert_eq!(p0.modifiers, vec!["ʰ"]);

    let SequenceElement::Phoneme(p1) = &seq.elements[1] else {
        panic!("expected Phoneme at index 1");
    };
    assert_eq!(p1.base, "ɑ");
    assert_eq!(p1.modifiers, vec!["ʰ"]);

    let SequenceElement::Phoneme(p2) = &seq.elements[2] else {
        panic!("expected Phoneme at index 2");
    };
    assert_eq!(p2.base, "p");
    assert!(p2.modifiers.is_empty());
}

#[test]
fn test_parse_multiple_modifiers() {
    let seq = PhonemeSequence::from_str("kʰʰɑʰːpː").unwrap();
    assert_eq!(seq.elements.len(), 3);
    assert_eq!(seq.to_string(), "kʰʰɑʰːpː");

    let SequenceElement::Phoneme(p0) = &seq.elements[0] else {
        panic!("expected Phoneme at index 0");
    };
    assert_eq!(p0.base, "k");
    assert_eq!(p0.modifiers, vec!["ʰ", "ʰ"]);

    let SequenceElement::Phoneme(p1) = &seq.elements[1] else {
        panic!("expected Phoneme at index 1");
    };
    assert_eq!(p1.base, "ɑ");
    assert_eq!(p1.modifiers, vec!["ʰ", "ː"]);
}

#[test]
fn test_parse_combined_modifier() {
    let seq = PhonemeSequence::from_str("sɑ̃").unwrap();
    assert_eq!(seq.elements.len(), 2);
    assert_eq!(seq.to_string(), "sɑ̃");

    let SequenceElement::Phoneme(p1) = &seq.elements[1] else {
        panic!("expected Phoneme at index 1");
    };
    assert_eq!(p1.base, "ɑ");
    assert_eq!(p1.modifiers, vec!["̃"]);
}

#[test]
fn test_parse_stress_and_syllable_break() {
    let seq = PhonemeSequence::from_str("'talk").unwrap();
    assert_eq!(seq.elements.len(), 5);
    assert!(matches!(
        seq.elements[0],
        SequenceElement::Prosody(ProsodyMarker::PrimaryStress)
    ));

    let seq2 = PhonemeSequence::from_str("ˌtalk").unwrap();
    assert_eq!(seq2.elements.len(), 5);
    assert!(matches!(
        seq2.elements[0],
        SequenceElement::Prosody(ProsodyMarker::SecondaryStress)
    ));

    let seq3 = PhonemeSequence::from_str("'sliːp.les").unwrap();
    assert_eq!(seq3.elements.len(), 9);
    assert!(matches!(
        seq3.elements[0],
        SequenceElement::Prosody(ProsodyMarker::PrimaryStress)
    ));
    assert!(matches!(seq3.elements[1], SequenceElement::Phoneme(_)));
    assert!(matches!(seq3.elements[5], SequenceElement::SyllableBreak));
}

#[test]
fn test_unrecognized_base_phoneme_errors() {
    let result = PhonemeSequence::from_str("p1a");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Unrecognized base phoneme"));
}

#[test]
fn test_modifier_without_base_errors() {
    let result = PhonemeSequence::from_str("ʰp");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Modifier") && err.contains("without a preceding base phoneme"));
}

#[test]
fn test_unrecognized_modifier_preserved() {
    let char_mod = '\u{1AB0}';
    let word = format!("p{char_mod}a");
    let seq = PhonemeSequence::from_str(&word).unwrap();
    assert_eq!(seq.elements.len(), 2);
    let SequenceElement::Phoneme(p0) = &seq.elements[0] else {
        panic!("expected Phoneme");
    };
    assert_eq!(p0.base, "p");
    assert_eq!(p0.modifiers, vec![char_mod.to_string()]);
}

#[test]
fn test_conversions() {
    let seq = PhonemeSequence::from_str("kʰɑʰp").unwrap();
    let ipa = IpaString::from(seq.clone());
    assert_eq!(ipa.as_str(), "kʰɑʰp");

    let SequenceElement::Phoneme(p0) = &seq.elements[0] else {
        panic!("expected Phoneme");
    };
    let ipa_p = IpaString::from(p0);
    assert_eq!(ipa_p.as_str(), "kʰ");
}
