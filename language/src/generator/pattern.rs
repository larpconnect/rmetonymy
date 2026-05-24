//! Word generator patterns AST and Pest parser.

use crate::sound_class::SoundClassKey;
use pest::Parser;
use pest_derive::Parser;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

/// The Pest parser for word generator patterns.
#[derive(Parser)]
#[grammar = "parser/generator.pest"]
pub struct GeneratorPatternParser;

/// Errors that can occur during pattern parsing.
#[derive(Debug, Error, PartialEq, Clone)]
pub enum GeneratorError {
    /// Failed to parse pattern syntax.
    #[error("Failed to parse pattern: {0}")]
    ParseError(String),
}

/// A single element of a word generator pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordPatternElement {
    /// Reference to a sound class (e.g. C, V).
    SoundClass(SoundClassKey),
    /// A literal string (e.g. "k", "t").
    Literal(String),
    /// An equiprobable choice set (e.g. {p,t,k}).
    Set(Vec<String>),
    /// An optional group with a percent probability (e.g. (C)15%).
    Optional(Box<WordPattern>, u8),
    /// Reference to another grammatical type generator (e.g. [verb], [noun.masculine]).
    GrammarRef {
        /// Primary grammatical type.
        primary: String,
        /// Optional secondary subtype.
        secondary: Option<String>,
    },
    /// A syllable break marker (represented by a dot `.`).
    SyllableBreak,
    /// A primary stress marker (represented by `ˈ`).
    StressMarker,
}

/// A parsed word generator pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordPattern {
    /// Ordered elements making up the pattern.
    pub elements: Vec<WordPatternElement>,
}

impl Display for WordPatternElement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SoundClass(sc) => write!(f, "{sc}"),
            Self::Literal(s) => write!(f, "{s}"),
            Self::Set(choices) => {
                write!(f, "{{")?;
                for (i, choice) in choices.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{choice}")?;
                }
                write!(f, "}}")
            }
            Self::Optional(pat, prob) => {
                write!(f, "({pat})")?;
                if *prob != 20 {
                    write!(f, "{prob}%")?;
                }
                Ok(())
            }
            Self::GrammarRef { primary, secondary } => {
                write!(f, "[{primary}")?;
                if let Some(sec) = secondary {
                    write!(f, ".{sec}")?;
                }
                write!(f, "]")
            }
            Self::SyllableBreak => write!(f, "."),
            Self::StressMarker => write!(f, "ˈ"),
        }
    }
}

impl Display for WordPattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for el in &self.elements {
            write!(f, "{el}")?;
        }
        Ok(())
    }
}

impl FromStr for WordPattern {
    type Err = GeneratorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut pairs = GeneratorPatternParser::parse(Rule::main, s)
            .map_err(|e| GeneratorError::ParseError(e.to_string()))?;

        let main_pair = pairs
            .next()
            .ok_or_else(|| GeneratorError::ParseError("Empty input".to_string()))?;

        let mut pattern_pair = None;
        for pair in main_pair.into_inner() {
            if pair.as_rule() == Rule::pattern {
                pattern_pair = Some(pair);
                break;
            }
        }

        let Some(pattern_pair) = pattern_pair else {
            return Err(GeneratorError::ParseError("Empty pattern".to_string()));
        };

        parse_pattern(pattern_pair)
    }
}

fn parse_pattern(pair: pest::iterators::Pair<Rule>) -> Result<WordPattern, GeneratorError> {
    let mut elements = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::sound_class => {
                let key = inner
                    .as_str()
                    .parse::<SoundClassKey>()
                    .map_err(|e| GeneratorError::ParseError(e.to_string()))?;
                elements.push(WordPatternElement::SoundClass(key));
            }
            Rule::literal => {
                elements.push(WordPatternElement::Literal(inner.as_str().to_string()));
            }
            Rule::syllable_break => {
                elements.push(WordPatternElement::SyllableBreak);
            }
            Rule::stress_marker => {
                elements.push(WordPatternElement::StressMarker);
            }
            Rule::grammar_ref => {
                let mut primary = String::new();
                let mut secondary = None;
                for ref_inner in inner.into_inner() {
                    match ref_inner.as_rule() {
                        Rule::primary_type => {
                            primary = ref_inner.as_str().to_string();
                        }
                        Rule::secondary_type => {
                            secondary = Some(ref_inner.as_str().to_string());
                        }
                        _ => {}
                    }
                }
                elements.push(WordPatternElement::GrammarRef { primary, secondary });
            }
            Rule::set_selector => {
                let mut choices = Vec::new();
                for set_inner in inner.into_inner() {
                    if set_inner.as_rule() == Rule::set_element {
                        choices.push(set_inner.as_str().to_string());
                    }
                }
                elements.push(WordPatternElement::Set(choices));
            }
            Rule::optional_group => {
                let mut prob = 20;
                let mut inner_pattern = None;
                for opt_inner in inner.into_inner() {
                    match opt_inner.as_rule() {
                        Rule::pattern => {
                            inner_pattern = Some(parse_pattern(opt_inner)?);
                        }
                        Rule::probability => {
                            let s = opt_inner.as_str();
                            let s = s.strip_suffix('%').unwrap_or(s);
                            prob = s.parse::<u8>().unwrap_or(20);
                        }
                        _ => {}
                    }
                }
                if let Some(pat) = inner_pattern {
                    elements.push(WordPatternElement::Optional(Box::new(pat), prob));
                }
            }
            _ => {}
        }
    }

    Ok(WordPattern { elements })
}

impl Serialize for WordPattern {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for WordPattern {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<Self>().map_err(serde::de::Error::custom)
    }
}
