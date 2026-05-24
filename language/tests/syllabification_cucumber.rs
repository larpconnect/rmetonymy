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
async fn given_lang_config(world: &mut SyllabificationWorld) {
    let json_str = r#"{
        "id": "018f4a3e-6b9f-7a1a-9b1a-2b3c4d5e6f7a",
        "name": { "endonym": "p" },
        "metadata": { "created_at": "2024-05-04T00:12:00Z" },
        "phonology": {
            "sound_classes": {},
            "illegal_patterns": []
        }
    }"#;
    world.config = Some(serde_json::from_str(json_str).unwrap());
}

#[given(expr = "a language configuration with illegal onsets:")]
async fn given_lang_config_with_illegals(
    world: &mut SyllabificationWorld,
    step: &cucumber::gherkin::Step,
) {
    let mut patterns = Vec::new();
    if let Some(table) = step.table.as_ref() {
        for row in table.rows.iter().skip(1) {
            patterns.push(row[0].clone());
        }
    }
    let patterns_json = serde_json::to_string(&patterns).unwrap();
    let json_str = format!(
        r#"{{
        "id": "018f4a3e-6b9f-7a1a-9b1a-2b3c4d5e6f7a",
        "name": {{ "endonym": "p" }},
        "metadata": {{ "created_at": "2024-05-04T00:12:00Z" }},
        "phonology": {{
            "sound_classes": {{}},
            "illegal_patterns": {patterns_json}
        }}
    }}"#
    );
    world.config = Some(serde_json::from_str(&json_str).unwrap());
}

#[when(expr = "I syllabify the IPA string {string}")]
async fn syllabify_string(world: &mut SyllabificationWorld, s: String) {
    let config = world.config.as_ref().expect("LanguageConfig should exist");
    let ipa_str = IpaString::from_str(&s).unwrap();
    let word = config.syllabify(&ipa_str).unwrap();
    world.word = Some(word);
}

#[then(expr = "the syllables should format to {string}")]
async fn syllables_should_format(world: &mut SyllabificationWorld, expected: String) {
    let word = world
        .word
        .as_ref()
        .expect("Syllables should have been computed");
    assert_eq!(word.to_string(), expected);
}

#[tokio::main]
async fn main() {
    SyllabificationWorld::cucumber()
        .run_and_exit("tests/features/syllabification.feature")
        .await;
}
