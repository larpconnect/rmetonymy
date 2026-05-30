// qual:allow(srp) - Cucumber test module with multiple step definitions
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

fn make_test_sound_changes(name: Option<String>, rule: String) -> SoundChanges {
    SoundChanges {
        preamble: Vec::new(),
        eras: vec![EraRules {
            era: 1,
            rules: vec![SoundChangeRule {
                name,
                changes: vec![rule],
            }],
        }],
    }
}

fn check_test_result(
    res_opt: Option<&Result<String, String>>,
    expected: &str,
    name: &str,
) -> Result<(), String> {
    let res = res_opt.ok_or_else(|| format!("No {name} was applied"))?;
    match res {
        Ok(actual) => {
            if actual == expected {
                Ok(())
            } else {
                Err(format!("Expected {name} '{expected}', got '{actual}'"))
            }
        }
        Err(err) => Err(format!("{name} failed: {err}")),
    }
}

fn fmt_debug<E: std::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

#[given(expr = "a default language configuration")]
pub fn given_default_language_config(world: &mut SoundChangeWorld) {
    world.config = Some(get_test_config());
}

#[when(expr = "I apply sound change rule {string} to the word {string}")]
fn when_apply_sound_change_rule(world: &mut SoundChangeWorld, rule: String, input: String) {
    let config = world
        .config
        .as_ref()
        .expect("LanguageConfig should be initialized");
    let res = (|| {
        let parsed_word = PhonemeSequence::from_str(&input).map_err(fmt_debug)?;
        let ipa_word = IpaWord::try_from_sequence(&parsed_word, config).map_err(fmt_debug)?;

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
        let (res_word, _) = apply_sound_changes(&ipa_word, &compiled, (1, 1), config, false)?;
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
        let parsed_word = PhonemeSequence::from_str(&input).map_err(fmt_debug)?;
        let ipa_word = IpaWord::try_from_sequence(&parsed_word, config).map_err(fmt_debug)?;

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
        let (res_word, _) = apply_sound_changes(&ipa_word, &compiled, (1, 1), config, true)?;
        Ok(res_word.to_string())
    })();
    world.result = Some(res);
    drop(input);
}

#[when(expr = "I compile sound change rule {string}")]
fn when_compile_sound_change_rule(world: &mut SoundChangeWorld, rule: String) {
    let sc = make_test_sound_changes(None, rule);
    let res = compile_sound_changes(&sc)
        .map(|_| "Compilation Succeeded".to_string())
        .map_err(|e| e.to_string());
    world.result = Some(res);
}

#[when(expr = "I compile a sound change rule named {string} with rule {string}")]
fn when_compile_named_sound_change_rule(world: &mut SoundChangeWorld, name: String, rule: String) {
    let sc = make_test_sound_changes(Some(name), rule);
    let res = compile_sound_changes(&sc)
        .map(|_| "Compilation Succeeded".to_string())
        .map_err(|e| e.to_string());
    world.result = Some(res);
}

#[then(expr = "the result should be {string}")]
fn then_result_should_be(world: &mut SoundChangeWorld, expected: String) -> Result<(), String> {
    let outcome = check_test_result(world.result.as_ref(), &expected, "result");
    drop(expected);
    outcome
}

#[then(expr = "the boundary result should be {string}")]
fn then_boundary_result_should_be(
    world: &mut SoundChangeWorld,
    expected: String,
) -> Result<(), String> {
    let outcome = check_test_result(world.result.as_ref(), &expected, "boundary result");
    drop(expected);
    outcome
}
fn get_compilation_result<'a>(
    world: &'a SoundChangeWorld,
    no_comp_msg: &str,
) -> Result<&'a Result<String, String>, String> {
    world.result.as_ref().ok_or_else(|| no_comp_msg.to_string())
}

fn check_failure_outcome(
    res: &Result<String, String>,
    expected_error: &str,
    success_err_msg: &str,
    prefix_err_msg: &str,
) -> Result<(), String> {
    match res {
        Ok(_) => Err(success_err_msg.to_string()),
        Err(err) => {
            if err.contains(expected_error) {
                Ok(())
            } else {
                Err(format!(
                    "{prefix_err_msg} containing '{expected_error}', got '{err}'"
                ))
            }
        }
    }
}

#[then(expr = "it should fail validation with message containing {string}")]
fn then_it_should_fail_validation_with_message(
    world: &mut SoundChangeWorld,
    expected_error: String,
) -> Result<(), String> {
    let res = get_compilation_result(world, "No compilation was performed")?;
    let outcome = check_failure_outcome(
        res,
        &expected_error,
        "Expected compilation to fail, but it succeeded",
        "Expected error",
    );
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
        let parsed_word = PhonemeSequence::from_str(&input).map_err(fmt_debug)?;
        let ipa_word = IpaWord::try_from_sequence(&parsed_word, config).map_err(fmt_debug)?;

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
        let parsed_word = PhonemeSequence::from_str(&input).map_err(fmt_debug)?;
        let ipa_word = IpaWord::try_from_sequence(&parsed_word, config).map_err(fmt_debug)?;

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
        let parsed_word = PhonemeSequence::from_str(&input).map_err(fmt_debug)?;
        let ipa_word = IpaWord::try_from_sequence(&parsed_word, config).map_err(fmt_debug)?;

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
    let res = get_compilation_result(world, "No orthography compilation was performed")?;
    let outcome = check_failure_outcome(
        res,
        &expected_error,
        "Expected orthography compilation to fail, but it succeeded",
        "Expected orthography error",
    );
    drop(expected_error);
    outcome
}

#[tokio::main]
#[expect(
    clippy::let_underscore_must_use,
    reason = "dummy block to keep functions in scope"
)]
async fn main() {
    if false {
        let mut world = SoundChangeWorld::default();
        given_default_language_config(&mut world);
        when_apply_sound_change_rule(&mut world, String::new(), String::new());
        when_apply_sound_change_rule_showing_boundaries(&mut world, String::new(), String::new());
        when_compile_sound_change_rule(&mut world, String::new());
        when_compile_named_sound_change_rule(&mut world, String::new(), String::new());
        let _ = then_result_should_be(&mut world, String::new());
        let _ = then_boundary_result_should_be(&mut world, String::new());
        let _ = then_it_should_fail_validation_with_message(&mut world, String::new());
        when_apply_orthography_rule(&mut world, String::new(), String::new());
        when_apply_orthography_rules_two(&mut world, String::new(), String::new(), String::new());
        when_apply_empty_orthography(&mut world, String::new());
        let _ = then_orthography_result_should_be(&mut world, String::new());
        when_compile_orthography_rule(&mut world, String::new());
        let _ = then_it_should_fail_orthography_validation_with_message(&mut world, String::new());
    }
    SoundChangeWorld::cucumber()
        .run_and_exit("tests/features/soundchange_evaluation.feature")
        .await;
}
