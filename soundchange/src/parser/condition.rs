use crate::ast::{
    ConditionBase, ConditionElement, ConditionExpr, ConditionOp, ConditionPattern, MatchQuantifier,
};
use crate::parser::Rule;
use crate::parser::error::SoundChangeParseError;
use crate::parser::pattern::{convert_base_element, convert_quantifier, convert_reference_rule};
use pest::iterators::Pair;

fn parse_condition_op(op_str: &str) -> Result<ConditionOp, SoundChangeParseError> {
    match op_str {
        "&" => Ok(ConditionOp::And),
        "|" => Ok(ConditionOp::Or),
        _ => Err(SoundChangeParseError::ConversionError(format!(
            "Invalid condition operator: {op_str}"
        ))),
    }
}

pub(crate) fn convert_condition_expr(
    pair: Pair<'_, Rule>,
) -> Result<ConditionExpr, SoundChangeParseError> {
    let mut inner_pairs = pair.into_inner();
    let first = inner_pairs.next().ok_or_else(|| {
        SoundChangeParseError::ConversionError("Empty condition expression".to_string())
    })?;

    if first.as_rule() == Rule::reference_rule {
        let name = convert_reference_rule(first)?;
        return Ok(ConditionExpr::Reference(name));
    }

    let mut current = convert_condition_term(first)?;

    while let Some(op_pair) = inner_pairs.next() {
        let op = parse_condition_op(op_pair.as_str())?;
        let next_term_pair = inner_pairs.next().ok_or_else(|| {
            SoundChangeParseError::ConversionError(
                "Condition expression ended unexpectedly".to_string(),
            )
        })?;
        let next_term = convert_condition_term(next_term_pair)?;
        current = ConditionExpr::Binary {
            left: Box::new(current),
            op,
            right: Box::new(next_term),
        };
    }

    Ok(current)
}

pub(crate) fn convert_condition_term(
    pair: Pair<'_, Rule>,
) -> Result<ConditionExpr, SoundChangeParseError> {
    let mut negated = false;
    let mut pattern = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::negated_condition => {
                negated = true;
            }
            Rule::condition_pattern => {
                pattern = Some(convert_condition_pattern(inner)?);
            }
            _ => {}
        }
    }

    let pattern = pattern.ok_or_else(|| {
        SoundChangeParseError::ConversionError("Condition term missing pattern".to_string())
    })?;

    Ok(ConditionExpr::Term { negated, pattern })
}

fn push_pending_element(
    elements: &mut Vec<ConditionElement>,
    base_opt: &mut Option<ConditionBase>,
    quantifier: &mut MatchQuantifier,
) {
    if let Some(base) = base_opt.take() {
        elements.push(ConditionElement {
            base,
            quantifier: std::mem::replace(quantifier, MatchQuantifier::None),
        });
    }
}

pub(crate) fn convert_condition_pattern(
    pair: Pair<'_, Rule>,
) -> Result<ConditionPattern, SoundChangeParseError> {
    let mut elements = Vec::new();
    let mut current_base = None;
    let mut current_quantifier = MatchQuantifier::None;

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::quantifier {
            current_quantifier = convert_quantifier(inner)?;
            push_pending_element(&mut elements, &mut current_base, &mut current_quantifier);
        } else {
            // If we have a pending element, push it before starting a new one
            push_pending_element(&mut elements, &mut current_base, &mut current_quantifier);

            if inner.as_rule() == Rule::match_placeholder || inner.as_str() == "_" {
                current_base = Some(ConditionBase::MatchPlaceholder);
            } else {
                let base_rule = inner.as_rule();
                let match_base = convert_base_element(inner, base_rule)?;
                current_base = Some(ConditionBase::Element(match_base));
            }
        }
    }

    push_pending_element(&mut elements, &mut current_base, &mut current_quantifier);

    Ok(ConditionPattern { elements })
}
