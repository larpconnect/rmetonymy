use crate::ast::{MatchBase, MatchQuantifier};
use crate::evaluator::match_base::{MatchParams, match_base};
use crate::evaluator::{EvalContext, MatchState, WorkingWord};

pub fn get_bounds_op(quantifier: &MatchQuantifier) -> Option<(usize, usize)> {
    match quantifier {
        MatchQuantifier::None => None,
        MatchQuantifier::ZeroOrMore => Some((0, usize::MAX)),
        MatchQuantifier::OneOrMore => Some((1, usize::MAX)),
        MatchQuantifier::ZeroOrMoreBounded(limit) => Some((0, *limit as usize)),
        MatchQuantifier::OneOrMoreBounded(limit) => Some((1, *limit as usize)),
    }
}

pub struct RepeatedMatchContext<'a, 'b, 'c> {
    pub base: &'a MatchBase,
    pub wildcard: bool,
    pub word: &'a WorkingWord,
    pub ctx: &'b EvalContext<'c>,
    pub results: &'a mut Vec<(usize, MatchState, std::ops::Range<usize>)>,
}

#[derive(Clone, Copy)]
pub struct RepeatedState {
    pub word_idx: usize,
    pub min: usize,
    pub max: usize,
    pub current_len: usize,
}

fn record_if_needed_op(
    context: &mut RepeatedMatchContext<'_, '_, '_>,
    rstate: RepeatedState,
    state: &MatchState,
) {
    if rstate.min == 0 {
        context.results.push((
            rstate.current_len,
            state.clone(),
            rstate.word_idx - rstate.current_len..rstate.word_idx,
        ));
    }
}

fn build_next_rstate_op(rstate: RepeatedState, len: usize) -> RepeatedState {
    RepeatedState {
        word_idx: rstate.word_idx + len,
        min: rstate.min.saturating_sub(1),
        max: rstate.max - 1,
        current_len: rstate.current_len + len,
    }
}

fn build_match_params_op<'a, 'b>(
    context: &RepeatedMatchContext<'a, 'b, '_>,
) -> MatchParams<'a, 'b> {
    MatchParams {
        wildcard: context.wildcard,
        word: context.word,
        ctx: context.ctx,
    }
}

fn should_recurse_op(rstate: RepeatedState, word_len: usize) -> bool {
    rstate.max > 0 && rstate.word_idx < word_len
}

fn recurse_matches_integration(
    context: &mut RepeatedMatchContext<'_, '_, '_>,
    rstate: RepeatedState,
    base_matches: Vec<(usize, MatchState)>,
) {
    base_matches
        .into_iter()
        .filter(|&(len, _)| len > 0)
        .for_each(|(len, next_state)| {
            let next_rstate = build_next_rstate_op(rstate, len);
            match_repeated(context, next_rstate, &next_state);
        });
}

fn recurse_if_needed_integration(
    context: &mut RepeatedMatchContext<'_, '_, '_>,
    rstate: RepeatedState,
    state: &MatchState,
) {
    if should_recurse_op(rstate, context.word.phonemes.len()) {
        let params = build_match_params_op(context);
        let base_matches = match_base(context.base, &params, rstate.word_idx, state);
        recurse_matches_integration(context, rstate, base_matches);
    }
}

pub(crate) fn match_repeated(
    context: &mut RepeatedMatchContext<'_, '_, '_>,
    rstate: RepeatedState,
    state: &MatchState,
) {
    record_if_needed_op(context, rstate, state);
    recurse_if_needed_integration(context, rstate, state);
}
