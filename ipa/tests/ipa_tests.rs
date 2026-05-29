// qual:allow(srp) - Test module containing multiple tests
use data::{SpeFeature, feature::Feature};
use ipa::IpaSystem;

const DUMMY_DATA: &str = include_str!("dummy_ipa_data.json");

#[test]
fn test_resolve_alias() {
    let system = IpaSystem::new(DUMMY_DATA).expect("Failed to initialize IPA system");

    // "g" is an alias for "ɡ"
    assert_eq!(system.resolve_alias("g"), Some("ɡ"));

    // "ʧ" and "t͜ʃ" are aliases for "t͡ʃ"
    assert_eq!(system.resolve_alias("ʧ"), Some("t͡ʃ"));
    assert_eq!(system.resolve_alias("t͜ʃ"), Some("t͡ʃ"));

    // Canonical symbols resolve to themselves
    assert_eq!(system.resolve_alias("n"), Some("n"));
    assert_eq!(system.resolve_alias("t͡ʃ"), Some("t͡ʃ"));
    assert_eq!(system.resolve_alias("ɫ"), Some("ɫ"));
}

#[test]
fn test_get_features() {
    let system = IpaSystem::new(DUMMY_DATA).expect("Failed to initialize IPA system");

    let n_data = system
        .get_phoneme_data("n")
        .expect("Phoneme 'n' should be present in the IPA system");
    assert_eq!(
        n_data.features,
        vec![
            SpeFeature::Plus(Feature::Nasal),
            SpeFeature::Plus(Feature::Voiced),
            SpeFeature::Plus(Feature::Coronal),
            SpeFeature::Plus(Feature::Anterior)
        ]
    );
    assert_eq!(n_data.place, vec!["alveolar"]);
    assert_eq!(n_data.manner, vec!["nasal"]);

    // Alias works for getting features
    let g_data = system.get_phoneme_data("g").expect("failed to unwrap");
    assert_eq!(
        g_data.features,
        vec![
            SpeFeature::Minus(Feature::Nasal),
            SpeFeature::Plus(Feature::Voiced),
            SpeFeature::Minus(Feature::Coronal),
            SpeFeature::Plus(Feature::High),
            SpeFeature::Plus(Feature::Back)
        ]
    );
    assert_eq!(g_data.place, vec!["velar"]);
    assert_eq!(g_data.manner, vec!["stop"]);
}

#[test]
fn test_combine_with_modifier() {
    let system = IpaSystem::new(DUMMY_DATA).expect("Failed to initialize IPA system");

    // Combine 'n' with '˜'
    let combined_features = system
        .combine_with_modifier("n", "˜")
        .expect("Failed to combine");

    // Original 'n' features were: ["+nasal", "+voiced", "+coronal", "+anterior"]
    // Added by '˜': ["+nasalized"]
    assert!(combined_features.contains(&SpeFeature::Plus(Feature::Nasal)));
    assert!(combined_features.contains(&SpeFeature::Plus(Feature::Nasalized)));

    // Combine using aliases
    let combined_features_alias = system
        .combine_with_modifier("g", "~")
        .expect("Failed to combine with aliases");

    // Original 'ɡ' features were: ["-nasal", "+voiced", "-coronal", "+high", "+back"]
    // Added by '~' (which is alias for '˜'): ["+nasalized"]
    assert!(combined_features_alias.contains(&SpeFeature::Minus(Feature::Nasal)));
    assert!(combined_features_alias.contains(&SpeFeature::Plus(Feature::Nasalized)));
}
