use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpeFeature {
    Plus(String),
    Minus(String),
}

impl<'de> Deserialize<'de> for SpeFeature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if let Some(feature) = s.strip_prefix('+') {
            Ok(SpeFeature::Plus(feature.to_string()))
        } else if let Some(feature) = s.strip_prefix('-') {
            Ok(SpeFeature::Minus(feature.to_string()))
        } else {
            Err(serde::de::Error::custom(format!(
                "Feature {} must start with '+' or '-'",
                s
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
        if let Some(feature) = s.strip_prefix('+') {
            Ok(SpeFeature::Plus(feature.to_string()))
        } else if let Some(feature) = s.strip_prefix('-') {
            Ok(SpeFeature::Minus(feature.to_string()))
        } else {
            Err(format!("Feature {} must start with '+' or '-'", s))
        }
    }
}

impl std::fmt::Display for SpeFeature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpeFeature::Plus(s) => write!(f, "+{}", s),
            SpeFeature::Minus(s) => write!(f, "-{}", s),
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
pub fn validate_ipa_data(data: &Value) -> Result<(), Vec<String>> {
    static VALIDATOR: std::sync::LazyLock<jsonschema::Validator> = std::sync::LazyLock::new(|| {
        let schema_json: Value =
            serde_json::from_str(IPA_SCHEMA_JSON).expect("Compiled schema must be valid JSON");
        jsonschema::validator_for(&schema_json)
            .expect("Compiled schema must be a valid JSON Schema")
    });

    if !VALIDATOR.is_valid(data) {
        let errors = VALIDATOR.iter_errors(data);
        let err_strings: Vec<String> = errors.map(|e| e.to_string()).collect();
        return Err(err_strings);
    }

    Ok(())
}

/// Helper function to parse a JSON string, validate it, and deserialize it into our structures.
pub fn parse_and_validate(json_str: &str) -> Result<IpaDataset, String> {
    let raw_data: Value =
        serde_json::from_str(json_str).map_err(|e| format!("JSON parsing error: {}", e))?;

    validate_ipa_data(&raw_data)
        .map_err(|errs| format!("Schema validation failed:\n{}", errs.join("\n")))?;

    serde_json::from_value(raw_data).map_err(|e| format!("Deserialization error: {}", e))
}
