use crate::ipa_string::{IpaString, IpaStringError};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

const MODIFIER_RANGE_1: std::ops::RangeInclusive<u32> = 0x02B0..=0x02FF;
const MODIFIER_RANGE_2: std::ops::RangeInclusive<u32> = 0xA700..=0xA71F;
const MODIFIER_RANGE_3: std::ops::RangeInclusive<u32> = 0x1AB0..=0x1AFF;
const MODIFIER_RANGE_4: std::ops::RangeInclusive<u32> = 0x0300..=0x036F;
const MODIFIER_RANGE_5: std::ops::RangeInclusive<u32> = 0x1DC0..=0x1DFF;
const MODIFIER_RANGE_6: std::ops::RangeInclusive<u32> = 0x2070..=0x209F;
const MODIFIER_RANGE_7: std::ops::RangeInclusive<u32> = 0x1D98..=0x1DBF;

/// Checks if a character is a valid modifier according to the allowed Unicode ranges.
#[must_use]
#[inline]
pub fn is_modifier(c: char) -> bool {
    let u = c as u32;
    MODIFIER_RANGE_1.contains(&u)
        || MODIFIER_RANGE_2.contains(&u)
        || MODIFIER_RANGE_3.contains(&u)
        || MODIFIER_RANGE_4.contains(&u)
        || MODIFIER_RANGE_5.contains(&u)
        || MODIFIER_RANGE_6.contains(&u)
        || MODIFIER_RANGE_7.contains(&u)
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

fn parse_elements_op<F, G>(
    s: &str,
    mut is_base_phoneme: F,
    mut is_mod: G,
) -> Result<Vec<SequenceElement>, IpaStringError>
where
    F: FnMut(&str) -> bool,
    G: FnMut(char) -> bool,
{
    use unicode_normalization::UnicodeNormalization;
    if s.is_empty() {
        return Ok(Vec::new());
    }
    let chars: Vec<char> = s.nfd().collect::<String>().chars().collect();
    let mut elements = Vec::new();
    let mut idx = 0;
    while idx < chars.len() {
        let c = chars[idx];
        if let Some(stress) = match c {
            '\'' | 'ˈ' => Some(ProsodyMarker::PrimaryStress),
            'ˌ' => Some(ProsodyMarker::SecondaryStress),
            _ => None,
        } {
            elements.push(SequenceElement::Prosody(stress));
            idx += 1;
        } else if c == '.' {
            elements.push(SequenceElement::SyllableBreak);
            idx += 1;
        } else if is_mod(c) {
            return Err(IpaStringError::InvalidSequence(format!(
                "Modifier '{c}' found without a preceding base phoneme at index {idx} in string \"{s}\""
            )));
        } else {
            let start = idx;
            let len = (1..=(chars.len() - start)).rev()
                .find(|&len| is_base_phoneme(&chars[start..(start + len)].iter().collect::<String>()))
                .ok_or_else(|| IpaStringError::InvalidSequence(format!(
                    "Unrecognized base phoneme starting with '{c}' at index {start} in string \"{s}\""
                )))?;
            let base: String = chars[start..(start + len)].iter().collect();
            idx += len;
            let mut modifiers = Vec::new();
            while idx < chars.len() && is_mod(chars[idx]) && !matches!(chars[idx], '\'' | 'ˈ' | 'ˌ' | '.') {
                modifiers.push(chars[idx].to_string());
                idx += 1;
            }
            elements.push(SequenceElement::Phoneme(Phoneme { base, modifiers }));
        }
    }
    Ok(elements)
}

impl PhonemeSequence {
    /// Parses an IPA string using a specific `IpaSystem`.
    ///
    /// # Errors
    /// Returns `Err` if parsing fails (e.g. unrecognized base phonemes, modifiers without base phonemes).
    pub fn parse_with_system(s: &str, system: &crate::IpaSystem) -> Result<Self, IpaStringError> {
        let elements = parse_elements_op(
            s,
            |prefix| system.get_phoneme_data(prefix).is_some(),
            is_modifier,
        )?;
        Ok(Self { elements })
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
