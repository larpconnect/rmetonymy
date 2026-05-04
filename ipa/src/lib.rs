use data::{IpaDataset, IpaEntry, PhonemeData, SpeFeature, parse_and_validate};
use std::collections::HashMap;
use std::sync::LazyLock;
use thiserror::Error;

pub const DEFAULT_IPA_JSON: &str = include_str!("../ipa.json");

pub static DEFAULT_SYSTEM: LazyLock<IpaSystem> =
    LazyLock::new(|| IpaSystem::new(DEFAULT_IPA_JSON).expect("DEFAULT_IPA_JSON should be valid"));

#[derive(Error, Debug)]
pub enum IpaError {
    #[error("Symbol not found: {0}")]
    NotFound(String),
    #[error("Parsing error: {0}")]
    ParseError(String),
}

#[derive(Clone, Debug)]
pub struct IpaSystem {
    dataset: IpaDataset,
    /// Maps aliases directly to their canonical symbol representations for fast O(1) lookups.
    alias_map: HashMap<String, String>,
}

impl Default for IpaSystem {
    fn default() -> Self {
        DEFAULT_SYSTEM.clone()
    }
}

impl IpaSystem {
    /// Creates a new `IpaSystem` from a JSON string.
    ///
    /// # Errors
    /// Returns `Err` if JSON parsing or validation fails.
    pub fn new(json_data: &str) -> Result<Self, IpaError> {
        let dataset = parse_and_validate(json_data).map_err(IpaError::ParseError)?;

        let mut alias_map = HashMap::new();
        for (canonical_sym, entry) in &dataset {
            let aliases = match entry {
                IpaEntry::Phoneme(d) | IpaEntry::Consonant(d) | IpaEntry::Vowel(d) => &d.aliases,
                IpaEntry::Modifier(d) => &d.aliases,
            };
            for alias in aliases {
                alias_map.insert(alias.clone(), canonical_sym.clone());
            }
        }

        Ok(Self { dataset, alias_map })
    }

    /// Resolves a symbol or alias to its canonical symbol representation.
    #[must_use]
    pub fn resolve_alias(&self, symbol: &str) -> Option<&str> {
        self.dataset
            .get_key_value(symbol)
            .map(|(k, _)| k.as_str())
            .or_else(|| self.alias_map.get(symbol).map(std::string::String::as_str))
    }

    /// Retrieves the entry for a given symbol, resolving aliases automatically.
    #[must_use]
    pub fn get_entry(&self, symbol: &str) -> Option<&IpaEntry> {
        if let Some(canonical) = self.dataset.get(symbol) {
            return Some(canonical);
        }
        let canonical_str = self.alias_map.get(symbol)?;
        self.dataset.get(canonical_str)
    }

    /// Retrieves features, place, and manner for a phoneme.
    /// Returns None if the symbol is a modifier or not found.
    #[must_use]
    pub fn get_phoneme_data(&self, symbol: &str) -> Option<&PhonemeData> {
        match self.get_entry(symbol)? {
            IpaEntry::Phoneme(data) | IpaEntry::Consonant(data) | IpaEntry::Vowel(data) => {
                Some(data)
            }
            IpaEntry::Modifier(_) => None,
        }
    }

    /// Dynamically combines a base phoneme and a modifier to produce an updated feature set.
    #[must_use]
    pub fn combine_with_modifier(&self, base: &str, modifier: &str) -> Option<Vec<SpeFeature>> {
        let base_data = self.get_phoneme_data(base)?;

        let modifier_entry = self.get_entry(modifier)?;
        let IpaEntry::Modifier(mod_data) = modifier_entry else {
            return None;
        };

        let mut features = base_data.features.clone();

        // Remove explicitly removed features
        features.retain(|f| !mod_data.removed_features.contains(f));

        // Add new features
        for new_f in &mod_data.added_features {
            if !features.contains(new_f) {
                features.push(new_f.clone());
            }
        }

        Some(features)
    }
}

/// Resolves a symbol or alias to its canonical symbol representation using the default IPA system.
#[must_use]
pub fn resolve_alias(symbol: &str) -> Option<&str> {
    DEFAULT_SYSTEM.resolve_alias(symbol)
}

/// Retrieves the entry for a given symbol, resolving aliases automatically, using the default IPA system.
#[must_use]
pub fn get_entry(symbol: &str) -> Option<&IpaEntry> {
    DEFAULT_SYSTEM.get_entry(symbol)
}

/// Retrieves features, place, and manner for a phoneme using the default IPA system.
/// Returns None if the symbol is a modifier or not found.
#[must_use]
pub fn get_phoneme_data(symbol: &str) -> Option<&PhonemeData> {
    DEFAULT_SYSTEM.get_phoneme_data(symbol)
}

/// Dynamically combines a base phoneme and a modifier to produce an updated feature set using the default IPA system.
#[must_use]
pub fn combine_with_modifier(base: &str, modifier: &str) -> Option<Vec<SpeFeature>> {
    DEFAULT_SYSTEM.combine_with_modifier(base, modifier)
}
pub mod string;
pub use string::IpaString;
