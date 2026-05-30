use language::config::{
    GeneratorConfig, LanguageConfig, MetadataConfig, NameConfig, PhonologyConfig,
    PhonotacticsConfig, SoundClass, ZipfConfig,
};
use language::generator::{SeedableRng, StdRng, WordPattern, generate_word, sample_index};
use std::collections::BTreeMap;
use time::OffsetDateTime;
use uuid::Uuid;

#[test]
fn test_word_pattern_parsing_and_display() {
    let cases = vec![
        "CVC",
        "VV",
        "kV",
        "{p,t,k}V",
        "CV(C)",
        "CV(C)15%",
        "RVC",
        "QVC",
        "CV(CV)",
        "CV(C(V)10%)VF",
        "CV.CV",
        "CVˈCV",
        "[noun.masculine]t",
        "V[verb]",
        "{C,p,t}V", // Set containing sound class C
    ];

    for case in cases {
        let parsed: WordPattern = case.parse().expect("failed to parse pattern");
        let displayed = parsed.to_string();
        assert_eq!(displayed, case, "Display mismatch for case: {case}");
    }
}

#[test]
fn test_rng_zipf_selection() {
    let config = GeneratorConfig::Zipf {
        config: ZipfConfig { a: 1.0, b: 0.0 },
    };
    let mut rng = StdRng::seed_from_u64(12345);
    let mut count_0 = 0;
    let mut count_1 = 0;
    for _ in 0..1000 {
        let idx = sample_index(2, &config, &mut rng);
        if idx == 0 {
            count_0 += 1;
        } else if idx == 1 {
            count_1 += 1;
        }
    }
    // With a=1.0 and b=0.0 on 2 choices:
    // rank 1 weight = 1.0, rank 2 weight = 0.5. Total = 1.5.
    // rank 1 prob = 66.6%, rank 2 prob = 33.3%
    assert!(count_0 > 600 && count_0 < 730);
    assert!(count_1 > 270 && count_1 < 400);
}

#[test]
fn test_word_generation() {
    let sound_classes = BTreeMap::from([
        ("C".parse().expect("valid sound class key C"), SoundClass { values: vec!["p".into(), "t".into(), "k".into()], generator: None }),
        ("V".parse().expect("valid sound class key V"), SoundClass { values: vec!["a".into(), "e".into(), "i".into()], generator: None }),
    ]);
    let generators = BTreeMap::from([
        ("default".into(), language::generator::WordGenerator {
            patterns: vec!["CVC".parse().expect("valid pattern CVC")],
            generator: GeneratorConfig::Equiprobable,
        }),
        ("noun".into(), language::generator::WordGenerator {
            patterns: vec!["(C)V[default]".parse().expect("valid pattern (C)V[default]")],
            generator: GeneratorConfig::Equiprobable,
        }),
    ]);
    let config = LanguageConfig {
        id: Uuid::now_v7(),
        name: NameConfig { endonym: "test".parse().expect("valid endonym"), exonym: None },
        metadata: MetadataConfig { created_at: OffsetDateTime::now_utc(), updated_at: None },
        phonology: PhonologyConfig {
            sound_classes,
            phonotactics: PhonotacticsConfig { generators },
            illegal_patterns: vec![],
            prosody: None,
        },
        sound_changes: None,
        orthography: None,
        derivations: None,
    };

    config.validate().expect("config should be valid");

    let mut rng = StdRng::seed_from_u64(12345);
    let mut warning_logged = false;
    let word = generate_word("noun", &config, &mut rng, 1, &mut warning_logged)
        .expect("should generate word");

    assert!(!word.is_empty());
}
