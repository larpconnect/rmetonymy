//! Validation system for the word generator configurations.

pub mod cycles;
pub use cycles::{validate_generator_cycles, validate_sound_class_cycles};

use crate::generator::{WordGenerator, WordPattern, WordPatternElement};
use crate::sound_class::SoundClassKey;
use std::collections::{BTreeMap, HashSet};
use thiserror::Error;

const MAX_SECONDARY_TYPE_LEN: usize = 32;
const MIN_DERIVATION_NAME_LEN: usize = 3;
const MAX_DERIVATION_NAME_LEN: usize = 31;

/// Validation errors that may occur when loading configuration.
#[derive(Debug, Error, PartialEq, Clone)]
pub enum ValidationError {
    /// Referenced sound class does not exist.
    #[error("Sound class '{0}' is referenced in pattern but not defined")]
    UndefinedSoundClass(String),

    /// Circular containment in sound classes (e.g. A contains B, B contains A).
    #[error("Circular containment detected in sound classes: {0}")]
    CircularSoundClassContainment(String),

    /// The default generator rule is missing.
    #[error("Missing required 'default' generator in phonotactics")]
    MissingDefaultGenerator,

    /// Key contains invalid grammatical type.
    #[error(
        "Invalid primary grammatical type '{0}' in generator or reference. Must be one of: default, noun, pronoun, verb, adjective, adverb, preposition, conjunction, determiner, interjection, number, particle, article"
    )]
    InvalidGrammaticalType(String),

    /// Key contains invalid secondary type.
    #[error(
        "Secondary type length of '{0}' must be between 1 and 32 characters, and contain only lowercase alphanumeric characters and '_'"
    )]
    InvalidSecondaryType(String),

    /// Grammar reference could not be resolved.
    #[error("Grammar reference '[{0}]' cannot be resolved to any defined generator")]
    UnresolvedGrammarRef(String),

    /// Directed cycle detected in word generator pattern references.
    #[error("Circular pattern references detected involving generator: {0}")]
    CircularPatternReferences(String),

    /// Invalid prosody configuration.
    #[error("Invalid prosody configuration: {0}")]
    InvalidProsodyConfig(String),

    /// Invalid derivation name.
    #[error(
        "Invalid derivation name '{0}': must be alphanumeric, all caps, dots/underscores allowed, length 3 to 31"
    )]
    InvalidDerivationName(String),

    /// Duplicate derivation name.
    #[error("Duplicate derivation name '{0}'")]
    DuplicateDerivationName(String),
}

/// Validates that all generator map keys conform to the expected syntax and types.
///
/// # Errors
/// Returns `Err` if keys contain invalid types or malformed subtypes, or if the
/// required `default` generator is missing.
pub fn validate_generator_keys(
    generators: &BTreeMap<String, WordGenerator>,
) -> Result<(), ValidationError> {
    if !generators.contains_key("default") {
        return Err(ValidationError::MissingDefaultGenerator);
    }

    for key in generators.keys() {
        validate_key_format(key)?;
    }

    Ok(())
}

fn validate_key_format(key: &str) -> Result<(), ValidationError> {
    let valid_primaries = [
        "default",
        "noun",
        "pronoun",
        "verb",
        "adjective",
        "adverb",
        "preposition",
        "conjunction",
        "determiner",
        "interjection",
        "number",
        "particle",
        "article",
    ];

    let (primary, secondary) = match key.split_once('.') {
        Some((p, s)) => (p, Some(s)),
        None => (key, None),
    };

    if !valid_primaries.contains(&primary) {
        return Err(ValidationError::InvalidGrammaticalType(primary.to_string()));
    }

    if let Some(sec) = secondary {
        if sec.is_empty() || sec.len() > MAX_SECONDARY_TYPE_LEN {
            return Err(ValidationError::InvalidSecondaryType(sec.to_string()));
        }
        if !sec
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(ValidationError::InvalidSecondaryType(sec.to_string()));
        }
    }

    Ok(())
}

/// Resolves a grammatical reference key to its actual generator rule using fallbacks.
///
/// # Errors
/// Returns `Err` if no generator matches the key or the default fallback.
pub fn resolve_generator_key(
    ref_key: &str,
    generators: &BTreeMap<String, WordGenerator>,
) -> Result<String, ValidationError> {
    if generators.contains_key(ref_key) {
        return Ok(ref_key.to_string());
    }
    if let Some(base) = ref_key
        .split_once('.')
        .map(|(b, _)| b)
        .filter(|b| generators.contains_key(*b))
    {
        return Ok(base.to_string());
    }
    if generators.contains_key("default") {
        return Ok("default".to_string());
    }
    Err(ValidationError::UnresolvedGrammarRef(ref_key.to_string()))
}

fn collect_pattern_references(
    pattern: &WordPattern,
    sound_classes: &mut Vec<SoundClassKey>,
    grammar_refs: &mut Vec<String>,
) {
    for el in &pattern.elements {
        collect_element_references(el, sound_classes, grammar_refs);
    }
}

fn collect_element_references(
    el: &WordPatternElement,
    sound_classes: &mut Vec<SoundClassKey>,
    grammar_refs: &mut Vec<String>,
) {
    match el {
        WordPatternElement::SoundClass(sc) => {
            sound_classes.push(sc.clone());
        }
        WordPatternElement::Optional(inner, _) => {
            collect_pattern_references(inner, sound_classes, grammar_refs);
        }
        WordPatternElement::Set(choices) => {
            for choice in choices {
                if let Ok(sc) = choice.parse::<SoundClassKey>() {
                    sound_classes.push(sc);
                }
            }
        }
        WordPatternElement::GrammarRef { primary, secondary } => {
            let mut ref_key = primary.clone();
            if let Some(sec) = secondary {
                ref_key.push('.');
                ref_key.push_str(sec);
            }
            grammar_refs.push(ref_key);
        }
        _ => {}
    }
}

/// Validates that all sound classes referenced by generator patterns are defined.
///
/// # Errors
/// Returns `Err` if any pattern references an undefined sound class.
pub fn validate_pattern_sound_classes<S: std::hash::BuildHasher>(
    generators: &BTreeMap<String, WordGenerator>,
    defined_sound_classes: &HashSet<SoundClassKey, S>,
) -> Result<(), ValidationError> {
    for generator in generators.values() {
        for pattern in &generator.patterns {
            let mut sound_classes = Vec::new();
            let mut grammar_refs = Vec::new();
            collect_pattern_references(pattern, &mut sound_classes, &mut grammar_refs);

            for sc in sound_classes {
                if !defined_sound_classes.contains(&sc) {
                    return Err(ValidationError::UndefinedSoundClass(sc.to_string()));
                }
            }
        }
    }
    Ok(())
}

/// Helper to validate if derivation name fits the required character set and length constraints.
#[must_use]
pub fn is_valid_derivation_name(name: &str) -> bool {
    if name.len() < MIN_DERIVATION_NAME_LEN || name.len() > MAX_DERIVATION_NAME_LEN {
        return false;
    }
    name.chars()
        .all(|c| (c.is_ascii_alphanumeric() && c.is_ascii_uppercase()) || c == '.' || c == '_')
}

/// Validates that derivation configurations conform to name rules and uniqueness requirements.
///
/// # Errors
/// Returns `Err` if any derivation has an invalid name or duplicate name.
fn validate_derivation_names_op(
    derivations: &[crate::config::Derivation],
) -> Result<(), ValidationError> {
    for deriv in derivations {
        if !is_valid_derivation_name(&deriv.name) {
            return Err(ValidationError::InvalidDerivationName(deriv.name.clone()));
        }
    }
    Ok(())
}

fn collect_all_types_op(derivations: &[crate::config::Derivation]) -> HashSet<String> {
    let mut all_types = HashSet::new();
    for deriv in derivations {
        if let Some(ref from_t) = deriv.from_type {
            all_types.insert(from_t.clone());
            if let Some((base, _)) = from_t.split_once('.') {
                all_types.insert(base.to_string());
            }
        }
    }
    all_types
}

fn validate_no_type_duplicates_op(
    derivations: &[crate::config::Derivation],
) -> Result<(), ValidationError> {
    let mut names = HashSet::new();
    for deriv in derivations {
        if !names.insert(&deriv.name) {
            return Err(ValidationError::DuplicateDerivationName(deriv.name.clone()));
        }
    }
    Ok(())
}

fn derivation_applies_to_type_op(from_type: Option<&str>, target_type: &str) -> bool {
    match from_type {
        None => true,
        Some(from_t) => {
            if from_t == target_type {
                true
            } else if let Some((t_base, _)) = target_type.split_once('.') {
                from_t == t_base
            } else {
                false
            }
        }
    }
}

fn validate_type_specific_duplicates_op(
    derivations: &[crate::config::Derivation],
    all_types: &HashSet<String>,
) -> Result<(), ValidationError> {
    for t in all_types {
        let mut names = HashSet::new();
        for deriv in derivations {
            let applies = derivation_applies_to_type_op(deriv.from_type.as_deref(), t);
            if applies && !names.insert(&deriv.name) {
                return Err(ValidationError::DuplicateDerivationName(deriv.name.clone()));
            }
        }
    }
    Ok(())
}

/// Validates the derivations config.
///
/// # Errors
/// Returns `Err` if derivation names are duplicate or invalid.
pub fn validate_derivations(
    derivations: &[crate::config::Derivation],
) -> Result<(), ValidationError> {
    validate_derivation_names_op(derivations)?;
    let all_types = collect_all_types_op(derivations);
    if all_types.is_empty() {
        validate_no_type_duplicates_op(derivations)
    } else {
        validate_type_specific_duplicates_op(derivations, &all_types)
    }
}
