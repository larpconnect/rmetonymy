use crate::sound_class::SoundClassKey;
use crate::sound_matcher::ast::{MatcherElement, QuantifiedElement, Quantifier, SoundMatcherError, SoundMatcherPattern};
use data::SpeFeature;
use ipa::IpaString;
use pest::Parser;
use pest_derive::Parser;
use std::str::FromStr;

#[derive(Parser)]
#[grammar = "parser/sound_matcher.pest"]
pub struct SoundMatcherParser;

impl FromStr for SoundMatcherPattern {
    type Err = SoundMatcherError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut pairs = SoundMatcherParser::parse(Rule::main, s)
            .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?;

        let main_pair = pairs
            .next()
            .ok_or_else(|| SoundMatcherError::ParseError("Empty input".into()))?;
        let mut pattern_pair = None;
        for pair in main_pair.into_inner() {
            if pair.as_rule() == Rule::pattern {
                pattern_pair = Some(pair);
                break;
            }
        }

        let Some(pattern_pair) = pattern_pair else {
            return Err(SoundMatcherError::ParseError("Empty pattern".into()));
        };

        Ok(SoundMatcherPattern {
            elements: parse_pattern(pattern_pair)?,
        })
    }
}

fn parse_pattern(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Vec<QuantifiedElement>, SoundMatcherError> {
    let mut elements = Vec::new();
    for element_pair in pair.into_inner() {
        if element_pair.as_rule() == Rule::element {
            let mut inner = element_pair.into_inner();
            let base_pair = inner
                .next()
                .ok_or_else(|| SoundMatcherError::ParseError("Missing base pair".into()))?;
            let mut quantifier = None;
            if let Some(q_pair) = inner.next() {
                quantifier = match q_pair.as_str() {
                    "*" => Some(Quantifier::ZeroOrMore),
                    "+" => Some(Quantifier::OneOrMore),
                    _ => None,
                };
            }

            let element = parse_base_element(base_pair)?;
            elements.push(QuantifiedElement {
                element,
                quantifier,
            });
        }
    }
    Ok(elements)
}

fn parse_base_element(
    pair: pest::iterators::Pair<Rule>,
) -> Result<MatcherElement, SoundMatcherError> {
    match pair.as_rule() {
        Rule::word_boundary => Ok(MatcherElement::WordBoundary),
        Rule::syllable_boundary => Ok(MatcherElement::SyllableBoundary),
        Rule::sound_class => {
            let key = pair
                .as_str()
                .parse::<SoundClassKey>()
                .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?;
            Ok(MatcherElement::SoundClass(key))
        }
        Rule::ipa_sequence => {
            let ipa = pair
                .as_str()
                .parse::<IpaString>()
                .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?;
            Ok(MatcherElement::IpaSequence(ipa))
        }
        Rule::descriptor => {
            let mut sound_class = None;
            let mut features = Vec::new();
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::sound_class {
                    sound_class = Some(
                        inner
                            .as_str()
                            .parse::<SoundClassKey>()
                            .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?,
                    );
                } else if inner.as_rule() == Rule::feature {
                    let mut sign = "+";
                    let mut name = "";
                    for f_inner in inner.into_inner() {
                        if f_inner.as_rule() == Rule::sign {
                            sign = f_inner.as_str();
                        } else if f_inner.as_rule() == Rule::feature_name {
                            name = f_inner.as_str();
                        }
                    }
                    let feature_str = format!("{sign}{name}");
                    let spe_feature = feature_str
                        .parse::<SpeFeature>()
                        .map_err(SoundMatcherError::InvalidFeature)?;
                    features.push(spe_feature);
                }
            }
            Ok(MatcherElement::Descriptor(sound_class, features))
        }
        Rule::set => {
            let mut elements = Vec::new();
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::set_element {
                    let set_inner = inner
                        .into_inner()
                        .next()
                        .ok_or_else(|| SoundMatcherError::ParseError("Missing set inner".into()))?;
                    elements.push(parse_base_element(set_inner)?);
                }
            }
            Ok(MatcherElement::Set(elements))
        }
        Rule::optional_group => {
            let pattern_pair = pair
                .into_inner()
                .next()
                .ok_or_else(|| SoundMatcherError::ParseError("Missing group inner".into()))?;
            let elements = parse_pattern(pattern_pair)?;
            Ok(MatcherElement::OptionalGroup(elements))
        }
        _ => Err(SoundMatcherError::ParseError(format!(
            "Unexpected rule: {:?}",
            pair.as_rule()
        ))),
    }
}
