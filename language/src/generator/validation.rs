//! Validation system for the word generator configurations.

use crate::config::SoundClass;
use crate::generator::{WordGenerator, WordPattern, WordPatternElement};
use crate::sound_class::SoundClassKey;
use std::collections::{BTreeMap, HashSet};
use thiserror::Error;

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
        if sec.is_empty() || sec.len() > 32 {
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

/// Validates that there are no circular containment relationships in sound classes.
///
/// # Errors
/// Returns `Err` if any containment cycle is detected.
pub fn validate_sound_class_cycles(
    sound_classes: &BTreeMap<SoundClassKey, SoundClass>,
) -> Result<(), ValidationError> {
    let mut graph = BTreeMap::new();
    for (key, sc) in sound_classes {
        let mut deps = Vec::new();
        for val in &sc.values {
            if let Some(nested_key) = val
                .parse::<SoundClassKey>()
                .ok()
                .filter(|k| sound_classes.contains_key(k))
            {
                deps.push(nested_key);
            }
        }
        graph.insert(key.clone(), deps);
    }

    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();

    for key in graph.keys() {
        if !visited.contains(key) {
            check_sound_class_cycle(key, &graph, &mut visiting, &mut visited)?;
        }
    }

    Ok(())
}

fn check_sound_class_cycle(
    node: &SoundClassKey,
    graph: &BTreeMap<SoundClassKey, Vec<SoundClassKey>>,
    visiting: &mut HashSet<SoundClassKey>,
    visited: &mut HashSet<SoundClassKey>,
) -> Result<(), ValidationError> {
    visiting.insert(node.clone());

    if let Some(deps) = graph.get(node) {
        for dep in deps {
            if visiting.contains(dep) {
                return Err(ValidationError::CircularSoundClassContainment(
                    node.to_string(),
                ));
            }
            if !visited.contains(dep) {
                check_sound_class_cycle(dep, graph, visiting, visited)?;
            }
        }
    }

    visiting.remove(node);
    visited.insert(node.clone());
    Ok(())
}

/// Validates that there are no circular generation dependencies.
///
/// # Errors
/// Returns `Err` if any pattern reference cycle is detected.
pub fn validate_generator_cycles(
    generators: &BTreeMap<String, WordGenerator>,
) -> Result<(), ValidationError> {
    let mut graph = BTreeMap::new();
    for (key, generator) in generators {
        let mut deps = HashSet::new();
        for pattern in &generator.patterns {
            let mut sound_classes = Vec::new();
            let mut grammar_refs = Vec::new();
            collect_pattern_references(pattern, &mut sound_classes, &mut grammar_refs);

            for r in grammar_refs {
                let resolved = resolve_generator_key(&r, generators)?;
                deps.insert(resolved);
            }
        }
        graph.insert(key.clone(), deps);
    }

    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();

    for key in graph.keys() {
        if !visited.contains(key) {
            check_generator_cycle(key, &graph, &mut visiting, &mut visited)?;
        }
    }

    Ok(())
}

fn check_generator_cycle(
    node: &str,
    graph: &BTreeMap<String, HashSet<String>>,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
) -> Result<(), ValidationError> {
    visiting.insert(node.to_string());

    if let Some(deps) = graph.get(node) {
        for dep in deps {
            if visiting.contains(dep) {
                return Err(ValidationError::CircularPatternReferences(node.to_string()));
            }
            if !visited.contains(dep) {
                check_generator_cycle(dep, graph, visiting, visited)?;
            }
        }
    }

    visiting.remove(node);
    visited.insert(node.to_string());
    Ok(())
}
