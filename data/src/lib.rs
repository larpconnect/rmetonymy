use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

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
    pub features: Vec<String>,
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
    pub added_features: Vec<String>,
    #[serde(default)]
    pub removed_features: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

/// The overall structured dictionary representing the IPA mapping.
pub type IpaDataset = HashMap<String, IpaEntry>;

/// Include the JSON schema directly at compile time.
pub const IPA_SCHEMA_JSON: &str = include_str!("../ipa_schema.json");

/// Validate a JSON value against the IPA schema.
pub fn validate_ipa_data(data: &Value) -> Result<(), Vec<String>> {
    let schema_json: Value =
        serde_json::from_str(IPA_SCHEMA_JSON).expect("Compiled schema must be valid JSON");

    let schema = jsonschema::validator_for(&schema_json)
        .expect("Compiled schema must be a valid JSON Schema");

    if let Err(errors) = schema.validate(data) {
        // Validation returns a single ValidationError or ValidationErrors wrapper.
        // We will just use its Display implementation.
        return Err(vec![errors.to_string()]);
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
