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
    pub marker: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SoundMatcherPattern {
    pub elements: Vec<PatternElement>,
}

impl BaseElement {
    fn write_features(f: &mut Formatter<'_>, features: &[FeatureDescriptor]) -> std::fmt::Result {
        for (i, feat) in features.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            let sign = if feat.sign { "+" } else { "-" };
            write!(f, "{sign}{}", feat.feature)?;
        }
        Ok(())
    }

    fn write_feature_class(
        f: &mut Formatter<'_>,
        sc: Option<&SoundClassKey>,
        features: &[FeatureDescriptor],
        marker: Option<u8>,
    ) -> std::fmt::Result {
        write!(f, "[")?;
        if let Some(sc) = sc {
            write!(f, "{sc}")?;
            if let Some(m) = marker {
                write!(f, "{m}")?;
            }
            if !features.is_empty() {
                write!(f, " ")?;
            }
        }
        Self::write_features(f, features)?;
        write!(f, "]")
    }

    fn write_set_element(f: &mut Formatter<'_>, set_el: &BaseElement) -> std::fmt::Result {
        match set_el {
            BaseElement::SoundClass(key) => write!(f, "{key}"),
            BaseElement::IpaSequence(ipa) => write!(f, "{ipa}"),
            _ => Ok(()),
        }
    }

    fn write_set(f: &mut Formatter<'_>, els: &[BaseElement]) -> std::fmt::Result {
        write!(f, "{{")?;
        for (i, set_el) in els.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            Self::write_set_element(f, set_el)?;
        }
        write!(f, "}}")
    }

    fn write_display(&self, f: &mut Formatter<'_>, marker: Option<u8>) -> std::fmt::Result {
        match self {
            BaseElement::WordBoundary => write!(f, "#"),
            BaseElement::SyllableBoundary => write!(f, "$"),
            BaseElement::SoundClass(key) => write!(f, "{key}"),
            BaseElement::IpaSequence(ipa) => write!(f, "{ipa}"),
            BaseElement::FeatureClass(sc, features) => {
                Self::write_feature_class(f, sc.as_ref(), features, marker)
            }
            BaseElement::Set(els) => Self::write_set(f, els),
            BaseElement::OptionalGroup(pat) => write!(f, "({pat})"),
        }
    }
}

impl SoundMatcherPattern {
    fn write_element(f: &mut Formatter<'_>, el: &PatternElement) -> std::fmt::Result {
        el.base.write_display(f, el.marker)?;
        if let Some(m) = el.marker
            && !matches!(el.base, BaseElement::FeatureClass(_, _))
        {
            write!(f, "{m}")?;
        }
        match el.quantifier {
            Quantifier::ZeroOrMore => write!(f, "*")?,
            Quantifier::OneOrMore => write!(f, "+")?,
            Quantifier::None => {}
        }
        Ok(())
    }
}

impl Display for SoundMatcherPattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for el in &self.elements {
            Self::write_element(f, el)?;
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
