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
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IpaString(String);

impl IpaString {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for IpaString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Ensure the parser checks every grapheme against the IpaSystem
impl FromStr for IpaString {
    type Err = IpaStringError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let _seq = crate::sequence::PhonemeSequence::from_str(s)?;
        Ok(IpaString(s.to_string()))
    }
}

impl crate::sequence::IpaSequence for IpaString {
    fn elements(&self) -> Vec<crate::sequence::SequenceElement> {
        crate::sequence::PhonemeSequence::from_str(self.as_str())
            .map(|seq| seq.elements)
            .unwrap_or_default()
    }
}


impl Serialize for IpaString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipa_string_valid() {
        let valid = "pa".parse::<IpaString>();
        assert!(valid.is_ok(), "Should parse valid IPA sequence");
    }

    #[test]
    fn test_ipa_string_invalid() {
        let invalid = "xyz123".parse::<IpaString>();
        assert!(invalid.is_err(), "Should reject non-IPA symbols");
    }
}
