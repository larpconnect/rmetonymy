use crate::config::LanguageConfig;
use data::IpaEntry;
use ipa::get_entry;
use ipa::sequence::Phoneme;

/// Check if a phoneme is a vowel.
#[must_use]
pub fn is_vowel(p: &Phoneme) -> bool {
    matches!(get_entry(&p.base), Some(IpaEntry::Vowel(_)))
}

fn is_breve_or_tie(s: &str) -> bool {
    s.contains('\u{0361}')     // tie bar
        || s.contains('\u{035C}') // tie bar below
        || s.contains('\u{032F}') // inverted breve below
        || s.contains('\u{0311}') // inverted breve above
}

/// Check if a phoneme has a tie bar or breve modifier.
#[must_use]
pub fn is_tied(p: &Phoneme) -> bool {
    is_breve_or_tie(&p.base) || p.modifiers.iter().any(|m| is_breve_or_tie(m))
}

/// Check if two phonemes form a diphthong.
#[must_use]
pub fn are_diphthong(p1: &Phoneme, p2: &Phoneme) -> bool {
    is_vowel(p1) && is_vowel(p2) && (is_tied(p1) || is_tied(p2))
}

/// Check if a phoneme has a specific SPE feature.
#[must_use]
pub fn has_feature(p: &Phoneme, target: data::feature::Feature) -> bool {
    let base_entry = get_entry(&p.base);
    let mut features = match base_entry {
        Some(IpaEntry::Vowel(d) | IpaEntry::Consonant(d) | IpaEntry::Phoneme(d)) => {
            d.features.clone()
        }
        _ => Vec::new(),
    };

    for modifier in &p.modifiers {
        if let Some(IpaEntry::Modifier(mod_data)) = get_entry(modifier) {
            features.retain(|f| !mod_data.removed_features.contains(f));
            for new_f in &mod_data.added_features {
                if !features.contains(new_f) {
                    features.push(new_f.clone());
                }
            }
        }
    }

    features.iter().any(|feat| match feat {
        data::SpeFeature::Plus(f) => *f == target,
        data::SpeFeature::Minus(_) => false,
    })
}

/// Get the sonority score of a phoneme.
#[must_use]
pub fn get_sonority(p: &Phoneme) -> i32 {
    match get_entry(&p.base) {
        Some(IpaEntry::Vowel(d) | IpaEntry::Consonant(d) | IpaEntry::Phoneme(d)) => d.sonority,
        _ => 0,
    }
}

/// Check if a vowel is a candidate for capture (not long, not rhotic).
#[must_use]
#[allow(dead_code)]
pub fn can_vowel_capture(v: &Phoneme) -> bool {
    is_vowel(v)
        && !has_feature(v, data::feature::Feature::Long)
        && !has_feature(v, data::feature::Feature::Rhotic)
}

/// Check if the proposed onset is illegal under the language configuration.
#[must_use]
pub fn is_illegal_onset(onset: &[Phoneme], is_word_initial: bool, config: &LanguageConfig) -> bool {
    if onset.is_empty() {
        return false;
    }
    let onset_str: String = onset.iter().map(std::string::ToString::to_string).collect();
    let test_str = if is_word_initial {
        onset_str
    } else {
        format!("a.{onset_str}")
    };

    for pattern in &config.phonology.illegal_patterns {
        let has_boundary = pattern.elements.iter().any(|el| {
            matches!(
                el.base,
                crate::matcher::BaseElement::WordBoundary
                    | crate::matcher::BaseElement::SyllableBoundary
            )
        });
        if has_boundary && pattern.matches(&test_str, &config.phonology.sound_classes) {
            return true;
        }
    }
    false
}
