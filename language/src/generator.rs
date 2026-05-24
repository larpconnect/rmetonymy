//! Word generator module for the `rmetonymy` language configuration.
//!
//! Provides the random generation engine, routing fallbacks, and integration
//! with the syllabifier and illegal pattern checks.

use crate::config::{GeneratorConfig, LanguageConfig, SoundClass};
use crate::sound_class::SoundClassKey;
use std::str::FromStr;
use thiserror::Error;

pub mod pattern;
pub mod rng;
pub mod validation;

pub use pattern::{GeneratorError, GeneratorPatternParser, WordPattern, WordPatternElement};
pub use rng::{Rng, RngExt, SeedableRng, StdRng, thread_rng};
pub use validation::{
    ValidationError, resolve_generator_key, validate_generator_cycles, validate_generator_keys,
    validate_pattern_sound_classes, validate_sound_class_cycles,
};

/// Configuration for a word generator.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct WordGenerator {
    /// List of word patterns to choose from.
    pub patterns: Vec<WordPattern>,
    /// Nested probabilistic algorithm/generator config.
    #[serde(flatten)]
    pub generator: GeneratorConfig,
}

/// Generation runtime errors.
#[derive(Debug, Error)]
pub enum GenerationError {
    /// Referenced generator is not defined.
    #[error("Undefined generator: {0}")]
    UndefinedGenerator(String),

    /// Cycle detected at runtime or generator depth limit reached.
    #[error("Circular generation/depth limit reached at: {0}")]
    CircularGeneration(String),

    /// Referenced sound class is not defined.
    #[error("Undefined sound class: {0}")]
    UndefinedSoundClass(String),

    /// Sound class has no values to choose from.
    #[error("Empty sound class: {0}")]
    EmptySoundClass(String),

    /// Containment cycle in sound class triggered at runtime.
    #[error("Circular sound class depth limit reached at: {0}")]
    CircularSoundClass(String),

    /// Configuration failed validation.
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),
}

/// Samples a random index using the configured probability distribution.
#[expect(
    clippy::cast_precision_loss,
    reason = "RNG choices safely scaled to length of patterns or classes"
)]
pub fn sample_index<R: Rng + ?Sized>(
    num_choices: usize,
    config: &GeneratorConfig,
    rng: &mut R,
) -> usize {
    if num_choices == 0 {
        return 0;
    }
    match config {
        GeneratorConfig::Equiprobable => rng.random_range(0..num_choices),
        GeneratorConfig::Zipf {
            config: zipf_config,
        } => {
            let a = zipf_config.a;
            let b = zipf_config.b;

            let mut sum = 0.0;
            for i in 1..=num_choices {
                sum += 1.0 / (i as f64 + b).powf(a);
            }

            let r = rng.random::<f64>() * sum;
            let mut accum = 0.0;
            for i in 1..=num_choices {
                let w = 1.0 / (i as f64 + b).powf(a);
                accum += w;
                if accum >= r {
                    return i - 1;
                }
            }
            num_choices - 1
        }
    }
}

fn get_generator<'a>(
    gen_key: &str,
    config: &'a LanguageConfig,
) -> Result<&'a WordGenerator, GenerationError> {
    let resolved_key = resolve_generator_key(gen_key, &config.phonology.phonotactics.generators)?;
    config
        .phonology
        .phonotactics
        .generators
        .get(&resolved_key)
        .ok_or_else(|| GenerationError::UndefinedGenerator(resolved_key.clone()))
}

fn generate_word_internal<R: Rng + ?Sized>(
    gen_key: &str,
    config: &LanguageConfig,
    rng: &mut R,
    depth: usize,
) -> Result<String, GenerationError> {
    if depth > 50 {
        return Err(GenerationError::CircularGeneration(gen_key.to_string()));
    }

    let generator = get_generator(gen_key, config)?;

    if generator.patterns.is_empty() {
        return Ok(String::new());
    }

    let pat_idx = sample_index(generator.patterns.len(), &generator.generator, rng);
    let pattern = generator
        .patterns
        .get(pat_idx)
        .expect("pattern index out of bounds");

    let mut result = String::new();
    for el in &pattern.elements {
        let el_str = evaluate_element(el, config, rng, depth)?;
        result.push_str(&el_str);
    }

    Ok(result)
}

fn evaluate_optional<R: Rng + ?Sized>(
    inner_pattern: &WordPattern,
    prob: u8,
    config: &LanguageConfig,
    rng: &mut R,
    depth: usize,
) -> Result<String, GenerationError> {
    let r = rng.random::<f64>() * 100.0;
    if r < f64::from(prob) {
        let mut result = String::new();
        for inner_el in &inner_pattern.elements {
            let s = evaluate_element(inner_el, config, rng, depth)?;
            result.push_str(&s);
        }
        Ok(result)
    } else {
        Ok(String::new())
    }
}

fn evaluate_set<R: Rng + ?Sized>(
    choices: &[String],
    config: &LanguageConfig,
    rng: &mut R,
) -> Result<String, GenerationError> {
    if choices.is_empty() {
        return Ok(String::new());
    }
    let idx = rng.random_range(0..choices.len());
    let selected = choices.get(idx).expect("set choice index out of bounds");

    if let Some(nested_key) = selected
        .parse::<SoundClassKey>()
        .ok()
        .filter(|k| config.phonology.sound_classes.contains_key(k))
    {
        return select_from_sound_class(&nested_key, config, rng, 0);
    }
    Ok(selected.clone())
}

fn evaluate_grammar_ref<R: Rng + ?Sized>(
    primary: &str,
    secondary: Option<&str>,
    config: &LanguageConfig,
    rng: &mut R,
    depth: usize,
) -> Result<String, GenerationError> {
    let mut ref_key = primary.to_string();
    if let Some(sec) = secondary {
        ref_key.push('.');
        ref_key.push_str(sec);
    }
    generate_word_internal(&ref_key, config, rng, depth + 1)
}

fn evaluate_element<R: Rng + ?Sized>(
    el: &WordPatternElement,
    config: &LanguageConfig,
    rng: &mut R,
    depth: usize,
) -> Result<String, GenerationError> {
    match el {
        WordPatternElement::SoundClass(sc_key) => select_from_sound_class(sc_key, config, rng, 0),
        WordPatternElement::Literal(s) => Ok(s.clone()),
        WordPatternElement::SyllableBreak => Ok(".".to_string()),
        WordPatternElement::StressMarker => Ok("ˈ".to_string()),
        WordPatternElement::Optional(inner_pattern, prob) => {
            evaluate_optional(inner_pattern, *prob, config, rng, depth)
        }
        WordPatternElement::Set(choices) => evaluate_set(choices, config, rng),
        WordPatternElement::GrammarRef { primary, secondary } => {
            evaluate_grammar_ref(primary, secondary.as_deref(), config, rng, depth)
        }
    }
}

fn get_sound_class<'a>(
    sc_key: &SoundClassKey,
    config: &'a LanguageConfig,
) -> Result<&'a SoundClass, GenerationError> {
    config
        .phonology
        .sound_classes
        .get(sc_key)
        .ok_or_else(|| GenerationError::UndefinedSoundClass(sc_key.to_string()))
}

fn select_from_sound_class<R: Rng + ?Sized>(
    sc_key: &SoundClassKey,
    config: &LanguageConfig,
    rng: &mut R,
    depth: usize,
) -> Result<String, GenerationError> {
    if depth > 100 {
        return Err(GenerationError::CircularSoundClass(sc_key.to_string()));
    }

    let sc = get_sound_class(sc_key, config)?;

    if sc.values.is_empty() {
        return Err(GenerationError::EmptySoundClass(sc_key.to_string()));
    }

    let gen_config = sc
        .generator
        .as_ref()
        .unwrap_or(&GeneratorConfig::Equiprobable);
    let idx = sample_index(sc.values.len(), gen_config, rng);
    let selected = sc
        .values
        .get(idx)
        .expect("sound class value index out of bounds");

    if let Some(nested_key) = selected
        .parse::<SoundClassKey>()
        .ok()
        .filter(|k| config.phonology.sound_classes.contains_key(k))
    {
        return select_from_sound_class(&nested_key, config, rng, depth + 1);
    }

    Ok(selected.clone())
}

/// Returns whether the word contains any patterns listed as illegal in the config.
#[inline]
#[must_use]
pub fn matches_illegal_patterns(word: &str, config: &LanguageConfig) -> bool {
    for pattern in &config.phonology.illegal_patterns {
        if pattern.matches(word, &config.phonology.sound_classes) {
            return true;
        }
    }
    false
}

/// Generates a word for a type using the generator.
///
/// # Errors
/// Returns `Err` if generation, resolution, or sound class expansion fails.
pub fn generate_word<R: Rng + ?Sized>(
    gen_key: &str,
    config: &LanguageConfig,
    rng: &mut R,
    max_attempts: usize,
    warning_logged: &mut bool,
) -> Result<String, GenerationError> {
    let mut last_generated = String::new();
    for _attempt in 1..=max_attempts {
        let word = generate_word_internal(gen_key, config, rng, 0)?;
        last_generated = word;
        let syllabified = ipa::sequence::PhonemeSequence::from_str(&last_generated)
            .ok()
            .and_then(|seq| crate::syllabifier::syllabify_sequence(&seq, config).ok());
        if let Some(syllabified_word) = syllabified {
            let syllabified_str = syllabified_word.to_string();
            last_generated = syllabified_str;
            if !matches_illegal_patterns(&last_generated, config) {
                return Ok(last_generated);
            }
        }
    }

    if !*warning_logged {
        tracing::warn!(
            "Failed to generate word without illegal patterns after {max_attempts} attempts. Using last generated: '{last_generated}'"
        );
        *warning_logged = true;
    }

    Ok(last_generated)
}
