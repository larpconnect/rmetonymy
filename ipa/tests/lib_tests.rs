use ipa::IpaSystem;

#[test]
fn test_get_phoneme_data_not_found() {
    let json_data = r"{}";
    let system = IpaSystem::new(json_data).expect("failed to unwrap");
    assert!(system.get_phoneme_data("p").is_none());
}

#[test]
fn test_get_phoneme_data_modifier() {
    let json_data = r#"{
        "h": {
            "type": "modifier",
            "added_features": ["+aspirated"]
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
            "added_features": ["+aspirated"]
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
            "features": ["-voice"]
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
            "features": ["-voice"]
        },
        "t": {
            "type": "consonant",
            "features": ["-voice"]
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
            "features": ["-voice", "+bilabial"]
        },
        "h": {
            "type": "modifier",
            "added_features": ["+aspirated"],
            "removed_features": ["-voice"]
        }
    }"#;
    let system = ipa::IpaSystem::new(json_data).expect("failed to unwrap");
    let combined = system
        .combine_with_modifier("p", "h")
        .expect("Combination should succeed");

    assert!(
        combined
            .iter()
            .any(|f| matches!(f, data::SpeFeature::Plus(s) if s == "bilabial"))
    );
    assert!(
        combined
            .iter()
            .any(|f| matches!(f, data::SpeFeature::Plus(s) if s == "aspirated"))
    );
    assert!(
        !combined
            .iter()
            .any(|f| matches!(f, data::SpeFeature::Minus(s) if s == "voice"))
    );
}
