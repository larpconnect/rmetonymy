use crate::ast::{
    FeatureClassKey, FeatureDescriptor, MatchBase, MatchPattern,
};
use crate::evaluator::engine::evaluate_match;
use crate::evaluator::{EvalContext, MatchState, WorkingWord};
use ipa::IpaSequence;
use ipa::sequence::Phoneme;

pub type MatchRepeatedContext<'a, 'b, 'c> = crate::evaluator::repeated::RepeatedMatchContext<'a, 'b, 'c>;
pub type RepeatedState = crate::evaluator::repeated::RepeatedState;

pub(crate) struct MatchParams<'a, 'b> {
    pub wildcard: bool,
    pub word: &'a WorkingWord,
    pub ctx: &'a EvalContext<'b>,
}

pub(crate) struct MatchContextParams<'a, 'b> {
    pub word: &'a WorkingWord,
    pub word_idx: usize,
    pub ctx: &'a EvalContext<'b>,
}

pub(crate) use super::lengths::get_match_element_lengths;

pub(crate) fn match_base(
    base: &MatchBase,
    params: &MatchParams<'_, '_>,
    word_idx: usize,
    state: &MatchState,
) -> Vec<(usize, MatchState)> {
    match base {
        MatchBase::WordBoundary => match_word_boundary(params.word, word_idx, state),
        MatchBase::SyllableBoundary => match_syllable_boundary(params.word, word_idx, state),
        MatchBase::SoundClass { key, marker } => {
            match_sound_class(key, *marker, params, word_idx, state)
        }
        MatchBase::SetExclusion { key, marker } => {
            match_set_exclusion(key, *marker, params, word_idx, state)
        }
        MatchBase::IpaSequence(ipa) => {
            match_ipa_sequence_element(ipa, params.wildcard, params.word, word_idx, state)
        }
        MatchBase::FeatureClass { key_opt, features } => match_feature_class(
            key_opt.as_ref(),
            features,
            params,
            word_idx,
            state,
        ),
        MatchBase::Set(bases) => match_set_element(bases, params, word_idx, state),
        MatchBase::OptionalGroup(pat) => match_optional_group(pat, params.word, word_idx, state, params.ctx),
    }
}

use super::boundary::{match_word_boundary, match_syllable_boundary};

fn match_class_logic(
    key_and_marker: (&language::sound_class::SoundClassKey, Option<u8>),
    params: &MatchParams<'_, '_>,
    word_idx: usize,
    state: &MatchState,
    should_match: bool,
) -> Vec<(usize, MatchState)> {
    let (key, marker) = key_and_marker;
    if let Some(p) = params.word.phonemes.get(word_idx) {
        if phoneme_in_class(p, key, params.wildcard, params.ctx) == should_match {
            let mut next_state = state.clone();
            bind_marker(
                Some(key.clone()),
                marker,
                word_idx..word_idx + 1,
                &mut next_state,
            );
            vec![(1, next_state)]
        } else {
            vec![]
        }
    } else {
        vec![]
    }
}

pub(crate) fn match_sound_class(
    key: &language::sound_class::SoundClassKey,
    marker: Option<u8>,
    params: &MatchParams<'_, '_>,
    word_idx: usize,
    state: &MatchState,
) -> Vec<(usize, MatchState)> {
    match_class_logic((key, marker), params, word_idx, state, true)
}

pub(crate) fn match_set_exclusion(
    key: &language::sound_class::SoundClassKey,
    marker: Option<u8>,
    params: &MatchParams<'_, '_>,
    word_idx: usize,
    state: &MatchState,
) -> Vec<(usize, MatchState)> {
    match_class_logic((key, marker), params, word_idx, state, false)
}

pub(crate) fn match_ipa_sequence_element(
    ipa: &ipa::IpaString,
    wildcard: bool,
    word: &WorkingWord,
    word_idx: usize,
    state: &MatchState,
) -> Vec<(usize, MatchState)> {
    if let Some(matched_len) = match_ipa_sequence(word, word_idx, ipa, wildcard) {
        vec![(matched_len, state.clone())]
    } else {
        vec![]
    }
}

pub(crate) fn match_feature_class(
    key_opt: Option<&FeatureClassKey>,
    features: &[FeatureDescriptor],
    params: &MatchParams<'_, '_>,
    word_idx: usize,
    state: &MatchState,
) -> Vec<(usize, MatchState)> {
    let Some(p) = params.word.phonemes.get(word_idx) else {
        return vec![];
    };
    let mut next_state = state.clone();
    if let Some(key) = key_opt {
        if let Some(class_key) = &key.key {
            let in_class = phoneme_in_class(p, class_key, params.wildcard, params.ctx);
            if in_class == key.exclude {
                return vec![];
            }
        }
        bind_marker(
            key.key.clone(),
            key.marker,
            word_idx..word_idx + 1,
            &mut next_state,
        );
    }
    if evaluate_feature_descriptors(features, p, word_idx, params.word.stress_index, &mut next_state) {
        vec![(1, next_state)]
    } else {
        vec![]
    }
}

pub(crate) fn match_set_element(
    bases: &[MatchBase],
    params: &MatchParams<'_, '_>,
    word_idx: usize,
    state: &MatchState,
) -> Vec<(usize, MatchState)> {
    let mut results = Vec::new();
    for b in bases {
        results.extend(match_base(b, params, word_idx, state));
    }
    results
}

pub(crate) fn match_optional_group(
    pat: &MatchPattern,
    word: &WorkingWord,
    word_idx: usize,
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Vec<(usize, MatchState)> {
    let mut results = Vec::new();
    if let Some((len, next_state)) = evaluate_match(pat, word, word_idx, ctx) {
        results.push((len, next_state));
    }
    results.push((0, state.clone()));
    results
}

pub(crate) fn bind_marker(
    class_key: Option<language::sound_class::SoundClassKey>,
    marker_opt: Option<u8>,
    range: std::ops::Range<usize>,
    state: &mut MatchState,
) {
    if let Some(m) = marker_opt {
        state.markers.insert((class_key, m), range);
    }
}

pub(crate) fn phoneme_in_class(
    p: &Phoneme,
    key: &language::sound_class::SoundClassKey,
    wildcard: bool,
    ctx: &EvalContext<'_>,
) -> bool {
    if !wildcard && !p.modifiers.is_empty() {
        return false;
    }
    match key.as_str() {
        "V" => is_vowel(p),
        "C" => is_consonant(p),
        "X" => true,
        _ => {
            if let Some(sc) = ctx.classes.get(key) {
                sc.values.iter().any(|val| val == &p.base)
            } else {
                false
            }
        }
    }
}

pub(crate) fn is_vowel(p: &Phoneme) -> bool {
    if let Some(entry) = ipa::get_entry(&p.base) {
        return matches!(entry, data::IpaEntry::Vowel(_));
    }
    false
}

pub(crate) fn is_consonant(p: &Phoneme) -> bool {
    if let Some(entry) = ipa::get_entry(&p.base) {
        return matches!(entry, data::IpaEntry::Consonant(_));
    }
    false
}

fn match_single_phoneme_in_sequence(wp: &Phoneme, tp: &Phoneme, wildcard: bool) -> bool {
    if wp.base != tp.base {
        return false;
    }
    if wildcard {
        for m in &tp.modifiers {
            if !wp.modifiers.contains(m) {
                return false;
            }
        }
        true
    } else {
        wp.modifiers == tp.modifiers
    }
}

pub(crate) fn match_ipa_sequence(
    word: &WorkingWord,
    word_idx: usize,
    ipa: &ipa::IpaString,
    wildcard: bool,
) -> Option<usize> {
    let target_phonemes = ipa.phonemes();
    if word_idx + target_phonemes.len() > word.phonemes.len() {
        return None;
    }
    for (i, tp) in target_phonemes.iter().enumerate() {
        let wp = word.phonemes.get(word_idx + i)?;
        if !match_single_phoneme_in_sequence(wp, tp, wildcard) {
            return None;
        }
    }
    Some(target_phonemes.len())
}

use super::descriptor::evaluate_feature_descriptors;
