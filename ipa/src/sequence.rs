use crate::ipa_string::{IpaString, IpaStringError};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

/// Checks if a character is a valid modifier according to the allowed Unicode ranges.
#[must_use]
#[inline]
pub fn is_modifier(c: char) -> bool {
    let u = c as u32;
    (0x02B0..=0x02FF).contains(&u)
        || (0xA700..=0xA71F).contains(&u)
        || (0x1AB0..=0x1AFF).contains(&u)
        || (0x0300..=0x036F).contains(&u)
        || (0x1DC0..=0x1DFF).contains(&u)
        || (0x2070..=0x209F).contains(&u)
        || (0x1D98..=0x1DBF).contains(&u)
}

/// A common interface for sequences of IPA symbols, allowing phonemic analysis.
pub trait IpaSequence: Display + std::fmt::Debug {
    /// Returns the sequence of elements (phonemes, prosodic markers, syllable breaks).
    fn elements(&self) -> Vec<SequenceElement>;

    /// Returns only the phonemes in the sequence, filtering out prosody and syllable breaks.
    #[must_use]
    fn phonemes(&self) -> Vec<Phoneme> {
        self.elements()
            .into_iter()
            .filter_map(|el| match el {
                SequenceElement::Phoneme(p) => Some(p),
                _ => None,
            })
            .collect()
    }
}

/// A marker for prosodic stress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProsodyMarker {
    /// Primary stress (ˈ or ')
    PrimaryStress,
    /// Secondary stress (ˌ)
    SecondaryStress,
}

impl Display for ProsodyMarker {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PrimaryStress => write!(f, "ˈ"),
            Self::SecondaryStress => write!(f, "ˌ"),
        }
    }
}

/// A phoneme consisting of a base symbol and zero or more modifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Phoneme {
    /// The base phoneme symbol (must be recognized by the `IpaSystem`).
    pub base: String,
    /// Modifiers attached to this phoneme.
    pub modifiers: Vec<String>,
}

impl Display for Phoneme {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.base)?;
        for modifier in &self.modifiers {
            write!(f, "{modifier}")?;
        }
        Ok(())
    }
}

/// An element in an IPA sequence.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SequenceElement {
    /// A single phoneme with its base and modifiers.
    Phoneme(Phoneme),
    /// A prosodic marker (stress).
    Prosody(ProsodyMarker),
    /// A syllable break (.).
    SyllableBreak,
}

impl Display for SequenceElement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Phoneme(p) => write!(f, "{p}"),
            Self::Prosody(pm) => write!(f, "{pm}"),
            Self::SyllableBreak => write!(f, "."),
        }
    }
}

/// A parsed sequence of phonemes, prosodic markers, and syllable breaks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PhonemeSequence {
    /// The elements comprising the sequence.
    pub elements: Vec<SequenceElement>,
}

impl Display for PhonemeSequence {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for element in &self.elements {
            write!(f, "{element}")?;
        }
        Ok(())
    }
}

impl IpaSequence for PhonemeSequence {
    fn elements(&self) -> Vec<SequenceElement> {
        self.elements.clone()
    }
}

impl FromStr for PhonemeSequence {
    type Err = IpaStringError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let system = crate::DEFAULT_SYSTEM.as_ref().map_err(|e| {
            IpaStringError::InvalidSequence(format!("Failed to load default IPA system: {e}"))
        })?;
        Self::parse_with_system(s, system)
    }
}

impl PhonemeSequence {
    /// Parses an IPA string using a specific `IpaSystem`.
    ///
    /// # Errors
    /// Returns `Err` if parsing fails (e.g. unrecognized base phonemes, modifiers without base phonemes).
    pub fn parse_with_system(s: &str, system: &crate::IpaSystem) -> Result<Self, IpaStringError> {
        if s.is_empty() {
            return Ok(Self { elements: Vec::new() });
        }

        let chars: Vec<char> = s.chars().collect();
        let mut elements = Vec::new();
        let mut idx = 0;

        while idx < chars.len() {
            let &c = chars.get(idx).ok_or_else(|| {
                IpaStringError::InvalidSequence(format!(
                    "Unexpected index boundary error at {idx} in string \"{s}\""
                ))
            })?;

            // 1. Check prosodic marks & syllable break
            if c == '\'' || c == 'ˈ' {
                elements.push(SequenceElement::Prosody(ProsodyMarker::PrimaryStress));
                idx += 1;
                continue;
            }
            if c == 'ˌ' {
                elements.push(SequenceElement::Prosody(ProsodyMarker::SecondaryStress));
                idx += 1;
                continue;
            }
            if c == '.' {
                elements.push(SequenceElement::SyllableBreak);
                idx += 1;
                continue;
            }

            // 2. Check if it's a modifier (without a base phoneme)
            if is_modifier(c) {
                return Err(IpaStringError::InvalidSequence(format!(
                    "Modifier '{c}' found without a preceding base phoneme at index {idx} in string \"{s}\""
                )));
            }

            // 3. Find the longest prefix starting at idx that is a recognized base phoneme
            let mut matched_len = None;
            for len in (1..=(chars.len() - idx)).rev() {
                let slice = chars.get(idx..(idx + len)).ok_or_else(|| {
                    IpaStringError::InvalidSequence(format!(
                        "Unexpected slice boundary error at {idx} with len {len} in string \"{s}\""
                    ))
                })?;
                let prefix: String = slice.iter().collect();
                if system.get_phoneme_data(&prefix).is_some() {
                    matched_len = Some(len);
                    break;
                }
            }

            if let Some(len) = matched_len {
                let slice = chars.get(idx..(idx + len)).ok_or_else(|| {
                    IpaStringError::InvalidSequence(format!(
                        "Unexpected slice boundary error at {idx} with len {len} in string \"{s}\""
                    ))
                })?;
                let base: String = slice.iter().collect();
                idx += len;

                // Accumulate modifiers
                let mut modifiers = Vec::new();
                while idx < chars.len() {
                    if let Some(&next_c) = chars.get(idx).filter(|&&c| is_modifier(c)) {
                        modifiers.push(next_c.to_string());
                        idx += 1;
                        continue;
                    }
                    break;
                }

                elements.push(SequenceElement::Phoneme(Phoneme { base, modifiers }));
            } else {
                return Err(IpaStringError::InvalidSequence(format!(
                    "Unrecognized base phoneme starting with '{c}' at index {idx} in string \"{s}\""
                )));
            }
        }

        Ok(Self { elements })
    }
}

impl TryFrom<Phoneme> for IpaString {
    type Error = IpaStringError;

    fn try_from(phoneme: Phoneme) -> Result<Self, Self::Error> {
        let s = phoneme.to_string();
        s.parse()
    }
}

impl TryFrom<&Phoneme> for IpaString {
    type Error = IpaStringError;

    fn try_from(phoneme: &Phoneme) -> Result<Self, Self::Error> {
        let s = phoneme.to_string();
        s.parse()
    }
}

impl TryFrom<PhonemeSequence> for IpaString {
    type Error = IpaStringError;

    fn try_from(seq: PhonemeSequence) -> Result<Self, Self::Error> {
        let s = seq.to_string();
        s.parse()
    }
}

impl TryFrom<&PhonemeSequence> for IpaString {
    type Error = IpaStringError;

    fn try_from(seq: &PhonemeSequence) -> Result<Self, Self::Error> {
        let s = seq.to_string();
        s.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_modifier() {
        // Test various modifiers from different unicode ranges
        assert!(is_modifier('ʰ')); // U+02B0
        assert!(is_modifier('̃')); // U+0303
        assert!(is_modifier('ː')); // U+02D0
        assert!(!is_modifier('a')); // normal letter
        assert!(!is_modifier('p')); // normal letter
    }

    #[test]
    fn test_parse_simple_word() -> Result<(), String> {
        let seq = PhonemeSequence::from_str("talk").map_err(|e| e.to_string())?;
        if seq.elements.len() != 4 {
            return Err(format!("Expected 4 elements, got {}", seq.elements.len()));
        }
        if seq.to_string() != "talk" {
            return Err(format!("Expected 'talk', got {seq}"));
        }

        let phonemes = seq.phonemes();
        if phonemes.len() != 4 {
            return Err(format!("Expected 4 phonemes, got {}", phonemes.len()));
        }
        if phonemes.first().map(|p| p.base.as_str()) != Some("t") {
            return Err("Expected first phoneme base 't'".to_string());
        }
        if phonemes.get(1).map(|p| p.base.as_str()) != Some("a") {
            return Err("Expected second phoneme base 'a'".to_string());
        }
        if phonemes.get(2).map(|p| p.base.as_str()) != Some("l") {
            return Err("Expected third phoneme base 'l'".to_string());
        }
        if phonemes.get(3).map(|p| p.base.as_str()) != Some("k") {
            return Err("Expected fourth phoneme base 'k'".to_string());
        }
        Ok(())
    }

    #[test]
    fn test_parse_modifiers() -> Result<(), String> {
        let seq = PhonemeSequence::from_str("kʰɑʰp").map_err(|e| e.to_string())?;
        if seq.elements.len() != 3 {
            return Err(format!("Expected 3 elements, got {}", seq.elements.len()));
        }
        if seq.to_string() != "kʰɑʰp" {
            return Err(format!("Expected 'kʰɑʰp', got {seq}"));
        }

        let Some(SequenceElement::Phoneme(p0)) = seq.elements.first() else {
            return Err("Expected Phoneme at index 0".to_string());
        };
        if p0.base != "k" {
            return Err(format!("Expected base 'k', got {}", p0.base));
        }
        if p0.modifiers != vec!["ʰ"] {
            return Err(format!("Expected modifiers ['ʰ'], got {:?}", p0.modifiers));
        }

        let Some(SequenceElement::Phoneme(p1)) = seq.elements.get(1) else {
            return Err("Expected Phoneme at index 1".to_string());
        };
        if p1.base != "ɑ" {
            return Err(format!("Expected base 'ɑ', got {}", p1.base));
        }
        if p1.modifiers != vec!["ʰ"] {
            return Err(format!("Expected modifiers ['ʰ'], got {:?}", p1.modifiers));
        }

        let Some(SequenceElement::Phoneme(p2)) = seq.elements.get(2) else {
            return Err("Expected Phoneme at index 2".to_string());
        };
        if p2.base != "p" {
            return Err(format!("Expected base 'p', got {}", p2.base));
        }
        if !p2.modifiers.is_empty() {
            return Err(format!("Expected no modifiers, got {:?}", p2.modifiers));
        }
        Ok(())
    }

    #[test]
    fn test_parse_multiple_modifiers() -> Result<(), String> {
        let seq = PhonemeSequence::from_str("kʰʰɑʰːpː").map_err(|e| e.to_string())?;
        if seq.elements.len() != 3 {
            return Err(format!("Expected 3 elements, got {}", seq.elements.len()));
        }
        if seq.to_string() != "kʰʰɑʰːpː" {
            return Err(format!("Expected 'kʰʰɑʰːpː', got {seq}"));
        }

        let Some(SequenceElement::Phoneme(p0)) = seq.elements.first() else {
            return Err("Expected Phoneme at index 0".to_string());
        };
        if p0.base != "k" {
            return Err(format!("Expected base 'k', got {}", p0.base));
        }
        if p0.modifiers != vec!["ʰ", "ʰ"] {
            return Err(format!("Expected modifiers ['ʰ', 'ʰ'], got {:?}", p0.modifiers));
        }

        let Some(SequenceElement::Phoneme(p1)) = seq.elements.get(1) else {
            return Err("Expected Phoneme at index 1".to_string());
        };
        if p1.base != "ɑ" {
            return Err(format!("Expected base 'ɑ', got {}", p1.base));
        }
        if p1.modifiers != vec!["ʰ", "ː"] {
            return Err(format!("Expected modifiers ['ʰ', 'ː'], got {:?}", p1.modifiers));
        }
        Ok(())
    }

    #[test]
    fn test_parse_combined_modifier() -> Result<(), String> {
        let seq = PhonemeSequence::from_str("sɑ̃").map_err(|e| e.to_string())?;
        if seq.elements.len() != 2 {
            return Err(format!("Expected 2 elements, got {}", seq.elements.len()));
        }
        if seq.to_string() != "sɑ̃" {
            return Err(format!("Expected 'sɑ̃', got {seq}"));
        }

        let Some(SequenceElement::Phoneme(p1)) = seq.elements.get(1) else {
            return Err("Expected Phoneme at index 1".to_string());
        };
        if p1.base != "ɑ" {
            return Err(format!("Expected base 'ɑ', got {}", p1.base));
        }
        if p1.modifiers != vec!["̃"] {
            return Err(format!("Expected modifiers ['̃'], got {:?}", p1.modifiers));
        }
        Ok(())
    }

    #[test]
    fn test_parse_stress_and_syllable_break() -> Result<(), String> {
        let seq = PhonemeSequence::from_str("'talk").map_err(|e| e.to_string())?;
        if seq.elements.len() != 5 {
            return Err(format!("Expected 5 elements, got {}", seq.elements.len()));
        }
        if !matches!(seq.elements.first(), Some(SequenceElement::Prosody(ProsodyMarker::PrimaryStress))) {
            return Err("Expected PrimaryStress".to_string());
        }

        let seq2 = PhonemeSequence::from_str("ˌtalk").map_err(|e| e.to_string())?;
        if seq2.elements.len() != 5 {
            return Err(format!("Expected 5 elements, got {}", seq2.elements.len()));
        }
        if !matches!(seq2.elements.first(), Some(SequenceElement::Prosody(ProsodyMarker::SecondaryStress))) {
            return Err("Expected SecondaryStress".to_string());
        }

        let seq3 = PhonemeSequence::from_str("'sliːp.les").map_err(|e| e.to_string())?;
        if seq3.elements.len() != 9 {
            return Err(format!("Expected 9 elements, got {}", seq3.elements.len()));
        }
        if !matches!(seq3.elements.first(), Some(SequenceElement::Prosody(ProsodyMarker::PrimaryStress))) {
            return Err("Expected PrimaryStress".to_string());
        }
        if !matches!(seq3.elements.get(1), Some(SequenceElement::Phoneme(_))) {
            return Err("Expected Phoneme".to_string());
        }
        if !matches!(seq3.elements.get(5), Some(SequenceElement::SyllableBreak)) {
            return Err("Expected SyllableBreak".to_string());
        }
        Ok(())
    }

    #[test]
    fn test_unrecognized_base_phoneme_errors() {
        let result = PhonemeSequence::from_str("p1a");
        assert!(result.is_err());
        if let Err(IpaStringError::InvalidSequence(msg)) = result {
            assert!(msg.contains("Unrecognized base phoneme"));
        }
    }

    #[test]
    fn test_modifier_without_base_errors() {
        let result = PhonemeSequence::from_str("ʰp");
        assert!(result.is_err());
        if let Err(IpaStringError::InvalidSequence(msg)) = result {
            assert!(msg.contains("Modifier") && msg.contains("without a preceding base phoneme"));
        }
    }

    #[test]
    fn test_unrecognized_modifier_preserved() -> Result<(), String> {
        let char_mod = '\u{1AB0}';
        let word = format!("p{char_mod}a");
        let seq = PhonemeSequence::from_str(&word).map_err(|e| e.to_string())?;
        if seq.elements.len() != 2 {
            return Err(format!("Expected 2 elements, got {}", seq.elements.len()));
        }
        let Some(SequenceElement::Phoneme(p0)) = seq.elements.first() else {
            return Err("Expected Phoneme".to_string());
        };
        if p0.base != "p" {
            return Err(format!("Expected base 'p', got {}", p0.base));
        }
        if p0.modifiers != vec![char_mod.to_string()] {
            return Err(format!("Expected modifiers [{:?}], got {:?}", char_mod, p0.modifiers));
        }
        Ok(())
    }

    #[test]
    fn test_conversions() -> Result<(), String> {
        let seq = PhonemeSequence::from_str("kʰɑʰp").map_err(|e| e.to_string())?;
        let ipa = IpaString::try_from(seq.clone()).map_err(|e| e.to_string())?;
        if ipa.as_str() != "kʰɑʰp" {
            return Err(format!("Expected 'kʰɑʰp', got {}", ipa.as_str()));
        }

        let Some(SequenceElement::Phoneme(p0)) = seq.elements.first() else {
            return Err("Expected Phoneme".to_string());
        };
        let ipa_p = IpaString::try_from(p0).map_err(|e| e.to_string())?;
        if ipa_p.as_str() != "kʰ" {
            return Err(format!("Expected 'kʰ', got {}", ipa_p.as_str()));
        }
        Ok(())
    }
}
