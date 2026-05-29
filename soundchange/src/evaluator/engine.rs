use crate::ast::{MatchElement, MatchPattern};
use crate::evaluator::condition::evaluate_conditions;
use crate::evaluator::match_base::get_match_element_lengths;
use crate::evaluator::{EvalContext, MatchState, WorkingWord};

pub(crate) fn find_all_matches(
    word: &WorkingWord,
    match_part: &MatchPattern,
    condition: Option<&crate::compiler::CompiledConditionExpr>,
    is_leftward: bool,
    ctx: &EvalContext<'_>,
) -> Vec<(std::ops::Range<usize>, MatchState)> {
    let mut results = Vec::new();
    let mut idx = if is_leftward { word.phonemes.len() } else { 0 };

    while let Some((range, state)) =
        find_next_match(word, match_part, condition, (idx, is_leftward), ctx)
    {
        results.push((range.clone(), state));
        if is_leftward {
            if range.start == 0 {
                break;
            }
            idx = range.start.saturating_sub(1);
        } else {
            idx = range.end.max(idx + 1);
            if idx > word.phonemes.len() {
                break;
            }
        }
    }
    results
}

pub(crate) fn find_next_match(
    word: &WorkingWord,
    match_part: &MatchPattern,
    condition: Option<&crate::compiler::CompiledConditionExpr>,
    scan: (usize, bool),
    ctx: &EvalContext<'_>,
) -> Option<(std::ops::Range<usize>, MatchState)> {
    let (scan_idx, is_leftward) = scan;
    if is_leftward {
        for start_pos in (0..=scan_idx).rev() {
            if let Some((len, state)) = evaluate_match(match_part, word, start_pos, ctx) {
                let range = start_pos..start_pos + len;
                if let Some(final_state) = evaluate_conditions(condition, word, &range, &state, ctx)
                {
                    return Some((range, final_state));
                }
            }
        }
    } else {
        for start_pos in scan_idx..=word.phonemes.len() {
            if let Some((len, state)) = evaluate_match(match_part, word, start_pos, ctx) {
                let range = start_pos..start_pos + len;
                if let Some(final_state) = evaluate_conditions(condition, word, &range, &state, ctx)
                {
                    return Some((range, final_state));
                }
            }
        }
    }
    None
}

pub(crate) struct TransparentLoopParams<'a, 'b> {
    pub(crate) match_part: &'a MatchPattern,
    pub(crate) condition: Option<&'a crate::compiler::CompiledConditionExpr>,
    pub(crate) is_leftward: bool,
    pub(crate) is_single: bool,
    pub(crate) ctx: &'a EvalContext<'b>,
}

pub(crate) fn evaluate_transparent_loop<F, E>(
    word: &mut WorkingWord,
    params: &TransparentLoopParams<'_, '_>,
    mut replace_fn: F,
) -> Result<(), E>
where
    F: FnMut(&mut WorkingWord, std::ops::Range<usize>, &MatchState) -> Result<std::ops::Range<usize>, E>,
{
    let mut scan_idx = if params.is_leftward { word.phonemes.len() } else { 0 };

    loop {
        if params.is_leftward && scan_idx > word.phonemes.len() {
            scan_idx = word.phonemes.len();
        }

        let match_opt = find_next_match(
            word,
            params.match_part,
            params.condition,
            (scan_idx, params.is_leftward),
            params.ctx,
        );
        let Some((range, state)) = match_opt else {
            break;
        };

        let new_range = replace_fn(word, range.clone(), &state)?;

        if params.is_single {
            break;
        }

        if params.is_leftward {
            if range.start == 0 {
                break;
            }
            scan_idx = range.start;
        } else {
            scan_idx = new_range.end;
            if scan_idx > word.phonemes.len() {
                break;
            }
        }
    }
    Ok(())
}

pub(crate) fn evaluate_match(
    pattern: &MatchPattern,
    word: &WorkingWord,
    word_idx: usize,
    ctx: &EvalContext<'_>,
) -> Option<(usize, MatchState)> {
    let mut state = MatchState::default();
    let mut results = Vec::new();
    let mctx = MatchPatternContext {
        pattern,
        word,
        ctx,
    };
    match_pattern(
        &mctx,
        &pattern.elements,
        word_idx,
        &mut state,
        &mut results,
    );

    // Return the longest match length
    results.sort_by_key(|r| std::cmp::Reverse(r.0));
    results.into_iter().next()
}

pub(crate) struct MatchPatternContext<'a, 'b> {
    pub pattern: &'a MatchPattern,
    pub word: &'a WorkingWord,
    pub ctx: &'a EvalContext<'b>,
}

pub(crate) fn match_pattern(
    mctx: &MatchPatternContext<'_, '_>,
    elements: &[MatchElement],
    word_idx: usize,
    state: &mut MatchState,
    results: &mut Vec<(usize, MatchState)>,
) {
    let res = match_pattern_integration(mctx, elements, word_idx, state);
    results.extend(res);
}

fn sort_element_lengths_op(
    element_lengths: &mut Vec<(usize, MatchState, std::ops::Range<usize>)>,
) {
    element_lengths.sort_by_key(|(len, _, _)| std::cmp::Reverse(*len));
}

fn match_pattern_integration(
    mctx: &MatchPatternContext<'_, '_>,
    elements: &[MatchElement],
    word_idx: usize,
    state: &MatchState,
) -> Vec<(usize, MatchState)> {
    let Some((el, rest)) = elements.split_first() else {
        return vec![(0, state.clone())];
    };
    let mut element_lengths = get_match_element_lengths(
        el,
        &crate::evaluator::match_base::MatchContextParams {
            word: mctx.word,
            word_idx,
            ctx: mctx.ctx,
        },
        state,
    );
    sort_element_lengths_op(&mut element_lengths);

    let element_index = mctx.pattern.elements.len() - elements.len();
    let word_len = mctx.word.phonemes.len();

    element_lengths
        .into_iter()
        .flat_map(|(len, new_state, element_range)| {
            let next_idx = word_idx + len;
            let sub_res = if next_idx <= word_len {
                let mut temp_state = new_state;
                temp_state
                    .element_ranges
                    .insert(element_index, element_range);
                match_pattern_integration(mctx, rest, next_idx, &temp_state)
            } else {
                Vec::new()
            };
            sub_res
                .into_iter()
                .map(move |(sub_len, final_state)| (len + sub_len, final_state))
        })
        .collect()
}
