use data::feature::Feature;
use ipa::sequence::Phoneme;
use std::collections::HashMap;

fn apply_single_modifier_op(features: &mut Vec<data::SpeFeature>, mod_data: &data::ModifierData) {
    if !mod_data.removed_features.is_empty() {
        let removed_set: std::collections::HashSet<_> = mod_data.removed_features.iter().collect();
        features.retain(|f| !removed_set.contains(f));
    }
    for new_f in &mod_data.added_features {
        if !features.contains(new_f) {
            features.push(new_f.clone());
        }
    }
}

fn apply_modifiers_op(
    mut features: Vec<data::SpeFeature>,
    modifiers: &[String],
) -> Vec<data::SpeFeature> {
    for modifier in modifiers {
        if let Some(data::IpaEntry::Modifier(mod_data)) = ipa::get_entry(modifier) {
            apply_single_modifier_op(&mut features, mod_data);
        }
    }
    features
}

pub(crate) fn get_phoneme_features_map(p: &Phoneme) -> HashMap<Feature, bool> {
    let mut map = HashMap::new();
    let initial_features = ipa::get_phoneme_data(&p.base)
        .map(|data| data.features.clone())
        .unwrap_or_default();

    let features = apply_modifiers_op(initial_features, &p.modifiers);

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
