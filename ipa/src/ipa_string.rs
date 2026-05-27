use crate::sequence::{IpaSequence, PhonemeSequence, SequenceElement};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::Display;
use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IpaStringError {
    #[error("Invalid IPA symbol or sequence: {0}")]
    InvalidSequence(String),
}

/// A validated string of IPA symbols and modifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IpaString {
    pub(crate) raw: String,
    pub(crate) elements: Vec<SequenceElement>,
}

impl PartialOrd for IpaString {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IpaString {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.raw.cmp(&other.raw)
    }
}

impl IpaString {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

impl Display for IpaString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

// Ensure the parser checks every grapheme against the IpaSystem
impl FromStr for IpaString {
    type Err = IpaStringError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use unicode_normalization::UnicodeNormalization;
        let nfc_str = s.nfc().collect::<String>();
        let seq = PhonemeSequence::from_str(&nfc_str)?;
        Ok(IpaString {
            raw: nfc_str,
            elements: seq.elements,
        })
    }
}

impl IpaSequence for IpaString {
    fn elements(&self) -> Vec<SequenceElement> {
        self.elements.clone()
    }
}

impl Serialize for IpaString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.raw)
    }
}

impl<'de> Deserialize<'de> for IpaString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<Self>().map_err(serde::de::Error::custom)
    }
}
