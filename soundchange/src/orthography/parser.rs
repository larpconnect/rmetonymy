use crate::ast::{MatchPattern, Operator, ParsedMatchPart};
use crate::parser::error::SoundChangeParseError;
use crate::parser::Rule;
use ipa::sequence::Phoneme;
use language::sound_class::SoundClassKey;


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrthoTransformElement {
    Empty,
    Literal {
        val: String,
        copy_modifiers: bool,
        append_modifiers: Vec<String>,
    },
    Ref {
        marker: Option<u8>,
        class_key: Option<SoundClassKey>,
        repeat: usize,
        copy_modifiers: bool,
        append_modifiers: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedOrthoRule {
    pub original_string: String,
    pub match_part: MatchPattern,
    pub operator: Operator,
    pub transform_part: Vec<OrthoTransformElement>,
    pub condition: Option<crate::ast::ConditionExpr>,
}

/// Parses a single orthography rule string.
///
/// # Errors
/// Returns an error if parsing or structure validation fails.
pub fn parse_ortho_rule(s: &str) -> Result<ParsedOrthoRule, SoundChangeParseError> {
    let s_trimmed = crate::parser::trim_input_op(s)?;
    let inner = crate::parser::parse_input_to_inner_op(&s_trimmed)?;
    convert_ortho_rule_dispatch_integration(inner, &s_trimmed)
}


fn convert_ortho_rule_dispatch_integration(
    inner: pest::iterators::Pair<'_, Rule>,
    s_trimmed: &str,
) -> Result<ParsedOrthoRule, SoundChangeParseError> {
    match get_ortho_rule_type_op(&inner)? {
        OrthoRuleType::Reference => Err(SoundChangeParseError::ValidationError(
            "Preamble references are not supported in orthography rules".to_string(),
        )),
        OrthoRuleType::Standard => convert_standard_ortho_rule(inner, s_trimmed),
    }
}

enum OrthoRuleType {
    Reference,
    Standard,
}

fn get_ortho_rule_type_op(
    pair: &pest::iterators::Pair<'_, Rule>,
) -> Result<OrthoRuleType, SoundChangeParseError> {
    match pair.as_rule() {
        Rule::reference_rule => Ok(OrthoRuleType::Reference),
        Rule::standard_rule => Ok(OrthoRuleType::Standard),
        _ => Err(SoundChangeParseError::ConversionError(format!(
            "Unexpected rule type {:?}",
            pair.as_rule()
        ))),
    }
}


fn convert_standard_ortho_rule(
    pair: pest::iterators::Pair<'_, Rule>,
    original: &str,
) -> Result<ParsedOrthoRule, SoundChangeParseError> {
    let mut match_part = None;
    let mut operator = Operator::RightMultipleTransparent;
    let mut transform_part = Vec::new();
    let mut condition = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::match_part => {
                match_part = Some(convert_ortho_match_part(inner)?);
            }
            Rule::arrow => {
                operator = crate::parser::parse_operator(inner.as_str())?;
            }
            Rule::transform_part => {
                transform_part = convert_ortho_transform_part(inner)?;
            }
            Rule::condition_expr => {
                condition = Some(crate::parser::condition::convert_condition_expr(inner)?);
            }
            _ => {}
        }
    }

    let match_part = extract_ortho_match_pattern(match_part)?;

    Ok(ParsedOrthoRule {
        original_string: original.to_string(),
        match_part,
        operator,
        transform_part,
        condition,
    })
}

fn extract_ortho_match_pattern(
    match_part_opt: Option<ParsedMatchPart>,
) -> Result<MatchPattern, SoundChangeParseError> {
    match match_part_opt {
        Some(ParsedMatchPart::Pattern(p)) => Ok(p),
        Some(ParsedMatchPart::Reference(_)) => Err(SoundChangeParseError::ValidationError(
            "Preamble references not supported in orthography match part".to_string(),
        )),
        None => Ok(MatchPattern {
            elements: Vec::new(),
        }),
    }
}

fn convert_ortho_match_part(
    pair: pest::iterators::Pair<'_, Rule>,
) -> Result<ParsedMatchPart, SoundChangeParseError> {
    crate::parser::pattern::convert_match_part_generic(pair, convert_ortho_base_element)
}

fn convert_ortho_base_element(
    pair: pest::iterators::Pair<'_, Rule>,
    rule: Rule,
) -> Result<crate::ast::MatchBase, SoundChangeParseError> {
    match rule {
        Rule::ipa_sequence => {
            let s = pair.as_str();
            let ipa = parse_ortho_ipa_string(s);
            Ok(crate::ast::MatchBase::IpaSequence(ipa))
        }
        _ => crate::parser::pattern::convert_base_element(pair, rule),
    }
}

fn parse_ortho_ipa_string(s: &str) -> ipa::IpaString {
    use std::str::FromStr;
    if let Ok(seq) = ipa::sequence::PhonemeSequence::from_str(s) {
        return ipa::IpaString::from(seq);
    }
    let mut elements = Vec::new();
    for c in s.chars() {
        elements.push(ipa::sequence::SequenceElement::Phoneme(Phoneme {
            base: c.to_string(),
            modifiers: Vec::new(),
        }));
    }
    ipa::IpaString::from(ipa::sequence::PhonemeSequence { elements })
}

fn convert_ortho_transform_part(
    pair: pest::iterators::Pair<'_, Rule>,
) -> Result<Vec<OrthoTransformElement>, SoundChangeParseError> {
    let inner = pair.into_inner().next().ok_or_else(|| {
        SoundChangeParseError::ConversionError("Empty transform part".to_string())
    })?;

    match inner.as_rule() {
        Rule::reference_rule => Err(SoundChangeParseError::ValidationError(
            "Preamble references are not supported in orthography rules".to_string(),
        )),
        Rule::transform_pattern => {
            let mut elements = Vec::new();
            for item in inner.into_inner() {
                if item.as_rule() == Rule::transform_element {
                    elements.push(convert_ortho_transform_element(item)?);
                }
            }
            Ok(elements)
        }
        Rule::empty_symbol => Ok(vec![OrthoTransformElement::Empty]),
        _ => Err(SoundChangeParseError::ConversionError(format!(
            "Invalid transform part rule: {:?}",
            inner.as_rule()
        ))),
    }
}

fn convert_ortho_transform_element(
    pair: pest::iterators::Pair<'_, Rule>,
) -> Result<OrthoTransformElement, SoundChangeParseError> {
    let mut inner_pairs = pair.into_inner();
    let inner = inner_pairs.next().ok_or_else(|| {
        SoundChangeParseError::ConversionError("Empty transform element".to_string())
    })?;

    let (modifier_wildcard, append_modifiers) =
        crate::parser::transform::parse_transform_modifiers(inner_pairs);

    match inner.as_rule() {
        Rule::feature_class => Err(SoundChangeParseError::ValidationError(
            "Distinctive feature transforms are not allowed in orthography rules".to_string(),
        )),
        Rule::reference_symbol => {
            convert_ortho_ref_symbol(inner, modifier_wildcard, append_modifiers)
        }
        Rule::ipa_sequence => {
            convert_ortho_ipa_sequence(&inner, modifier_wildcard, append_modifiers)
        }
        _ => Err(SoundChangeParseError::ConversionError(format!(
            "Invalid transform element type: {:?}",
            inner.as_rule()
        ))),
    }
}

fn convert_ortho_ref_symbol(
    inner: pest::iterators::Pair<'_, Rule>,
    wildcard: bool,
    appends: Vec<String>,
) -> Result<OrthoTransformElement, SoundChangeParseError> {
    let (marker, class_key, repeat) =
        crate::parser::pattern::parse_transform_reference_symbol(inner)?;
    if let (None, Some(key)) = (marker, class_key.as_ref()) {
        let class_str = key.as_str();
        if class_str != "C" && class_str != "D" && class_str != "L" && class_str != "V" {
            return Err(SoundChangeParseError::ValidationError(format!(
                "Capital letters are banned in orthography rule transforms: '{class_str}'"
            )));
        }
    }
    Ok(OrthoTransformElement::Ref {
        marker,
        class_key,
        repeat,
        copy_modifiers: wildcard,
        append_modifiers: appends,
    })
}

fn convert_ortho_ipa_sequence(
    inner: &pest::iterators::Pair<'_, Rule>,
    wildcard: bool,
    appends: Vec<String>,
) -> Result<OrthoTransformElement, SoundChangeParseError> {
    let val = inner.as_str().to_string();
    if val.chars().any(|c| c.is_ascii_uppercase()) {
        return Err(SoundChangeParseError::ValidationError(format!(
            "Capital letters are banned in orthography rule transforms: '{val}'"
        )));
    }
    Ok(OrthoTransformElement::Literal {
        val,
        copy_modifiers: wildcard,
        append_modifiers: appends,
    })
}
