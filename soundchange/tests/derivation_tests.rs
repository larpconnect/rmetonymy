use language::config::{
    Derivation, LanguageConfig, MetadataConfig, NameConfig, PhonologyConfig, PhonotacticsConfig,
    SoundClass,
};
use language::syllable::IpaWord;
use soundchange::apply_derivations;
use std::collections::BTreeMap;
use time::OffsetDateTime;
use uuid::Uuid;

fn create_test_config(derivations: Vec<Derivation>) -> LanguageConfig {
    let mut sound_classes = BTreeMap::new();
    let defaults = ["C", "D", "L", "V"];
    for default_key in defaults {
        let key = default_key.parse().unwrap();
        sound_classes.insert(
            key,
            SoundClass {
                values: Vec::new(),
                generator: None,
            },
        );
    }

    // Insert actual phonemes for C and V so syllabifier works
    let key_c = "C".parse().unwrap();
    let key_v = "V".parse().unwrap();
    sound_classes.insert(
        key_c,
        SoundClass {
            values: vec!["p".to_string(), "t".to_string(), "k".to_string()],
            generator: None,
        },
    );
    sound_classes.insert(
        key_v,
        SoundClass {
            values: vec!["a".to_string(), "i".to_string(), "u".to_string()],
            generator: None,
        },
    );

    LanguageConfig {
        id: Uuid::now_v7(),
        name: NameConfig {
            endonym: "test".parse().unwrap(),
            exonym: None,
        },
        metadata: MetadataConfig {
            created_at: OffsetDateTime::now_utc(),
            updated_at: None,
        },
        phonology: PhonologyConfig {
            sound_classes,
            phonotactics: PhonotacticsConfig::default(),
            illegal_patterns: Vec::new(),
            prosody: Some(language::prosody::ProsodicConfig::Unstressed),
        },
        sound_changes: None,
        orthography: None,
        derivations: Some(derivations),
    }
}

#[test]
fn test_apply_derivations_prefix_suffix() {
    let derivations = vec![Derivation {
        name: "PLURAL".to_string(),
        era: None,
        transforms: vec!["a-".to_string(), "-i".to_string()],
        from_type: Some("noun".to_string()),
        to_type: Some("noun.plural".to_string()),
    }];

    let config = create_test_config(derivations);
    let word = IpaWord::try_from_sequence(&"pataka".parse().unwrap(), &config).unwrap();

    let res =
        apply_derivations(&word, "noun", &vec!["PLURAL".to_string()], &config, 0).unwrap();

    assert_eq!(res.word.to_string(), "a.pa.ta.ka.i");
    assert_eq!(res.final_type, "noun.plural");
    assert_eq!(res.final_era, 0);

    // "a" prefix (1 phoneme) -> tag Some(1)
    // "pataka" (6 phonemes) -> tag None
    // "i" suffix (1 phoneme) -> tag Some(1)
    assert_eq!(
        res.tags,
        vec![Some(1), None, None, None, None, None, None, Some(1)]
    );
}

#[test]
fn test_apply_derivations_sound_change() {
    let derivations = vec![Derivation {
        name: "MUTATION".to_string(),
        era: None,
        transforms: vec!["p => t / _a".to_string()],
        from_type: None,
        to_type: None,
    }];

    let config = create_test_config(derivations);
    let word = IpaWord::try_from_sequence(&"pataka".parse().unwrap(), &config).unwrap();

    let res =
        apply_derivations(&word, "verb", &vec!["MUTATION".to_string()], &config, 0).unwrap();

    // "p" at index 0 changes to "t"
    assert_eq!(res.word.to_string(), "ta.ta.ka");
    assert_eq!(res.final_type, "verb");
    assert_eq!(res.final_era, 0);

    // index 0 was modified by derivation 1, others unchanged
    assert_eq!(res.tags, vec![Some(1), None, None, None, None, None]);
}

#[test]
fn test_apply_derivations_type_constraints() {
    let derivations = vec![Derivation {
        name: "GERUND".to_string(),
        era: None,
        transforms: vec!["-i".to_string()],
        from_type: Some("verb".to_string()),
        to_type: Some("noun".to_string()),
    }];

    let config = create_test_config(derivations);
    let word = IpaWord::try_from_sequence(&"pataka".parse().unwrap(), &config).unwrap();

    // noun is not matching verb, so should fail
    let err = apply_derivations(&word, "noun", &vec!["GERUND".to_string()], &config, 0).unwrap_err();
    assert!(err.contains("does not match expected"));
}

#[test]
fn test_apply_derivations_era_tracking_and_validation() {
    let derivations = vec![
        Derivation {
            name: "EARLY".to_string(),
            era: Some(1),
            transforms: vec!["a-".to_string()],
            from_type: None,
            to_type: None,
        },
        Derivation {
            name: "LATE".to_string(),
            era: Some(3),
            transforms: vec!["-i".to_string()],
            from_type: None,
            to_type: None,
        },
        Derivation {
            name: "OUT_OF_ORDER".to_string(),
            era: Some(2),
            transforms: vec!["-u".to_string()],
            from_type: None,
            to_type: None,
        },
    ];

    let config = create_test_config(derivations);
    let word = IpaWord::try_from_sequence(&"pataka".parse().unwrap(), &config).unwrap();

    // Valid: Word era 0 <= EARLY era 1 <= LATE era 3
    let res =
        apply_derivations(&word, "noun", &vec!["EARLY".to_string(), "LATE".to_string()], &config, 0).unwrap();
    assert_eq!(res.word.to_string(), "a.pa.ta.ka.i");
    assert_eq!(res.final_era, 3);

    // Invalid: Word era 2 > EARLY era 1
    let err1 = apply_derivations(&word, "noun", &vec!["EARLY".to_string()], &config, 2).unwrap_err();
    assert!(err1.contains("word era 2 is after derivation era 1"));

    // Invalid: EARLY era 1 -> LATE era 3 -> OUT_OF_ORDER era 2 (3 > 2)
    let err2 = apply_derivations(&word, "noun", &vec!["EARLY".to_string(), "LATE".to_string(), "OUT_OF_ORDER".to_string()], &config, 0).unwrap_err();
    assert!(err2.contains("word era 3 is after derivation era 2"));
}

#[test]
fn test_apply_derivations_intermediate_sound_changes() {
    use language::config::{EraRules, SoundChangeRule, SoundChanges};

    let derivations = vec![Derivation {
        name: "LATE_DERIV".to_string(),
        era: Some(2),
        transforms: vec!["-i".to_string()],
        from_type: None,
        to_type: None,
    }];

    let mut config = create_test_config(derivations);
    config.sound_changes = Some(SoundChanges {
        preamble: Vec::new(),
        eras: vec![EraRules {
            era: 1,
            rules: vec![SoundChangeRule {
                name: None,
                changes: vec!["p => t / _a".to_string()],
            }],
        }],
    });

    let word = IpaWord::try_from_sequence(&"pataka".parse().unwrap(), &config).unwrap();

    // Applying LATE_DERIV (era 2) to "pataka" (era 0).
    // Word should undergo the era 1 sound change "p => t / _a" before the derivation is applied.
    let res =
        apply_derivations(&word, "noun", &vec!["LATE_DERIV".to_string()], &config, 0).unwrap();

    // "pataka" -> (era 1 sound change) -> "tataka" -> (derivation suffix) -> "tatakai"
    assert_eq!(res.word.to_string(), "ta.ta.ka.i");
    assert_eq!(res.final_era, 2);
    // The derived suffix 'i' should be tagged Some(1), but the sound-changed 't' should be None
    // since sound changes do not set derivation tags.
    // phonemes: t a t a k a i
    assert_eq!(
        res.tags,
        vec![None, None, None, None, None, None, Some(1)]
    );
}
