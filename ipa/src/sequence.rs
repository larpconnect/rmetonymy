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
        use unicode_normalization::UnicodeNormalization;
        let mut temp = String::new();
        for element in &self.elements {
            temp.push_str(&element.to_string());
        }
        let normalized = temp.nfc().collect::<String>();
        write!(f, "{normalized}")
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
        use unicode_normalization::UnicodeNormalization;
        if s.is_empty() {
            return Ok(Self {
                elements: Vec::new(),
            });
        }

        let normalized = s.nfd().collect::<String>();
        let chars: Vec<char> = normalized.chars().collect();
        let mut elements = Vec::new();
        let mut idx = 0;

        while idx < chars.len() {
            let element = Self::parse_next_element(&chars, &mut idx, system, s)?;
            elements.push(element);
        }

        Ok(Self { elements })
    }

    /// Parses the next `SequenceElement` from the characters at `idx`.
    #[expect(clippy::indexing_slicing, reason = "bounds are checked in the caller")]
    fn parse_next_element(
        chars: &[char],
        idx: &mut usize,
        system: &crate::IpaSystem,
        s: &str,
    ) -> Result<SequenceElement, IpaStringError> {
        let c = chars[*idx];

        // 1. Check prosodic marks & syllable break
        if c == '\'' || c == 'ˈ' {
            *idx += 1;
            return Ok(SequenceElement::Prosody(ProsodyMarker::PrimaryStress));
        }
        if c == 'ˌ' {
            *idx += 1;
            return Ok(SequenceElement::Prosody(ProsodyMarker::SecondaryStress));
        }
        if c == '.' {
            *idx += 1;
            return Ok(SequenceElement::SyllableBreak);
        }

        // 2. Check if it's a modifier (without a base phoneme)
        if is_modifier(c) {
            return Err(IpaStringError::InvalidSequence(format!(
                "Modifier '{c}' found without a preceding base phoneme at index {idx} in string \"{s}\""
            )));
        }

        // 3. Find the longest prefix starting at idx that is a recognized base phoneme
        Self::parse_phoneme(chars, idx, system, s)
    }

    /// Parses a base phoneme and its modifiers starting at `idx`.
    #[expect(clippy::indexing_slicing, reason = "bounds are checked in the caller")]
    fn parse_phoneme(
        chars: &[char],
        idx: &mut usize,
        system: &crate::IpaSystem,
        s: &str,
    ) -> Result<SequenceElement, IpaStringError> {
        let start = *idx;
        let mut matched_len = None;

        for len in (1..=(chars.len() - start)).rev() {
            let prefix: String = chars[start..(start + len)].iter().collect();
            if system.get_phoneme_data(&prefix).is_some() {
                matched_len = Some(len);
                break;
            }
        }

        if let Some(len) = matched_len {
            let base: String = chars[start..(start + len)].iter().collect();
            *idx += len;

            // Accumulate modifiers
            let mut modifiers = Vec::new();
            while *idx < chars.len()
                && is_modifier(chars[*idx])
                && chars[*idx] != '\''
                && chars[*idx] != 'ˈ'
                && chars[*idx] != 'ˌ'
                && chars[*idx] != '.'
            {
                modifiers.push(chars[*idx].to_string());
                *idx += 1;
            }

            Ok(SequenceElement::Phoneme(Phoneme { base, modifiers }))
        } else {
            let c = chars[start];
            Err(IpaStringError::InvalidSequence(format!(
                "Unrecognized base phoneme starting with '{c}' at index {start} in string \"{s}\""
            )))
        }
    }
}

impl From<Phoneme> for IpaString {
    fn from(p: Phoneme) -> Self {
        let raw = p.to_string();
        let elements = vec![SequenceElement::Phoneme(p)];
        IpaString { raw, elements }
    }
}

impl From<&Phoneme> for IpaString {
    fn from(p: &Phoneme) -> Self {
        let raw = p.to_string();
        let elements = vec![SequenceElement::Phoneme(p.clone())];
        IpaString { raw, elements }
    }
}

impl From<PhonemeSequence> for IpaString {
    fn from(seq: PhonemeSequence) -> Self {
        IpaString {
            raw: seq.to_string(),
            elements: seq.elements,
        }
    }
}

impl From<&PhonemeSequence> for IpaString {
    fn from(seq: &PhonemeSequence) -> Self {
        IpaString {
            raw: seq.to_string(),
            elements: seq.elements.clone(),
        }
    }
}
