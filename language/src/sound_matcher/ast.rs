use crate::sound_class::SoundClassKey;
use data::SpeFeature;
use ipa::IpaString;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum SoundMatcherError {
    #[error("Failed to parse pattern: {0}")]
    ParseError(String),
    #[error("Invalid feature name: {0}")]
    InvalidFeature(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Quantifier {
    ZeroOrMore,
    OneOrMore,
}

impl Display for Quantifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroOrMore => write!(f, "*"),
            Self::OneOrMore => write!(f, "+"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MatcherElement {
    WordBoundary,
    SyllableBoundary,
    SoundClass(SoundClassKey),
    Descriptor(Option<SoundClassKey>, Vec<SpeFeature>),
    IpaSequence(IpaString),
    Set(Vec<MatcherElement>),
    OptionalGroup(Vec<QuantifiedElement>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QuantifiedElement {
    pub element: MatcherElement,
    pub quantifier: Option<Quantifier>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SoundMatcherPattern {
    pub elements: Vec<QuantifiedElement>,
}

impl Serialize for SoundMatcherPattern {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for SoundMatcherPattern {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<Self>().map_err(serde::de::Error::custom)
    }
}

impl Display for MatcherElement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WordBoundary => write!(f, "#"),
            Self::SyllableBoundary => write!(f, "$"),
            Self::SoundClass(sc) => write!(f, "{sc}"),
            Self::Descriptor(sc, features) => {
                write!(f, "[")?;
                if let Some(sc) = sc {
                    write!(f, "{sc} ")?;
                }
                for (i, feat) in features.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{feat}")?;
                }
                write!(f, "]")
            }
            Self::IpaSequence(ipa) => write!(f, "{ipa}"),
            Self::Set(elements) => {
                write!(f, "{{")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{elem}")?;
                }
                write!(f, "}}")
            }
            Self::OptionalGroup(elements) => {
                write!(f, "(")?;
                for elem in elements {
                    write!(f, "{elem}")?;
                }
                write!(f, ")")
            }
        }
    }
}

impl Display for QuantifiedElement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.element)?;
        if let Some(q) = &self.quantifier {
            write!(f, "{q}")?;
        }
        Ok(())
    }
}

impl Display for SoundMatcherPattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for elem in &self.elements {
            write!(f, "{elem}")?;
        }
        Ok(())
    }
}
