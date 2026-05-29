use crate::ast::{ConditionBase, ConditionElement, MatchBase};
use crate::evaluator::match_base::{match_base, MatchParams};
use crate::evaluator::repeated::{RepeatedMatchContext, RepeatedState, match_repeated, get_bounds_op};
use crate::evaluator::{EvalContext, MatchState, WorkingWord};

pub(crate) fn evaluate_match_pattern_condition(
    pattern: &crate::ast::ConditionPattern,
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
    let mctx = ConditionMatchParams {
        word,
        word_idx,
        ctx,
    };
    let base_opts = resolve_base_options_integration(el, &mctx, state);
    evaluate_match_elements_condition_recurse(rest, word, word_idx, base_opts, ctx)
}

pub struct ConditionMatchParams<'a, 'b> {
    pub word: &'a WorkingWord,
    pub word_idx: usize,
    pub ctx: &'a EvalContext<'b>,
}

fn resolve_base_options_integration(
    el: &ConditionElement,
    mctx: &ConditionMatchParams<'_, '_>,
    state: &MatchState,
) -> Vec<(usize, MatchState)> {
    match_condition_base_op(
        el,
        || vec![(0, state.clone())],
        |base| get_match_element_lengths_condition(el, base, mctx, state),
    )
}

fn match_condition_base_op<F, G>(
    el: &ConditionElement,
    mut placeholder_fn: F,
    mut elem_fn: G,
) -> Vec<(usize, MatchState)>
where
    F: FnMut() -> Vec<(usize, MatchState)>,
    G: FnMut(&MatchBase) -> Vec<(usize, MatchState)>,
{
    match &el.base {
        ConditionBase::MatchPlaceholder => placeholder_fn(),
        ConditionBase::Element(base) => elem_fn(base),
    }
}

fn evaluate_match_elements_condition_recurse(
    rest: &[ConditionElement],
    word: &WorkingWord,
    word_idx: usize,
    base_opts: Vec<(usize, MatchState)>,
    ctx: &EvalContext<'_>,
) -> Vec<(usize, MatchState)> {
    base_opts
        .into_iter()
        .flat_map(|(len, next_state)| {
            let next_idx = word_idx + len;
            let sub_res = if next_idx <= word.phonemes.len() {
                evaluate_match_elements_condition(rest, word, next_idx, &next_state, ctx)
            } else {
                Vec::new()
            };
            sub_res
                .into_iter()
                .map(move |(sub_len, final_state)| (len + sub_len, final_state))
        })
        .collect()
}

fn get_match_element_lengths_condition(
    el: &ConditionElement,
    base: &MatchBase,
    mctx: &ConditionMatchParams<'_, '_>,
    state: &MatchState,
) -> Vec<(usize, MatchState)> {
    let bounds = get_bounds_op(&el.quantifier);
    get_lengths_integration(bounds, base, mctx, state)
}

fn get_lengths_integration(
    bounds: Option<(usize, usize)>,
    base: &MatchBase,
    mctx: &ConditionMatchParams<'_, '_>,
    state: &MatchState,
) -> Vec<(usize, MatchState)> {
    match_repeated_or_base_op(
        bounds,
        |range| {
            let mut results = Vec::new();
            let mut context = RepeatedMatchContext {
                base,
                wildcard: false,
                word: mctx.word,
                ctx: mctx.ctx,
                results: &mut results,
            };
            let rstate = RepeatedState {
                word_idx: mctx.word_idx,
                min: range.0,
                max: range.1,
                current_len: 0,
            };
            match_repeated(&mut context, rstate, state);
            results.into_iter().map(|(len, ns, _)| (len, ns)).collect()
        },
        || {
            let match_params = MatchParams {
                wildcard: false,
                word: mctx.word,
                ctx: mctx.ctx,
            };
            match_base(base, &match_params, mctx.word_idx, state)
        },
    )
}

fn match_repeated_or_base_op<F, G>(
    bounds: Option<(usize, usize)>,
    mut rep_fn: F,
    mut base_fn: G,
) -> Vec<(usize, MatchState)>
where
    F: FnMut((usize, usize)) -> Vec<(usize, MatchState)>,
    G: FnMut() -> Vec<(usize, MatchState)>,
{
    if let Some(range) = bounds {
        rep_fn(range)
    } else {
        base_fn()
    }
}
