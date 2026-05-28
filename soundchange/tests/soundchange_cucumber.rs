use cucumber::{World, given, then, when};
use ipa::sequence::PhonemeSequence;
use language::config::{EraRules, LanguageConfig, SoundChangeRule, SoundChanges};
use language::syllable::IpaWord;
use soundchange::compile_sound_changes;
use soundchange::evaluator::apply_sound_changes;
use std::str::FromStr;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Default, World)]
pub struct SoundChangeWorld {
    pub config: Option<LanguageConfig>,
    pub result: Option<Result<String, String>>,
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
    serde_json::from_str(json).expect("valid test configuration")
}

#[given(expr = "a default language configuration")]
fn given_default_language_config(world: &mut SoundChangeWorld) {
    world.config = Some(get_test_config());
}

#[when(expr = "I apply sound change rule {string} to the word {string}")]
fn when_apply_sound_change_rule(world: &mut SoundChangeWorld, rule: String, input: String) {
    let config = world
        .config
        .as_ref()
        .expect("LanguageConfig should be initialized");
    let res = (|| {
        let parsed_word = PhonemeSequence::from_str(&input).map_err(|e| format!("{e:?}"))?;
        let ipa_word =
            IpaWord::try_from_sequence(&parsed_word, config).map_err(|e| format!("{e:?}"))?;

        let sc = SoundChanges {
            preamble: Vec::new(),
            eras: vec![EraRules {
                era: 1,
                rules: vec![SoundChangeRule {
                    name: None,
                    changes: vec![rule],
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
    world.result = Some(res);
    drop(input);
}

#[when(expr = "I apply sound change rule {string} to the word {string} showing boundaries")]
fn when_apply_sound_change_rule_showing_boundaries(
    world: &mut SoundChangeWorld,
    rule: String,
    input: String,
) {
    let config = world
        .config
        .as_ref()
        .expect("LanguageConfig should be initialized");
    let res = (|| {
        let parsed_word = PhonemeSequence::from_str(&input).map_err(|e| format!("{e:?}"))?;
        let ipa_word =
            IpaWord::try_from_sequence(&parsed_word, config).map_err(|e| format!("{e:?}"))?;

        let sc = SoundChanges {
            preamble: Vec::new(),
            eras: vec![EraRules {
                era: 1,
                rules: vec![SoundChangeRule {
                    name: None,
                    changes: vec![rule],
                }],
            }],
        };

        let compiled = compile_sound_changes(&sc).map_err(|e| e.to_string())?;
        let (res_word, _) = apply_sound_changes(&ipa_word, &compiled, 1, 1, config, true)?;
        Ok(res_word.to_string())
    })();
    world.result = Some(res);
    drop(input);
}

#[when(expr = "I compile sound change rule {string}")]
fn when_compile_sound_change_rule(world: &mut SoundChangeWorld, rule: String) {
    let sc = SoundChanges {
        preamble: Vec::new(),
        eras: vec![EraRules {
            era: 1,
            rules: vec![SoundChangeRule {
                name: None,
                changes: vec![rule],
            }],
        }],
    };
    let res = compile_sound_changes(&sc)
        .map(|_| "Compilation Succeeded".to_string())
        .map_err(|e| e.to_string());
    world.result = Some(res);
}

#[when(expr = "I compile a sound change rule named {string} with rule {string}")]
fn when_compile_named_sound_change_rule(world: &mut SoundChangeWorld, name: String, rule: String) {
    let sc = SoundChanges {
        preamble: Vec::new(),
        eras: vec![EraRules {
            era: 1,
            rules: vec![SoundChangeRule {
                name: Some(name),
                changes: vec![rule],
            }],
        }],
    };
    let res = compile_sound_changes(&sc)
        .map(|_| "Compilation Succeeded".to_string())
        .map_err(|e| e.to_string());
    world.result = Some(res);
}

#[then(expr = "the result should be {string}")]
fn then_result_should_be(world: &mut SoundChangeWorld, expected: String) -> Result<(), String> {
    let res = world
        .result
        .as_ref()
        .ok_or_else(|| "No sound change was applied".to_string())?;
    let outcome = match res {
        Ok(actual) => {
            if actual == &expected {
                Ok(())
            } else {
                Err(format!("Expected result '{expected}', got '{actual}'"))
            }
        }
        Err(err) => Err(format!("Sound change failed: {err}")),
    };
    drop(expected);
    outcome
}

#[then(expr = "the boundary result should be {string}")]
fn then_boundary_result_should_be(
    world: &mut SoundChangeWorld,
    expected: String,
) -> Result<(), String> {
    let res = world
        .result
        .as_ref()
        .ok_or_else(|| "No sound change was applied".to_string())?;
    let outcome = match res {
        Ok(actual) => {
            if actual == &expected {
                Ok(())
            } else {
                Err(format!(
                    "Expected boundary result '{expected}', got '{actual}'"
                ))
            }
        }
        Err(err) => Err(format!("Sound change failed: {err}")),
    };
    drop(expected);
    outcome
}

#[then(expr = "it should fail validation with message containing {string}")]
fn then_it_should_fail_validation_with_message(
    world: &mut SoundChangeWorld,
    expected_error: String,
) -> Result<(), String> {
    let res = world
        .result
        .as_ref()
        .ok_or_else(|| "No compilation was performed".to_string())?;
    let outcome = match res {
        Ok(_) => Err("Expected compilation to fail, but it succeeded".to_string()),
        Err(err) => {
            if err.contains(&expected_error) {
                Ok(())
            } else {
                Err(format!(
                    "Expected error containing '{expected_error}', got '{err}'"
                ))
            }
        }
    };
    drop(expected_error);
    outcome
}

#[when(expr = "I apply orthography rule {string} to the word {string}")]
fn when_apply_orthography_rule(world: &mut SoundChangeWorld, rule: String, input: String) {
    let config = world
        .config
        .as_ref()
        .expect("LanguageConfig should be initialized");
    let res = (|| {
        let parsed_word = PhonemeSequence::from_str(&input).map_err(|e| format!("{e:?}"))?;
        let ipa_word =
            IpaWord::try_from_sequence(&parsed_word, config).map_err(|e| format!("{e:?}"))?;

        let compiled_ortho =
            soundchange::compile_ortho_rules(&[rule]).map_err(|e| e.to_string())?;
        let (ortho_res, _) =
            soundchange::apply_orthography(&ipa_word, &compiled_ortho, config, false)?;
        let nfc = ortho_res.nfc().collect::<String>();
        Ok(nfc)
    })();
    world.result = Some(res);
    drop(input);
}
#[when(expr = "I apply orthography rules {string} and {string} to the word {string}")]
fn when_apply_orthography_rules_two(
    world: &mut SoundChangeWorld,
    rule1: String,
    rule2: String,
    input: String,
) {
    let config = world
        .config
        .as_ref()
        .expect("LanguageConfig should be initialized");
    let res = (|| {
        let parsed_word = PhonemeSequence::from_str(&input).map_err(|e| format!("{e:?}"))?;
        let ipa_word =
            IpaWord::try_from_sequence(&parsed_word, config).map_err(|e| format!("{e:?}"))?;

        let compiled_ortho =
            soundchange::compile_ortho_rules(&[rule1, rule2]).map_err(|e| e.to_string())?;
        let (ortho_res, _) =
            soundchange::apply_orthography(&ipa_word, &compiled_ortho, config, false)?;
        let nfc = ortho_res.nfc().collect::<String>();
        Ok(nfc)
    })();
    world.result = Some(res);
    drop(input);
}
#[when(expr = "I apply empty orthography to the word {string}")]
fn when_apply_empty_orthography(world: &mut SoundChangeWorld, input: String) {
    let config = world
        .config
        .as_ref()
        .expect("LanguageConfig should be initialized");
    let res = (|| {
        let parsed_word = PhonemeSequence::from_str(&input).map_err(|e| format!("{e:?}"))?;
        let ipa_word =
            IpaWord::try_from_sequence(&parsed_word, config).map_err(|e| format!("{e:?}"))?;

        let compiled_ortho = Vec::new();
        let (ortho_res, _) =
            soundchange::apply_orthography(&ipa_word, &compiled_ortho, config, false)?;
        let nfc = ortho_res.nfc().collect::<String>();
        Ok(nfc)
    })();
    world.result = Some(res);
    drop(input);
}

#[then(expr = "the orthography result should be {string}")]
fn then_orthography_result_should_be(
    world: &mut SoundChangeWorld,
    expected: String,
) -> Result<(), String> {
    let res = world
        .result
        .as_ref()
        .ok_or_else(|| "No orthography change was applied".to_string())?;
    let outcome = match res {
        Ok(actual) => {
            let actual_nfc = actual.nfc().collect::<String>();
            let expected_nfc = expected.nfc().collect::<String>();
            if actual_nfc == expected_nfc {
                Ok(())
            } else {
                Err(format!(
                    "Expected orthography result '{expected_nfc}', got '{actual_nfc}'"
                ))
            }
        }
        Err(err) => Err(format!("Orthography apply failed: {err}")),
    };
    drop(expected);
    outcome
}

#[when(expr = "I compile orthography rule {string}")]
fn when_compile_orthography_rule(world: &mut SoundChangeWorld, rule: String) {
    let res = soundchange::compile_ortho_rules(&[rule])
        .map(|_| "Compilation Succeeded".to_string())
        .map_err(|e| e.to_string());
    world.result = Some(res);
}

#[then(expr = "it should fail orthography validation with message containing {string}")]
fn then_it_should_fail_orthography_validation_with_message(
    world: &mut SoundChangeWorld,
    expected_error: String,
) -> Result<(), String> {
    let res = world
        .result
        .as_ref()
        .ok_or_else(|| "No orthography compilation was performed".to_string())?;
    let outcome = match res {
        Ok(_) => Err("Expected orthography compilation to fail, but it succeeded".to_string()),
        Err(err) => {
            if err.contains(&expected_error) {
                Ok(())
            } else {
                Err(format!(
                    "Expected orthography error containing '{expected_error}', got '{err}'"
                ))
            }
        }
    };
    drop(expected_error);
    outcome
}

#[tokio::main]
async fn main() {
    SoundChangeWorld::cucumber()
        .run_and_exit("tests/features/soundchange_evaluation.feature")
        .await;
}
