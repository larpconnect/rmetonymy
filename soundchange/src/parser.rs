use crate::ast::{Operator, ParsedSoundChange};
use pest::Parser;
use pest_derive::Parser;

pub mod condition;
pub mod error;
#[macro_use]
pub mod pattern;
pub mod transform;
pub mod alpha;
pub mod set_group;
pub mod quantifier;

pub use error::SoundChangeParseError;

#[derive(Parser)]
#[grammar = "parser/soundchange.pest"]
pub struct SoundChangeParserInternal;

/// Parses a sound change rule from a string.
///
/// # Errors
/// Returns `SoundChangeParseError` if the input cannot be parsed or if AST conversion fails.
pub fn parse_rule_string(s: &str) -> Result<ParsedSoundChange, SoundChangeParseError> {
    let s_trimmed = trim_input_op(s)?;
    let inner = parse_input_to_inner_op(&s_trimmed)?;
    convert_sound_change(inner)
}

pub(crate) fn trim_input_op(s: &str) -> Result<String, SoundChangeParseError> {
    use unicode_normalization::UnicodeNormalization;
    let s_normalized = s.nfd().collect::<String>();
    let s_trimmed = s_normalized.trim().to_string();
    if s_trimmed.is_empty() {
        return Err(SoundChangeParseError::ConversionError(
            "Empty input".to_string(),
        ));
    }
    Ok(s_trimmed)
}

pub(crate) fn parse_input_to_inner_op(
    s_trimmed: &str,
) -> Result<pest::iterators::Pair<'_, Rule>, SoundChangeParseError> {
    let mut pairs = SoundChangeParserInternal::parse(Rule::sound_change, s_trimmed)
        .map_err(|e| SoundChangeParseError::PestError(e.to_string()))?;
    let main_pair = pairs
        .next()
        .ok_or_else(|| SoundChangeParseError::ConversionError("Empty input".to_string()))?;
    let inner = main_pair
        .into_inner()
        .next()
        .ok_or_else(|| SoundChangeParseError::ConversionError("No rules found".to_string()))?;
    Ok(inner)
}


fn convert_sound_change(
    pair: pest::iterators::Pair<'_, Rule>,
) -> Result<ParsedSoundChange, SoundChangeParseError> {
    let rule_type = get_rule_type_op(&pair)?;
    match_rule_type_integration(rule_type, pair)
}

enum RuleType {
    Reference,
    Standard,
}

fn get_rule_type_op(
    pair: &pest::iterators::Pair<'_, Rule>,
) -> Result<RuleType, SoundChangeParseError> {
    match pair.as_rule() {
        Rule::reference_rule => Ok(RuleType::Reference),
        Rule::standard_rule => Ok(RuleType::Standard),
        _ => Err(SoundChangeParseError::ConversionError(format!(
            "Unexpected rule type {:?}",
            pair.as_rule()
        ))),
    }
}

fn match_rule_type_integration(
    rule_type: RuleType,
    pair: pest::iterators::Pair<'_, Rule>,
) -> Result<ParsedSoundChange, SoundChangeParseError> {
    // Clone pair since it is captured by multiple closures
    let pair_for_ref = pair.clone();
    match_rule_type_op(
        rule_type,
        || {
            let name = pattern::convert_reference_rule(pair_for_ref)?;
            Ok(ParsedSoundChange::Reference(name))
        },
        || convert_standard_rule(pair),
    )
}

fn match_rule_type_op<F, G>(
    rule_type: RuleType,
    ref_fn: F,
    std_fn: G,
) -> Result<ParsedSoundChange, SoundChangeParseError>
where
    F: FnOnce() -> Result<ParsedSoundChange, SoundChangeParseError>,
    G: FnOnce() -> Result<ParsedSoundChange, SoundChangeParseError>,
{
    match rule_type {
        RuleType::Reference => ref_fn(),
        RuleType::Standard => std_fn(),
    }
}


struct StandardRuleBuilder {
    match_part: Option<crate::ast::ParsedMatchPart>,
    operator: Operator,
    transform_part: Option<crate::ast::ParsedTransformPart>,
    condition: Option<crate::ast::ConditionExpr>,
}

fn convert_standard_rule(
    pair: pest::iterators::Pair<'_, Rule>,
) -> Result<ParsedSoundChange, SoundChangeParseError> {
    let mut builder = StandardRuleBuilder {
        match_part: None,
        operator: Operator::RightMultipleTransparent,
        transform_part: None,
        condition: None,
    };
    for_each_pair_integration(pair.into_inner(), &mut builder)?;
    Ok(ParsedSoundChange::Rule {
        match_part: builder.match_part,
        operator: builder.operator,
        transform_part: builder.transform_part,
        condition: builder.condition,
    })
}

fn for_each_pair_integration(
    mut pairs: pest::iterators::Pairs<'_, Rule>,
    builder: &mut StandardRuleBuilder,
) -> Result<(), SoundChangeParseError> {
    pairs.try_for_each(|inner| {
        dispatch_rule_pair_integration(inner, builder)
    })
}

fn dispatch_rule_pair_integration(
    inner: pest::iterators::Pair<'_, Rule>,
    builder: &mut StandardRuleBuilder,
) -> Result<(), SoundChangeParseError> {
    let rule = inner.as_rule();
    match_rule_dispatch_op(rule, inner, |r, pair| {
        match r {
            Rule::match_part => {
                builder.match_part = Some(pattern::convert_match_part(pair)?);
            }
            Rule::arrow => {
                builder.operator = parse_operator(pair.as_str())?;
            }
            Rule::transform_part => {
                builder.transform_part = Some(transform::convert_transform_part(pair)?);
            }
            Rule::condition_expr => {
                builder.condition = Some(condition::convert_condition_expr(pair)?);
            }
            _ => {}
        }
        Ok(())
    })
}

fn match_rule_dispatch_op<F>(
    rule: Rule,
    inner: pest::iterators::Pair<'_, Rule>,
    action_fn: F,
) -> Result<(), SoundChangeParseError>
where
    F: FnOnce(Rule, pest::iterators::Pair<'_, Rule>) -> Result<(), SoundChangeParseError>,
{
    action_fn(rule, inner)
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
