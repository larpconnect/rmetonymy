use crate::ast::{MatchBase, MatchQuantifier};
use crate::evaluator::match_base::{match_base, MatchParams};
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

pub(crate) fn match_repeated(
    context: &mut RepeatedMatchContext<'_, '_, '_>,
    rstate: RepeatedState,
    state: &MatchState,
) {
    push_min_match_op(rstate.min, context.results, rstate.current_len, rstate.word_idx, state);
    let can_match = can_continue_match_op(rstate.max, rstate.word_idx, context.word.phonemes.len());
    match_repeated_step_integration(context, rstate, state, can_match);
}

fn push_min_match_op(
    min: usize,
    results: &mut Vec<(usize, MatchState, std::ops::Range<usize>)>,
    current_len: usize,
    word_idx: usize,
    state: &MatchState,
) {
    if min == 0 {
        results.push((current_len, state.clone(), word_idx - current_len..word_idx));
    }
}

pub fn can_continue_match_op(max: usize, word_idx: usize, word_len: usize) -> bool {
    max > 0 && word_idx < word_len
}

fn match_repeated_step_integration(
    context: &mut RepeatedMatchContext<'_, '_, '_>,
    rstate: RepeatedState,
    state: &MatchState,
    can_match: bool,
) {
    branch_on_match_op(can_match, || {
        let params = MatchParams {
            wildcard: context.wildcard,
            word: context.word,
            ctx: context.ctx,
        };
        let bases = match_base(context.base, &params, rstate.word_idx, state);
        match_repeated_recurse_integration(context, rstate, bases);
    });
}

pub fn branch_on_match_op<F>(can_match: bool, mut match_fn: F)
where
    F: FnMut(),
{
    if can_match {
        match_fn();
    }
}

fn match_repeated_recurse_integration(
    context: &mut RepeatedMatchContext<'_, '_, '_>,
    rstate: RepeatedState,
    bases: Vec<(usize, MatchState)>,
) {
    bases.into_iter().for_each(|(len, next_state)| {
        let is_positive = len > 0;
        branch_on_match_op(is_positive, || {
            let next_rstate = RepeatedState {
                word_idx: rstate.word_idx + len,
                min: rstate.min.saturating_sub(1),
                max: rstate.max - 1,
                current_len: rstate.current_len + len,
            };
            match_repeated(context, next_rstate, &next_state);
        });
    });
}
