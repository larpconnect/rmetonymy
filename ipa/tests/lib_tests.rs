use data::feature::Feature;
use ipa::IpaSystem;

#[test]
fn test_get_phoneme_data_not_found() {
    let json_data = r"{}";
    let system =
        IpaSystem::new(json_data).expect("IpaSystem should initialize correctly with empty JSON");
    assert!(system.get_phoneme_data("p").is_none());
}

#[test]
fn test_get_phoneme_data_modifier() {
    let json_data = r#"{
        "h": {
            "type": "modifier",
            "added_features": ["+aspirated"],
            "sonority": 0
        }
    }"#;
    let system = IpaSystem::new(json_data).expect("failed to unwrap");
    assert!(system.get_phoneme_data("h").is_none());
}

#[test]
fn test_combine_with_modifier_base_not_found() {
    let json_data = r#"{
        "h": {
            "type": "modifier",
            "added_features": ["+aspirated"],
            "sonority": 0
        }
    }"#;
    let system = IpaSystem::new(json_data).expect("failed to unwrap");
    assert!(system.combine_with_modifier("p", "h").is_none());
}

#[test]
fn test_combine_with_modifier_mod_not_found() {
    let json_data = r#"{
        "p": {
            "type": "consonant",
            "features": ["-voiced"],
            "sonority": 25
        }
    }"#;
    let system = IpaSystem::new(json_data).expect("failed to unwrap");
    assert!(system.combine_with_modifier("p", "h").is_none());
}

#[test]
fn test_combine_with_modifier_mod_is_not_modifier() {
    let json_data = r#"{
        "p": {
            "type": "consonant",
            "features": ["-voiced"],
            "sonority": 25
        },
        "t": {
            "type": "consonant",
            "features": ["-voiced"],
            "sonority": 25
        }
    }"#;
    let system = IpaSystem::new(json_data).expect("failed to unwrap");
    assert!(system.combine_with_modifier("p", "t").is_none());
}

#[test]
fn test_ipa_system_new_parse_error() {
    let json_data = r"{ invalid json }";
    let result = IpaSystem::new(json_data);
    assert!(result.is_err());
}

#[test]
fn test_combine_with_modifier_success() {
    let json_data = r#"{
        "p": {
            "type": "consonant",
            "features": ["-voiced", "+bilabial"],
            "sonority": 25
        },
        "h": {
            "type": "modifier",
            "added_features": ["+aspirated"],
            "removed_features": ["-voiced"],
            "sonority": 0
        }
    }"#;
    let system = ipa::IpaSystem::new(json_data).expect("failed to unwrap");
    let combined = system
        .combine_with_modifier("p", "h")
        .expect("Combination should succeed");

    assert!(
        combined
            .iter()
            .any(|f| matches!(f, data::SpeFeature::Plus(Feature::Bilabial)))
    );
    assert!(
        combined
            .iter()
            .any(|f| matches!(f, data::SpeFeature::Plus(Feature::Aspirated)))
    );
    assert!(
        !combined
            .iter()
            .any(|f| matches!(f, data::SpeFeature::Minus(Feature::Voiced)))
    );
}

#[test]
fn test_global_resolve_alias() {
    // "g" is an alias for "ɡ"
    assert_eq!(ipa::resolve_alias("g"), Some("ɡ"));
}

#[test]
fn test_global_get_entry() {
    assert!(ipa::get_entry("p").is_some());
}

#[test]
fn test_global_get_phoneme_data() {
    assert!(ipa::get_phoneme_data("p").is_some());
}

#[test]
fn test_global_combine_with_modifier() {
    assert!(ipa::combine_with_modifier("p", "ʰ").is_none());
}
