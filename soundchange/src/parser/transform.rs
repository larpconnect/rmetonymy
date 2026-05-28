use crate::ast::{
    FeatureClassKey, FeatureDescriptor, ParsedTransformPart, TransformElement, TransformPattern,
};
use crate::parser::Rule;
use crate::parser::error::SoundChangeParseError;
use crate::parser::pattern::{
    convert_feature_descriptor, convert_reference_rule, convert_set_exclusion,
    parse_transform_reference_symbol,
};
use ipa::IpaString;
use pest::iterators::Pair;

pub(crate) fn convert_transform_part(
    pair: Pair<'_, Rule>,
) -> Result<ParsedTransformPart, SoundChangeParseError> {
    let inner = pair.into_inner().next().ok_or_else(|| {
        SoundChangeParseError::ConversionError("Empty transform part".to_string())
    })?;
    match inner.as_rule() {
        Rule::reference_rule => {
            let name = convert_reference_rule(inner)?;
            Ok(ParsedTransformPart::Reference(name))
        }
        Rule::transform_pattern => {
            let pattern = convert_transform_pattern(inner)?;
            Ok(ParsedTransformPart::Pattern(pattern))
        }
        Rule::empty_symbol => Ok(ParsedTransformPart::Empty),
        _ => Err(SoundChangeParseError::ConversionError(format!(
            "Invalid transform part rule: {:?}",
            inner.as_rule()
        ))),
    }
}

pub(crate) fn convert_transform_pattern(
    pair: Pair<'_, Rule>,
) -> Result<TransformPattern, SoundChangeParseError> {
    let mut elements = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::transform_element {
            elements.push(convert_transform_element(inner)?);
        }
    }
    Ok(TransformPattern { elements })
}

pub(crate) fn parse_transform_modifiers(
    inner_pairs: pest::iterators::Pairs<'_, Rule>,
) -> (bool, Vec<String>) {
    let mut modifier_wildcard = false;
    let mut append_modifiers = Vec::new();
    for sibling in inner_pairs {
        match sibling.as_rule() {
            Rule::modifier_wildcard => {
                modifier_wildcard = true;
            }
            Rule::append_modifier => {
                append_modifiers.push(sibling.as_str().to_string());
            }
            _ => {}
        }
    }
    (modifier_wildcard, append_modifiers)
}

fn convert_transform_element_inner(
    inner: Pair<'_, Rule>,
    modifier_wildcard: bool,
    append_modifiers: Vec<String>,
) -> Result<TransformElement, SoundChangeParseError> {
    match inner.as_rule() {
        Rule::feature_class => {
            let (key_opt, feature_changes) = parse_transform_feature_class(inner)?;
            let marker = key_opt.as_ref().and_then(|k| k.marker);
            let class_key = key_opt.and_then(|k| k.key);
            Ok(TransformElement::Ref {
                marker,
                class_key,
                repeat: 1,
                copy_modifiers: modifier_wildcard,
                append_modifiers,
                feature_changes,
            })
        }
        Rule::reference_symbol => {
            let (marker, class_key, repeat) = parse_transform_reference_symbol(inner)?;
            Ok(TransformElement::Ref {
                marker,
                class_key,
                repeat,
                copy_modifiers: modifier_wildcard,
                append_modifiers,
                feature_changes: Vec::new(),
            })
        }
        Rule::ipa_sequence => {
            let ipa = inner.as_str().parse::<IpaString>().map_err(|e| {
                SoundChangeParseError::ConversionError(format!("Invalid IPA: {e:?}"))
            })?;
            Ok(TransformElement::Literal {
                ipa,
                copy_modifiers: modifier_wildcard,
                append_modifiers,
            })
        }
        _ => {
            let inner_rule = inner.as_rule();
            Err(SoundChangeParseError::ConversionError(format!(
                "Invalid transform element type: {inner_rule:?}"
            )))
        }
    }
}

pub(crate) fn convert_transform_element(
    pair: Pair<'_, Rule>,
) -> Result<TransformElement, SoundChangeParseError> {
    let mut inner_pairs = pair.into_inner();
    let inner = inner_pairs.next().ok_or_else(|| {
        SoundChangeParseError::ConversionError("Empty transform element".to_string())
    })?;

    let (modifier_wildcard, append_modifiers) = parse_transform_modifiers(inner_pairs);
    convert_transform_element_inner(inner, modifier_wildcard, append_modifiers)
}

pub(crate) fn parse_transform_feature_class(
    pair: Pair<'_, Rule>,
) -> Result<(Option<FeatureClassKey>, Vec<FeatureDescriptor>), SoundChangeParseError> {
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
                if let crate::ast::MatchBase::SetExclusion { key, marker } =
                    convert_set_exclusion(inner)?
                {
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

    Ok((key_opt, features))
}
