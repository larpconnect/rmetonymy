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
    // 1. Unbound sound classes in transform
    validate_transform_bindings(rule)?;

    // 2. First condition has no `_`
    if let Some(ref cond) = rule.condition {
        validate_condition_has_placeholder(cond)?;
    }

    // 3. Referenced alpha variables in transform are captured in match/condition
    validate_alpha_variables(rule)?;

    // 4. Null match requires a condition
    validate_null_match(rule)?;

    // 5. Operator single-use cannot be opaque
    validate_opaque_single_use(rule)?;

    Ok(())
}

fn get_match_markers(pattern: &MatchPattern) -> HashSet<u8> {
    let mut markers = HashSet::new();
    for el in &pattern.elements {
        collect_base_markers(&el.base, &mut markers);
    }
    markers
}

fn collect_base_markers(base: &MatchBase, markers: &mut HashSet<u8>) {
    match base {
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
                collect_base_markers(el, markers);
            }
        }
        MatchBase::OptionalGroup(pattern) => {
            for el in &pattern.elements {
                collect_base_markers(&el.base, markers);
            }
        }
        _ => {}
    }
}

fn validate_transform_bindings(rule: &CompiledRuleChange) -> Result<(), SoundChangeParseError> {
    let match_markers = get_match_markers(&rule.match_part);
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

fn validate_condition_has_placeholder(
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

fn get_captured_alpha_variables(rule: &CompiledRuleChange) -> HashSet<String> {
    let mut alphas = HashSet::new();
    for el in &rule.match_part.elements {
        collect_base_alphas(&el.base, &mut alphas);
    }
    if let Some(ref cond) = rule.condition {
        collect_condition_alphas(cond, &mut alphas);
    }
    alphas
}

fn collect_base_alphas(base: &MatchBase, alphas: &mut HashSet<String>) {
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
                collect_base_alphas(el, alphas);
            }
        }
        MatchBase::OptionalGroup(pattern) => {
            for el in &pattern.elements {
                collect_base_alphas(&el.base, alphas);
            }
        }
        _ => {}
    }
}

fn collect_condition_alphas(cond: &CompiledConditionExpr, alphas: &mut HashSet<String>) {
    match cond {
        CompiledConditionExpr::Term { pattern, .. } => {
            for el in &pattern.elements {
                if let ConditionBase::Element(ref base) = el.base {
                    collect_base_alphas(base, alphas);
                }
            }
        }
        CompiledConditionExpr::Binary { left, right, .. } => {
            collect_condition_alphas(left, alphas);
            collect_condition_alphas(right, alphas);
        }
    }
}

fn validate_alpha_variables(rule: &CompiledRuleChange) -> Result<(), SoundChangeParseError> {
    let captured = get_captured_alpha_variables(rule);
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
