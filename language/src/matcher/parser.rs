// qual:allow(srp) - Pattern parser implementation
use crate::matcher::ast::{
    BaseElement, FeatureDescriptor, PatternElement, Quantifier, SoundMatcherError,
    SoundMatcherPattern,
};
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
        let pairs = run_parser_integration(s)?;
        let pattern_pair = crate::parser_utils::extract_pattern_pair_op(pairs, Rule::pattern)
            .map_err(SoundMatcherError::ParseError)?;
        parse_pattern(pattern_pair)
    }
}

fn run_parser_integration(s: &str) -> Result<pest::iterators::Pairs<'_, Rule>, SoundMatcherError> {
    SoundMatcherParser::parse(Rule::main, s)
        .map_err(|e| SoundMatcherError::ParseError(e.to_string()))
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

fn map_rule_to_quantifier_op(rule: Rule) -> Option<Quantifier> {
    match rule {
        Rule::zero_or_more => Some(Quantifier::ZeroOrMore),
        Rule::one_or_more => Some(Quantifier::OneOrMore),
        _ => None,
    }
}

fn map_rule_to_element_op(
    inner: pest::iterators::Pair<'_, Rule>,
) -> Result<Option<(BaseElement, Option<u8>)>, SoundMatcherError> {
    match inner.as_rule() {
        Rule::word_boundary => Ok(Some((BaseElement::WordBoundary, None))),
        Rule::syllable_boundary => Ok(Some((BaseElement::SyllableBoundary, None))),
        Rule::marked_sound_class => {
            let (sc, m) = parse_marked_sound_class(inner)?;
            Ok(Some((sc, m)))
        }
        Rule::feature_class => {
            let (fc, m) = parse_feature_class(inner)?;
            Ok(Some((fc, m)))
        }
        Rule::ipa_sequence => {
            let seq = parse_ipa_sequence(&inner)?;
            Ok(Some((seq, None)))
        }
        Rule::set => {
            let set = parse_set(inner)?;
            Ok(Some((set, None)))
        }
        Rule::optional_group => {
            let opt = parse_optional_group(inner)?;
            Ok(Some((opt, None)))
        }
        _ => Ok(None),
    }
}

fn parse_pattern_element(
    pair: pest::iterators::Pair<Rule>,
) -> Result<PatternElement, SoundMatcherError> {
    let mut quantifier = Quantifier::None;
    let mut base_element = None;
    let mut marker = None;

    for inner in pair.into_inner() {
        if let Some(q) = map_rule_to_quantifier_op(inner.as_rule()) {
            quantifier = q;
        } else if let Some((base, m)) = map_rule_to_element_op(inner)? {
            base_element = Some(base);
            marker = m;
        }
    }

    let base = base_element
        .ok_or_else(|| SoundMatcherError::ParseError("Missing base element".to_string()))?;
    Ok(PatternElement {
        base,
        quantifier,
        marker,
    })
}

fn parse_sound_class(
    inner: &pest::iterators::Pair<Rule>,
) -> Result<BaseElement, SoundMatcherError> {
    let key = inner
        .as_str()
        .parse::<SoundClassKey>()
        .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?;
    Ok(BaseElement::SoundClass(key))
}

fn parse_marked_sound_class(
    pair: pest::iterators::Pair<Rule>,
) -> Result<(BaseElement, Option<u8>), SoundMatcherError> {
    let mut sound_class_opt = None;
    let mut marker = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::sound_class => {
                sound_class_opt = Some(parse_sound_class(&inner)?);
            }
            Rule::marker => {
                let m = inner.as_str().parse::<u8>().map_err(|e| {
                    SoundMatcherError::ParseError(format!("Failed to parse marker: {e}"))
                })?;
                marker = Some(m);
            }
            _ => {}
        }
    }

    let base = sound_class_opt
        .ok_or_else(|| SoundMatcherError::ParseError("Missing sound class".to_string()))?;
    Ok((base, marker))
}

fn parse_ipa_sequence(
    inner: &pest::iterators::Pair<Rule>,
) -> Result<BaseElement, SoundMatcherError> {
    let ipa = inner
        .as_str()
        .parse::<IpaString>()
        .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?;
    Ok(BaseElement::IpaSequence(ipa))
}

fn parse_optional_group(
    inner: pest::iterators::Pair<Rule>,
) -> Result<BaseElement, SoundMatcherError> {
    for opt_inner in inner.into_inner() {
        if opt_inner.as_rule() == Rule::pattern {
            let pat = parse_pattern(opt_inner)?;
            return Ok(BaseElement::OptionalGroup(Box::new(pat)));
        }
    }
    Err(SoundMatcherError::ParseError(
        "Empty optional group".to_string(),
    ))
}

fn parse_feature_descriptor_op(
    pair: pest::iterators::Pair<Rule>,
) -> Result<FeatureDescriptor, SoundMatcherError> {
    let mut sign = true;
    let mut feature_name = "";
    for fd_inner in pair.into_inner() {
        if fd_inner.as_rule() == Rule::feature_sign {
            sign = fd_inner.as_str() == "+";
        } else if fd_inner.as_rule() == Rule::feature_name {
            feature_name = fd_inner.as_str();
        }
    }
    let feature = Feature::from_str(feature_name)
        .map_err(|_e| SoundMatcherError::ParseError(format!("Unknown feature: {feature_name}")))?;
    Ok(FeatureDescriptor { sign, feature })
}

fn parse_feature_class(
    pair: pest::iterators::Pair<Rule>,
) -> Result<(BaseElement, Option<u8>), SoundMatcherError> {
    let mut class_key = None;
    let mut marker = None;
    let mut features = Vec::new();
    for fc_inner in pair.into_inner() {
        match fc_inner.as_rule() {
            Rule::marked_sound_class => {
                let (sc, m) = parse_marked_sound_class(fc_inner)?;
                if let BaseElement::SoundClass(key) = sc {
                    class_key = Some(key);
                }
                marker = m;
            }
            Rule::feature_descriptor => {
                let descriptor = parse_feature_descriptor_op(fc_inner)?;
                features.push(descriptor);
            }
            _ => {}
        }
    }
    Ok((BaseElement::FeatureClass(class_key, features), marker))
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
