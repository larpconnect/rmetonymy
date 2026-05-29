use ipa::IpaString;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Include the JSON schema directly at compile time.
pub const DICTIONARY_SCHEMA_JSON: &str = include_str!("../dictionary.schema.json");

/// Helper to generate a new Base62 representation of a `UUIDv7`.
#[must_use]
pub fn generate_base62_uuid() -> String {
    let uuid = Uuid::now_v7();
    base62::encode(uuid.as_u128())
}

/// Helper to decode a Base62 representation of a `UUIDv7`.
///
/// # Errors
/// Returns an error if the base62 decoding or UUID construction fails.
#[allow(dead_code)]
pub fn parse_base62_uuid(s: &str) -> Result<Uuid, String> {
    let val = base62::decode(s).map_err(|e| format!("Invalid Base62 UUID: {e}"))?;
    Ok(Uuid::from_u128(val))
}

/// Validate a JSON value against the dictionary schema.
///
/// # Errors
/// Returns a list of error strings if validation fails.
pub fn validate_dictionary_data(data: &serde_json::Value) -> Result<(), String> {
    static VALIDATOR: std::sync::LazyLock<Result<jsonschema::Validator, String>> =
        std::sync::LazyLock::new(|| {
            let schema_json: serde_json::Value = serde_json::from_str(DICTIONARY_SCHEMA_JSON)
                .map_err(|e| format!("Failed to parse schema JSON: {e}"))?;
            jsonschema::validator_for(&schema_json)
                .map_err(|e| format!("Failed to compile JSON Schema: {e}"))
        });

    data::validate_with_schema(data, &VALIDATOR)
}

/// Helper function to write content atomically to a file.
///
/// # Errors
/// Returns a standard IO error if any operation fails.
pub fn atomic_write<P: AsRef<Path>>(path: P, content: &[u8]) -> std::io::Result<()> {
    let path = path.as_ref();
    let dir = path.parent().unwrap_or(Path::new(""));

    // Generate a unique temporary file name in the same directory
    let file_name = match path.file_name() {
        Some(s) => s.to_string_lossy().into_owned(),
        None => "dictionary.json".to_string(),
    };

    let temp_name = format!(".{}.tmp-{}", file_name, Uuid::now_v7());
    let temp_path = if dir.as_os_str().is_empty() {
        PathBuf::from(temp_name)
    } else {
        dir.join(temp_name)
    };

    // Write contents to temporary file
    let write_result = (|| -> std::io::Result<()> {
        let mut file = File::create(&temp_path)?;
        file.write_all(content)?;
        file.sync_all()?;
        Ok(())
    })();

    if let Err(e) = write_result {
        let _ignored = std::fs::remove_file(&temp_path);
        return Err(e);
    }

    // Rename temporary file to target path (atomic rename)
    if let Err(e) = std::fs::rename(&temp_path, path) {
        let _ignored = std::fs::remove_file(&temp_path);
        return Err(e);
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DictionaryEntry {
    pub id: String,
    pub meaning: IpaString,
    pub definition: IpaString,
    #[serde(rename = "type")]
    pub r#type: String,
    pub era: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etymology: Option<BTreeMap<u32, Vec<String>>>,
    pub usage_notes: String,
}

/// Data for a new dictionary entry to be added.
#[derive(Debug, Clone)]
pub struct NewEntry {
    pub meaning: IpaString,
    pub definition: IpaString,
    pub r#type: String,
    pub era: Option<u32>,
    pub etymology: Option<BTreeMap<u32, Vec<String>>>,
    pub usage_notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Era {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<IpaString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// qual:allow(srp) - Struct handles entry adding, removing, and serialization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dictionary {
    pub id: Uuid,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub eras: BTreeMap<u32, Era>,
    pub entries: Vec<DictionaryEntry>,
}

impl std::str::FromStr for Dictionary {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let val: serde_json::Value =
            serde_json::from_str(s).map_err(|e| format!("Failed to parse JSON: {e}"))?;

        validate_dictionary_data(&val)
            .map_err(|errs| format!("Schema validation failed:\n{errs}"))?;

        let dict: Dictionary = serde_json::from_value(val)
            .map_err(|e| format!("Failed to deserialize Dictionary: {e}"))?;

        Ok(dict)
    }
}

impl Dictionary {
    /// Create a new blank dictionary for a language configuration.
    #[must_use]
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            eras: BTreeMap::new(),
            entries: Vec::new(),
        }
    }

    /// Serialize the dictionary to a pretty JSON string.
    ///
    /// # Errors
    /// Returns serialization errors.
    pub fn to_string(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize Dictionary: {e}"))
    }

    /// Save the dictionary atomically to a file path.
    ///
    /// # Errors
    /// Returns an error if serialization or filesystem writes fail.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let content = self.to_string()?;
        atomic_write(path, content.as_bytes())
            .map_err(|e| format!("Failed to save dictionary atomically: {e}"))
    }

    /// Add a new era to the dictionary.
    ///
    /// # Errors
    /// Returns an error if the specified era number already exists.
    pub fn add_era(
        &mut self,
        era_num: Option<u32>,
        name: Option<IpaString>,
        description: Option<String>,
    ) -> Result<(u32, String), String> {
        let num = era_num.unwrap_or_else(|| {
            self.eras
                .keys()
                .next_back()
                .copied()
                .or_else(|| self.entries.iter().map(|e| e.era).max())
                .map_or(0, |k| k + 1)
        });

        if self.eras.contains_key(&num) {
            return Err(format!("Era number {num} already exists in the dictionary"));
        }

        let id = generate_base62_uuid();
        let era = Era {
            id: id.clone(),
            name,
            description,
        };

        self.eras.insert(num, era);
        Ok((num, id))
    }

    /// Add a new word entry to the dictionary, returning the generated Base62 ID.
    pub fn add_entry(&mut self, entry: NewEntry) -> String {
        let id = generate_base62_uuid();
        let era = entry.era.unwrap_or_else(|| {
            self.eras
                .keys()
                .next_back()
                .copied()
                .or_else(|| self.entries.iter().map(|e| e.era).max())
                .unwrap_or(0)
        });

        self.eras.entry(era).or_insert_with(|| Era {
            id: generate_base62_uuid(),
            name: None,
            description: None,
        });

        let entry = DictionaryEntry {
            id: id.clone(),
            meaning: entry.meaning,
            definition: entry.definition,
            r#type: entry.r#type,
            era,
            etymology: entry.etymology,
            usage_notes: entry.usage_notes,
        };
        self.entries.push(entry);
        id
    }

    /// Remove a word entry from the dictionary by its Base62 ID.
    /// Returns true if the entry was found and removed, false otherwise.
    pub fn remove_entry(&mut self, id: &str) -> bool {
        let original_len = self.entries.len();
        self.entries.retain(|entry| entry.id != id);
        self.entries.len() < original_len
    }
}

/// Helper to check if a dictionary entry's type matches a filter type.
pub fn type_matches(entry_type: &str, filter_type: &str) -> bool {
    let (w_base, w_sub) = entry_type.split_once('.').unwrap_or((entry_type, ""));
    let (f_base, f_sub) = filter_type.split_once('.').unwrap_or((filter_type, ""));

    if w_base != f_base {
        return false;
    }
    if !f_sub.is_empty() && w_sub != f_sub {
        return false;
    }
    true
}
