use crate::sound_class::SoundClassKey;
use data::SpeFeature;
use ipa::IpaString;
use pest::Parser;
use pest_derive::Parser;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

#[derive(Parser)]
#[grammar = "parser/sound_matcher.pest"]
pub struct SoundMatcherParser;

#[derive(Error, Debug, PartialEq)]
pub enum SoundMatcherError {
    #[error("Failed to parse pattern: {0}")]
    ParseError(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Quantifier {
    ZeroOrMore,
    OneOrMore,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SoundMatcherElement {
    WordBoundary,
    SyllableBoundary,
    SoundClass(SoundClassKey),
    IpaSequence(IpaString),
    FeatureDescriptor(Option<SoundClassKey>, Vec<SpeFeature>),
    Set(Vec<SoundMatcherElement>), // SoundClass or IpaSequence
    OptionalGroup(Vec<SoundMatcherPatternItem>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SoundMatcherPatternItem {
    pub element: SoundMatcherElement,
    pub quantifier: Option<Quantifier>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SoundMatcherPattern {
    pub items: Vec<SoundMatcherPatternItem>,
}

impl FromStr for SoundMatcherPattern {
    type Err = SoundMatcherError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut pairs = SoundMatcherParser::parse(Rule::main, s)
            .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?;

        let main_pair = pairs
            .next()
            .ok_or_else(|| SoundMatcherError::ParseError("Empty input".to_string()))?;

        let mut pattern_pair = None;
        for pair in main_pair.into_inner() {
            if pair.as_rule() == Rule::pattern {
                pattern_pair = Some(pair);
                break;
            }
        }

        let Some(pattern_pair) = pattern_pair else {
            return Err(SoundMatcherError::ParseError("Empty pattern".to_string()));
        };

        Ok(SoundMatcherPattern {
            items: parse_pattern(pattern_pair)?,
        })
    }
}

fn parse_pattern(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Vec<SoundMatcherPatternItem>, SoundMatcherError> {
    let mut items = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::pattern_element {
            items.push(parse_pattern_element(inner)?);
        }
    }

    Ok(items)
}

fn parse_pattern_element(
    pair: pest::iterators::Pair<Rule>,
) -> Result<SoundMatcherPatternItem, SoundMatcherError> {
    let mut element = None;
    let mut quantifier = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::quantifier_zero_or_more => quantifier = Some(Quantifier::ZeroOrMore),
            Rule::quantifier_one_or_more => quantifier = Some(Quantifier::OneOrMore),
            Rule::base_element => {
                element = Some(parse_base_element(inner)?);
            }
            _ => {}
        }
    }

    let element =
        element.ok_or_else(|| SoundMatcherError::ParseError("Missing base element".to_string()))?;
    Ok(SoundMatcherPatternItem {
        element,
        quantifier,
    })
}

fn parse_base_element(
    pair: pest::iterators::Pair<Rule>,
) -> Result<SoundMatcherElement, SoundMatcherError> {
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| SoundMatcherError::ParseError("Empty base element".to_string()))?;
    match inner.as_rule() {
        Rule::word_boundary => Ok(SoundMatcherElement::WordBoundary),
        Rule::syllable_boundary => Ok(SoundMatcherElement::SyllableBoundary),
        Rule::sound_class => {
            let key = inner
                .as_str()
                .parse::<SoundClassKey>()
                .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?;
            Ok(SoundMatcherElement::SoundClass(key))
        }
        Rule::ipa_sequence => {
            let ipa = inner
                .as_str()
                .parse::<IpaString>()
                .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?;
            Ok(SoundMatcherElement::IpaSequence(ipa))
        }
        Rule::feature_descriptor => {
            let mut class = None;
            let mut features = Vec::new();
            for item in inner.into_inner() {
                if item.as_rule() == Rule::sound_class {
                    class = Some(
                        item.as_str()
                            .parse::<SoundClassKey>()
                            .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?,
                    );
                } else if item.as_rule() == Rule::feature {
                    features.push(
                        item.as_str()
                            .parse::<SpeFeature>()
                            .map_err(|e| SoundMatcherError::ParseError(e.clone()))?,
                    );
                }
            }
            Ok(SoundMatcherElement::FeatureDescriptor(class, features))
        }
        Rule::set => {
            let mut items = Vec::new();
            for item in inner.into_inner() {
                if item.as_rule() == Rule::set_item {
                    let set_inner = item
                        .into_inner()
                        .next()
                        .expect("Expected item in pest pair");
                    if set_inner.as_rule() == Rule::sound_class {
                        items.push(SoundMatcherElement::SoundClass(
                            set_inner
                                .as_str()
                                .parse::<SoundClassKey>()
                                .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?,
                        ));
                    } else if set_inner.as_rule() == Rule::ipa_sequence {
                        items.push(SoundMatcherElement::IpaSequence(
                            set_inner
                                .as_str()
                                .parse::<IpaString>()
                                .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?,
                        ));
                    }
                }
            }
            Ok(SoundMatcherElement::Set(items))
        }
        Rule::optional_group => {
            let pattern_pair = inner
                .into_inner()
                .next()
                .expect("Expected item in pest pair");
            let items = parse_pattern(pattern_pair)?;
            Ok(SoundMatcherElement::OptionalGroup(items))
        }
        _ => Err(SoundMatcherError::ParseError(format!(
            "Unknown rule: {:?}",
            inner.as_rule()
        ))),
    }
}

impl Display for SoundMatcherPattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for item in &self.items {
            write!(f, "{item}")?;
        }
        Ok(())
    }
}

impl Display for SoundMatcherPatternItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.element)?;
        match self.quantifier {
            Some(Quantifier::ZeroOrMore) => write!(f, "*")?,
            Some(Quantifier::OneOrMore) => write!(f, "+")?,
            None => {}
        }
        Ok(())
    }
}

impl Display for SoundMatcherElement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SoundMatcherElement::WordBoundary => write!(f, "#"),
            SoundMatcherElement::SyllableBoundary => write!(f, "$"),
            SoundMatcherElement::SoundClass(c) => write!(f, "{c}"),
            SoundMatcherElement::IpaSequence(ipa) => write!(f, "{ipa}"),
            SoundMatcherElement::FeatureDescriptor(c, features) => {
                write!(f, "[")?;
                if let Some(c) = c {
                    write!(f, "{c} ")?;
                }
                for (i, feat) in features.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{feat}")?;
                }
                write!(f, "]")
            }
            SoundMatcherElement::Set(items) => {
                write!(f, "{{")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "}}")
            }
            SoundMatcherElement::OptionalGroup(items) => {
                write!(f, "(")?;
                for item in items {
                    write!(f, "{item}")?;
                }
                write!(f, ")")
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_patterns() {
        let cases = [
            "aCa",
            "#aCa",
            "$ba",
            "[+voice]V",
            "[-voice]V",
            "[F -voice]",
            "{a, b}",
            "{C, i}",
            "C+",
            "(ta)*",
            "V*",
            "C(V)C",
        ];

        for case in cases {
            let res = case.parse::<SoundMatcherPattern>();
            assert!(res.is_ok(), "Failed to parse: {case} -> {res:?}");
            assert_eq!(
                res.expect("Expected item in pest pair").to_string(),
                case.replace(" ", " ").replace(",b", ", b")
            ); // normalized
        }
    }
}
