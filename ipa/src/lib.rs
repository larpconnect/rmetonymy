use data::{IpaDataset, IpaEntry, PhonemeData, SpeFeature, parse_and_validate};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IpaError {
    #[error("Symbol not found: {0}")]
    NotFound(String),
    #[error("Parsing error: {0}")]
    ParseError(String),
}

pub struct IpaSystem {
    dataset: IpaDataset,
    /// Maps aliases directly to their canonical symbol representations for fast O(1) lookups.
    alias_map: HashMap<String, String>,
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
