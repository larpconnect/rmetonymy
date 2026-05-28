use data::feature::Feature;
use ipa::sequence::Phoneme;
use std::collections::HashMap;

pub(crate) fn get_phoneme_features_map(p: &Phoneme) -> HashMap<Feature, bool> {
    let mut map = HashMap::new();
    let mut features = if let Some(data) = ipa::get_phoneme_data(&p.base) {
        data.features.clone()
    } else {
        Vec::new()
    };

    for modifier in &p.modifiers {
        if let Some(data::IpaEntry::Modifier(mod_data)) = ipa::get_entry(modifier) {
            // Remove explicitly removed features
            if !mod_data.removed_features.is_empty() {
                let removed_set: std::collections::HashSet<_> =
                    mod_data.removed_features.iter().collect();
                features.retain(|f| !removed_set.contains(f));
            }
            // Add new features
            for new_f in &mod_data.added_features {
                if !features.contains(new_f) {
                    features.push(new_f.clone());
                }
            }
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
