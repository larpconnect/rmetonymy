use crate::ast::{
    AlphaVariable, FeatureClassKey, FeatureDescriptor, MatchBase, MatchElement, MatchPattern,
    MatchQuantifier, ParsedMatchPart,
};
use crate::parser::Rule;
use crate::parser::error::SoundChangeParseError;
use data::feature::Feature;
use ipa::IpaString;
use pest::iterators::Pair;
use std::str::FromStr;

pub(crate) fn convert_match_part(
    pair: Pair<'_, Rule>,
) -> Result<ParsedMatchPart, SoundChangeParseError> {
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| SoundChangeParseError::ConversionError("Empty match part".to_string()))?;
    match inner.as_rule() {
        Rule::reference_rule => {
            let name = convert_reference_rule(inner)?;
            Ok(ParsedMatchPart::Reference(name))
        }
        Rule::pattern => {
            let pattern = convert_pattern(inner)?;
            Ok(ParsedMatchPart::Pattern(pattern))
        }
        Rule::empty_symbol => Ok(ParsedMatchPart::Pattern(MatchPattern {
            elements: Vec::new(),
        })),
        _ => Err(SoundChangeParseError::ConversionError(format!(
            "Invalid match part rule: {:?}",
            inner.as_rule()
        ))),
    }
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

pub(crate) fn convert_pattern(pair: Pair<'_, Rule>) -> Result<MatchPattern, SoundChangeParseError> {
    let mut elements = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::pattern_element {
            elements.push(convert_pattern_element(inner)?);
        }
    }
    Ok(MatchPattern { elements })
}

pub(crate) fn convert_pattern_element(
    pair: Pair<'_, Rule>,
) -> Result<MatchElement, SoundChangeParseError> {
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
                base = Some(convert_base_element(inner, rule)?);
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

pub(crate) fn convert_quantifier(
    pair: Pair<'_, Rule>,
) -> Result<MatchQuantifier, SoundChangeParseError> {
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| SoundChangeParseError::ConversionError("Empty quantifier".to_string()))?;
    let text = inner.as_str();
    let (is_zero, num_str) = if let Some(stripped) = text.strip_prefix('*') {
        (true, stripped)
    } else if let Some(stripped) = text.strip_prefix('+') {
        (false, stripped)
    } else {
        return Err(SoundChangeParseError::ConversionError(format!(
            "Invalid quantifier: {text}"
        )));
    };
    let num_opt = if num_str.is_empty() {
        None
    } else {
        Some(num_str.parse::<u32>().map_err(|e| {
            SoundChangeParseError::ConversionError(format!("Invalid quantifier limit: {e}"))
        })?)
    };

    match (is_zero, num_opt) {
        (true, None) => Ok(MatchQuantifier::ZeroOrMore),
        (false, None) => Ok(MatchQuantifier::OneOrMore),
        (true, Some(n)) => Ok(MatchQuantifier::ZeroOrMoreBounded(n)),
        (false, Some(n)) => Ok(MatchQuantifier::OneOrMoreBounded(n)),
    }
}

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

pub(crate) fn convert_marked_sound_class(
    pair: Pair<'_, Rule>,
) -> Result<MatchBase, SoundChangeParseError> {
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
        SoundChangeParseError::ConversionError("Marked sound class missing key".to_string())
    })?;

    Ok(MatchBase::SoundClass { key, marker })
}

pub(crate) fn convert_set_exclusion(
    pair: Pair<'_, Rule>,
) -> Result<MatchBase, SoundChangeParseError> {
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
        SoundChangeParseError::ConversionError("Set exclusion missing key".to_string())
    })?;

    Ok(MatchBase::SetExclusion { key, marker })
}

pub(crate) fn convert_feature_class(
    pair: Pair<'_, Rule>,
) -> Result<MatchBase, SoundChangeParseError> {
    let mut key_opt = None;
    let mut features = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::reference_symbol => {
                let (marker, class_key, _repeat) = parse_transform_reference_symbol(inner)?;
                key_opt = Some(FeatureClassKey {
                    key: class_key,
                    exclude: false,
                    marker,
                });
            }
            Rule::set_exclusion => {
                if let MatchBase::SetExclusion { key, marker } = convert_set_exclusion(inner)? {
                    key_opt = Some(FeatureClassKey {
                        key: Some(key),
                        exclude: true,
                        marker,
                    });
                }
            }
            Rule::feature_descriptor => {
                features.push(convert_feature_descriptor(inner)?);
            }
            _ => {}
        }
    }

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

pub(crate) fn convert_alpha_variable(pair: Pair<'_, Rule>) -> AlphaVariable {
    let mut sign = false;
    let mut greek = 'α';
    let mut name = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::feature_sign => {
                sign = inner.as_str() == "-";
            }
            Rule::greek_letter => {
                greek = inner.as_str().chars().next().unwrap_or('α');
            }
            Rule::name => {
                name = inner.as_str().to_string();
            }
            _ => {}
        }
    }

    AlphaVariable { greek, name, sign }
}

pub(crate) fn convert_set(pair: Pair<'_, Rule>) -> Result<MatchBase, SoundChangeParseError> {
    let mut elements = Vec::new();
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::sound_class => {
                let key = inner
                    .as_str()
                    .parse::<language::sound_class::SoundClassKey>()
                    .map_err(|e| {
                        SoundChangeParseError::ConversionError(format!(
                            "Invalid sound class key: {e:?}"
                        ))
                    })?;
                elements.push(MatchBase::SoundClass { key, marker: None });
            }
            Rule::ipa_sequence => {
                let ipa = inner.as_str().parse::<IpaString>().map_err(|e| {
                    SoundChangeParseError::ConversionError(format!("Invalid IPA: {e:?}"))
                })?;
                elements.push(MatchBase::IpaSequence(ipa));
            }
            _ => {}
        }
    }
    Ok(MatchBase::Set(elements))
}

pub(crate) fn convert_optional_group(
    pair: Pair<'_, Rule>,
) -> Result<MatchBase, SoundChangeParseError> {
    let inner = pair.into_inner().next().ok_or_else(|| {
        SoundChangeParseError::ConversionError("Empty optional group".to_string())
    })?;
    let pattern = convert_pattern(inner)?;
    Ok(MatchBase::OptionalGroup(pattern))
}

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
