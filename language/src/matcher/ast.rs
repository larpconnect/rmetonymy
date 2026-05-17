use crate::sound_class::SoundClassKey;
use data::feature::Feature;
use ipa::IpaString;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};

use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum SoundMatcherError {
    #[error("Failed to parse pattern: {0}")]
    ParseError(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Quantifier {
    ZeroOrMore,
    OneOrMore,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FeatureDescriptor {
    pub sign: bool, // true for +, false for -
    pub feature: Feature,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BaseElement {
    WordBoundary,
    SyllableBoundary,
    SoundClass(SoundClassKey),
    IpaSequence(IpaString),
    FeatureClass(Option<SoundClassKey>, Vec<FeatureDescriptor>),
    Set(Vec<BaseElement>), // Can only contain SoundClass or IpaSequence based on the pest grammar
    OptionalGroup(Box<SoundMatcherPattern>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PatternElement {
    pub base: BaseElement,
    pub quantifier: Quantifier,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SoundMatcherPattern {
    pub elements: Vec<PatternElement>,
}

impl Display for SoundMatcherPattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for el in &self.elements {
            match &el.base {
                BaseElement::WordBoundary => write!(f, "#")?,
                BaseElement::SyllableBoundary => write!(f, "$")?,
                BaseElement::SoundClass(key) => write!(f, "{key}")?,
                BaseElement::IpaSequence(ipa) => write!(f, "{ipa}")?,
                BaseElement::FeatureClass(sc, features) => {
                    write!(f, "[")?;
                    if let Some(sc) = sc {
                        write!(f, "{sc} ")?;
                    }
                    for (i, feat) in features.iter().enumerate() {
                        if i > 0 {
                            write!(f, " ")?;
                        }
                        let sign = if feat.sign { "+" } else { "-" };
                        write!(f, "{sign}{}", feat.feature)?;
                    }
                    write!(f, "]")?;
                }
                BaseElement::Set(els) => {
                    write!(f, "{{")?;
                    for (i, set_el) in els.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        match set_el {
                            BaseElement::SoundClass(key) => write!(f, "{key}")?,
                            BaseElement::IpaSequence(ipa) => write!(f, "{ipa}")?,
                            _ => {}
                        }
                    }
                    write!(f, "}}")?;
                }
                BaseElement::OptionalGroup(pat) => write!(f, "({pat})")?,
            }
            match el.quantifier {
                Quantifier::ZeroOrMore => write!(f, "*")?,
                Quantifier::OneOrMore => write!(f, "+")?,
                Quantifier::None => {}
            }
        }
        Ok(())
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Boundary(String), // word boundary "#", syllable boundary "$", etc
    Phoneme(String),
}
