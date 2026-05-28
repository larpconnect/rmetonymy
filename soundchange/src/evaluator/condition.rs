use crate::ast::{ConditionBase, ConditionElement, ConditionPattern, MatchBase, MatchQuantifier};
use crate::compiler::CompiledConditionExpr;
use crate::evaluator::match_base::match_base;
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
    evaluate_condition_expr(cond, word, match_range, state, ctx)
}

fn evaluate_term_condition(
    negated: bool,
    pattern: &ConditionPattern,
    word: &WorkingWord,
    match_range: &std::ops::Range<usize>,
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Option<MatchState> {
    let has_placeholder = pattern
        .elements
        .iter()
        .any(|el| matches!(el.base, ConditionBase::MatchPlaceholder));
    let opt_state = if has_placeholder {
        evaluate_condition_with_placeholder(pattern, word, match_range, state, ctx)
    } else {
        let mut matched_state = None;
        for idx in 0..=word.phonemes.len() {
            if let Some(s) = evaluate_match_pattern_condition(pattern, word, idx, state, ctx) {
                matched_state = Some(s);
                break;
            }
        }
        matched_state
    };
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
    word: &WorkingWord,
    match_range: &std::ops::Range<usize>,
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Option<MatchState> {
    let left_res = evaluate_condition_expr(left, word, match_range, state, ctx);
    match op {
        crate::ast::ConditionOp::And => {
            if let Some(left_state) = left_res {
                evaluate_condition_expr(right, word, match_range, &left_state, ctx)
            } else {
                None
            }
        }
        crate::ast::ConditionOp::Or => {
            if let Some(left_state) = left_res {
                Some(left_state)
            } else {
                evaluate_condition_expr(right, word, match_range, state, ctx)
            }
        }
    }
}

pub(crate) fn evaluate_condition_expr(
    cond: &CompiledConditionExpr,
    word: &WorkingWord,
    match_range: &std::ops::Range<usize>,
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Option<MatchState> {
    match cond {
        CompiledConditionExpr::Term { negated, pattern } => {
            evaluate_term_condition(*negated, pattern, word, match_range, state, ctx)
        }
        CompiledConditionExpr::Binary { left, op, right } => {
            evaluate_binary_condition(left, *op, right, word, match_range, state, ctx)
        }
    }
}

pub(crate) fn evaluate_condition_with_placeholder(
    pattern: &ConditionPattern,
    word: &WorkingWord,
    match_range: &std::ops::Range<usize>,
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Option<MatchState> {
    let placeholder_idx = pattern
        .elements
        .iter()
        .position(|el| matches!(el.base, ConditionBase::MatchPlaceholder))
        .unwrap_or(0);

    let left_elements = pattern.elements.get(0..placeholder_idx).unwrap_or(&[]);
    let right_elements = pattern.elements.get(placeholder_idx + 1..).unwrap_or(&[]);

    let left_state = matches_ending_at(left_elements, word, match_range.start, state, ctx)?;
    let right_res =
        evaluate_match_elements_condition(right_elements, word, match_range.end, &left_state, ctx)
            .into_iter()
            .next()?;
    Some(right_res.1)
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
    for start in 0..=end_idx {
        let sub_res = evaluate_match_elements_condition(elements, word, start, state, ctx);
        for (len, ns) in sub_res {
            if start + len == end_idx {
                return Some(ns);
            }
        }
    }
    None
}

pub(crate) fn evaluate_match_pattern_condition(
    pattern: &ConditionPattern,
    word: &WorkingWord,
    word_idx: usize,
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Option<MatchState> {
    evaluate_match_elements_condition(&pattern.elements, word, word_idx, state, ctx)
        .into_iter()
        .map(|(_, s)| s)
        .next()
}

pub(crate) fn evaluate_match_elements_condition(
    elements: &[ConditionElement],
    word: &WorkingWord,
    word_idx: usize,
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Vec<(usize, MatchState)> {
    if elements.is_empty() {
        return vec![(0, state.clone())];
    }
    let Some((el, rest)) = elements.split_first() else {
        return vec![];
    };

    let mut base_opts = Vec::new();
    match &el.base {
        ConditionBase::MatchPlaceholder => {
            base_opts.push((0, state.clone()));
        }
        ConditionBase::Element(base) => {
            let mut element_lengths =
                get_match_element_lengths_condition(el, base, word, word_idx, state, ctx);
            base_opts.append(&mut element_lengths);
        }
    }

    let mut results = Vec::new();
    for (len, next_state) in base_opts {
        let next_idx = word_idx + len;
        if next_idx <= word.phonemes.len() {
            let sub_res = evaluate_match_elements_condition(rest, word, next_idx, &next_state, ctx);
            for (sub_len, final_state) in sub_res {
                results.push((len + sub_len, final_state));
            }
        }
    }
    results
}

pub(crate) fn get_match_element_lengths_condition(
    el: &ConditionElement,
    base: &MatchBase,
    word: &WorkingWord,
    word_idx: usize,
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Vec<(usize, MatchState)> {
    let mut results = Vec::new();
    let bounds = match &el.quantifier {
        MatchQuantifier::None => None,
        MatchQuantifier::ZeroOrMore => Some((0, usize::MAX)),
        MatchQuantifier::OneOrMore => Some((1, usize::MAX)),
        MatchQuantifier::ZeroOrMoreBounded(limit) => Some((0, *limit as usize)),
        MatchQuantifier::OneOrMoreBounded(limit) => Some((1, *limit as usize)),
    };

    if let Some((min, max)) = bounds {
        let mut context = RepeatedMatchContext {
            base,
            word,
            ctx,
            results: &mut results,
        };
        match_repeated_condition(&mut context, word_idx, min, max, 0, state);
    } else {
        for (len, next_state) in match_base(base, false, word, word_idx, state, ctx) {
            results.push((len, next_state));
        }
    }
    results
}

pub struct RepeatedMatchContext<'a, 'b, 'c> {
    pub base: &'a MatchBase,
    pub word: &'a WorkingWord,
    pub ctx: &'b EvalContext<'c>,
    pub results: &'a mut Vec<(usize, MatchState)>,
}

pub(crate) fn match_repeated_condition(
    context: &mut RepeatedMatchContext<'_, '_, '_>,
    word_idx: usize,
    min: usize,
    max: usize,
    current_len: usize,
    state: &MatchState,
) {
    if min == 0 {
        context.results.push((current_len, state.clone()));
    }
    if max > 0 && word_idx < context.word.phonemes.len() {
        for (len, next_state) in match_base(
            context.base,
            false,
            context.word,
            word_idx,
            state,
            context.ctx,
        ) {
            if len > 0 {
                match_repeated_condition(
                    context,
                    word_idx + len,
                    min.saturating_sub(1),
                    max - 1,
                    current_len + len,
                    &next_state,
                );
            }
        }
    }
}
