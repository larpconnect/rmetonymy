use language::config::{GeneratorConfig, SoundClass};
use language::generator::validation::{
    ValidationError, validate_generator_keys, validate_sound_class_cycles,
    validate_pattern_sound_classes, validate_generator_cycles,
};
use language::generator::WordGenerator;
use std::collections::{BTreeMap, HashSet};

#[test]
fn test_validate_generator_keys() {
    // Valid cases
    let mut generators = BTreeMap::new();
    generators.insert("default".to_string(), WordGenerator {
        patterns: vec![],
        generator: GeneratorConfig::Equiprobable,
    });
    generators.insert("noun.masculine".to_string(), WordGenerator {
        patterns: vec![],
        generator: GeneratorConfig::Equiprobable,
    });
    generators.insert("verb".to_string(), WordGenerator {
        patterns: vec![],
        generator: GeneratorConfig::Equiprobable,
    });
    assert!(validate_generator_keys(&generators).is_ok());

    // Missing default
    let mut generators_no_default = BTreeMap::new();
    generators_no_default.insert("verb".to_string(), WordGenerator {
        patterns: vec![],
        generator: GeneratorConfig::Equiprobable,
    });
    assert_eq!(
        validate_generator_keys(&generators_no_default).unwrap_err(),
        ValidationError::MissingDefaultGenerator
    );

    // Invalid primary type
    let mut generators_invalid_primary = generators.clone();
    generators_invalid_primary.insert("invalid_type".to_string(), WordGenerator {
        patterns: vec![],
        generator: GeneratorConfig::Equiprobable,
    });
    assert_eq!(
        validate_generator_keys(&generators_invalid_primary).unwrap_err(),
        ValidationError::InvalidGrammaticalType("invalid_type".to_string())
    );

    // Invalid secondary type - uppercase
    let mut generators_invalid_sec = generators.clone();
    generators_invalid_sec.insert("noun.Masculine".to_string(), WordGenerator {
        patterns: vec![],
        generator: GeneratorConfig::Equiprobable,
    });
    assert_eq!(
        validate_generator_keys(&generators_invalid_sec).unwrap_err(),
        ValidationError::InvalidSecondaryType("Masculine".to_string())
    );

    // Invalid secondary type - too long
    let mut generators_too_long = generators.clone();
    let long_sec = "a".repeat(33);
    generators_too_long.insert(format!("noun.{long_sec}"), WordGenerator {
        patterns: vec![],
        generator: GeneratorConfig::Equiprobable,
    });
    assert_eq!(
        validate_generator_keys(&generators_too_long).unwrap_err(),
        ValidationError::InvalidSecondaryType(long_sec)
    );
}

#[test]
fn test_validate_sound_class_cycles() {
    let mut scs = BTreeMap::new();
    scs.insert("C".parse().unwrap(), SoundClass {
        values: vec!["p".to_string(), "t".to_string(), "F".to_string()],
        generator: None,
    });
    scs.insert("F".parse().unwrap(), SoundClass {
        values: vec!["f".to_string(), "v".to_string()],
        generator: None,
    });
    // No cycles
    assert!(validate_sound_class_cycles(&scs).is_ok());

    // Add a cycle: F contains C
    scs.insert("F".parse().unwrap(), SoundClass {
        values: vec!["f".to_string(), "C".to_string()],
        generator: None,
    });
    assert_eq!(
        validate_sound_class_cycles(&scs).unwrap_err(),
        ValidationError::CircularSoundClassContainment("F".to_string())
    );
}

#[test]
fn test_validate_pattern_sound_classes() {
    let mut generators = BTreeMap::new();
    generators.insert("default".to_string(), WordGenerator {
        patterns: vec!["CV".parse().unwrap()],
        generator: GeneratorConfig::Equiprobable,
    });

    let mut defined = HashSet::new();
    defined.insert("C".parse().unwrap());
    defined.insert("V".parse().unwrap());

    // All defined
    assert!(validate_pattern_sound_classes(&generators, &defined).is_ok());

    // Undefined referenced
    generators.insert("default".to_string(), WordGenerator {
        patterns: vec!["CX".parse().unwrap()],
        generator: GeneratorConfig::Equiprobable,
    });
    assert_eq!(
        validate_pattern_sound_classes(&generators, &defined).unwrap_err(),
        ValidationError::UndefinedSoundClass("X".to_string())
    );
}

#[test]
fn test_validate_generator_cycles() {
    let mut generators = BTreeMap::new();
    generators.insert("default".to_string(), WordGenerator {
        patterns: vec!["CVC".parse().unwrap()],
        generator: GeneratorConfig::Equiprobable,
    });
    generators.insert("noun".to_string(), WordGenerator {
        patterns: vec!["[verb]V".parse().unwrap()],
        generator: GeneratorConfig::Equiprobable,
    });
    generators.insert("verb".to_string(), WordGenerator {
        patterns: vec!["C[default]".parse().unwrap()],
        generator: GeneratorConfig::Equiprobable,
    });
    // No cycles: noun -> verb -> default -> CVC (no ref)
    assert!(validate_generator_cycles(&generators).is_ok());

    // Cycle: verb -> noun -> verb
    generators.insert("verb".to_string(), WordGenerator {
        patterns: vec!["C[noun]".parse().unwrap()],
        generator: GeneratorConfig::Equiprobable,
    });
    assert!(validate_generator_cycles(&generators).is_err());
}
