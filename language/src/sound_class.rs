use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::Display;
use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SoundClassKeyError {
    #[error(
        "Invalid sound class key: {0}. Must be an English capital letter (A-Z), a Greek capital letter (Α-Ω), or a Hebrew letter (א-ת), with an optional subscript digit ₀-₉."
    )]
    InvalidKey(String),
}

/// Represents a Sound Class name (key).
/// Valid forms: (A-Z, Α-Ω, א-ת) optionally followed by a single numeric subscript (₀-₉).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SoundClassKey(String);

impl SoundClassKey {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for SoundClassKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for SoundClassKey {
    type Err = SoundClassKeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();

        let Some(first_char) = chars.next() else {
            return Err(SoundClassKeyError::InvalidKey(s.to_string()));
        };

        // Allowed base letters
        let is_english = first_char.is_ascii_uppercase();
        // Greek uppercase Α-Ω (U+0391 to U+03A9), omitting empty code points if strict, but let's just use the range.
        let is_greek = ('\u{0391}'..='\u{03A9}').contains(&first_char);
        // Hebrew letters א-ת (U+05D0 to U+05EA)
        let is_hebrew = ('\u{05D0}'..='\u{05EA}').contains(&first_char);

        if !is_english && !is_greek && !is_hebrew {
            return Err(SoundClassKeyError::InvalidKey(s.to_string()));
        }

        // Subscript check
        let second_char = chars.next();
        let third_char = chars.next();

        if third_char.is_some() {
            // Cannot have more than 2 characters
            return Err(SoundClassKeyError::InvalidKey(s.to_string()));
        }

        if let Some(c) = second_char {
            // Check if it's a valid subscript 0-9
            let is_subscript = ('\u{2080}'..='\u{2089}').contains(&c);
            if !is_subscript {
                return Err(SoundClassKeyError::InvalidKey(s.to_string()));
            }
        }

        Ok(SoundClassKey(s.to_string()))
    }
}

impl Serialize for SoundClassKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SoundClassKey {
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
    fn test_sound_class_key_valid() {
        assert!("C".parse::<SoundClassKey>().is_ok());
        assert!("Z".parse::<SoundClassKey>().is_ok());
        assert!("Γ".parse::<SoundClassKey>().is_ok());
        assert!("א".parse::<SoundClassKey>().is_ok());
        assert!("A₀".parse::<SoundClassKey>().is_ok());
        assert!("Ω₉".parse::<SoundClassKey>().is_ok());
        assert!("ת₅".parse::<SoundClassKey>().is_ok());
    }

    #[test]
    fn test_sound_class_key_invalid() {
        assert!("c".parse::<SoundClassKey>().is_err()); // Lowercase
        assert!("AA".parse::<SoundClassKey>().is_err()); // Too many characters
        assert!("A0".parse::<SoundClassKey>().is_err()); // ASCII digit instead of subscript
        assert!("A₀₀".parse::<SoundClassKey>().is_err()); // Too many subscripts
        assert!("".parse::<SoundClassKey>().is_err()); // Empty
        assert!("1".parse::<SoundClassKey>().is_err()); // Number
    }
}
