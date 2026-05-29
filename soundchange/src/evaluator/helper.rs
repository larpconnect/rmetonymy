use crate::evaluator::EvalContext;
use crate::evaluator::features::get_phoneme_features_map_from_data;
use data::feature::Feature;
use std::collections::HashMap;

const MISMATCH_PENALTY: usize = 100;

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
            diff += MISMATCH_PENALTY;
        }
        if !target_manner.is_empty() && phoneme_data.manner != target_manner {
            diff += MISMATCH_PENALTY;
        }

        if diff < min_diff {
            min_diff = diff;
            best_base = Some(sym.clone());
        }
    }

    best_base.ok_or_else(|| "No phoneme base matches feature changes".to_string())
}
