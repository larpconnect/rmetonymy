use crate::ast::{
    FeatureClassKey, FeatureDescriptor, MatchBase, MatchElement, MatchPattern, MatchQuantifier,
};
use crate::evaluator::engine::evaluate_match;
use crate::evaluator::{CapturedAlpha, EvalContext, MatchState, WorkingWord};
use data::feature::Feature;
use ipa::IpaSequence;
use ipa::sequence::Phoneme;
use std::collections::HashMap;

pub struct MatchRepeatedContext<'a, 'b, 'c> {
    pub base: &'a MatchBase,
    pub wildcard: bool,
    pub word: &'a WorkingWord,
    pub ctx: &'b EvalContext<'c>,
    pub results: &'a mut Vec<(usize, MatchState, std::ops::Range<usize>)>,
}

pub(crate) fn get_match_element_lengths(
    el: &MatchElement,
    word: &WorkingWord,
    word_idx: usize,
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Vec<(usize, MatchState, std::ops::Range<usize>)> {
    let mut results = Vec::new();
    let bounds = match &el.quantifier {
        MatchQuantifier::None => None,
        MatchQuantifier::ZeroOrMore => Some((0, usize::MAX)),
        MatchQuantifier::OneOrMore => Some((1, usize::MAX)),
        MatchQuantifier::ZeroOrMoreBounded(limit) => Some((0, *limit as usize)),
        MatchQuantifier::OneOrMoreBounded(limit) => Some((1, *limit as usize)),
    };

    if let Some((min, max)) = bounds {
        let mut context = MatchRepeatedContext {
            base: &el.base,
            wildcard: el.modifiers_wildcard,
            word,
            ctx,
            results: &mut results,
        };
        match_repeated(&mut context, word_idx, min, max, 0, state);
    } else {
        for (len, next_state) in
            match_base(&el.base, el.modifiers_wildcard, word, word_idx, state, ctx)
        {
            results.push((len, next_state, word_idx..word_idx + len));
        }
    }
    results
}

pub(crate) fn match_repeated(
    context: &mut MatchRepeatedContext<'_, '_, '_>,
    word_idx: usize,
    min: usize,
    max: usize,
    current_len: usize,
    state: &MatchState,
) {
    if min == 0 {
        context
            .results
            .push((current_len, state.clone(), word_idx - current_len..word_idx));
    }
    if max > 0 && word_idx < context.word.phonemes.len() {
        for (len, next_state) in match_base(
            context.base,
            context.wildcard,
            context.word,
            word_idx,
            state,
            context.ctx,
        ) {
            if len > 0 {
                match_repeated(
                    context,
                    word_idx + len,
                    min.saturating_sub(1),
                    max - 1,
                    current_len + len,
                    &next_state,
                );
            }
        }
    }
}

pub(crate) fn match_base(
    base: &MatchBase,
    wildcard: bool,
    word: &WorkingWord,
    word_idx: usize,
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Vec<(usize, MatchState)> {
    match base {
        MatchBase::WordBoundary => match_word_boundary(word, word_idx, state),
        MatchBase::SyllableBoundary => match_syllable_boundary(word, word_idx, state),
        MatchBase::SoundClass { key, marker } => {
            match_sound_class(key, *marker, wildcard, word, word_idx, state, ctx)
        }
        MatchBase::SetExclusion { key, marker } => {
            match_set_exclusion(key, *marker, wildcard, word, word_idx, state, ctx)
        }
        MatchBase::IpaSequence(ipa) => {
            match_ipa_sequence_element(ipa, wildcard, word, word_idx, state)
        }
        MatchBase::FeatureClass { key_opt, features } => match_feature_class(
            key_opt.as_ref(),
            features,
            wildcard,
            word,
            word_idx,
            state,
            ctx,
        ),
        MatchBase::Set(bases) => match_set_element(bases, wildcard, word, word_idx, state, ctx),
        MatchBase::OptionalGroup(pat) => match_optional_group(pat, word, word_idx, state, ctx),
    }
}

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

pub(crate) fn match_sound_class(
    key: &language::sound_class::SoundClassKey,
    marker: Option<u8>,
    wildcard: bool,
    word: &WorkingWord,
    word_idx: usize,
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Vec<(usize, MatchState)> {
    if let Some(p) = word.phonemes.get(word_idx) {
        if phoneme_in_class(p, key, wildcard, ctx) {
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

pub(crate) fn match_set_exclusion(
    key: &language::sound_class::SoundClassKey,
    marker: Option<u8>,
    wildcard: bool,
    word: &WorkingWord,
    word_idx: usize,
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Vec<(usize, MatchState)> {
    if let Some(p) = word.phonemes.get(word_idx) {
        if phoneme_in_class(p, key, wildcard, ctx) {
            vec![]
        } else {
            let mut next_state = state.clone();
            bind_marker(
                Some(key.clone()),
                marker,
                word_idx..word_idx + 1,
                &mut next_state,
            );
            vec![(1, next_state)]
        }
    } else {
        vec![]
    }
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
    wildcard: bool,
    word: &WorkingWord,
    word_idx: usize,
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Vec<(usize, MatchState)> {
    let Some(p) = word.phonemes.get(word_idx) else {
        return vec![];
    };
    let mut next_state = state.clone();
    if let Some(key) = key_opt {
        if let Some(class_key) = &key.key {
            let in_class = phoneme_in_class(p, class_key, wildcard, ctx);
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
    if evaluate_feature_descriptors(features, p, word_idx, word.stress_index, &mut next_state) {
        vec![(1, next_state)]
    } else {
        vec![]
    }
}

pub(crate) fn match_set_element(
    bases: &[MatchBase],
    wildcard: bool,
    word: &WorkingWord,
    word_idx: usize,
    state: &MatchState,
    ctx: &EvalContext<'_>,
) -> Vec<(usize, MatchState)> {
    let mut results = Vec::new();
    for b in bases {
        results.extend(match_base(b, wildcard, word, word_idx, state, ctx));
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
        if wp.base != tp.base {
            return None;
        }
        if wildcard {
            for m in &tp.modifiers {
                if !wp.modifiers.contains(m) {
                    return None;
                }
            }
        } else if wp.modifiers != tp.modifiers {
            return None;
        }
    }
    Some(target_phonemes.len())
}

pub(crate) fn evaluate_place_manner_descriptor(
    fd: &FeatureDescriptor,
    p: &Phoneme,
    state: &mut MatchState,
) -> bool {
    let phoneme_strings = if let Some(data) = ipa::get_phoneme_data(&p.base) {
        if fd.feature == Feature::Place {
            data.place.clone()
        } else {
            data.manner.clone()
        }
    } else {
        Vec::new()
    };

    if let Some(ref alpha) = fd.alpha {
        match state.alpha.get(&alpha.name) {
            Some(CapturedAlpha::Strings(s)) => phoneme_strings == *s,
            None => {
                state
                    .alpha
                    .insert(alpha.name.clone(), CapturedAlpha::Strings(phoneme_strings));
                true
            }
            _ => false,
        }
    } else {
        false
    }
}

pub(crate) fn evaluate_standard_descriptor(
    fd: &FeatureDescriptor,
    _p: &Phoneme,
    word_idx: usize,
    stress_idx: Option<usize>,
    p_features: &HashMap<Feature, bool>,
    state: &mut MatchState,
) -> bool {
    let target_sign = if let Some(ref alpha) = fd.alpha {
        match state.alpha.get(&alpha.name) {
            Some(CapturedAlpha::Sign(s)) => {
                if alpha.sign {
                    !s
                } else {
                    *s
                }
            }
            None => {
                let captured_val = if fd.feature == Feature::Stress {
                    stress_idx == Some(word_idx)
                } else {
                    *p_features.get(&fd.feature).unwrap_or(&false)
                };
                state
                    .alpha
                    .insert(alpha.name.clone(), CapturedAlpha::Sign(captured_val));
                if alpha.sign {
                    !captured_val
                } else {
                    captured_val
                }
            }
            _ => return false,
        }
    } else {
        fd.sign
    };

    if fd.feature == Feature::Stress {
        let is_stressed = stress_idx == Some(word_idx);
        is_stressed == target_sign
    } else {
        let val = *p_features.get(&fd.feature).unwrap_or(&false);
        val == target_sign
    }
}

pub(crate) fn evaluate_feature_descriptors(
    features: &[FeatureDescriptor],
    p: &Phoneme,
    word_idx: usize,
    stress_idx: Option<usize>,
    state: &mut MatchState,
) -> bool {
    let p_features = get_phoneme_features_map(p);
    for fd in features {
        if fd.feature == Feature::Place || fd.feature == Feature::Manner {
            if !evaluate_place_manner_descriptor(fd, p, state) {
                return false;
            }
        } else if !evaluate_standard_descriptor(fd, p, word_idx, stress_idx, &p_features, state) {
            return false;
        }
    }
    true
}

pub(crate) fn get_phoneme_features_map(p: &Phoneme) -> HashMap<Feature, bool> {
    let mut map = HashMap::new();
    let mut features = if let Some(data) = ipa::get_phoneme_data(&p.base) {
        data.features.clone()
    } else {
        Vec::new()
    };

    for modifier in &p.modifiers {
        if let Some(combined) = ipa::combine_with_modifier(&p.base, modifier) {
            features = combined;
        }
    }

    for sf in features {
        match sf {
            data::SpeFeature::Plus(f) => {
                map.insert(f, true);
            }
            data::SpeFeature::Minus(f) => {
                map.insert(f, false);
            }
        }
    }
    map
}

pub(crate) fn get_phoneme_features_map_from_data(d: &data::PhonemeData) -> HashMap<Feature, bool> {
    let mut map = HashMap::new();
    for sf in &d.features {
        match sf {
            data::SpeFeature::Plus(f) => {
                map.insert(*f, true);
            }
            data::SpeFeature::Minus(f) => {
                map.insert(*f, false);
            }
        }
    }
    map
}
