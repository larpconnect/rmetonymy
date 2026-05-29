use crate::ast::{
    ConditionExpr, ConditionOp, PreambleItem, PreambleType,
};
use crate::compiler::CompiledConditionExpr;
use crate::compiler::resolver::get_preamble_value_op;
use crate::parser::{SoundChangeParseError, parse_rule_string};
use std::collections::HashMap;

pub fn extract_condition_op(
    parsed: crate::ast::ParsedSoundChange,
    name: &str,
) -> Result<Option<ConditionExpr>, SoundChangeParseError> {
    if let crate::ast::ParsedSoundChange::Rule { condition, .. } = parsed {
        Ok(condition)
    } else {
        Err(SoundChangeParseError::ReferenceError(format!(
            "Failed to parse preamble condition pattern for '{name}'"
        )))
    }
}

pub fn resolve_condition_from_parsed(
    parsed: crate::ast::ParsedSoundChange,
    name: &str,
    preamble: &HashMap<String, PreambleItem>,
) -> Result<Option<CompiledConditionExpr>, SoundChangeParseError> {
    let cond = extract_condition_op(parsed, name)?;
    resolve_condition_expr(cond, preamble)
}

pub fn resolve_condition_reference(
    name: &str,
    preamble: &HashMap<String, PreambleItem>,
) -> Result<Option<CompiledConditionExpr>, SoundChangeParseError> {
    let val = get_preamble_value_op(name, preamble, PreambleType::Condition)?;
    let parsed = parse_rule_string(&format!("∅ => ∅ / {val}"))?;
    resolve_condition_from_parsed(parsed, name, preamble)
}

pub fn match_condition_expr_op<F, G>(
    cond_opt: Option<ConditionExpr>,
    mut ref_fn: F,
    mut binary_fn: G,
) -> Result<Option<CompiledConditionExpr>, SoundChangeParseError>
where
    F: FnMut(String) -> Result<Option<CompiledConditionExpr>, SoundChangeParseError>,
    G: FnMut(
        ConditionExpr,
        ConditionOp,
        ConditionExpr,
    ) -> Result<Option<CompiledConditionExpr>, SoundChangeParseError>,
{
    let Some(cond) = cond_opt else {
        return Ok(None);
    };
    match cond {
        ConditionExpr::Reference(name) => ref_fn(name),
        ConditionExpr::Term { negated, pattern } => {
            Ok(Some(CompiledConditionExpr::Term { negated, pattern }))
        }
        ConditionExpr::Binary { left, op, right } => binary_fn(*left, op, *right),
    }
}

pub fn resolve_condition_expr(
    cond_opt: Option<ConditionExpr>,
    preamble: &HashMap<String, PreambleItem>,
) -> Result<Option<CompiledConditionExpr>, SoundChangeParseError> {
    match_condition_expr_op(
        cond_opt,
        |name| resolve_condition_reference(&name, preamble),
        |left, op, right| {
            let l = resolve_condition_expr(Some(left), preamble)?.ok_or_else(|| {
                SoundChangeParseError::ReferenceError(
                    "Empty left condition binary branch".to_string(),
                )
            })?;
            let r = resolve_condition_expr(Some(right), preamble)?.ok_or_else(|| {
                SoundChangeParseError::ReferenceError(
                    "Empty right condition binary branch".to_string(),
                )
            })?;
            Ok(Some(CompiledConditionExpr::Binary {
                left: Box::new(l),
                op,
                right: Box::new(r),
            }))
        },
    )
}
