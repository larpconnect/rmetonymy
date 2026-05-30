use crate::ast::{ConditionBase, MatchBase, MatchPattern, Operator, TransformElement};
use crate::compiler::{CompiledConditionExpr, CompiledRuleChange};
use crate::parser::SoundChangeParseError;
use std::collections::HashSet;

fn validate_null_match(rule: &CompiledRuleChange) -> Result<(), SoundChangeParseError> {
    if rule.match_part.elements.is_empty() && rule.condition.is_none() {
        return Err(SoundChangeParseError::ValidationError(format!(
            "Null match (∅) in '{}' requires at least one condition.",
            rule.original_string
        )));
    }
    Ok(())
}

fn validate_opaque_single_use(rule: &CompiledRuleChange) -> Result<(), SoundChangeParseError> {
    if matches!(
        rule.operator,
        Operator::RightSingleTransparent | Operator::LeftSingleTransparent
    ) && (rule.original_string.contains("-:>") || rule.original_string.contains("<-:"))
    {
        return Err(SoundChangeParseError::ValidationError(format!(
            "Opaque modifier (:) cannot be used with a single-change operator in '{}'.",
            rule.original_string
        )));
    }
    Ok(())
}

pub(crate) fn validate_compiled_rule(
    rule: &CompiledRuleChange,
) -> Result<(), SoundChangeParseError> {
    validate_transform_bindings(rule)?;
    validate_condition_wrapper(rule)?;
    validate_alpha_variables(rule)?;
    validate_null_match(rule)?;
    validate_opaque_single_use(rule)?;
    Ok(())
}

fn validate_condition_wrapper(rule: &CompiledRuleChange) -> Result<(), SoundChangeParseError> {
    rule.condition
        .as_ref()
        .map_or(Ok(()), validate_condition_has_placeholder)
}

pub(crate) fn get_match_markers(pattern: &MatchPattern) -> HashSet<u8> {
    get_match_markers_op(pattern, collect_base_markers_op)
}

fn get_match_markers_op<F>(pattern: &MatchPattern, mut collect_fn: F) -> HashSet<u8>
where
    F: FnMut(&MatchBase) -> HashSet<u8>,
{
    let mut markers = HashSet::new();
    for el in &pattern.elements {
        markers.extend(collect_fn(&el.base));
    }
    markers
}

fn collect_base_markers_op(base: &MatchBase) -> HashSet<u8> {
    let mut markers = HashSet::new();
    let mut stack = vec![base];
    while let Some(current) = stack.pop() {
        match current {
            MatchBase::SoundClass {
                marker: Some(m), ..
            }
            | MatchBase::SetExclusion {
                marker: Some(m), ..
            } => {
                markers.insert(*m);
            }
            MatchBase::FeatureClass {
                key_opt: Some(key), ..
            } => {
                if let Some(m) = key.marker {
                    markers.insert(m);
                }
            }
            MatchBase::Set(elements) => {
                for el in elements {
                    stack.push(el);
                }
            }
            MatchBase::OptionalGroup(pattern) => {
                for el in &pattern.elements {
                    stack.push(&el.base);
                }
            }
            _ => {}
        }
    }
    markers
}

fn validate_transform_bindings(rule: &CompiledRuleChange) -> Result<(), SoundChangeParseError> {
    let match_markers = get_match_markers(&rule.match_part);
    validate_transform_bindings_op(rule, &match_markers)
}

fn validate_transform_bindings_op(
    rule: &CompiledRuleChange,
    match_markers: &HashSet<u8>,
) -> Result<(), SoundChangeParseError> {
    for el in &rule.transform_part.elements {
        if let TransformElement::Ref {
            marker,
            class_key,
            repeat: _,
            ..
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

pub(crate) fn validate_condition_has_placeholder(
    cond: &CompiledConditionExpr,
) -> Result<(), SoundChangeParseError> {
    let leftmost = find_leftmost_condition(cond);
    if !has_match_placeholder(leftmost) {
        return Err(SoundChangeParseError::ValidationError(
            "No use of the match (_) in the first conditional".to_string(),
        ));
    }
    Ok(())
}

fn find_leftmost_condition(cond: &CompiledConditionExpr) -> &CompiledConditionExpr {
    match cond {
        CompiledConditionExpr::Binary { left, .. } => find_leftmost_condition(left),
        CompiledConditionExpr::Term { .. } => cond,
    }
}

fn has_match_placeholder(cond: &CompiledConditionExpr) -> bool {
    match cond {
        CompiledConditionExpr::Term { pattern, .. } => pattern
            .elements
            .iter()
            .any(|el| matches!(el.base, ConditionBase::MatchPlaceholder)),
        CompiledConditionExpr::Binary { .. } => false,
    }
}

enum AlphaNode<'a> {
    Base(&'a MatchBase),
    Condition(&'a CompiledConditionExpr),
}

fn process_alpha_base_op<'a>(
    base: &'a MatchBase,
    stack: &mut Vec<AlphaNode<'a>>,
    alphas: &mut HashSet<String>,
) {
    match base {
        MatchBase::FeatureClass { features, .. } => {
            for f in features {
                if let Some(ref alpha) = f.alpha {
                    alphas.insert(alpha.name.clone());
                }
            }
        }
        MatchBase::Set(elements) => {
            for el in elements {
                stack.push(AlphaNode::Base(el));
            }
        }
        MatchBase::OptionalGroup(pattern) => {
            for el in &pattern.elements {
                stack.push(AlphaNode::Base(&el.base));
            }
        }
        _ => {}
    }
}

fn process_alpha_condition_op<'a>(cond: &'a CompiledConditionExpr, stack: &mut Vec<AlphaNode<'a>>) {
    match cond {
        CompiledConditionExpr::Term { pattern, .. } => {
            for el in &pattern.elements {
                if let ConditionBase::Element(ref base) = el.base {
                    stack.push(AlphaNode::Base(base));
                }
            }
        }
        CompiledConditionExpr::Binary { left, right, .. } => {
            stack.push(AlphaNode::Condition(left));
            stack.push(AlphaNode::Condition(right));
        }
    }
}

fn collect_alphas_op(rule: &CompiledRuleChange) -> HashSet<String> {
    let mut alphas = HashSet::new();
    let mut stack = Vec::new();

    for el in &rule.match_part.elements {
        stack.push(AlphaNode::Base(&el.base));
    }
    if let Some(ref cond) = rule.condition {
        stack.push(AlphaNode::Condition(cond));
    }

    while let Some(node) = stack.pop() {
        match node {
            AlphaNode::Base(base) => process_alpha_base_op(base, &mut stack, &mut alphas),
            AlphaNode::Condition(cond) => process_alpha_condition_op(cond, &mut stack),
        }
    }
    alphas
}

fn get_captured_alpha_variables(rule: &CompiledRuleChange) -> HashSet<String> {
    collect_alphas_op(rule)
}

fn validate_alpha_variables(rule: &CompiledRuleChange) -> Result<(), SoundChangeParseError> {
    let captured = get_captured_alpha_variables(rule);
    validate_transform_alphas_op(rule, &captured)
}

fn validate_transform_alphas_op(
    rule: &CompiledRuleChange,
    captured: &HashSet<String>,
) -> Result<(), SoundChangeParseError> {
    for el in &rule.transform_part.elements {
        if let TransformElement::Ref {
            feature_changes, ..
        } = el
        {
            for fc in feature_changes {
                if let Some(alpha) = fc.alpha.as_ref().filter(|a| !captured.contains(&a.name)) {
                    return Err(SoundChangeParseError::ValidationError(format!(
                        "Alpha variable '{}' used in transform but never captured in '{}'.",
                        alpha.name, rule.original_string
                    )));
                }
            }
        }
    }
    Ok(())
}
