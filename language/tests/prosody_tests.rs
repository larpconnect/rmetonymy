// qual:allow(srp) - Prosody test suite
use ipa::IpaString;
use language::config::{
    LanguageConfig, MetadataConfig, NameConfig, PhonologyConfig, PhonotacticsConfig,
};
use language::generator::rng::{SeedableRng, StdRng};
use language::prosody::{
    AlternatingConfig, FootSize, MainStress, PatternedConfig, ProsodicConfig, StressLocation,
};
use std::str::FromStr;

fn get_test_config(prosody: Option<ProsodicConfig>) -> LanguageConfig {
    let sound_classes = std::collections::BTreeMap::new();
    LanguageConfig {
        id: uuid::Uuid::parse_str("018f4a3e-6b9f-7a1a-9b1a-2b3c4d5e6f7a").expect("valid uuid"),
        name: NameConfig {
            endonym: IpaString::from_str("test").expect("valid ipa"),
            exonym: None,
        },
        metadata: MetadataConfig {
            created_at: time::macros::datetime!(2024-05-04 00:12:00 +00:00),
            updated_at: None,
        },
        phonology: PhonologyConfig {
            sound_classes,
            phonotactics: PhonotacticsConfig::default(),
            illegal_patterns: vec![],
            prosody,
        },
        sound_changes: None,
        orthography: None,
        derivations: None,
    }
}

#[test]
fn test_unstressed() {
    let config = get_test_config(Some(ProsodicConfig::Unstressed));
    // Test that existing stress is kept, but no new stress is added.
    let ipa_str = IpaString::from_str("ˈkat.ka").expect("valid ipa");
    let word = config.syllabify(&ipa_str).expect("valid syllabification");
    assert_eq!(word.to_string(), "ˈkat.ka");

    let ipa_str_no_stress = IpaString::from_str("kat.ka").expect("valid ipa");
    let word_no_stress = config
        .syllabify(&ipa_str_no_stress)
        .expect("valid syllabification");
    assert_eq!(word_no_stress.to_string(), "kat.ka");
}

#[test]
fn test_alternating_first() {
    let config = get_test_config(Some(ProsodicConfig::Alternating {
        option: AlternatingConfig::FirstSyllable,
        stress_open_monosyllables: None,
    }));
    let word = config
        .syllabify(&IpaString::from_str("kat.ka.kat.ka").expect("valid ipa"))
        .expect("valid syllabification");
    assert_eq!(word.to_string(), "ˈkat.kaˌkat.ka");
}

#[test]
fn test_alternating_second() {
    let config = get_test_config(Some(ProsodicConfig::Alternating {
        option: AlternatingConfig::SecondSyllable,
        stress_open_monosyllables: None,
    }));
    let word = config
        .syllabify(&IpaString::from_str("kat.ka.kat.ka").expect("valid ipa"))
        .expect("valid syllabification");
    assert_eq!(word.to_string(), "katˈkak.atˌka");
}

#[test]
fn test_alternating_penultimate() {
    let config = get_test_config(Some(ProsodicConfig::Alternating {
        option: AlternatingConfig::Penultimate,
        stress_open_monosyllables: None,
    }));
    let word = config
        .syllabify(&IpaString::from_str("kat.ka.kat.ka").expect("valid ipa"))
        .expect("valid syllabification");
    assert_eq!(word.to_string(), "ˌkat.kaˈkat.ka");
}

#[test]
fn test_alternating_antepenultimate() {
    let config = get_test_config(Some(ProsodicConfig::Alternating {
        option: AlternatingConfig::Antepenultimate,
        stress_open_monosyllables: None,
    }));
    let word = config
        .syllabify(&IpaString::from_str("kat.ka.kat.ka").expect("valid ipa"))
        .expect("valid syllabification");
    assert_eq!(word.to_string(), "katˈkak.atˌka");
}

#[test]
fn test_alternating_ultimate() {
    let config = get_test_config(Some(ProsodicConfig::Alternating {
        option: AlternatingConfig::Ultimate,
        stress_open_monosyllables: None,
    }));
    let word = config
        .syllabify(&IpaString::from_str("kat.ka.kat.ka").expect("valid ipa"))
        .expect("valid syllabification");
    assert_eq!(word.to_string(), "katˌkak.atˈka");
}

#[test]
fn test_alternating_single_syllable() {
    let config = get_test_config(Some(ProsodicConfig::Alternating {
        option: AlternatingConfig::Penultimate,
        stress_open_monosyllables: Some(false),
    }));

    // Closed syllable: "kat" -> primary stress
    let word_closed = config
        .syllabify(&IpaString::from_str("kat").expect("valid ipa"))
        .expect("valid syllabification");
    assert_eq!(word_closed.to_string(), "ˈkat");

    // Open syllable: "ka" -> unstressed
    let word_open = config
        .syllabify(&IpaString::from_str("ka").expect("valid ipa"))
        .expect("valid syllabification");
    assert_eq!(word_open.to_string(), "ka");
}

#[test]
fn test_alternating_monosyllable_default_none() {
    let config = get_test_config(Some(ProsodicConfig::Alternating {
        option: AlternatingConfig::Penultimate,
        stress_open_monosyllables: None,
    }));

    // Open syllable: "ka" -> stressed by default
    let word = config
        .syllabify(&IpaString::from_str("ka").expect("valid ipa"))
        .expect("valid syllabification");
    assert_eq!(word.to_string(), "ˈka");
}

#[test]
fn test_patterned_stress_last() {
    // Foot size 2, stress location 1st, Main stress Last
    let config = get_test_config(Some(ProsodicConfig::Patterned(PatternedConfig {
        foot: FootSize::Two,
        stress_location: StressLocation::First,
        main_stress: MainStress::Last,
    })));

    assert_eq!(
        config
            .syllabify(&IpaString::from_str("ka").expect("valid ipa"))
            .expect("valid syllabification")
            .to_string(),
        "ka"
    );
    assert_eq!(
        config
            .syllabify(&IpaString::from_str("kat.ka").expect("valid ipa"))
            .expect("valid syllabification")
            .to_string(),
        "ˈkat.ka"
    );
    assert_eq!(
        config
            .syllabify(&IpaString::from_str("ka.kat.ka").expect("valid ipa"))
            .expect("valid syllabification")
            .to_string(),
        "kaˈkat.ka"
    );
    assert_eq!(
        config
            .syllabify(&IpaString::from_str("kat.ka.kat.ka").expect("valid ipa"))
            .expect("valid syllabification")
            .to_string(),
        "ˌkat.kaˈkat.ka"
    );
}

#[test]
fn test_patterned_stress_first() {
    // Foot size 2, stress location 1st, Main stress First
    let config = get_test_config(Some(ProsodicConfig::Patterned(PatternedConfig {
        foot: FootSize::Two,
        stress_location: StressLocation::First,
        main_stress: MainStress::First,
    })));

    assert_eq!(
        config
            .syllabify(&IpaString::from_str("ka").expect("valid ipa"))
            .expect("valid syllabification")
            .to_string(),
        "ka"
    );
    assert_eq!(
        config
            .syllabify(&IpaString::from_str("kat.ka").expect("valid ipa"))
            .expect("valid syllabification")
            .to_string(),
        "ˈkat.ka"
    );
    assert_eq!(
        config
            .syllabify(&IpaString::from_str("kat.ka.kat").expect("valid ipa"))
            .expect("valid syllabification")
            .to_string(),
        "ˈkat.ka.kat"
    );
    assert_eq!(
        config
            .syllabify(&IpaString::from_str("kat.ka.kat.ka").expect("valid ipa"))
            .expect("valid syllabification")
            .to_string(),
        "ˈkat.kaˌkat.ka"
    );
}

#[test]
fn test_no_fixed_stress_zipf_rng() {
    let prosody = ProsodicConfig::NoFixedStress {
        config: language::config::ZipfConfig { a: 1.0, b: 1.0 },
    };
    let config = get_test_config(None);
    let ipa_str = IpaString::from_str("kat.ka.kat.ka").expect("valid ipa");
    let word = config.syllabify(&ipa_str).expect("valid syllabification");

    let mut rng = StdRng::seed_from_u64(42);
    let res = prosody.apply_prosody_with_rng(&word, &config, &mut rng);
    let result_str = res.to_string();
    assert!(result_str.contains('ˈ'));
    assert!(result_str.contains('ˌ'));
}

#[test]
fn test_stress_propagation_capture() {
    let config = get_test_config(Some(ProsodicConfig::Alternating {
        option: AlternatingConfig::Penultimate,
        stress_open_monosyllables: None,
    }));
    let ipa_str = IpaString::from_str("pəlɪtɪkəl").expect("valid ipa");
    let word = config.syllabify(&ipa_str).expect("valid syllabification");
    assert_eq!(word.to_string(), "ˌpəl.ɪˈtɪk.əl");
}

// qual:allow(dry) - Test setup boilerplate
#[test]
fn test_prosody_validation() {
    let config_valid = ProsodicConfig::Patterned(PatternedConfig {
        foot: FootSize::Two,
        stress_location: StressLocation::First,
        main_stress: MainStress::First,
    });
    config_valid.validate().expect("valid config");

    let config_invalid = ProsodicConfig::Patterned(PatternedConfig {
        foot: FootSize::Two,
        stress_location: StressLocation::Third,
        main_stress: MainStress::First,
    });
    assert!(config_invalid.validate().is_err());
}

#[test]
fn test_alternating_open_monosyllable_configurable() {
    let config_stressed = get_test_config(Some(ProsodicConfig::Alternating {
        option: AlternatingConfig::Penultimate,
        stress_open_monosyllables: Some(true),
    }));
    let word = config_stressed
        .syllabify(&IpaString::from_str("ka").expect("valid ipa"))
        .expect("valid syllabification");
    assert_eq!(word.to_string(), "ˈka");

    let config_unstressed = get_test_config(Some(ProsodicConfig::Alternating {
        option: AlternatingConfig::Penultimate,
        stress_open_monosyllables: Some(false),
    }));
    let word2 = config_unstressed
        .syllabify(&IpaString::from_str("ka").expect("valid ipa"))
        .expect("valid syllabification");
    assert_eq!(word2.to_string(), "ka");
}

#[test]
fn test_stress_anchoring_secondary_to_primary() {
    let config = get_test_config(Some(ProsodicConfig::Alternating {
        option: AlternatingConfig::Penultimate,
        stress_open_monosyllables: None,
    }));
    let word = config
        .syllabify(&IpaString::from_str("ˌkat.ka.kat.ka").expect("valid ipa"))
        .expect("valid syllabification");
    assert_eq!(word.to_string(), "ˈkat.kaˌkat.ka");
}

#[test]
fn test_patterned_fallback_short_words() {
    let config = get_test_config(Some(ProsodicConfig::Patterned(PatternedConfig {
        foot: FootSize::Three,
        stress_location: StressLocation::First,
        main_stress: MainStress::First,
    })));

    let word_closed = config
        .syllabify(&IpaString::from_str("kat").expect("valid ipa"))
        .expect("valid syllabification");
    assert_eq!(word_closed.to_string(), "ˈkat");

    let word_open = config
        .syllabify(&IpaString::from_str("ka").expect("valid ipa"))
        .expect("valid syllabification");
    assert_eq!(word_open.to_string(), "ka");

    let word_two = config
        .syllabify(&IpaString::from_str("ka.ka").expect("valid ipa"))
        .expect("valid syllabification");
    // "ka" is stressed, so "a" (lax vowel) captures the onset "k" from the next syllable
    assert_eq!(word_two.to_string(), "ˈkak.a");
}

#[test]
fn test_patterned_fallback_anchoring_short_word() {
    let config = get_test_config(Some(ProsodicConfig::Patterned(PatternedConfig {
        foot: FootSize::Three,
        stress_location: StressLocation::First,
        main_stress: MainStress::First,
    })));

    let word = config
        .syllabify(&IpaString::from_str("kaˈka").expect("valid ipa"))
        .expect("valid syllabification");
    assert_eq!(word.to_string(), "kaˈka");
}
