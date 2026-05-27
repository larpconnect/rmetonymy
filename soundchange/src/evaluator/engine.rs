use crate::ast::{MatchElement, MatchPattern};
use crate::compiler::CompiledRuleChange;
use crate::evaluator::condition::evaluate_conditions;
use crate::evaluator::match_base::get_match_element_lengths;
use crate::evaluator::{EvalContext, MatchState, WorkingWord};

pub(crate) fn find_all_matches(
    word: &WorkingWord,
    change: &CompiledRuleChange,
    is_leftward: bool,
    ctx: &EvalContext<'_>,
) -> Vec<(std::ops::Range<usize>, MatchState)> {
    let mut results = Vec::new();
    let mut idx = if is_leftward { word.phonemes.len() } else { 0 };

    while let Some((range, state)) = find_next_match(word, change, idx, is_leftward, ctx) {
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
    change: &CompiledRuleChange,
    scan_idx: usize,
    is_leftward: bool,
    ctx: &EvalContext<'_>,
) -> Option<(std::ops::Range<usize>, MatchState)> {
    if is_leftward {
        for start_pos in (0..=scan_idx).rev() {
            if let Some((len, state)) = evaluate_match(&change.match_part, word, start_pos, ctx) {
                let range = start_pos..start_pos + len;
                if let Some(final_state) =
                    evaluate_conditions(change.condition.as_ref(), word, &range, &state, ctx)
                {
                    return Some((range, final_state));
                }
            }
        }
    } else {
        for start_pos in scan_idx..=word.phonemes.len() {
            if let Some((len, state)) = evaluate_match(&change.match_part, word, start_pos, ctx) {
                let range = start_pos..start_pos + len;
                if let Some(final_state) =
                    evaluate_conditions(change.condition.as_ref(), word, &range, &state, ctx)
                {
                    return Some((range, final_state));
                }
            }
        }
    }
    None
}

pub(crate) fn evaluate_match(
    pattern: &MatchPattern,
    word: &WorkingWord,
    word_idx: usize,
    ctx: &EvalContext<'_>,
) -> Option<(usize, MatchState)> {
    let mut state = MatchState::default();
    let mut results = Vec::new();
    match_pattern(
        pattern,
        &pattern.elements,
        word,
        word_idx,
        &mut state,
        &mut results,
        ctx,
    );

    // Return the longest match length
    results.sort_by_key(|r| std::cmp::Reverse(r.0));
    results.into_iter().next()
}

pub(crate) fn match_pattern(
    pattern: &MatchPattern,
    elements: &[MatchElement],
    word: &WorkingWord,
    word_idx: usize,
    state: &mut MatchState,
    results: &mut Vec<(usize, MatchState)>,
    ctx: &EvalContext<'_>,
) {
    let Some((el, rest)) = elements.split_first() else {
        results.push((0, state.clone()));
        return;
    };
    let mut element_lengths = get_match_element_lengths(el, word, word_idx, state, ctx);

    // Prioritize longer element matches
    element_lengths.sort_by_key(|(len, _, _)| std::cmp::Reverse(*len));

    for (len, new_state, element_range) in element_lengths {
        let next_idx = word_idx + len;
        if next_idx <= word.phonemes.len() {
            let element_index = pattern.elements.len() - elements.len();
            let mut temp_state = new_state;
            temp_state
                .element_ranges
                .insert(element_index, element_range);

            let mut sub_results = Vec::new();
            match_pattern(
                pattern,
                rest,
                word,
                next_idx,
                &mut temp_state,
                &mut sub_results,
                ctx,
            );
            for (sub_len, final_state) in sub_results {
                results.push((len + sub_len, final_state));
            }
        }
    }
}
