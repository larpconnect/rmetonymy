use crate::matcher::ast::{SoundMatcherPattern, SoundMatcherError, PatternElement, Quantifier, BaseElement, FeatureDescriptor};
use crate::sound_class::SoundClassKey;
use data::feature::Feature;
use ipa::IpaString;
use pest::Parser;
use pest_derive::Parser;
use std::str::FromStr;

#[derive(Parser)]
#[grammar = "parser/matcher.pest"]
pub struct SoundMatcherParser;

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

        parse_pattern(pattern_pair)
    }
}

fn parse_pattern(
    pair: pest::iterators::Pair<Rule>,
) -> Result<SoundMatcherPattern, SoundMatcherError> {
    let mut elements = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::pattern_element {
            elements.push(parse_pattern_element(inner)?);
        }
    }

    Ok(SoundMatcherPattern { elements })
}

fn parse_pattern_element(
    pair: pest::iterators::Pair<Rule>,
) -> Result<PatternElement, SoundMatcherError> {
    let mut quantifier = Quantifier::None;
    let mut base_element = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::zero_or_more => quantifier = Quantifier::ZeroOrMore,
            Rule::one_or_more => quantifier = Quantifier::OneOrMore,
            Rule::word_boundary => base_element = Some(BaseElement::WordBoundary),
            Rule::syllable_boundary => base_element = Some(BaseElement::SyllableBoundary),
            Rule::sound_class => {
                let key = inner
                    .as_str()
                    .parse::<SoundClassKey>()
                    .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?;
                base_element = Some(BaseElement::SoundClass(key));
            }
            Rule::ipa_sequence => {
                let ipa = inner
                    .as_str()
                    .parse::<IpaString>()
                    .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?;
                base_element = Some(BaseElement::IpaSequence(ipa));
            }
            Rule::feature_class => {
                base_element = Some(parse_feature_class(inner)?);
            }
            Rule::set => {
                base_element = Some(parse_set(inner)?);
            }
            Rule::optional_group => {
                for opt_inner in inner.into_inner() {
                    if opt_inner.as_rule() == Rule::pattern {
                        let pat = parse_pattern(opt_inner)?;
                        base_element = Some(BaseElement::OptionalGroup(Box::new(pat)));
                    }
                }
            }
            _ => {}
        }
    }

    let base = base_element
        .ok_or_else(|| SoundMatcherError::ParseError("Missing base element".to_string()))?;
    Ok(PatternElement { base, quantifier })
}

fn parse_feature_class(
    pair: pest::iterators::Pair<Rule>,
) -> Result<BaseElement, SoundMatcherError> {
    let mut class_key = None;
    let mut features = Vec::new();
    for fc_inner in pair.into_inner() {
        match fc_inner.as_rule() {
            Rule::sound_class => {
                class_key = Some(
                    fc_inner
                        .as_str()
                        .parse::<SoundClassKey>()
                        .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?,
                );
            }
            Rule::feature_descriptor => {
                let mut sign = true;
                let mut feature_name = "";
                for fd_inner in fc_inner.into_inner() {
                    if fd_inner.as_rule() == Rule::feature_sign {
                        sign = fd_inner.as_str() == "+";
                    } else if fd_inner.as_rule() == Rule::feature_name {
                        feature_name = fd_inner.as_str();
                    }
                }
                let feature = Feature::from_str(feature_name).map_err(|_e| {
                    SoundMatcherError::ParseError(format!("Unknown feature: {feature_name}"))
                })?;
                features.push(FeatureDescriptor { sign, feature });
            }
            _ => {}
        }
    }
    Ok(BaseElement::FeatureClass(class_key, features))
}

fn parse_set(pair: pest::iterators::Pair<Rule>) -> Result<BaseElement, SoundMatcherError> {
    let mut set_elements = Vec::new();
    for set_inner in pair.into_inner() {
        if set_inner.as_rule() == Rule::sound_class {
            let key = set_inner
                .as_str()
                .parse::<SoundClassKey>()
                .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?;
            set_elements.push(BaseElement::SoundClass(key));
        } else if set_inner.as_rule() == Rule::ipa_sequence {
            let ipa = set_inner
                .as_str()
                .parse::<IpaString>()
                .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?;
            set_elements.push(BaseElement::IpaSequence(ipa));
        }
    }
    Ok(BaseElement::Set(set_elements))
}
