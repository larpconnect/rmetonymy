use data::{SpeFeature, feature::Feature, parse_and_validate, validate_ipa_data};
use serde_json::json;

#[test]
fn test_spe_feature_deserialize_valid() {
    let plus_feature: SpeFeature =
        serde_json::from_value(json!("+nasal")).expect("Valid '+nasal' feature should deserialize");
    assert_eq!(plus_feature, SpeFeature::Plus(Feature::Nasal));

    let minus_feature: SpeFeature =
        serde_json::from_value(json!("-voice")).expect("failed to unwrap");
    assert_eq!(minus_feature, SpeFeature::Minus(Feature::Voice));
}

#[test]
fn test_spe_feature_deserialize_invalid() {
    let result: Result<SpeFeature, _> = serde_json::from_value(json!("nasal"));
    result
        .expect_err("Deserializing a plain string as SpeFeature should fail (missing +/- prefix)");
}

#[test]
fn test_spe_feature_serialize() {
    let plus_feature = SpeFeature::Plus(Feature::Nasal);
    assert_eq!(
        serde_json::to_value(plus_feature).expect("failed to unwrap"),
        json!("+nasal")
    );

    let minus_feature = SpeFeature::Minus(Feature::Voice);
    assert_eq!(
        serde_json::to_value(minus_feature).expect("failed to unwrap"),
        json!("-voice")
    );
}

#[test]
fn test_spe_feature_from_str() {
    assert_eq!(
        "+nasal".parse::<SpeFeature>().expect("failed to unwrap"),
        SpeFeature::Plus(Feature::Nasal)
    );
    assert_eq!(
        "-voice".parse::<SpeFeature>().expect("failed to unwrap"),
        SpeFeature::Minus(Feature::Voice)
    );
    "invalid".parse::<SpeFeature>().expect_err("expected error");
}

#[test]
fn test_spe_feature_display() {
    assert_eq!(format!("{}", SpeFeature::Plus(Feature::Nasal)), "+nasal");
    assert_eq!(format!("{}", SpeFeature::Minus(Feature::Voice)), "-voice");
}

#[test]
fn test_parse_and_validate_success() {
    let json_str = r#"{
        "p": {
            "type": "consonant",
            "features": ["-voice", "+bilabial", "+stop"]
        }
    }"#;
    let result = parse_and_validate(json_str);
    result.expect("Valid IPA JSON should be successfully parsed and validated");
}

#[test]
fn test_parse_and_validate_invalid_json() {
    let result = parse_and_validate("{ invalid json ");
    result.expect_err("expected error");
}

#[test]
fn test_parse_and_validate_invalid_schema() {
    let json_str = r#"{
        "p": {
            "type": "invalid_type",
            "features": ["-voice"]
        }
    }"#;
    let result = parse_and_validate(json_str);
    result.expect_err("expected error");
}

#[test]
fn test_validate_ipa_data_invalid() {
    let invalid_data = json!({
        "p": {
            "type": "invalid_type"
        }
    });
    let result = validate_ipa_data(&invalid_data);
    result.expect_err("expected error");
}

#[test]
fn test_parse_and_validate_deserialization_error() {
    let json_str = r#"{
        "p": {
            "type": "consonant",
            "features": ["invalid"]
        }
    }"#;
    let result = parse_and_validate(json_str);
    result.expect_err("expected error");
}

#[test]
fn test_validate_ipa_data_multiple_errors() {
    let invalid_data = json!({
        "p": {
            "type": "invalid_type"
        },
        "t": {
            "type": "another_invalid"
        }
    });
    let result = validate_ipa_data(&invalid_data);
    let errs = result.expect_err("expected err");
    assert!(errs.len() > 1);
    assert!(errs.iter().any(|e| e.contains("invalid_type")));
    assert!(errs.iter().any(|e| e.contains("another_invalid")));
}
