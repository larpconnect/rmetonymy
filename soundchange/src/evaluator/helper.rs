use crate::evaluator::EvalContext;
use crate::evaluator::features::get_phoneme_features_map_from_data;
use data::feature::Feature;
use std::collections::HashMap;

const MISMATCH_PENALTY: usize = 100;

fn calculate_features_diff_op(
    phoneme_feats: &HashMap<Feature, bool>,
    target_features: &HashMap<Feature, bool>,
) -> usize {
    let mut diff = 0;
    for (&feat, &target_val) in target_features {
        let val = *phoneme_feats.get(&feat).unwrap_or(&false);
        if val != target_val {
            diff += 1;
        }
    }
    for (&feat, &val) in phoneme_feats {
        if val && !target_features.contains_key(&feat) {
            diff += 1;
        }
    }
    diff
}

fn calculate_place_manner_penalty_op(
    phoneme_data: &data::PhonemeData,
    target_place: &[String],
    target_manner: &[String],
) -> usize {
    let mut penalty = 0;
    if !target_place.is_empty() && phoneme_data.place != target_place {
        penalty += MISMATCH_PENALTY;
    }
    if !target_manner.is_empty() && phoneme_data.manner != target_manner {
        penalty += MISMATCH_PENALTY;
    }
    penalty
}

fn calculate_phoneme_diff_op(
    phoneme_data: &data::PhonemeData,
    target_features: &HashMap<Feature, bool>,
    target_place: &[String],
    target_manner: &[String],
) -> usize {
    let phoneme_feats = get_phoneme_features_map_from_data(phoneme_data);
    let feat_diff = calculate_features_diff_op(&phoneme_feats, target_features);
    let penalty = calculate_place_manner_penalty_op(phoneme_data, target_place, target_manner);
    feat_diff + penalty
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

        let diff =
            calculate_phoneme_diff_op(phoneme_data, target_features, target_place, target_manner);

        if diff < min_diff {
            min_diff = diff;
            best_base = Some(sym.clone());
        }
    }

    best_base.ok_or_else(|| "No phoneme base matches feature changes".to_string())
}
