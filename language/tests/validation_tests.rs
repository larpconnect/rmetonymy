use language::config::{GeneratorConfig, SoundClass};
use language::generator::WordGenerator;
use language::generator::validation::{
    ValidationError, validate_generator_cycles, validate_generator_keys,
    validate_pattern_sound_classes, validate_sound_class_cycles,
};
use std::collections::{BTreeMap, HashSet};

#[test]
fn test_validate_generator_keys_valid() {
    let mut generators = BTreeMap::new();
    generators.insert(
        "default".to_string(),
        WordGenerator {
            patterns: vec![],
            generator: GeneratorConfig::Equiprobable,
        },
    );
    generators.insert(
        "noun.masculine".to_string(),
        WordGenerator {
            patterns: vec![],
            generator: GeneratorConfig::Equiprobable,
        },
    );
    generators.insert(
        "verb".to_string(),
        WordGenerator {
            patterns: vec![],
            generator: GeneratorConfig::Equiprobable,
        },
    );
    validate_generator_keys(&generators).expect("keys should be valid");
}

#[test]
fn test_validate_generator_keys_missing_default() {
    let mut generators = BTreeMap::new();
    generators.insert(
        "verb".to_string(),
        WordGenerator {
            patterns: vec![],
            generator: GeneratorConfig::Equiprobable,
        },
    );
    assert_eq!(
        validate_generator_keys(&generators).expect_err("should fail with missing default"),
        ValidationError::MissingDefaultGenerator
    );
}

#[test]
fn test_validate_generator_keys_invalid_primary() {
    let mut generators = BTreeMap::new();
    generators.insert(
        "default".to_string(),
        WordGenerator {
            patterns: vec![],
            generator: GeneratorConfig::Equiprobable,
        },
    );
    generators.insert(
        "invalid_type".to_string(),
        WordGenerator {
            patterns: vec![],
            generator: GeneratorConfig::Equiprobable,
        },
    );
    assert_eq!(
        validate_generator_keys(&generators).expect_err("should fail with invalid primary type"),
        ValidationError::InvalidGrammaticalType("invalid_type".to_string())
    );
}

#[test]
fn test_validate_generator_keys_invalid_sec_uppercase() {
    let mut generators = BTreeMap::new();
    generators.insert(
        "default".to_string(),
        WordGenerator {
            patterns: vec![],
            generator: GeneratorConfig::Equiprobable,
        },
    );
    generators.insert(
        "noun.Masculine".to_string(),
        WordGenerator {
            patterns: vec![],
            generator: GeneratorConfig::Equiprobable,
        },
    );
    assert_eq!(
        validate_generator_keys(&generators)
            .expect_err("should fail with uppercase secondary type"),
        ValidationError::InvalidSecondaryType("Masculine".to_string())
    );
}

#[test]
fn test_validate_generator_keys_invalid_sec_too_long() {
    let mut generators = BTreeMap::new();
    generators.insert(
        "default".to_string(),
        WordGenerator {
            patterns: vec![],
            generator: GeneratorConfig::Equiprobable,
        },
    );
    let long_sec = "a".repeat(33);
    generators.insert(
        format!("noun.{long_sec}"),
        WordGenerator {
            patterns: vec![],
            generator: GeneratorConfig::Equiprobable,
        },
    );
    assert_eq!(
        validate_generator_keys(&generators).expect_err("should fail with too long secondary type"),
        ValidationError::InvalidSecondaryType(long_sec)
    );
}

#[test]
fn test_validate_sound_class_cycles_valid() {
    let mut scs = BTreeMap::new();
    scs.insert(
        "C".parse().expect("valid sound class key"),
        SoundClass {
            values: vec!["p".to_string(), "t".to_string(), "F".to_string()],
            generator: None,
        },
    );
    scs.insert(
        "F".parse().expect("valid sound class key"),
        SoundClass {
            values: vec!["f".to_string(), "v".to_string()],
            generator: None,
        },
    );
    validate_sound_class_cycles(&scs).expect("should not detect cycles");
}

#[test]
fn test_sound_class_cycles_detected() {
    let mut scs = BTreeMap::new();
    scs.insert(
        "C".parse().expect("valid sound class key"),
        SoundClass {
            values: vec!["p".to_string(), "t".to_string(), "F".to_string()],
            generator: None,
        },
    );
    scs.insert(
        "F".parse().expect("valid sound class key"),
        SoundClass {
            values: vec!["f".to_string(), "C".to_string()],
            generator: None,
        },
    );
    assert_eq!(
        validate_sound_class_cycles(&scs).expect_err("should detect containment cycle"),
        ValidationError::CircularSoundClassContainment("F".to_string())
    );
}

#[test]
fn test_validate_pattern_sound_classes_valid() {
    let mut generators = BTreeMap::new();
    generators.insert(
        "default".to_string(),
        WordGenerator {
            patterns: vec!["CV".parse().expect("valid pattern")],
            generator: GeneratorConfig::Equiprobable,
        },
    );
    let mut defined = HashSet::new();
    defined.insert("C".parse().expect("valid sound class key"));
    defined.insert("V".parse().expect("valid sound class key"));
    validate_pattern_sound_classes(&generators, &defined).expect("should be valid");
}

#[test]
fn test_validate_pattern_sound_classes_undefined() {
    let mut generators = BTreeMap::new();
    generators.insert(
        "default".to_string(),
        WordGenerator {
            patterns: vec!["CX".parse().expect("valid pattern")],
            generator: GeneratorConfig::Equiprobable,
        },
    );
    let mut defined = HashSet::new();
    defined.insert("C".parse().expect("valid sound class key"));
    defined.insert("V".parse().expect("valid sound class key"));
    assert_eq!(
        validate_pattern_sound_classes(&generators, &defined)
            .expect_err("should find undefined sound class"),
        ValidationError::UndefinedSoundClass("X".to_string())
    );
}

#[test]
fn test_validate_generator_cycles_valid() {
    let mut generators = BTreeMap::new();
    generators.insert(
        "default".to_string(),
        WordGenerator {
            patterns: vec!["CVC".parse().expect("valid pattern")],
            generator: GeneratorConfig::Equiprobable,
        },
    );
    generators.insert(
        "noun".to_string(),
        WordGenerator {
            patterns: vec!["[verb]V".parse().expect("valid pattern")],
            generator: GeneratorConfig::Equiprobable,
        },
    );
    generators.insert(
        "verb".to_string(),
        WordGenerator {
            patterns: vec!["C[default]".parse().expect("valid pattern")],
            generator: GeneratorConfig::Equiprobable,
        },
    );
    validate_generator_cycles(&generators).expect("should not detect cycles");
}

#[test]
fn test_validate_generator_cycles_detected() {
    let mut generators = BTreeMap::new();
    generators.insert(
        "default".to_string(),
        WordGenerator {
            patterns: vec!["CVC".parse().expect("valid pattern")],
            generator: GeneratorConfig::Equiprobable,
        },
    );
    generators.insert(
        "noun".to_string(),
        WordGenerator {
            patterns: vec!["[verb]V".parse().expect("valid pattern")],
            generator: GeneratorConfig::Equiprobable,
        },
    );
    generators.insert(
        "verb".to_string(),
        WordGenerator {
            patterns: vec!["C[noun]".parse().expect("valid pattern")],
            generator: GeneratorConfig::Equiprobable,
        },
    );
    let _err = validate_generator_cycles(&generators).expect_err("should detect cycles");
}

#[test]
fn test_validate_derivations_valid() {
    use language::config::Derivation;
    use language::generator::validation::validate_derivations;

    let derivations = vec![
        Derivation {
            name: "PLURAL".to_string(),
            era: None,
            transforms: vec!["-a".to_string()],
            from_type: Some("noun".to_string()),
            to_type: Some("noun".to_string()),
        },
        Derivation {
            name: "PAST".to_string(),
            era: None,
            transforms: vec!["-ed".to_string()],
            from_type: Some("verb".to_string()),
            to_type: Some("verb".to_string()),
        },
        Derivation {
            name: "DIMINUTIVE".to_string(),
            era: None,
            transforms: vec!["-y".to_string()],
            from_type: None,
            to_type: None,
        },
    ];

    validate_derivations(&derivations).expect("derivations should be valid");
}

#[test]
fn test_validate_derivations_invalid_name() {
    use language::config::Derivation;
    use language::generator::validation::validate_derivations;

    let derivations = vec![Derivation {
        name: "plural".to_string(),
        era: None,
        transforms: vec!["-a".to_string()],
        from_type: None,
        to_type: None,
    }];

    assert_eq!(
        validate_derivations(&derivations).expect_err("should fail on invalid name"),
        ValidationError::InvalidDerivationName("plural".to_string())
    );
}

#[test]
fn test_validate_derivations_duplicate() {
    use language::config::Derivation;
    use language::generator::validation::validate_derivations;

    let derivations = vec![
        Derivation {
            name: "DUP".to_string(),
            era: None,
            transforms: vec!["-a".to_string()],
            from_type: Some("noun".to_string()),
            to_type: None,
        },
        Derivation {
            name: "DUP".to_string(),
            era: None,
            transforms: vec!["-b".to_string()],
            from_type: None,
            to_type: None,
        },
    ];

    assert_eq!(
        validate_derivations(&derivations).expect_err("should fail on duplicate name"),
        ValidationError::DuplicateDerivationName("DUP".to_string())
    );
}

#[test]
fn test_validate_derivations_subtypes_no_conflict() {
    use language::config::Derivation;
    use language::generator::validation::validate_derivations;

    let derivations = vec![
        Derivation {
            name: "PLURAL".to_string(),
            era: None,
            transforms: vec!["-a".to_string()],
            from_type: Some("noun.masculine".to_string()),
            to_type: None,
        },
        Derivation {
            name: "PLURAL".to_string(),
            era: None,
            transforms: vec!["-b".to_string()],
            from_type: Some("noun.feminine".to_string()),
            to_type: None,
        },
    ];

    validate_derivations(&derivations).expect("different subtypes should not conflict");
}

#[test]
fn test_validate_derivations_subtype_base_conflict() {
    use language::config::Derivation;
    use language::generator::validation::validate_derivations;

    let derivations = vec![
        Derivation {
            name: "PLURAL".to_string(),
            era: None,
            transforms: vec!["-a".to_string()],
            from_type: Some("noun".to_string()),
            to_type: None,
        },
        Derivation {
            name: "PLURAL".to_string(),
            era: None,
            transforms: vec!["-b".to_string()],
            from_type: Some("noun.masculine".to_string()),
            to_type: None,
        },
    ];

    assert_eq!(
        validate_derivations(&derivations).expect_err("base type and subtype should conflict"),
        ValidationError::DuplicateDerivationName("PLURAL".to_string())
    );
}

