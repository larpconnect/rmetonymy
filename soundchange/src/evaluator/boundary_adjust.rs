use crate::evaluator::WorkingWord;
use std::collections::BTreeSet;

pub(crate) fn adjust_boundaries_and_stress(
    word: &mut WorkingWord,
    range: &std::ops::Range<usize>,
    lens: (usize, usize),
    has_new_stress: bool,
    new_stress_index: Option<usize>,
) {
    let (original_len, new_len) = lens;
    let mut updated_boundaries = BTreeSet::new();
    for &b in &word.syllable_boundaries {
        if b < range.start {
            updated_boundaries.insert(b);
        } else if b >= range.end {
            updated_boundaries.insert(b - original_len + new_len);
        }
    }
    word.syllable_boundaries = updated_boundaries;

    if has_new_stress {
        if let Some(local_idx) = new_stress_index {
            word.stress_index = Some(range.start + local_idx);
        } else {
            word.stress_index = None;
        }
    } else if let Some(s_idx) = word.stress_index {
        if s_idx < range.start {
            // Before the match, index is unchanged
        } else if s_idx >= range.end {
            // After the match, index shifts by difference in length
            word.stress_index = Some(s_idx - original_len + new_len);
        } else if new_len > 0 {
            // Within the match, preserve the relative offset if possible
            let off = s_idx - range.start;
            word.stress_index = Some(range.start + off.min(new_len - 1));
        } else {
            word.stress_index = None;
        }
    }
}
