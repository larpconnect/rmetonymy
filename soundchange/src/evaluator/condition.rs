use crate::ast::{ConditionBase, ConditionElement, ConditionPattern};
use crate::compiler::CompiledConditionExpr;
use crate::evaluator::condition_match::{
    evaluate_match_elements_condition, evaluate_match_pattern_condition,
};
use crate::evaluator::{EvalContext, MatchState, WorkingWord};

pub(crate) fn evaluate_conditions(
    cond_opt: Option<&CompiledConditionExpr>,
    word: &WorkingWord,
    match_range: &std::ops::Range<usize>,
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Option<MatchState> {
    let Some(cond) = cond_opt else {
        return Some(state.clone());
    };
    let ectx = ConditionEvalContext {
        word,
        match_range,
        ctx,
    };
    evaluate_condition_expr(cond, state, &ectx)
}

pub(crate) struct ConditionEvalContext<'a, 'b> {
    word: &'a WorkingWord,
    match_range: &'a std::ops::Range<usize>,
    ctx: &'a EvalContext<'b>,
}

fn evaluate_term_condition(
    negated: bool,
    pattern: &ConditionPattern,
    state: &MatchState,
    ectx: &ConditionEvalContext<'_, '_>,
) -> Option<MatchState> {
    let has_placeholder = has_placeholder_op(pattern);
    let opt_state = get_term_state(has_placeholder, pattern, state, ectx);
    apply_negation_op(negated, opt_state, state)
}

fn has_placeholder_op(pattern: &ConditionPattern) -> bool {
    pattern
        .elements
        .iter()
        .any(|el| matches!(el.base, ConditionBase::MatchPlaceholder))
}

fn get_term_state(
    has_placeholder: bool,
    pattern: &ConditionPattern,
    state: &MatchState,
    ectx: &ConditionEvalContext<'_, '_>,
) -> Option<MatchState> {
    get_term_state_op(
        has_placeholder,
        || evaluate_condition_with_placeholder(pattern, state, ectx),
        || evaluate_condition_no_placeholder(pattern, state, ectx),
    )
}

fn get_term_state_op<F, G>(
    has_placeholder: bool,
    mut with_p: F,
    mut without_p: G,
) -> Option<MatchState>
where
    F: FnMut() -> Option<MatchState>,
    G: FnMut() -> Option<MatchState>,
{
    if has_placeholder {
        with_p()
    } else {
        without_p()
    }
}

fn evaluate_condition_no_placeholder(
    pattern: &ConditionPattern,
    state: &MatchState,
    ectx: &ConditionEvalContext<'_, '_>,
) -> Option<MatchState> {
    (0..=ectx.word.phonemes.len())
        .find_map(|idx| evaluate_match_pattern_condition(pattern, ectx.word, idx, state, ectx.ctx))
}

fn apply_negation_op(
    negated: bool,
    opt_state: Option<MatchState>,
    state: &MatchState,
) -> Option<MatchState> {
    if negated {
        if opt_state.is_some() {
            None
        } else {
            Some(state.clone())
        }
    } else {
        opt_state
    }
}

fn evaluate_binary_condition(
    left: &CompiledConditionExpr,
    op: crate::ast::ConditionOp,
    right: &CompiledConditionExpr,
    state: &MatchState,
    ectx: &ConditionEvalContext<'_, '_>,
) -> Option<MatchState> {
    let left_res = evaluate_condition_expr(left, state, ectx);
    evaluate_binary_condition_op(left_res, op, right, (state, ectx), evaluate_condition_expr)
}

fn evaluate_binary_condition_op<F>(
    left_res: Option<MatchState>,
    op: crate::ast::ConditionOp,
    right: &CompiledConditionExpr,
    state_ctx: (&MatchState, &ConditionEvalContext<'_, '_>),
    mut eval_fn: F,
) -> Option<MatchState>
where
    F: FnMut(&CompiledConditionExpr, &MatchState, &ConditionEvalContext<'_, '_>) -> Option<MatchState>,
{
    let (state, ectx) = state_ctx;
    match op {
        crate::ast::ConditionOp::And => {
            if let Some(left_state) = left_res {
                eval_fn(right, &left_state, ectx)
            } else {
                None
            }
        }
        crate::ast::ConditionOp::Or => {
            if let Some(left_state) = left_res {
                Some(left_state)
            } else {
                eval_fn(right, state, ectx)
            }
        }
    }
}

pub(crate) fn evaluate_condition_expr(
    cond: &CompiledConditionExpr,
    state: &MatchState,
    ectx: &ConditionEvalContext<'_, '_>,
) -> Option<MatchState> {
    match cond {
        CompiledConditionExpr::Term { negated, pattern } => {
            evaluate_term_condition(*negated, pattern, state, ectx)
        }
        CompiledConditionExpr::Binary { left, op, right } => {
            evaluate_binary_condition(left, *op, right, state, ectx)
        }
    }
}

pub(crate) fn evaluate_condition_with_placeholder(
    pattern: &ConditionPattern,
    state: &MatchState,
    ectx: &ConditionEvalContext<'_, '_>,
) -> Option<MatchState> {
    let (left, right) = split_around_placeholder_op(pattern);
    evaluate_condition_with_placeholder_integration(&left, &right, ectx, state)
}

fn split_around_placeholder_op(
    pattern: &ConditionPattern,
) -> (Vec<ConditionElement>, Vec<ConditionElement>) {
    let placeholder_idx = pattern
        .elements
        .iter()
        .position(|el| matches!(el.base, ConditionBase::MatchPlaceholder))
        .unwrap_or(0);
    let left = pattern.elements.get(0..placeholder_idx).unwrap_or(&[]).to_vec();
    let right = pattern.elements.get(placeholder_idx + 1..).unwrap_or(&[]).to_vec();
    (left, right)
}

fn evaluate_condition_with_placeholder_integration(
    left: &[ConditionElement],
    right: &[ConditionElement],
    ectx: &ConditionEvalContext<'_, '_>,
    state: &MatchState,
) -> Option<MatchState> {
    let left_state = matches_ending_at(left, ectx.word, ectx.match_range.start, state, ectx.ctx)?;
    let right_res = evaluate_match_elements_condition(right, ectx.word, ectx.match_range.end, &left_state, ectx.ctx);
    extract_first_state_op(right_res)
}

fn extract_first_state_op(res: Vec<(usize, MatchState)>) -> Option<MatchState> {
    res.into_iter().next().map(|(_, s)| s)
}

pub(crate) fn matches_ending_at(
    elements: &[ConditionElement],
    word: &WorkingWord,
    end_idx: usize,
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Option<MatchState> {
    if elements.is_empty() {
        return Some(state.clone());
    }
    (0..=end_idx).find_map(|start| {
        let sub_res = evaluate_match_elements_condition(elements, word, start, state, ctx);
        find_matching_length_op(sub_res, start, end_idx)
    })
}

fn find_matching_length_op(
    res: Vec<(usize, MatchState)>,
    start: usize,
    end_idx: usize,
) -> Option<MatchState> {
    for (len, ns) in res {
        if start + len == end_idx {
            return Some(ns);
        }
    }
    None
}
