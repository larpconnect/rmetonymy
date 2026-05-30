pub mod feature;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpeFeature {
    Plus(crate::feature::Feature),
    Minus(crate::feature::Feature),
}

impl<'de> Deserialize<'de> for SpeFeature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if let Some(feature_str) = s.strip_prefix('+') {
            let feature = feature_str
                .parse::<crate::feature::Feature>()
                .map_err(|e| serde::de::Error::custom(e.to_string()))?;
            Ok(SpeFeature::Plus(feature))
        } else if let Some(feature_str) = s.strip_prefix('-') {
            let feature = feature_str
                .parse::<crate::feature::Feature>()
                .map_err(|e| serde::de::Error::custom(e.to_string()))?;
            Ok(SpeFeature::Minus(feature))
        } else {
            Err(serde::de::Error::custom(format!(
                "Feature {s} must start with '+' or '-'"
            )))
        }
    }
}

impl Serialize for SpeFeature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl std::str::FromStr for SpeFeature {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(feature_str) = s.strip_prefix('+') {
            let feature = feature_str
                .parse::<crate::feature::Feature>()
                .map_err(|e| format!("Invalid feature: {feature_str} - {e}"))?;
            Ok(SpeFeature::Plus(feature))
        } else if let Some(feature_str) = s.strip_prefix('-') {
            let feature = feature_str
                .parse::<crate::feature::Feature>()
                .map_err(|e| format!("Invalid feature: {feature_str} - {e}"))?;
            Ok(SpeFeature::Minus(feature))
        } else {
            Err(format!("Feature {s} must start with '+' or '-'"))
        }
    }
}

impl std::fmt::Display for SpeFeature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpeFeature::Plus(feat) => write!(f, "+{feat}"),
            SpeFeature::Minus(feat) => write!(f, "-{feat}"),
        }
    }
}

/// Representation of a single entry in the IPA dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IpaEntry {
    #[serde(rename = "phoneme")]
    Phoneme(PhonemeData),
    #[serde(rename = "vowel")]
    Vowel(PhonemeData),
    #[serde(rename = "consonant")]
    Consonant(PhonemeData),
    #[serde(rename = "modifier")]
    Modifier(ModifierData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhonemeData {
    pub sonority: i32,
    #[serde(default)]
    pub features: Vec<SpeFeature>,
    #[serde(default)]
    pub place: Vec<String>,
    #[serde(default)]
    pub manner: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifierData {
    pub sonority: i32,
    #[serde(default)]
    pub added_features: Vec<SpeFeature>,
    #[serde(default)]
    pub removed_features: Vec<SpeFeature>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

/// The overall structured dictionary representing the IPA mapping.
pub type IpaDataset = HashMap<String, IpaEntry>;

/// Include the JSON schema directly at compile time.
pub const IPA_SCHEMA_JSON: &str = include_str!("../ipa_schema.json");

/// Validate a JSON value against the IPA schema.
/// Validate a JSON value against a pre-compiled JSON schema.
///
/// # Errors
/// Returns `Err` if the validation against the schema fails.
pub fn validate_with_schema(data: &Value, validator: &jsonschema::Validator) -> Result<(), String> {
    if !validator.is_valid(data) {
        let errors = validator.iter_errors(data);
        let err_strings: Vec<String> = errors.map(|e| e.to_string()).collect();
        return Err(err_strings.join("\n"));
    }

    Ok(())
}

/// Validate a JSON value against the IPA schema.
///
/// # Errors
/// Returns `Err` if the validation against `ipa_schema.json` fails.
pub fn validate_ipa_data(data: &Value) -> Result<(), String> {
    static VALIDATOR: std::sync::LazyLock<Result<jsonschema::Validator, String>> =
        std::sync::LazyLock::new(|| {
            let schema_json: Value = serde_json::from_str(IPA_SCHEMA_JSON)
                .map_err(|e| format!("Failed to parse schema JSON: {e}"))?;
            jsonschema::validator_for(&schema_json)
                .map_err(|e| format!("Failed to compile JSON Schema: {e}"))
        });

    let validator = VALIDATOR.as_ref().map_err(String::clone)?;
    validate_with_schema(data, validator)
}

/// Helper function to parse a JSON string, validate it, and deserialize it into our structures.
///
/// # Errors
/// Returns `Err` if JSON parsing or deserialization fails, or if validation against `ipa_schema.json` fails.
pub fn parse_and_validate(json_str: &str) -> Result<IpaDataset, String> {
    let raw_data: Value =
        serde_json::from_str(json_str).map_err(|e| format!("JSON parsing error: {e}"))?;

    validate_ipa_data(&raw_data).map_err(|errs| format!("Schema validation failed:\n{errs}"))?;

    serde_json::from_value(raw_data).map_err(|e| format!("Deserialization error: {e}"))
}
