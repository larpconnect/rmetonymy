use super::parser::{OrthoTransformElement, ParsedOrthoRule};
use crate::ast::{ConditionExpr, MatchPattern, Operator};
use crate::compiler::CompiledConditionExpr;
use crate::parser::error::SoundChangeParseError;

#[derive(Debug, Clone)]
pub struct CompiledOrthoRule {
    pub original_string: String,
    pub match_part: MatchPattern,
    pub operator: Operator,
    pub transform_part: Vec<OrthoTransformElement>,
    pub condition: Option<CompiledConditionExpr>,
}

/// Compiles a set of orthography rules.
///
/// # Errors
/// Returns an error if any rule fails to parse or validate.
pub fn compile_ortho_rules(
    rules: &[String],
) -> Result<Vec<CompiledOrthoRule>, SoundChangeParseError> {
    let mut compiled = Vec::new();
    for rule_str in rules {
        let parsed = super::parser::parse_ortho_rule(rule_str)?;
        let compiled_rule = compile_ortho_rule(parsed)?;
        compiled.push(compiled_rule);
    }
    Ok(compiled)
}

fn compile_ortho_rule(parsed: ParsedOrthoRule) -> Result<CompiledOrthoRule, SoundChangeParseError> {
    let cond = parsed.condition.map(resolve_ortho_condition).transpose()?;
    let compiled = CompiledOrthoRule {
        original_string: parsed.original_string,
        match_part: parsed.match_part,
        operator: parsed.operator,
        transform_part: parsed.transform_part,
        condition: cond,
    };
    validate_compiled_ortho_rule(&compiled)?;
    Ok(compiled)
}

fn resolve_ortho_condition(
    cond: ConditionExpr,
) -> Result<CompiledConditionExpr, SoundChangeParseError> {
    match cond {
        ConditionExpr::Reference(name) => Err(SoundChangeParseError::ReferenceError(format!(
            "Preamble reference '{name}' is not supported in orthography conditions"
        ))),
        ConditionExpr::Term { negated, pattern } => {
            Ok(CompiledConditionExpr::Term { negated, pattern })
        }
        ConditionExpr::Binary { left, op, right } => {
            let l = resolve_ortho_condition(*left)?;
            let r = resolve_ortho_condition(*right)?;
            Ok(CompiledConditionExpr::Binary {
                left: Box::new(l),
                op,
                right: Box::new(r),
            })
        }
    }
}

fn validate_compiled_ortho_rule(rule: &CompiledOrthoRule) -> Result<(), SoundChangeParseError> {
    validate_ortho_transform_bindings(rule)?;

    if let Some(ref cond) = rule.condition {
        crate::compiler::validation::validate_condition_has_placeholder(cond)?;
    }

    validate_ortho_rule_structure(rule)?;
    validate_ortho_operator_restrictions(rule)?;

    Ok(())
}

fn validate_ortho_rule_structure(rule: &CompiledOrthoRule) -> Result<(), SoundChangeParseError> {
    if rule.match_part.elements.is_empty() && rule.condition.is_none() {
        return Err(SoundChangeParseError::ValidationError(format!(
            "Null match (∅) in '{}' requires at least one condition.",
            rule.original_string
        )));
    }
    Ok(())
}

fn validate_ortho_operator_restrictions(
    rule: &CompiledOrthoRule,
) -> Result<(), SoundChangeParseError> {
    let is_single = matches!(
        rule.operator,
        Operator::RightSingleTransparent | Operator::LeftSingleTransparent
    );
    if is_single && (rule.original_string.contains("-:>") || rule.original_string.contains("<-:")) {
        return Err(SoundChangeParseError::ValidationError(format!(
            "Opaque modifier (:) cannot be used with a single-change operator in '{}'.",
            rule.original_string
        )));
    }
    Ok(())
}

fn validate_ortho_transform_bindings(
    rule: &CompiledOrthoRule,
) -> Result<(), SoundChangeParseError> {
    let match_markers = crate::compiler::validation::get_match_markers(&rule.match_part);
    for el in &rule.transform_part {
        if let OrthoTransformElement::Ref {
            marker, class_key, ..
        } = el
        {
            if class_key.is_some() && marker.is_none() {
                return Err(SoundChangeParseError::ValidationError(format!(
                    "Unbound sound class in transform of '{}': all sound classes must have markers.",
                    rule.original_string
                )));
            }
            if let Some(m) = marker.filter(|m| !match_markers.contains(m)) {
                return Err(SoundChangeParseError::ValidationError(format!(
                    "Transform refers to marker '{m}' which is not bound in the match of '{}'.",
                    rule.original_string
                )));
            }
        }
    }
    Ok(())
}
