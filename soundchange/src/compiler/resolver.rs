use crate::ast::{
    ConditionExpr, MatchPattern, Operator, ParsedMatchPart, ParsedSoundChange, ParsedTransformPart,
    PreambleItem, PreambleType, TransformElement, TransformPattern,
};
use crate::compiler::cond_resolver::resolve_condition_expr;
use crate::compiler::{CompiledConditionExpr, CompiledRuleChange};
use crate::parser::{SoundChangeParseError, parse_rule_string};
use std::collections::HashMap;

pub fn get_preamble_value_op(
    name: &str,
    preamble: &HashMap<String, PreambleItem>,
    expected_type: PreambleType,
) -> Result<String, SoundChangeParseError> {
    let item = preamble.get(name).ok_or_else(|| {
        SoundChangeParseError::ReferenceError(format!("Preamble item '{name}' not found"))
    })?;
    if item.r#type != expected_type {
        return Err(SoundChangeParseError::ReferenceError(format!(
            "Preamble item '{name}' is not of type {expected_type:?}"
        )));
    }
    let val = item.value.as_ref().ok_or_else(|| {
        SoundChangeParseError::ReferenceError(format!("Preamble item '{name}' has no value"))
    })?;
    Ok(val.clone())
}

pub fn extract_match_pattern_op(
    parsed: ParsedSoundChange,
    name: &str,
) -> Result<MatchPattern, SoundChangeParseError> {
    if let ParsedSoundChange::Rule {
        match_part: Some(ParsedMatchPart::Pattern(p)),
        ..
    } = parsed
    {
        Ok(p)
    } else {
        Err(SoundChangeParseError::ReferenceError(format!(
            "Failed to parse preamble match pattern for '{name}'"
        )))
    }
}

pub fn resolve_match_reference(
    name: &str,
    preamble: &HashMap<String, PreambleItem>,
) -> Result<MatchPattern, SoundChangeParseError> {
    let val = get_preamble_value_op(name, preamble, PreambleType::Match)?;
    let parsed = parse_rule_string(&format!("{val} => ∅"))?;
    extract_match_pattern_op(parsed, name)
}

pub fn match_match_part_op<F>(
    part_opt: Option<ParsedMatchPart>,
    mut ref_fn: F,
) -> Result<MatchPattern, SoundChangeParseError>
where
    F: FnMut(String) -> Result<MatchPattern, SoundChangeParseError>,
{
    let Some(part) = part_opt else {
        return Ok(MatchPattern {
            elements: Vec::new(),
        });
    };
    match part {
        ParsedMatchPart::Pattern(p) => Ok(p),
        ParsedMatchPart::Reference(name) => ref_fn(name),
    }
}

pub fn resolve_match_part(
    part_opt: Option<ParsedMatchPart>,
    preamble: &HashMap<String, PreambleItem>,
) -> Result<MatchPattern, SoundChangeParseError> {
    match_match_part_op(part_opt, |name| resolve_match_reference(&name, preamble))
}

pub fn extract_transform_pattern_op(
    parsed: ParsedSoundChange,
    name: &str,
) -> Result<TransformPattern, SoundChangeParseError> {
    if let ParsedSoundChange::Rule {
        transform_part: Some(ParsedTransformPart::Pattern(p)),
        ..
    } = parsed
    {
        Ok(p)
    } else {
        Err(SoundChangeParseError::ReferenceError(format!(
            "Failed to parse preamble transform pattern for '{name}'"
        )))
    }
}

pub fn resolve_transform_reference(
    name: &str,
    preamble: &HashMap<String, PreambleItem>,
) -> Result<TransformPattern, SoundChangeParseError> {
    let val = get_preamble_value_op(name, preamble, PreambleType::Transform)?;
    let parsed = parse_rule_string(&format!("∅ => {val}"))?;
    extract_transform_pattern_op(parsed, name)
}

pub fn match_transform_part_op<F>(
    part_opt: Option<ParsedTransformPart>,
    mut ref_fn: F,
) -> Result<TransformPattern, SoundChangeParseError>
where
    F: FnMut(String) -> Result<TransformPattern, SoundChangeParseError>,
{
    let Some(part) = part_opt else {
        return Ok(TransformPattern {
            elements: vec![TransformElement::Empty],
        });
    };
    match part {
        ParsedTransformPart::Pattern(p) => Ok(p),
        ParsedTransformPart::Empty => Ok(TransformPattern {
            elements: vec![TransformElement::Empty],
        }),
        ParsedTransformPart::Reference(name) => ref_fn(name),
    }
}

pub fn resolve_transform_part(
    part_opt: Option<ParsedTransformPart>,
    preamble: &HashMap<String, PreambleItem>,
) -> Result<TransformPattern, SoundChangeParseError> {
    match_transform_part_op(part_opt, |name| {
        resolve_transform_reference(&name, preamble)
    })
}

pub fn get_full_preamble_item_op(
    name: &str,
    preamble: &HashMap<String, PreambleItem>,
    visited: &mut std::collections::HashSet<String>,
) -> Result<PreambleItem, SoundChangeParseError> {
    if !visited.insert(name.to_string()) {
        return Err(SoundChangeParseError::ReferenceError(format!(
            "Circular reference detected in preamble: '{name}'"
        )));
    }
    let item = preamble.get(name).ok_or_else(|| {
        visited.remove(name);
        SoundChangeParseError::ReferenceError(format!("Preamble item '{name}' not found"))
    })?;
    if item.r#type != PreambleType::Full {
        visited.remove(name);
        return Err(SoundChangeParseError::ReferenceError(format!(
            "Preamble item '{name}' is not of type 'full'"
        )));
    }
    Ok(item.clone())
}

pub fn expand_preamble_changes_list(
    changes: &[String],
    preamble: &HashMap<String, PreambleItem>,
    visited: &mut std::collections::HashSet<String>,
) -> Result<Vec<CompiledRuleChange>, SoundChangeParseError> {
    changes
        .iter()
        .map(|sub_str| expand_single_preamble_change(sub_str, preamble, visited))
        .collect::<Result<Vec<Vec<CompiledRuleChange>>, _>>()
        .map(|v| v.into_iter().flatten().collect())
}

pub fn expand_single_preamble_change(
    sub_str: &str,
    preamble: &HashMap<String, PreambleItem>,
    visited: &mut std::collections::HashSet<String>,
) -> Result<Vec<CompiledRuleChange>, SoundChangeParseError> {
    let sub_parsed = parse_rule_string(sub_str)?;
    expand_references_rec(sub_parsed, preamble, sub_str, visited)
}

pub fn expand_preamble_item_changes(
    item: &PreambleItem,
    preamble: &HashMap<String, PreambleItem>,
    visited: &mut std::collections::HashSet<String>,
) -> Result<Vec<CompiledRuleChange>, SoundChangeParseError> {
    let res = expand_preamble_changes_list(&item.changes, preamble, visited);
    visited.remove(&item.name);
    res
}

pub fn expand_preamble_reference(
    name: &str,
    preamble: &HashMap<String, PreambleItem>,
    visited: &mut std::collections::HashSet<String>,
) -> Result<Vec<CompiledRuleChange>, SoundChangeParseError> {
    let item = get_full_preamble_item_op(name, preamble, visited)?;
    expand_preamble_item_changes(&item, preamble, visited)
}

pub fn make_compiled_rule_change_op(
    original: &str,
    m: MatchPattern,
    operator: Operator,
    t: TransformPattern,
    c: Option<CompiledConditionExpr>,
) -> Vec<CompiledRuleChange> {
    vec![CompiledRuleChange {
        original_string: original.to_string(),
        match_part: m,
        operator,
        transform_part: t,
        condition: c,
    }]
}

pub fn match_parsed_sound_change_op<F, G>(
    parsed: ParsedSoundChange,
    mut ref_fn: F,
    mut rule_fn: G,
) -> Result<Vec<CompiledRuleChange>, SoundChangeParseError>
where
    F: FnMut(String) -> Result<Vec<CompiledRuleChange>, SoundChangeParseError>,
    G: FnMut(
        Option<ParsedMatchPart>,
        Operator,
        Option<ParsedTransformPart>,
        Option<ConditionExpr>,
    ) -> Result<Vec<CompiledRuleChange>, SoundChangeParseError>,
{
    match parsed {
        ParsedSoundChange::Reference(name) => ref_fn(name),
        ParsedSoundChange::Rule {
            match_part,
            operator,
            transform_part,
            condition,
        } => rule_fn(match_part, operator, transform_part, condition),
    }
}

pub fn expand_references_rec(
    parsed: ParsedSoundChange,
    preamble: &HashMap<String, PreambleItem>,
    original: &str,
    visited: &mut std::collections::HashSet<String>,
) -> Result<Vec<CompiledRuleChange>, SoundChangeParseError> {
    match_parsed_sound_change_op(
        parsed,
        |name| expand_preamble_reference(&name, preamble, visited),
        |match_part, operator, transform_part, condition| {
            let m = resolve_match_part(match_part, preamble)?;
            let t = resolve_transform_part(transform_part, preamble)?;
            let c = resolve_condition_expr(condition, preamble)?;
            Ok(make_compiled_rule_change_op(original, m, operator, t, c))
        },
    )
}

pub fn expand_references(
    parsed: ParsedSoundChange,
    preamble: &HashMap<String, PreambleItem>,
    original: &str,
) -> Result<Vec<CompiledRuleChange>, SoundChangeParseError> {
    let mut visited = std::collections::HashSet::new();
    expand_references_rec(parsed, preamble, original, &mut visited)
}
