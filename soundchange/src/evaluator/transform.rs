use crate::ast::{FeatureDescriptor, TransformElement, TransformPattern};
use crate::evaluator::{EvalContext, MatchState, StressUpdate, WorkingWord};
use data::feature::Feature;
use ipa::IpaSequence;
use ipa::sequence::{Phoneme, PhonemeSequence};
use std::str::FromStr;

pub(crate) struct TransformContext<'a, 'b> {
    pub word: &'a WorkingWord,
    pub state: &'a MatchState,
    pub ctx: &'a EvalContext<'b>,
}

pub(crate) fn replace_range(
    word: &mut WorkingWord,
    range: std::ops::Range<usize>,
    state: &MatchState,
    transform: &TransformPattern,
    ctx: &EvalContext<'_>,
) -> Result<std::ops::Range<usize>, String> {
    let tctx = TransformContext { word, state, ctx };
    let (new_phonemes, new_stress_index, has_new_stress) =
        build_transform_phonemes(transform, &range, &tctx)?;

    let original_len = range.end - range.start;
    let new_len = new_phonemes.len();

    word.phonemes.splice(range.clone(), new_phonemes);

    let new_tags = if let Some(tag) = ctx.active_tag {
        vec![Some(tag); new_len]
    } else {
        let replaced_tags = word
            .tags
            .get(range.clone())
            .ok_or_else(|| format!("Invalid tags range: {range:?}"))?;
        let inherit_tag = replaced_tags.iter().copied().flatten().next();
        vec![inherit_tag; new_len]
    };
    word.tags.splice(range.clone(), new_tags);

    adjust_boundaries_and_stress(
        word,
        &range,
        (original_len, new_len),
        has_new_stress,
        new_stress_index,
    );

    Ok(range.start..range.start + new_len)
}

pub(crate) fn build_transform_phonemes(
    transform: &TransformPattern,
    range: &std::ops::Range<usize>,
    tctx: &TransformContext<'_, '_>,
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
                let phonemes =
                    eval_transform_literal(ipa, *copy_modifiers, append_modifiers, el_idx, tctx)?;
                new_phonemes.extend(phonemes);
            }
            TransformElement::Ref { .. } => {
                let (phonemes, stress_update) =
                    eval_transform_ref(el, new_phonemes.len(), range, tctx)?;
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
    tctx: &TransformContext<'_, '_>,
) -> Result<Vec<Phoneme>, String> {
    let parsed_seq = PhonemeSequence::from_str(ipa.as_str())
        .map_err(|e| format!("Invalid IPA in transform: {e:?}"))?;
    let mut phonemes = Vec::new();
    for seq_el in parsed_seq.phonemes() {
        let mut p = seq_el.clone();
        if copy_modifiers {
            p.modifiers.extend(get_captured_modifiers_for_element(
                tctx.state, el_idx, tctx.word,
            ));
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
    tctx: &TransformContext<'_, '_>,
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
        for m in get_captured_modifiers_for_element(tctx.state, 0, tctx.word) {
            if !p.modifiers.contains(&m) {
                p.modifiers.push(m);
            }
        }
    }
    for m in append_modifiers {
        if !p.modifiers.contains(m) {
            p.modifiers.push(m.clone());
        }
    }

    let mut stress_update = StressUpdate::Keep;
    if !feature_changes.is_empty() {
        p = apply_feature_changes(&p, feature_changes, tctx.state, tctx.ctx)?;
        stress_update = eval_feature_changes_stress(
            feature_changes,
            tctx.state,
            current_pos,
            orig_idx,
            tctx.word.stress_index,
        );
    }
    Ok((p, stress_update))
}

pub(crate) fn eval_transform_ref(
    el: &TransformElement,
    current_phonemes_len: usize,
    match_range: &std::ops::Range<usize>,
    tctx: &TransformContext<'_, '_>,
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

    let source_phonemes = get_referenced_phonemes(
        tctx.word,
        *marker,
        class_key.as_ref(),
        tctx.state,
        match_range,
    );
    let source_phoneme_indices = get_referenced_phoneme_indices(
        tctx.word,
        *marker,
        class_key.as_ref(),
        tctx.state,
        match_range,
    );
    let mut phonemes = Vec::new();
    let mut stress_update = StressUpdate::Keep;

    for _ in 0..*repeat {
        for (i, sp) in source_phonemes.iter().enumerate() {
            let orig_idx = source_phoneme_indices.get(i).copied();
            let current_pos = current_phonemes_len + phonemes.len();
            let (p, su) = transform_single_source_phoneme(sp, orig_idx, el, current_pos, tctx)?;
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

fn eval_stress_sign_op(fd: &FeatureDescriptor, state: &MatchState) -> bool {
    super::descriptor::evaluate_descriptor_sign_op(fd, state)
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
            let sign = eval_stress_sign_op(fd, state);
            if sign {
                update = StressUpdate::Set(phoneme_pos);
            } else if orig_idx.is_some() && orig_idx == word_stress_index {
                update = StressUpdate::Clear;
            }
        }
    }
    update
}

use super::boundary_adjust::adjust_boundaries_and_stress;

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

use super::feature_changes::apply_feature_changes;
