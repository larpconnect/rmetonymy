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

const DEFAULT_OPTIONAL_PROBABILITY: u8 = 20;

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
                if *prob != DEFAULT_OPTIONAL_PROBABILITY {
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
        let pattern_pair = parse_to_pattern_pair_integration(s)?;
        parse_pattern(pattern_pair)
    }
}

fn parse_to_pattern_pair_integration(s: &str) -> Result<pest::iterators::Pair<'_, Rule>, GeneratorError> {
    let pairs = GeneratorPatternParser::parse(Rule::main, s)
        .map_err(|e| GeneratorError::ParseError(e.to_string()))?;
    crate::parser_utils::extract_pattern_pair_op(pairs, Rule::pattern)
        .map_err(GeneratorError::ParseError)
}

fn parse_pattern(pair: pest::iterators::Pair<Rule>) -> Result<WordPattern, GeneratorError> {
    let mut elements = Vec::new();
    for inner in pair.into_inner() {
        if let Some(el) = parse_pattern_element(inner)? {
            elements.push(el);
        }
    }
    Ok(WordPattern { elements })
}

fn parse_pattern_element(
    inner: pest::iterators::Pair<Rule>,
) -> Result<Option<WordPatternElement>, GeneratorError> {
    match inner.as_rule() {
        Rule::sound_class => {
            let key = inner
                .as_str()
                .parse::<SoundClassKey>()
                .map_err(|e| GeneratorError::ParseError(e.to_string()))?;
            Ok(Some(WordPatternElement::SoundClass(key)))
        }
        Rule::literal => Ok(Some(WordPatternElement::Literal(
            inner.as_str().to_string(),
        ))),
        Rule::syllable_break => Ok(Some(WordPatternElement::SyllableBreak)),
        Rule::stress_marker => Ok(Some(WordPatternElement::StressMarker)),
        Rule::grammar_ref => Ok(Some(parse_grammar_ref(inner))),
        Rule::set_selector => Ok(Some(parse_set_selector(inner))),
        Rule::optional_group => parse_optional_group(inner).map(Some),
        _ => Ok(None),
    }
}

fn parse_grammar_ref(pair: pest::iterators::Pair<Rule>) -> WordPatternElement {
    let mut primary = String::new();
    let mut secondary = None;
    for ref_inner in pair.into_inner() {
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
    WordPatternElement::GrammarRef { primary, secondary }
}

fn parse_set_selector(pair: pest::iterators::Pair<Rule>) -> WordPatternElement {
    let mut choices = Vec::new();
    for set_inner in pair.into_inner() {
        if set_inner.as_rule() == Rule::set_element {
            choices.push(set_inner.as_str().to_string());
        }
    }
    WordPatternElement::Set(choices)
}

fn parse_optional_group(
    pair: pest::iterators::Pair<Rule>,
) -> Result<WordPatternElement, GeneratorError> {
    let mut prob = DEFAULT_OPTIONAL_PROBABILITY;
    let mut inner_pattern = None;
    for opt_inner in pair.into_inner() {
        match opt_inner.as_rule() {
            Rule::pattern => {
                inner_pattern = Some(parse_pattern(opt_inner)?);
            }
            Rule::probability => {
                let s = opt_inner.as_str();
                let s = s.strip_suffix('%').unwrap_or(s);
                prob = s
                    .parse::<u8>()
                    .map_err(|e| GeneratorError::ParseError(format!("Invalid probability: {e}")))?;
            }
            _ => {}
        }
    }
    let pat = inner_pattern
        .ok_or_else(|| GeneratorError::ParseError("Empty optional group pattern".to_string()))?;
    Ok(WordPatternElement::Optional(Box::new(pat), prob))
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
