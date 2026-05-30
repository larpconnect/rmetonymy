use crate::ast::FeatureDescriptor;
use crate::evaluator::{CapturedAlpha, MatchState};
use crate::evaluator::features::get_phoneme_features_map;
use data::feature::Feature;
use ipa::sequence::Phoneme;
use std::collections::HashMap;

pub(crate) fn evaluate_place_manner_descriptor(
    fd: &FeatureDescriptor,
    p: &Phoneme,
    state: &mut MatchState,
) -> bool {
    let Some(ref alpha) = fd.alpha else {
        return false;
    };

    let phoneme_strings = if let Some(data) = ipa::get_phoneme_data(&p.base) {
        if fd.feature == Feature::Place {
            &data.place
        } else {
            &data.manner
        }
    } else {
        &[] as &[String]
    };

    match state.alpha.get(&alpha.name) {
        Some(CapturedAlpha::Strings(s)) => phoneme_strings == s.as_slice(),
        None => {
            state
                .alpha
                .insert(alpha.name.clone(), CapturedAlpha::Strings(phoneme_strings.to_vec()));
            true
        }
        _ => false,
    }
}

pub(crate) fn evaluate_standard_descriptor(
    fd: &FeatureDescriptor,
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
        } else if !evaluate_standard_descriptor(fd, word_idx, stress_idx, &p_features, state) {
            return false;
        }
    }
    true
}
