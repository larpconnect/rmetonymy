use crate::evaluator::{MatchState, WorkingWord};

pub(crate) fn match_word_boundary(
    word: &WorkingWord,
    word_idx: usize,
    state: &MatchState,
) -> Vec<(usize, MatchState)> {
    if word_idx == 0 || word_idx == word.phonemes.len() {
        vec![(0, state.clone())]
    } else {
        vec![]
    }
}

pub(crate) fn match_syllable_boundary(
    word: &WorkingWord,
    word_idx: usize,
    state: &MatchState,
) -> Vec<(usize, MatchState)> {
    if word.syllable_boundaries.contains(&word_idx)
        || word_idx == 0
        || word_idx == word.phonemes.len()
    {
        vec![(0, state.clone())]
    } else {
        vec![]
    }
}
