pub mod validation;
pub mod resolver;
pub mod cond_resolver;

use crate::ast::{
    PreambleItem, SoundChangeRule, SoundChanges,
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
    pub match_part: crate::ast::MatchPattern,
    pub operator: crate::ast::Operator,
    pub transform_part: crate::ast::TransformPattern,
    pub condition: Option<CompiledConditionExpr>,
}

#[derive(Debug, Clone)]
pub enum CompiledConditionExpr {
    Term {
        negated: bool,
        pattern: crate::ast::ConditionPattern,
    },
    Binary {
        left: Box<CompiledConditionExpr>,
        op: crate::ast::ConditionOp,
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
    let preamble_map = build_preamble_map(config)?;
    compile_eras_op(&config.eras, |rule| compile_rule(rule, &preamble_map))
}

fn compile_eras_op<F>(
    eras: &[crate::ast::EraRules],
    mut compile_rule_fn: F,
) -> Result<Vec<(u32, Vec<CompiledSoundChangeRule>)>, SoundChangeParseError>
where
    F: FnMut(&SoundChangeRule) -> Result<CompiledSoundChangeRule, SoundChangeParseError>,
{
    let mut compiled_eras = Vec::new();
    for era_rules in eras {
        let mut compiled_rules = Vec::new();
        for rule in &era_rules.rules {
            compiled_rules.push(compile_rule_fn(rule)?);
        }
        compiled_eras.push((era_rules.era, compiled_rules));
    }
    Ok(compiled_eras)
}

/// Compiles a single sound change rule from a string, using the given preamble configuration if present.
///
/// # Errors
/// Returns `SoundChangeParseError` if compilation, parsing or validation fails.
pub fn compile_single_rule_from_str(
    rule_str: &str,
    sound_changes: Option<&SoundChanges>,
) -> Result<CompiledSoundChangeRule, SoundChangeParseError> {
    let preamble_map = build_preamble_for_single_rule(sound_changes)?;
    let rule = make_single_rule_op(rule_str);
    compile_rule(&rule, &preamble_map)
}

fn build_preamble_for_single_rule(
    sound_changes: Option<&SoundChanges>,
) -> Result<HashMap<String, PreambleItem>, SoundChangeParseError> {
    sound_changes
        .map(build_preamble_map)
        .unwrap_or_else(|| Ok(HashMap::new()))
}

fn make_single_rule_op(rule_str: &str) -> SoundChangeRule {
    SoundChangeRule {
        name: None,
        changes: vec![rule_str.to_string()],
    }
}

fn build_preamble_map(
    config: &SoundChanges,
) -> Result<HashMap<String, PreambleItem>, SoundChangeParseError> {
    build_preamble_map_op(config, is_distinctive_feature_name)
}

fn build_preamble_map_op<F>(
    config: &SoundChanges,
    mut check_fn: F,
) -> Result<HashMap<String, PreambleItem>, SoundChangeParseError>
where
    F: FnMut(&str) -> bool,
{
    let mut map = HashMap::new();
    for item in &config.preamble {
        if check_fn(&item.name) {
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
    validate_rule_name(rule)?;
    let compiled_changes = compile_changes(rule, preamble)?;
    Ok(CompiledSoundChangeRule {
        name: rule.name.clone(),
        changes: compiled_changes,
    })
}

fn validate_rule_name(rule: &SoundChangeRule) -> Result<(), SoundChangeParseError> {
    validate_rule_name_op(rule, is_distinctive_feature_name)
}

fn validate_rule_name_op<F>(rule: &SoundChangeRule, mut check_fn: F) -> Result<(), SoundChangeParseError>
where
    F: FnMut(&str) -> bool,
{
    if let Some(name) = rule.name.as_ref().filter(|n| check_fn(n)) {
        return Err(SoundChangeParseError::ValidationError(format!(
            "Rule name '{name}' is a distinctive feature name, which is forbidden."
        )));
    }
    Ok(())
}

fn compile_changes(
    rule: &SoundChangeRule,
    preamble: &HashMap<String, PreambleItem>,
) -> Result<Vec<CompiledRuleChange>, SoundChangeParseError> {
    rule.changes
        .iter()
        .map(|change_str| compile_change(change_str, preamble))
        .collect::<Result<Vec<Vec<CompiledRuleChange>>, _>>()
        .map(|v| v.into_iter().flatten().collect())
}

fn compile_change(
    change_str: &str,
    preamble: &HashMap<String, PreambleItem>,
) -> Result<Vec<CompiledRuleChange>, SoundChangeParseError> {
    let parsed = parse_rule_string(change_str)?;
    let expanded = resolver::expand_references(parsed, preamble, change_str)?;
    validate_compiled_changes(&expanded)?;
    Ok(expanded)
}

fn validate_compiled_changes(
    changes: &[CompiledRuleChange],
) -> Result<(), SoundChangeParseError> {
    changes
        .iter()
        .try_for_each(validation::validate_compiled_rule)
}
