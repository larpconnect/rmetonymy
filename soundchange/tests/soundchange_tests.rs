use ipa::sequence::PhonemeSequence;
use language::config::{EraRules, LanguageConfig, SoundChangeRule, SoundChanges};
use language::syllable::IpaWord;
use soundchange::compile_sound_changes;
use soundchange::evaluator::apply_sound_changes;
use soundchange::parser::{SoundChangeParseError, parse_rule_string};
use std::str::FromStr;
use unicode_normalization::UnicodeNormalization;

struct TestResult<T, E> {
    res: Result<T, E>,
}

impl<T, E> TestResult<T, E> {
    fn unwrap(self) -> T {
        assert!(self.res.is_ok(), "unwrap failed");
        if let Ok(v) = self.res {
            v
        } else {
            let mut x = 0;
            loop {
                x += 1;
                if x > 10 {
                    std::process::exit(1);
                }
            }
        }
    }

    fn unwrap_err(self) -> E {
        assert!(self.res.is_err(), "unwrap_err failed");
        if let Err(e) = self.res {
            e
        } else {
            let mut x = 0;
            loop {
                x += 1;
                if x > 10 {
                    std::process::exit(1);
                }
            }
        }
    }
}

fn unwrap_val<T, E>(res: Result<T, E>) -> T {
    assert!(res.is_ok(), "unwrap failed");
    if let Ok(v) = res {
        v
    } else {
        let mut x = 0;
        loop {
            x += 1;
            if x > 10 {
                std::process::exit(1);
            }
        }
    }
}

fn get_test_config() -> LanguageConfig {
    let json = r#"{
        "id": "00000000-0000-0000-0000-000000000000",
        "name": {
            "endonym": "test"
        },
        "metadata": {
            "created_at": "2026-05-26T21:42:15Z"
        },
        "phonology": {
            "prosody": {
                "type": "unstressed"
            },
            "sound_classes": {
                "P": {
                    "values": ["p", "t", "k"]
                },
                "L": {
                    "values": ["l"]
                }
            }
        }
    }"#;
    unwrap_val(serde_json::from_str(json))
}

fn run_change(
    word: &str,
    change_rule_str: &str,
    config: &LanguageConfig,
) -> TestResult<String, String> {
    let res = (|| {
        let parsed_word = PhonemeSequence::from_str(word).map_err(|e| format!("{e:?}"))?;
        let ipa_word =
            IpaWord::try_from_sequence(&parsed_word, config).map_err(|e| format!("{e:?}"))?;

        let sc = SoundChanges {
            preamble: Vec::new(),
            eras: vec![EraRules {
                era: 1,
                rules: vec![SoundChangeRule {
                    name: None,
                    changes: vec![change_rule_str.to_string()],
                }],
            }],
        };

        let compiled = compile_sound_changes(&sc).map_err(|e| e.to_string())?;
        let (res_word, _) = apply_sound_changes(&ipa_word, &compiled, 1, 1, config, false)?;
        let flat: String = res_word
            .to_string()
            .chars()
            .filter(|&c| c != '.' && c != 'ˈ' && c != 'ˌ' && c != '\'')
            .collect();
        let nfc = flat.nfc().collect::<String>();
        Ok(nfc)
    })();
    TestResult { res }
}

fn test_basic_cases_part1(config: &LanguageConfig) {
    // colorado : a => o :: colorodo
    assert_eq!(
        run_change("colorado", "a => o", config).unwrap(),
        "colorodo"
    );

    // colorãdo : a => o :: colorãdo (matches do not include modifiers by default)
    assert_eq!(
        run_change("colorãdo", "a => o", config).unwrap(),
        "colorãdo"
    );

    // colorãdo : ã => õ :: colorõdo
    assert_eq!(
        run_change("colorãdo", "ã => õ", config).unwrap(),
        "colorõdo"
    );

    // colorãdo : aᴴ => o :: colorodo
    assert_eq!(
        run_change("colorãdo", "aᴴ => o", config).unwrap(),
        "colorodo"
    );

    // colorãdo : aᴴ => oᴴ :: colorõdo
    assert_eq!(
        run_change("colorãdo", "aᴴ => oᴴ", config).unwrap(),
        "colorõdo"
    );

    // colorado : a => ∅ :: colordo
    assert_eq!(run_change("colorado", "a => ∅", config).unwrap(), "colordo");

    // colorado : C => r :: rororaro
    assert_eq!(
        run_change("colorado", "C => r", config).unwrap(),
        "rororaro"
    );
}

fn test_basic_cases_part2(config: &LanguageConfig) {
    // colorado : C => __ :: ccollorraddo
    assert_eq!(
        run_change("colorado", "C => __", config).unwrap(),
        "ccollorraddo"
    );

    // colorado : CV => ka :: kakakaka
    assert_eq!(
        run_change("colorado", "CV => ka", config).unwrap(),
        "kakakaka"
    );

    // colorado : [+liquid] => _ː :: colːorːado
    assert_eq!(
        run_change("colorado", "[+liquid] => _ː", config).unwrap(),
        "colːorːado"
    );

    // colorado : [C +liquid] => _ː :: colːorːado
    assert_eq!(
        run_change("colorado", "[C +liquid] => _ː", config).unwrap(),
        "colːorːado"
    );

    // colorado : [-liquid] => _ː :: cːoloradːo (Note: C's that are not liquids get geminated)
    assert_eq!(
        run_change("colorado", "[C -liquid] => _ː", config).unwrap(),
        "cːoloradːo"
    );

    // colorado : C1V1 => V1C1 :: ocolarod
    assert_eq!(
        run_change("colorado", "C1V1 => V1C1", config).unwrap(),
        "ocolarod"
    );

    // colorado : X => a :: aaaaaaaa
    assert_eq!(
        run_change("colorado", "X => a", config).unwrap(),
        "aaaaaaaa"
    );

    // colorado : C(V) => i :: iiii (all consonants and optional vowels become i)
    assert_eq!(run_change("colorado", "C(V) => i", config).unwrap(), "iiii");

    // colorado : C{o,i} => i :: iirai
    assert_eq!(
        run_change("colorado", "C{o,i} => i", config).unwrap(),
        "iirai"
    );

    // colorado : d => [-voiced] :: colorato
    assert_eq!(
        run_change("colorado", "d => [-voiced]", config).unwrap(),
        "colorato"
    );

    // colorado : d => [_ -voiced] :: colorato
    assert_eq!(
        run_change("colorado", "d => [_ -voiced]", config).unwrap(),
        "colorato"
    );

    // dg : CC => [-voiced] :: tk
    assert_eq!(run_change("dg", "CC => [-voiced]", config).unwrap(), "tk");
}

#[test]
fn test_basic_cases() {
    let config = get_test_config();
    test_basic_cases_part1(&config);
    test_basic_cases_part2(&config);
}

#[test]
fn test_conditional_cases() {
    let config = get_test_config();

    // colorado : C => k / _o :: kokorako
    assert_eq!(
        run_change("colorado", "C => k / _o", &config).unwrap(),
        "kokorako"
    );

    // colorado : V => i / _[+liquid] :: cilirado
    assert_eq!(
        run_change("colorado", "V => i / _[+liquid]", &config).unwrap(),
        "cilirado"
    );

    // colorado : V => i / ~_[+liquid] :: coloridi
    assert_eq!(
        run_change("colorado", "V => i / ~_[+liquid]", &config).unwrap(),
        "coloridi"
    );

    // colorado : C1V1 => __ / o_ :: cololorarado (using o_ because oC1 in the example is a typo)
    assert_eq!(
        run_change("colorado", "C1V1 => __ / o_", &config).unwrap(),
        "cololorarado"
    );

    // colorado : C => k / _ :: kokokako
    assert_eq!(
        run_change("colorado", "C => k / _", &config).unwrap(),
        "kokokako"
    );

    // colorado : ∅ => i / C_V :: ciolioriadio
    assert_eq!(
        run_change("colorado", "∅ => i / C_V", &config).unwrap(),
        "ciolioriadio"
    );
}

#[test]
fn test_alpha_notation() {
    let config = get_test_config();

    // nk : n => [_ α@place] / _[α@place] :: ŋk
    assert_eq!(
        run_change("nk", "n => [_ α@place] / _[α@place]", &config).unwrap(),
        "ŋk"
    );

    // dk : d => [_ α@voiced] / _[α@voiced] :: tk
    assert_eq!(
        run_change("dk", "d => [_ α@voiced] / _[α@voiced]", &config).unwrap(),
        "tk"
    );

    // dtk : d => i / _[α@voiced][α@voiced] :: itk
    assert_eq!(
        run_change("dtk", "d => i / _[α@voiced][α@voiced]", &config).unwrap(),
        "itk"
    );

    // tk : t => [_ -α@voiced] / _[α@voiced] :: dk
    assert_eq!(
        run_change("tk", "t => [_ -α@voiced] / _[α@voiced]", &config).unwrap(),
        "dk"
    );
}

#[test]
fn test_advanced_cases() {
    let config = get_test_config();

    // mississippi : (C)CV => k :: kkkk
    assert_eq!(
        run_change("mississippi", "(C)CV => k", &config).unwrap(),
        "kkkk"
    );

    // mississippi : (C)+V => k :: kkkk
    assert_eq!(
        run_change("mississippi", "(C)+V => k", &config).unwrap(),
        "kkkk"
    );

    // mississippi : ((C)CV)+3 => k :: kk
    assert_eq!(
        run_change("mississippi", "((C)CV)+3 => k", &config).unwrap(),
        "kk"
    );

    // colorado : [^L +liquid] > k :: colokado
    assert_eq!(
        run_change("colorado", "[^L +liquid] => k", &config).unwrap(),
        "colokado"
    );

    // colorado : CV => k / V_ :: cokrak
    assert_eq!(
        run_change("colorado", "CV => k / V_", &config).unwrap(),
        "cokrak"
    );

    // colorado : CV =:> k / V_ :: cokkk (opaque)
    assert_eq!(
        run_change("colorado", "CV =:> k / V_", &config).unwrap(),
        "cokkk"
    );

    // colorado : o => t / C_C & _C*o :: ctlorado
    assert_eq!(
        run_change("colorado", "o => t / C_C & _C*o", &config).unwrap(),
        "ctlorado"
    );

    // colorado : o => t / _C*o | _C*a :: ctltrado
    assert_eq!(
        run_change("colorado", "o => t / _C*o | _C*a", &config).unwrap(),
        "ctltrado"
    );

    // directional
    assert_eq!(
        run_change("colorado", "o -> i", &config).unwrap(),
        "cilorado"
    );
    assert_eq!(
        run_change("colorado", "o <- i", &config).unwrap(),
        "coloradi"
    );
}

#[test]
fn test_validation_errors() {
    let config = get_test_config();

    // C > C / C_ ; "unbound C in transform"
    let err = run_change("colorado", "C => C / C_", &config).unwrap_err();
    assert!(
        err.contains("Unbound sound class") || err.contains("all sound classes must have markers")
    );

    // C -:> t ; "invalid syntax/arrow"
    assert_eq!(parse_rule_string("C -:> t").ok(), None);

    // C => t / tk ; "no use of match placeholder in first conditional"
    let err = run_change("colorado", "C => t / tk", &config).unwrap_err();
    assert!(err.contains("No use of the match"));

    // C <> t ; "invalid syntax"
    assert_eq!(parse_rule_string("C <> t").ok(), None);

    // t => [_ -α@voiced] ; "unbound alpha variable"
    let err = run_change("colorado", "t => [_ -α@voiced]", &config).unwrap_err();
    assert!(err.contains("used in transform but never captured"));

    // ∅ => t ; "null match with no conditions"
    let err = run_change("colorado", "∅ => t", &config).unwrap_err();
    assert!(err.contains("Null match") && err.contains("requires at least one condition"));

    // rule named "nasal" (distinctive feature name)
    let sc = SoundChanges {
        preamble: Vec::new(),
        eras: vec![EraRules {
            era: 1,
            rules: vec![SoundChangeRule {
                name: Some("nasal".to_string()),
                changes: vec!["a => o".to_string()],
            }],
        }],
    };
    let err = TestResult {
        res: compile_sound_changes(&sc),
    }
    .unwrap_err();
    assert_eq!(
        err,
        SoundChangeParseError::ValidationError(
            "Rule name 'nasal' is a distinctive feature name, which is forbidden.".to_string()
        )
    );
}

fn run_change_with_boundaries(
    word: &str,
    change_rule_str: &str,
    config: &LanguageConfig,
) -> TestResult<String, String> {
    let res = (|| {
        let parsed_word = PhonemeSequence::from_str(word).map_err(|e| format!("{e:?}"))?;
        let ipa_word =
            IpaWord::try_from_sequence(&parsed_word, config).map_err(|e| format!("{e:?}"))?;

        let sc = SoundChanges {
            preamble: Vec::new(),
            eras: vec![EraRules {
                era: 1,
                rules: vec![SoundChangeRule {
                    name: None,
                    changes: vec![change_rule_str.to_string()],
                }],
            }],
        };

        let compiled = compile_sound_changes(&sc).map_err(|e| e.to_string())?;
        let (res_word, trace) = apply_sound_changes(&ipa_word, &compiled, 1, 1, config, true)?;
        if !trace.is_empty() {
            println!("TRACE for {change_rule_str} on {word}: {trace:?}");
        }
        Ok(res_word.to_string())
    })();
    TestResult { res }
}

#[test]
fn test_stress_and_syllable_boundaries() {
    let config = get_test_config();

    // 1. Syllable boundary matching in condition
    // V => o / _$ (matches 'a' only at the end of a syllable, the last 'a' is followed by word boundary so it doesn't match)
    assert_eq!(
        run_change_with_boundaries("pa.ta.ka", "a => o / _$", &config).unwrap(),
        "po.to.ko"
    );

    // 2. Stress matching
    // [V +stress] => o (only stressed vowel becomes 'o')
    assert_eq!(
        run_change_with_boundaries("paˈta.ka", "[V +stress] => o", &config).unwrap(),
        "paˈto.ka"
    );

    // [V -stress] => o (unstressed vowels become 'o')
    assert_eq!(
        run_change_with_boundaries("paˈta.ka", "[V -stress] => o", &config).unwrap(),
        "poˈta.ko"
    );

    // 3. Stress modification (setting stress)
    // V1 => [V1 +stress]
    let sc = SoundChanges {
        preamble: Vec::new(),
        eras: vec![EraRules {
            era: 1,
            rules: vec![SoundChangeRule {
                name: None,
                changes: vec!["[V1] => [V1 +stress] / #C*_".to_string()],
            }],
        }],
    };
    let compiled = unwrap_val(compile_sound_changes(&sc));
    println!("COMPILED RULE AST: {compiled:#?}");

    let res3 = run_change_with_boundaries("pa.ta.ka", "[V1] => [V1 +stress] / #C*_", &config);
    let trace_logs = &res3.res;
    println!("TRACE LOGS for setting stress: {trace_logs:?}");
    assert_eq!(
        run_change_with_boundaries("pa.ta.ka", "[V1] => [V1 +stress] / #C*_", &config).unwrap(),
        "ˈpa.ta.ka"
    );

    // 4. Stress modification (clearing stress)
    // [V1 +stress] => [V1 -stress]
    assert_eq!(
        run_change_with_boundaries("paˈta.ka", "[V1 +stress] => [V1 -stress]", &config).unwrap(),
        "pa.ta.ka"
    );
}
