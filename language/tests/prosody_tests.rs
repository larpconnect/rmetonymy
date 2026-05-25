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
fn test_alternating_stress_placement() {
    // 1. FirstSyllable
    let config = get_test_config(Some(ProsodicConfig::Alternating {
        option: AlternatingConfig::FirstSyllable,
    }));
    let word = config
        .syllabify(&IpaString::from_str("kat.ka.kat.ka").expect("valid ipa"))
        .expect("valid syllabification");
    assert_eq!(word.to_string(), "ˈkat.kaˌkat.ka");

    // 2. SecondSyllable
    let config = get_test_config(Some(ProsodicConfig::Alternating {
        option: AlternatingConfig::SecondSyllable,
    }));
    let word = config
        .syllabify(&IpaString::from_str("kat.ka.kat.ka").expect("valid ipa"))
        .expect("valid syllabification");
    assert_eq!(word.to_string(), "katˈkak.atˌka");

    // 3. Penultimate
    let config = get_test_config(Some(ProsodicConfig::Alternating {
        option: AlternatingConfig::Penultimate,
    }));
    let word = config
        .syllabify(&IpaString::from_str("kat.ka.kat.ka").expect("valid ipa"))
        .expect("valid syllabification");
    assert_eq!(word.to_string(), "ˌkat.kaˈkat.ka");

    // 4. Antepenultimate
    let config = get_test_config(Some(ProsodicConfig::Alternating {
        option: AlternatingConfig::Antepenultimate,
    }));
    let word = config
        .syllabify(&IpaString::from_str("kat.ka.kat.ka").expect("valid ipa"))
        .expect("valid syllabification");
    assert_eq!(word.to_string(), "katˈkak.atˌka");

    // 5. Ultimate
    let config = get_test_config(Some(ProsodicConfig::Alternating {
        option: AlternatingConfig::Ultimate,
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
fn test_patterned_stress() {
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
    let word = config.syllabify(&ipa_str).expect("valid syllabification"); // Default is No Fixed Stress

    // Run with standard apply_prosody_with_rng using seeded StdRng
    let mut rng = StdRng::seed_from_u64(42);
    let res = prosody.apply_prosody_with_rng(&word, &config, &mut rng);
    // Ensure it places primary stress and alternates secondary stress
    let result_str = res.to_string();
    assert!(result_str.contains('ˈ'));
    assert!(result_str.contains('ˌ'));
}

#[test]
fn test_stress_propagation_capture() {
    // Under unstressed or no-stress condition, "pəlɪtɪkəl" splits as: pə.lɪ.tɪ.kəl
    // Under Alternating Penultimate stress config, "tɪ" is stressed.
    // Since "tɪ" has a short vowel ("ɪ") and is stressed, it captures the "k" from the next syllable "kəl".
    // This results in "pə.lɪ.ˈtɪk.əl".
    let config = get_test_config(Some(ProsodicConfig::Alternating {
        option: AlternatingConfig::Penultimate,
    }));
    let ipa_str = IpaString::from_str("pəlɪtɪkəl").expect("valid ipa");
    let word = config.syllabify(&ipa_str).expect("valid syllabification");
    assert_eq!(word.to_string(), "ˌpəl.ɪˈtɪk.əl");
}

#[test]
fn test_prosody_validation() {
    // Valid: Foot size 2, stress location 1st
    let config_valid = ProsodicConfig::Patterned(PatternedConfig {
        foot: FootSize::Two,
        stress_location: StressLocation::First,
        main_stress: MainStress::First,
    });
    config_valid.validate().expect("valid config");

    // Invalid: Foot size 2, stress location 3rd
    let config_invalid = ProsodicConfig::Patterned(PatternedConfig {
        foot: FootSize::Two,
        stress_location: StressLocation::Third,
        main_stress: MainStress::First,
    });
    assert!(config_invalid.validate().is_err());
}
