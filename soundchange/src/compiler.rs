pub mod validation;

use crate::ast::{
    ConditionExpr, ConditionOp, ConditionPattern, MatchPattern, Operator, ParsedMatchPart,
    ParsedSoundChange, ParsedTransformPart, PreambleItem, PreambleType, SoundChangeRule,
    SoundChanges, TransformElement, TransformPattern,
};
use crate::parser::{SoundChangeParseError, parse_rule_string};
use data::feature::Feature;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct CompiledSoundChangeRule {
    pub name: Option<String>,
    pub changes: Vec<CompiledRuleChange>,
}

#[derive(Debug, Clone)]
pub struct CompiledRuleChange {
    pub original_string: String,
    pub match_part: MatchPattern,
    pub operator: Operator,
    pub transform_part: TransformPattern,
    pub condition: Option<CompiledConditionExpr>,
}

#[derive(Debug, Clone)]
pub enum CompiledConditionExpr {
    Term {
        negated: bool,
        pattern: ConditionPattern,
    },
    Binary {
        left: Box<CompiledConditionExpr>,
        op: ConditionOp,
        right: Box<CompiledConditionExpr>,
    },
}

#[must_use]
pub fn is_distinctive_feature_name(name: &str) -> bool {
    let normalized = name.replace('-', "_").to_lowercase();
    Feature::from_str(&normalized).is_ok()
}

/// Compiles sound changes from the configuration.
///
/// # Errors
/// Returns `SoundChangeParseError` if any sound change or preamble cannot be parsed, resolved, or validated.
pub fn compile_sound_changes(
    config: &SoundChanges,
) -> Result<Vec<(u32, Vec<CompiledSoundChangeRule>)>, SoundChangeParseError> {
    // 1. Build and validate the preamble map
    let preamble_map = build_preamble_map(config)?;

    // 2. Compile era rules
    let mut compiled_eras = Vec::new();
    for era_rules in &config.eras {
        let mut compiled_rules = Vec::new();
        for rule in &era_rules.rules {
            compiled_rules.push(compile_rule(rule, &preamble_map)?);
        }
        compiled_eras.push((era_rules.era, compiled_rules));
    }

    Ok(compiled_eras)
}

fn build_preamble_map(
    config: &SoundChanges,
) -> Result<HashMap<String, PreambleItem>, SoundChangeParseError> {
    let mut map = HashMap::new();
    for item in &config.preamble {
        if is_distinctive_feature_name(&item.name) {
            return Err(SoundChangeParseError::ValidationError(format!(
                "Preamble item name '{}' is a distinctive feature name, which is forbidden.",
                item.name
            )));
        }
        map.insert(item.name.clone(), item.clone());
    }
    Ok(map)
}

fn compile_rule(
    rule: &SoundChangeRule,
    preamble: &HashMap<String, PreambleItem>,
) -> Result<CompiledSoundChangeRule, SoundChangeParseError> {
    if let Some(name) = rule
        .name
        .as_ref()
        .filter(|n| is_distinctive_feature_name(n))
    {
        return Err(SoundChangeParseError::ValidationError(format!(
            "Rule name '{name}' is a distinctive feature name, which is forbidden."
        )));
    }

    let mut compiled_changes = Vec::new();
    for change_str in &rule.changes {
        let parsed = parse_rule_string(change_str)?;
        let expanded = expand_references(parsed, preamble, change_str)?;
        for r in expanded {
            validation::validate_compiled_rule(&r)?;
            compiled_changes.push(r);
        }
    }

    Ok(CompiledSoundChangeRule {
        name: rule.name.clone(),
        changes: compiled_changes,
    })
}

fn expand_references(
    parsed: ParsedSoundChange,
    preamble: &HashMap<String, PreambleItem>,
    original: &str,
) -> Result<Vec<CompiledRuleChange>, SoundChangeParseError> {
    match parsed {
        ParsedSoundChange::Reference(name) => {
            let item = preamble.get(&name).ok_or_else(|| {
                SoundChangeParseError::ReferenceError(format!("Preamble item '{name}' not found"))
            })?;
            if item.r#type != PreambleType::Full {
                return Err(SoundChangeParseError::ReferenceError(format!(
                    "Preamble item '{name}' is not of type 'full'"
                )));
            }
            let mut changes = Vec::new();
            for sub_str in &item.changes {
                let sub_parsed = parse_rule_string(sub_str)?;
                let mut sub_expanded = expand_references(sub_parsed, preamble, sub_str)?;
                changes.append(&mut sub_expanded);
            }
            Ok(changes)
        }
        ParsedSoundChange::Rule {
            match_part,
            operator,
            transform_part,
            condition,
        } => {
            let m = resolve_match_part(match_part, preamble)?;
            let t = resolve_transform_part(transform_part, preamble)?;
            let c = resolve_condition_expr(condition, preamble)?;

            Ok(vec![CompiledRuleChange {
                original_string: original.to_string(),
                match_part: m,
                operator,
                transform_part: t,
                condition: c,
            }])
        }
    }
}

fn resolve_match_part(
    part_opt: Option<ParsedMatchPart>,
    preamble: &HashMap<String, PreambleItem>,
) -> Result<MatchPattern, SoundChangeParseError> {
    let Some(part) = part_opt else {
        return Ok(MatchPattern {
            elements: Vec::new(),
        });
    };
    match part {
        ParsedMatchPart::Pattern(p) => Ok(p),
        ParsedMatchPart::Reference(name) => {
            let item = preamble.get(&name).ok_or_else(|| {
                SoundChangeParseError::ReferenceError(format!("Preamble item '{name}' not found"))
            })?;
            if item.r#type != PreambleType::Match {
                return Err(SoundChangeParseError::ReferenceError(format!(
                    "Preamble item '{name}' is not of type 'match'"
                )));
            }
            let val = item.value.as_ref().ok_or_else(|| {
                SoundChangeParseError::ReferenceError(format!(
                    "Preamble match item '{name}' has no value"
                ))
            })?;
            let parsed = parse_rule_string(&format!("{val} => ∅"))?;
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
    }
}

fn resolve_transform_part(
    part_opt: Option<ParsedTransformPart>,
    preamble: &HashMap<String, PreambleItem>,
) -> Result<TransformPattern, SoundChangeParseError> {
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
        ParsedTransformPart::Reference(name) => {
            let item = preamble.get(&name).ok_or_else(|| {
                SoundChangeParseError::ReferenceError(format!("Preamble item '{name}' not found"))
            })?;
            if item.r#type != PreambleType::Transform {
                return Err(SoundChangeParseError::ReferenceError(format!(
                    "Preamble item '{name}' is not of type 'transform'"
                )));
            }
            let val = item.value.as_ref().ok_or_else(|| {
                SoundChangeParseError::ReferenceError(format!(
                    "Preamble transform item '{name}' has no value"
                ))
            })?;
            let parsed = parse_rule_string(&format!("∅ => {val}"))?;
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
    }
}

fn resolve_condition_reference(
    name: &str,
    preamble: &HashMap<String, PreambleItem>,
) -> Result<Option<CompiledConditionExpr>, SoundChangeParseError> {
    let item = preamble.get(name).ok_or_else(|| {
        SoundChangeParseError::ReferenceError(format!("Preamble item '{name}' not found"))
    })?;
    if item.r#type != PreambleType::Condition {
        return Err(SoundChangeParseError::ReferenceError(format!(
            "Preamble item '{name}' is not of type 'condition'"
        )));
    }
    let val = item.value.as_ref().ok_or_else(|| {
        SoundChangeParseError::ReferenceError(format!(
            "Preamble condition item '{name}' has no value"
        ))
    })?;
    let parsed = parse_rule_string(&format!("∅ => ∅ / {val}"))?;
    if let ParsedSoundChange::Rule {
        condition: Some(c), ..
    } = parsed
    {
        resolve_condition_expr(Some(c), preamble)
    } else {
        Err(SoundChangeParseError::ReferenceError(format!(
            "Failed to parse preamble condition pattern for '{name}'"
        )))
    }
}

fn resolve_condition_expr(
    cond_opt: Option<ConditionExpr>,
    preamble: &HashMap<String, PreambleItem>,
) -> Result<Option<CompiledConditionExpr>, SoundChangeParseError> {
    let Some(cond) = cond_opt else {
        return Ok(None);
    };
    match cond {
        ConditionExpr::Reference(name) => resolve_condition_reference(&name, preamble),
        ConditionExpr::Term { negated, pattern } => {
            Ok(Some(CompiledConditionExpr::Term { negated, pattern }))
        }
        ConditionExpr::Binary { left, op, right } => {
            let l = resolve_condition_expr(Some(*left), preamble)?.ok_or_else(|| {
                SoundChangeParseError::ReferenceError(
                    "Empty left condition binary branch".to_string(),
                )
            })?;
            let r = resolve_condition_expr(Some(*right), preamble)?.ok_or_else(|| {
                SoundChangeParseError::ReferenceError(
                    "Empty right condition binary branch".to_string(),
                )
            })?;
            Ok(Some(CompiledConditionExpr::Binary {
                left: Box::new(l),
                op,
                right: Box::new(r),
            }))
        }
    }
}
