use super::match_base::{
    MatchContextParams, MatchParams, MatchRepeatedContext, RepeatedState, match_base,
};
use crate::ast::MatchElement;
use crate::evaluator::repeated::get_bounds_op;
use crate::evaluator::{MatchState, repeated::match_repeated};

pub(crate) fn get_match_element_lengths(
    el: &MatchElement,
    mctx: &MatchContextParams<'_, '_>,
    state: &MatchState,
) -> Vec<(usize, MatchState, std::ops::Range<usize>)> {
    let bounds = get_bounds_op(&el.quantifier);
    get_lengths_integration(bounds, el, mctx, state)
}

fn get_lengths_integration(
    bounds: Option<(usize, usize)>,
    el: &MatchElement,
    mctx: &MatchContextParams<'_, '_>,
    state: &MatchState,
) -> Vec<(usize, MatchState, std::ops::Range<usize>)> {
    match_repeated_or_base_op(
        bounds,
        |range| {
            let mut results = Vec::new();
            let mut context = MatchRepeatedContext {
                base: &el.base,
                wildcard: el.modifiers_wildcard,
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
            results
        },
        || {
            let params = MatchParams {
                wildcard: el.modifiers_wildcard,
                word: mctx.word,
                ctx: mctx.ctx,
            };
            match_base(&el.base, &params, mctx.word_idx, state)
                .into_iter()
                .map(|(len, next_state)| (len, next_state, mctx.word_idx..mctx.word_idx + len))
                .collect()
        },
    )
}

fn match_repeated_or_base_op<F, G>(
    bounds: Option<(usize, usize)>,
    mut rep_fn: F,
    mut base_fn: G,
) -> Vec<(usize, MatchState, std::ops::Range<usize>)>
where
    F: FnMut((usize, usize)) -> Vec<(usize, MatchState, std::ops::Range<usize>)>,
    G: FnMut() -> Vec<(usize, MatchState, std::ops::Range<usize>)>,
{
    if let Some(range) = bounds {
        rep_fn(range)
    } else {
        base_fn()
    }
}
