use crate::evaluator::WorkingWord;
use std::collections::BTreeSet;

pub(crate) fn adjust_boundaries_op(
    word: &mut WorkingWord,
    start: usize,
    end: usize,
    original_len: usize,
    new_len: usize,
) {
    let mut updated_boundaries = BTreeSet::new();
    for &b in &word.syllable_boundaries {
        if b < start {
            updated_boundaries.insert(b);
        } else if b >= end {
            updated_boundaries.insert(b - original_len + new_len);
        }
    }
    word.syllable_boundaries = updated_boundaries;
}

fn calculate_new_stress_index_op(
    s_idx: usize,
    start: usize,
    end: usize,
    original_len: usize,
    new_len: usize,
) -> Option<usize> {
    if s_idx < start {
        Some(s_idx)
    } else if s_idx >= end {
        Some(s_idx - original_len + new_len)
    } else if new_len > 0 {
        let off = s_idx - start;
        Some(start + off.min(new_len - 1))
    } else {
        None
    }
}

fn adjust_stress_op(
    word: &mut WorkingWord,
    range: &std::ops::Range<usize>,
    lens: (usize, usize),
    has_new_stress: bool,
    new_stress_index: Option<usize>,
) {
    let (original_len, new_len) = lens;
    if has_new_stress {
        word.stress_index = new_stress_index.map(|local_idx| range.start + local_idx);
    } else if let Some(s_idx) = word.stress_index {
        word.stress_index =
            calculate_new_stress_index_op(s_idx, range.start, range.end, original_len, new_len);
    }
}

pub(crate) fn adjust_boundaries_and_stress(
    word: &mut WorkingWord,
    range: &std::ops::Range<usize>,
    lens: (usize, usize),
    has_new_stress: bool,
    new_stress_index: Option<usize>,
) {
    let (original_len, new_len) = lens;
    adjust_boundaries_op(word, range.start, range.end, original_len, new_len);
    adjust_stress_op(word, range, lens, has_new_stress, new_stress_index);
}
