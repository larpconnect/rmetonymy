use crate::ast::{FeatureDescriptor, TransformElement, TransformPattern};
use crate::evaluator::features::{get_phoneme_features_map, get_phoneme_features_map_from_data};
use crate::evaluator::{CapturedAlpha, EvalContext, MatchState, StressUpdate, WorkingWord};
use data::feature::Feature;
use ipa::IpaSequence;
use ipa::sequence::{Phoneme, PhonemeSequence};
use std::collections::{BTreeSet, HashMap};
use std::str::FromStr;

pub(crate) fn replace_range(
    word: &mut WorkingWord,
    range: std::ops::Range<usize>,
    state: &MatchState,
    transform: &TransformPattern,
    ctx: &EvalContext<'_>,
) -> Result<std::ops::Range<usize>, String> {
    let (new_phonemes, new_stress_index, has_new_stress) =
        build_transform_phonemes(transform, word, &range, state, ctx)?;

    let original_len = range.end - range.start;
    let new_len = new_phonemes.len();

    word.phonemes.splice(range.clone(), new_phonemes);
    adjust_boundaries_and_stress(
        word,
        &range,
        original_len,
        new_len,
        has_new_stress,
        new_stress_index,
    );

    Ok(range.start..range.start + new_len)
}

pub(crate) fn build_transform_phonemes(
    transform: &TransformPattern,
    word: &WorkingWord,
    range: &std::ops::Range<usize>,
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Result<(Vec<Phoneme>, Option<usize>, bool), String> {
    let mut new_phonemes = Vec::new();
    let mut new_stress_index = None;
    let mut has_new_stress = false;

    for (el_idx, el) in transform.elements.iter().enumerate() {
        match el {
            TransformElement::Empty => {}
            TransformElement::Literal {
                ipa,
                copy_modifiers,
                append_modifiers,
            } => {
                let phonemes = eval_transform_literal(
                    ipa,
                    *copy_modifiers,
                    append_modifiers,
                    el_idx,
                    state,
                    word,
                )?;
                new_phonemes.extend(phonemes);
            }
            TransformElement::Ref { .. } => {
                let (phonemes, stress_update) =
                    eval_transform_ref(el, new_phonemes.len(), state, word, range, ctx)?;
                new_phonemes.extend(phonemes);
                match stress_update {
                    StressUpdate::Set(idx) => {
                        new_stress_index = Some(idx);
                        has_new_stress = true;
                    }
                    StressUpdate::Clear => {
                        new_stress_index = None;
                        has_new_stress = true;
                    }
                    StressUpdate::Keep => {}
                }
            }
        }
    }
    Ok((new_phonemes, new_stress_index, has_new_stress))
}

pub(crate) fn eval_transform_literal(
    ipa: &ipa::IpaString,
    copy_modifiers: bool,
    append_modifiers: &[String],
    el_idx: usize,
    state: &MatchState,
    word: &WorkingWord,
) -> Result<Vec<Phoneme>, String> {
    let parsed_seq = PhonemeSequence::from_str(ipa.as_str())
        .map_err(|e| format!("Invalid IPA in transform: {e:?}"))?;
    let mut phonemes = Vec::new();
    for seq_el in parsed_seq.phonemes() {
        let mut p = seq_el.clone();
        if copy_modifiers {
            p.modifiers
                .extend(get_captured_modifiers_for_element(state, el_idx, word));
        }
        p.modifiers.extend(append_modifiers.iter().cloned());
        phonemes.push(p);
    }
    Ok(phonemes)
}

pub(crate) fn get_referenced_phoneme_indices(
    _word: &WorkingWord,
    marker: Option<u8>,
    class_key: Option<&language::sound_class::SoundClassKey>,
    state: &MatchState,
    match_range: &std::ops::Range<usize>,
) -> Vec<usize> {
    if let Some(range) = marker.and_then(|m| state.markers.get(&(class_key.cloned(), m))) {
        return range.clone().collect();
    }
    match_range.clone().collect()
}

fn transform_single_source_phoneme(
    sp: &Phoneme,
    orig_idx: Option<usize>,
    el: &TransformElement,
    current_pos: usize,
    word: &WorkingWord,
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Result<(Phoneme, StressUpdate), String> {
    let TransformElement::Ref {
        copy_modifiers,
        append_modifiers,
        feature_changes,
        ..
    } = el
    else {
        return Err("Expected TransformElement::Ref".to_string());
    };

    let mut p = sp.clone();
    if *copy_modifiers {
        p.modifiers
            .extend(get_captured_modifiers_for_element(state, 0, word));
    }
    p.modifiers.extend(append_modifiers.iter().cloned());

    let mut stress_update = StressUpdate::Keep;
    if !feature_changes.is_empty() {
        p = apply_feature_changes(&p, feature_changes, state, ctx)?;
        stress_update = eval_feature_changes_stress(
            feature_changes,
            state,
            current_pos,
            orig_idx,
            word.stress_index,
        );
    }
    Ok((p, stress_update))
}

pub(crate) fn eval_transform_ref(
    el: &TransformElement,
    current_phonemes_len: usize,
    state: &MatchState,
    word: &WorkingWord,
    match_range: &std::ops::Range<usize>,
    ctx: &EvalContext<'_>,
) -> Result<(Vec<Phoneme>, StressUpdate), String> {
    let TransformElement::Ref {
        marker,
        class_key,
        repeat,
        ..
    } = el
    else {
        return Err("Expected TransformElement::Ref".to_string());
    };

    let source_phonemes =
        get_referenced_phonemes(word, *marker, class_key.as_ref(), state, match_range);
    let source_phoneme_indices =
        get_referenced_phoneme_indices(word, *marker, class_key.as_ref(), state, match_range);
    let mut phonemes = Vec::new();
    let mut stress_update = StressUpdate::Keep;

    for _ in 0..*repeat {
        for (i, sp) in source_phonemes.iter().enumerate() {
            let orig_idx = source_phoneme_indices.get(i).copied();
            let current_pos = current_phonemes_len + phonemes.len();
            let (p, su) =
                transform_single_source_phoneme(sp, orig_idx, el, current_pos, word, state, ctx)?;
            match su {
                StressUpdate::Set(stress_idx) => {
                    stress_update = StressUpdate::Set(stress_idx);
                }
                StressUpdate::Clear => {
                    stress_update = StressUpdate::Clear;
                }
                StressUpdate::Keep => {}
            }
            phonemes.push(p);
        }
    }
    Ok((phonemes, stress_update))
}

pub(crate) fn eval_feature_changes_stress(
    feature_changes: &[FeatureDescriptor],
    state: &MatchState,
    phoneme_pos: usize,
    orig_idx: Option<usize>,
    word_stress_index: Option<usize>,
) -> StressUpdate {
    let mut update = StressUpdate::Keep;
    for fd in feature_changes {
        if fd.feature == Feature::Stress {
            let sign = if let Some(ref alpha) = fd.alpha {
                match state.alpha.get(&alpha.name) {
                    Some(CapturedAlpha::Sign(s)) => {
                        if alpha.sign {
                            !s
                        } else {
                            *s
                        }
                    }
                    _ => false,
                }
            } else {
                fd.sign
            };
            if sign {
                update = StressUpdate::Set(phoneme_pos);
            } else if orig_idx.is_some() && orig_idx == word_stress_index {
                update = StressUpdate::Clear;
            }
        }
    }
    update
}

pub(crate) fn adjust_boundaries_and_stress(
    word: &mut WorkingWord,
    range: &std::ops::Range<usize>,
    original_len: usize,
    new_len: usize,
    has_new_stress: bool,
    new_stress_index: Option<usize>,
) {
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

pub(crate) fn get_captured_modifiers_for_element(
    state: &MatchState,
    el_idx: usize,
    word: &WorkingWord,
) -> Vec<String> {
    let range_opt = state
        .element_ranges
        .get(&el_idx)
        .or_else(|| state.element_ranges.get(&0));
    if let Some(range) = range_opt {
        let mut mods = Vec::new();
        for idx in range.clone() {
            if let Some(p) = word.phonemes.get(idx) {
                mods.extend(p.modifiers.clone());
            }
        }
        mods
    } else {
        Vec::new()
    }
}

pub(crate) fn get_referenced_phonemes(
    word: &WorkingWord,
    marker: Option<u8>,
    class_key: Option<&language::sound_class::SoundClassKey>,
    state: &MatchState,
    match_range: &std::ops::Range<usize>,
) -> Vec<Phoneme> {
    if let Some(slice) = marker
        .and_then(|m| state.markers.get(&(class_key.cloned(), m)))
        .and_then(|range| word.phonemes.get(range.clone()))
    {
        return slice.to_vec();
    }
    word.phonemes
        .get(match_range.clone())
        .map(ToOwned::to_owned)
        .unwrap_or_default()
}

pub(crate) fn apply_feature_changes(
    p: &Phoneme,
    changes: &[FeatureDescriptor],
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Result<Phoneme, String> {
    let mut map = get_phoneme_features_map(p);
    let mut target_place = if let Some(d) = ipa::get_phoneme_data(&p.base) {
        d.place.clone()
    } else {
        Vec::new()
    };
    let mut target_manner = if let Some(d) = ipa::get_phoneme_data(&p.base) {
        d.manner.clone()
    } else {
        Vec::new()
    };

    for fd in changes {
        if fd.feature == Feature::Stress {
            continue;
        }
        if fd.feature == Feature::Place || fd.feature == Feature::Manner {
            if let Some(CapturedAlpha::Strings(s)) = fd
                .alpha
                .as_ref()
                .and_then(|alpha| state.alpha.get(&alpha.name))
            {
                if fd.feature == Feature::Place {
                    target_place.clone_from(s);
                } else {
                    target_manner.clone_from(s);
                }
            }
            continue;
        }
        let sign = if let Some(ref alpha) = fd.alpha {
            match state.alpha.get(&alpha.name) {
                Some(CapturedAlpha::Sign(s)) => {
                    if alpha.sign {
                        !s
                    } else {
                        *s
                    }
                }
                _ => false,
            }
        } else {
            fd.sign
        };
        map.insert(fd.feature, sign);
    }

    let best_base = find_best_phoneme_base(&map, &target_place, &target_manner, ctx)?;
    Ok(Phoneme {
        base: best_base,
        modifiers: p.modifiers.clone(),
    })
}

pub(crate) fn find_best_phoneme_base(
    target_features: &HashMap<Feature, bool>,
    target_place: &[String],
    target_manner: &[String],
    ctx: &EvalContext<'_>,
) -> Result<String, String> {
    let mut best_base = None;
    let mut min_diff = usize::MAX;

    for (sym, entry) in ctx.system.dataset() {
        let (data::IpaEntry::Phoneme(phoneme_data)
        | data::IpaEntry::Consonant(phoneme_data)
        | data::IpaEntry::Vowel(phoneme_data)) = entry
        else {
            continue;
        };

        let mut diff = 0;
        let phoneme_feats = get_phoneme_features_map_from_data(phoneme_data);
        for (&feat, &target_val) in target_features {
            let val = *phoneme_feats.get(&feat).unwrap_or(&false);
            if val != target_val {
                diff += 1;
            }
        }

        for (&feat, &val) in &phoneme_feats {
            if val && !target_features.contains_key(&feat) {
                diff += 1;
            }
        }

        if !target_place.is_empty() && phoneme_data.place != target_place {
            diff += 100;
        }
        if !target_manner.is_empty() && phoneme_data.manner != target_manner {
            diff += 100;
        }

        if diff < min_diff {
            min_diff = diff;
            best_base = Some(sym.clone());
        }
    }

    best_base.ok_or_else(|| "No phoneme base matches feature changes".to_string())
}
