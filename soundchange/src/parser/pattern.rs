use crate::ast::{
    FeatureClassKey, FeatureDescriptor, MatchBase, MatchElement, MatchPattern,
    MatchQuantifier, ParsedMatchPart,
};
use crate::parser::Rule;
use crate::parser::error::SoundChangeParseError;
use data::feature::Feature;
use ipa::IpaString;
use pest::iterators::Pair;
use std::str::FromStr;

macro_rules! convert_part_ast {
    ($pair:expr, $pattern_rule:pat, $pattern_conv:expr, $ref_variant:expr, $pat_variant:expr, $empty_val:expr) => {{
        let inner = $pair.into_inner().next().ok_or_else(|| {
            $crate::parser::error::SoundChangeParseError::ConversionError("Empty part".to_string())
        })?;
        match inner.as_rule() {
            $crate::parser::Rule::reference_rule => {
                let name = $crate::parser::pattern::convert_reference_rule(inner)?;
                Ok($ref_variant(name))
            }
            $pattern_rule => {
                let pattern = $pattern_conv(inner)?;
                Ok($pat_variant(pattern))
            }
            $crate::parser::Rule::empty_symbol => Ok($empty_val),
            _ => Err($crate::parser::error::SoundChangeParseError::ConversionError(format!(
                "Invalid part rule: {:?}",
                inner.as_rule()
            ))),
        }
    }};
}

pub(crate) fn convert_match_part_generic<F>(
    pair: Pair<'_, Rule>,
    mut convert_base_fn: F,
) -> Result<ParsedMatchPart, SoundChangeParseError>
where
    F: FnMut(Pair<'_, Rule>, Rule) -> Result<MatchBase, SoundChangeParseError>,
{
    convert_part_ast!(
        pair,
        Rule::pattern,
        |inner| convert_pattern_generic(inner, &mut convert_base_fn),
        ParsedMatchPart::Reference,
        ParsedMatchPart::Pattern,
        ParsedMatchPart::Pattern(MatchPattern { elements: Vec::new() })
    )
}

pub(crate) fn convert_match_part(
    pair: Pair<'_, Rule>,
) -> Result<ParsedMatchPart, SoundChangeParseError> {
    convert_match_part_generic(pair, convert_base_element)
}

pub(crate) fn convert_reference_rule(
    pair: Pair<'_, Rule>,
) -> Result<String, SoundChangeParseError> {
    let name_pair = pair
        .into_inner()
        .next()
        .ok_or_else(|| SoundChangeParseError::ConversionError("Empty reference".to_string()))?;
    Ok(name_pair.as_str().to_string())
}

pub(crate) fn convert_pattern_generic<F>(
    pair: Pair<'_, Rule>,
    mut convert_base_fn: F,
) -> Result<MatchPattern, SoundChangeParseError>
where
    F: FnMut(Pair<'_, Rule>, Rule) -> Result<MatchBase, SoundChangeParseError>,
{
    let mut elements = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::pattern_element {
            elements.push(convert_pattern_element_generic(inner, &mut convert_base_fn)?);
        }
    }
    Ok(MatchPattern { elements })
}

pub(crate) fn convert_pattern(pair: Pair<'_, Rule>) -> Result<MatchPattern, SoundChangeParseError> {
    convert_pattern_generic(pair, convert_base_element)
}

pub(crate) fn convert_pattern_element_generic<F>(
    pair: Pair<'_, Rule>,
    mut convert_base_fn: F,
) -> Result<MatchElement, SoundChangeParseError>
where
    F: FnMut(Pair<'_, Rule>, Rule) -> Result<MatchBase, SoundChangeParseError>,
{
    let mut base = None;
    let mut modifiers_wildcard = false;
    let mut quantifier = MatchQuantifier::None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::modifier_wildcard => {
                modifiers_wildcard = true;
            }
            Rule::quantifier => {
                quantifier = convert_quantifier(inner)?;
            }
            rule => {
                base = Some(convert_base_fn(inner, rule)?);
            }
        }
    }

    let base = base.ok_or_else(|| {
        SoundChangeParseError::ConversionError("Pattern element missing base".to_string())
    })?;

    Ok(MatchElement {
        base,
        modifiers_wildcard,
        quantifier,
    })
}


use super::quantifier::convert_quantifier;

pub(crate) fn convert_base_element(
    pair: Pair<'_, Rule>,
    rule: Rule,
) -> Result<MatchBase, SoundChangeParseError> {
    match rule {
        Rule::word_boundary => Ok(MatchBase::WordBoundary),
        Rule::syllable_boundary => Ok(MatchBase::SyllableBoundary),
        Rule::marked_sound_class => convert_marked_sound_class(pair),
        Rule::set_exclusion => convert_set_exclusion(pair),
        Rule::ipa_sequence => {
            let ipa = pair.as_str().parse::<IpaString>().map_err(|e| {
                SoundChangeParseError::ConversionError(format!("Invalid IPA: {e:?}"))
            })?;
            Ok(MatchBase::IpaSequence(ipa))
        }
        Rule::feature_class => convert_feature_class(pair),
        Rule::set => convert_set(pair),
        Rule::optional_group => convert_optional_group(pair),
        _ => Err(SoundChangeParseError::ConversionError(format!(
            "Unexpected base element rule: {rule:?}"
        ))),
    }
}

fn parse_sound_class_and_marker(
    pair: Pair<'_, Rule>,
    error_msg: &str,
) -> Result<(language::sound_class::SoundClassKey, Option<u8>), SoundChangeParseError> {
    let mut key = None;
    let mut marker = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::sound_class => {
                key = Some(
                    inner
                        .as_str()
                        .parse::<language::sound_class::SoundClassKey>()
                        .map_err(|e| {
                            SoundChangeParseError::ConversionError(format!(
                                "Invalid sound class key: {e:?}"
                            ))
                        })?,
                );
            }
            Rule::marker => {
                marker = Some(inner.as_str().parse::<u8>().map_err(|e| {
                    SoundChangeParseError::ConversionError(format!("Invalid marker: {e}"))
                })?);
            }
            _ => {}
        }
    }

    let key = key.ok_or_else(|| {
        SoundChangeParseError::ConversionError(error_msg.to_string())
    })?;

    Ok((key, marker))
}

pub(crate) fn convert_marked_sound_class(
    pair: Pair<'_, Rule>,
) -> Result<MatchBase, SoundChangeParseError> {
    let (key, marker) = parse_sound_class_and_marker(pair, "Marked sound class missing key")?;
    Ok(MatchBase::SoundClass { key, marker })
}

pub(crate) fn convert_set_exclusion(
    pair: Pair<'_, Rule>,
) -> Result<MatchBase, SoundChangeParseError> {
    let (key, marker) = parse_sound_class_and_marker(pair, "Set exclusion missing key")?;
    Ok(MatchBase::SetExclusion { key, marker })
}

pub(crate) fn parse_feature_class_inner(
    pair: Pair<'_, Rule>,
) -> Result<(Option<FeatureClassKey>, Vec<FeatureDescriptor>), SoundChangeParseError> {
    let mut parsed_key = None;
    let mut features = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::reference_symbol => {
                let (marker, class_key, _repeat) = parse_transform_reference_symbol(inner)?;
                parsed_key = Some((class_key, false, marker));
            }
            Rule::set_exclusion => {
                if let MatchBase::SetExclusion { key, marker } = convert_set_exclusion(inner)? {
                    parsed_key = Some((Some(key), true, marker));
                }
            }
            Rule::feature_descriptor => {
                features.push(convert_feature_descriptor(inner)?);
            }
            _ => {}
        }
    }

    let key_opt = parsed_key.map(|(key, exclude, marker)| FeatureClassKey {
        key,
        exclude,
        marker,
    });

    Ok((key_opt, features))
}

pub(crate) fn convert_feature_class(
    pair: Pair<'_, Rule>,
) -> Result<MatchBase, SoundChangeParseError> {
    let (key_opt, features) = parse_feature_class_inner(pair)?;
    Ok(MatchBase::FeatureClass { key_opt, features })
}

pub(crate) fn convert_feature_descriptor(
    pair: Pair<'_, Rule>,
) -> Result<FeatureDescriptor, SoundChangeParseError> {
    let mut sign = true;
    let mut alpha = None;
    let mut feature_name = "";

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::feature_sign => {
                sign = inner.as_str() == "+";
            }
            Rule::alpha_variable => {
                alpha = Some(convert_alpha_variable(inner));
            }
            Rule::feature_name => {
                feature_name = inner.as_str();
            }
            _ => {}
        }
    }

    let feature = Feature::from_str(feature_name).map_err(|_e| {
        SoundChangeParseError::ConversionError(format!("Unknown feature: {feature_name}"))
    })?;

    Ok(FeatureDescriptor {
        sign,
        alpha,
        feature,
    })
}

use super::alpha::convert_alpha_variable;

use super::set_group::{convert_set, convert_optional_group};

pub(crate) fn parse_transform_reference_symbol(
    pair: Pair<'_, Rule>,
) -> Result<
    (
        Option<u8>,
        Option<language::sound_class::SoundClassKey>,
        usize,
    ),
    SoundChangeParseError,
> {
    let raw_str = pair.as_str();
    let mut inner_pairs = pair.into_inner();
    if let Some(inner) = inner_pairs.next() {
        if inner.as_rule() == Rule::marked_sound_class {
            if let MatchBase::SoundClass { key, marker } = convert_marked_sound_class(inner)? {
                Ok((marker, Some(key), 1))
            } else {
                Err(SoundChangeParseError::ConversionError(
                    "Invalid marked sound class reference".to_string(),
                ))
            }
        } else {
            let count = inner.as_str().len();
            Ok((None, None, count))
        }
    } else {
        let count = raw_str.len();
        Ok((None, None, count))
    }
}
