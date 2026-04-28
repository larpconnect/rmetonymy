use data::{SpeFeature, parse_and_validate, validate_ipa_data};
use serde_json::json;

#[test]
fn test_spe_feature_deserialize_valid() {
    let plus_feature: SpeFeature = serde_json::from_value(json!("+nasal")).unwrap();
    assert_eq!(plus_feature, SpeFeature::Plus("nasal".to_string()));

    let minus_feature: SpeFeature = serde_json::from_value(json!("-voice")).unwrap();
    assert_eq!(minus_feature, SpeFeature::Minus("voice".to_string()));
}

#[test]
fn test_spe_feature_deserialize_invalid() {
    let result: Result<SpeFeature, _> = serde_json::from_value(json!("nasal"));
    assert!(result.is_err());
}

#[test]
fn test_spe_feature_serialize() {
    let plus_feature = SpeFeature::Plus("nasal".to_string());
    assert_eq!(serde_json::to_value(plus_feature).unwrap(), json!("+nasal"));

    let minus_feature = SpeFeature::Minus("voice".to_string());
    assert_eq!(
        serde_json::to_value(minus_feature).unwrap(),
        json!("-voice")
    );
}

#[test]
fn test_spe_feature_from_str() {
    assert_eq!(
        "+nasal".parse::<SpeFeature>().unwrap(),
        SpeFeature::Plus("nasal".to_string())
    );
    assert_eq!(
        "-voice".parse::<SpeFeature>().unwrap(),
        SpeFeature::Minus("voice".to_string())
    );
    assert!("invalid".parse::<SpeFeature>().is_err());
}

#[test]
fn test_spe_feature_display() {
    assert_eq!(
        format!("{}", SpeFeature::Plus("nasal".to_string())),
        "+nasal"
    );
    assert_eq!(
        format!("{}", SpeFeature::Minus("voice".to_string())),
        "-voice"
    );
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
    assert!(result.is_ok());
}

#[test]
fn test_parse_and_validate_invalid_json() {
    let result = parse_and_validate("{ invalid json ");
    assert!(result.is_err());
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
    assert!(result.is_err());
}

#[test]
fn test_validate_ipa_data_invalid() {
    let invalid_data = json!({
        "p": {
            "type": "invalid_type"
        }
    });
    let result = validate_ipa_data(&invalid_data);
    assert!(result.is_err());
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
    assert!(result.is_err());
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
    assert!(result.is_err());
}
