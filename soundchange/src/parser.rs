use crate::ast::{Operator, ParsedSoundChange};
use pest::Parser;
use pest_derive::Parser;

pub mod condition;
pub mod error;
pub mod pattern;
pub mod transform;

pub use error::SoundChangeParseError;

#[derive(Parser)]
#[grammar = "parser/soundchange.pest"]
pub struct SoundChangeParserInternal;

/// Parses a sound change rule from a string.
///
/// # Errors
/// Returns `SoundChangeParseError` if the input cannot be parsed or if AST conversion fails.
pub fn parse_rule_string(s: &str) -> Result<ParsedSoundChange, SoundChangeParseError> {
    use unicode_normalization::UnicodeNormalization;
    let s_normalized = s.nfd().collect::<String>();
    let s_trimmed = s_normalized.trim();
    if s_trimmed.is_empty() {
        return Err(SoundChangeParseError::ConversionError(
            "Empty input".to_string(),
        ));
    }
    let mut pairs = SoundChangeParserInternal::parse(Rule::sound_change, s_trimmed)
        .map_err(|e| SoundChangeParseError::PestError(e.to_string()))?;
    let main_pair = pairs
        .next()
        .ok_or_else(|| SoundChangeParseError::ConversionError("Empty input".to_string()))?;
    let inner = main_pair
        .into_inner()
        .next()
        .ok_or_else(|| SoundChangeParseError::ConversionError("No rules found".to_string()))?;
    convert_sound_change(inner)
}

fn convert_sound_change(
    pair: pest::iterators::Pair<'_, Rule>,
) -> Result<ParsedSoundChange, SoundChangeParseError> {
    match pair.as_rule() {
        Rule::reference_rule => {
            let name = pattern::convert_reference_rule(pair)?;
            Ok(ParsedSoundChange::Reference(name))
        }
        Rule::standard_rule => convert_standard_rule(pair),
        _ => Err(SoundChangeParseError::ConversionError(format!(
            "Unexpected rule type {:?}",
            pair.as_rule()
        ))),
    }
}

fn convert_standard_rule(
    pair: pest::iterators::Pair<'_, Rule>,
) -> Result<ParsedSoundChange, SoundChangeParseError> {
    let mut match_part = None;
    let mut operator = Operator::RightMultipleTransparent;
    let mut transform_part = None;
    let mut condition = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::match_part => {
                match_part = Some(pattern::convert_match_part(inner)?);
            }
            Rule::arrow => {
                operator = parse_operator(inner.as_str())?;
            }
            Rule::transform_part => {
                transform_part = Some(transform::convert_transform_part(inner)?);
            }
            Rule::condition_expr => {
                condition = Some(condition::convert_condition_expr(inner)?);
            }
            _ => {}
        }
    }

    Ok(ParsedSoundChange::Rule {
        match_part,
        operator,
        transform_part,
        condition,
    })
}

pub(crate) fn parse_operator(s: &str) -> Result<Operator, SoundChangeParseError> {
    match s {
        "=>" | ">" => Ok(Operator::RightMultipleTransparent),
        "->" => Ok(Operator::RightSingleTransparent),
        "=:>" => Ok(Operator::RightMultipleOpaque),
        "<=" | "<" => Ok(Operator::LeftMultipleTransparent),
        "<-" => Ok(Operator::LeftSingleTransparent),
        "<:=" => Ok(Operator::LeftMultipleOpaque),
        _ => Err(SoundChangeParseError::ConversionError(format!(
            "Invalid operator: {s}"
        ))),
    }
}
