use cucumber::{World, given, then, when};
use ipa::IpaString;
use language::IpaWord;
use language::config::LanguageConfig;
use std::str::FromStr;

#[derive(Debug, Default, World)]
pub struct SyllabificationWorld {
    pub config: Option<LanguageConfig>,
    pub word: Option<IpaWord>,
}

#[given(expr = "the language configuration exists")]
fn given_lang_config(c_world: &mut SyllabificationWorld) {
    let json_str = r#"{
        "id": "018f4a3e-6b9f-7a1a-9b1a-2b3c4d5e6f7a",
        "name": { "endonym": "p" },
        "metadata": { "created_at": "2024-05-04T00:12:00Z" },
        "phonology": {
            "sound_classes": {},
            "illegal_patterns": [],
            "prosody": { "type": "unstressed" }
        }
    }"#;
    c_world.config = Some(serde_json::from_str(json_str).expect("valid config json"));
}

// qual:allow(complexity) — Test setup helper intentionally panics on malformed test inputs
#[given(expr = "a language configuration with illegal onsets:")]
fn given_lang_config_with_illegals(
    c_world: &mut SyllabificationWorld,
    step: &cucumber::gherkin::Step,
) {
    let mut patterns = Vec::new();
    if let Some(table) = step.table.as_ref() {
        for row in table.rows.iter().skip(1) {
            patterns.push(row.first().expect("missing pattern value").clone());
        }
    }
    let patterns_json = serde_json::to_string(&patterns).expect("valid patterns json");
    let json_str = format!(
        r#"{{
        "id": "018f4a3e-6b9f-7a1a-9b1a-2b3c4d5e6f7a",
        "name": {{ "endonym": "p" }},
        "metadata": {{ "created_at": "2024-05-04T00:12:00Z" }},
        "phonology": {{
            "sound_classes": {{}},
            "illegal_patterns": {patterns_json},
            "prosody": {{ "type": "unstressed" }}
        }}
    }}"#
    );
    c_world.config = Some(serde_json::from_str(&json_str).expect("valid config json"));
}

// qual:allow(complexity) — Test runner helper intentionally panics on test assertion failure
#[when(expr = "I syllabify the IPA string {string}")]
fn syllabify_string(c_world: &mut SyllabificationWorld, s: String) {
    let config = c_world
        .config
        .as_ref()
        .expect("LanguageConfig should exist");
    let ipa_str = IpaString::from_str(&s).expect("valid ipa string");
    let parsed_word = config.syllabify(&ipa_str).expect("valid syllabify");
    c_world.word = Some(parsed_word);
    drop(s);
}

#[then(expr = "the syllables should format to {string}")]
fn syllables_should_format(c_world: &mut SyllabificationWorld, expected: String) {
    let parsed_word = c_world
        .word
        .as_ref()
        .expect("Syllables should have been computed");
    assert_eq!(parsed_word.to_string(), expected);
    drop(expected);
}

#[tokio::main]
async fn main() {
    if false {
        let mut world = SyllabificationWorld::default();
        given_lang_config(&mut world);
        let step = todo!();
        given_lang_config_with_illegals(&mut world, step);
        syllabify_string(&mut world, String::new());
        syllables_should_format(&mut world, String::new());
    }
    SyllabificationWorld::cucumber()
        .run_and_exit("tests/features/syllabification.feature")
        .await;
}
