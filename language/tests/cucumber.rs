use cucumber::{World, given, then, when};
use language::config::SoundClass;
use language::sound_class::SoundClassKey;
use language::sound_matcher::SoundMatcherPattern;
use std::collections::BTreeMap;

#[derive(Debug, Default, World)]
pub struct LanguageWorld {
    pub sound_classes: BTreeMap<SoundClassKey, SoundClass>,
    pub last_match_result: Option<bool>,
}

#[given("the following sound classes exist:")]
fn given_sound_classes_exist(world: &mut LanguageWorld, step: &cucumber::gherkin::Step) {
    if let Some(table) = step.table.as_ref() {
        for row in table.rows.iter().skip(1) {
            let class_key: SoundClassKey = row[0].parse().unwrap();
            let values: Vec<String> = row[1]
                .split(',')
                .map(|s: &str| s.trim().to_string())
                .collect();
            world.sound_classes.insert(
                class_key,
                SoundClass {
                    values,
                    generator: None,
                },
            );
        }
    }
}

#[when(expr = "I check the pattern {string} against the word {string}")]
fn check_pattern_against_word(world: &mut LanguageWorld, pattern_str: String, word: String) {
    let pattern: SoundMatcherPattern = pattern_str.parse().expect("Failed to parse pattern");
    let result = pattern.matches(&word, &world.sound_classes);
    world.last_match_result = Some(result);
}

#[then("the pattern should match")]
fn pattern_should_match(world: &mut LanguageWorld) {
    assert_eq!(
        world.last_match_result,
        Some(true),
        "Expected pattern to match, but it did not"
    );
}

#[then("the pattern should not match")]
fn pattern_should_not_match(world: &mut LanguageWorld) {
    assert_eq!(
        world.last_match_result,
        Some(false),
        "Expected pattern NOT to match, but it did"
    );
}

#[tokio::main]
async fn main() {
    LanguageWorld::cucumber()
        .run_and_exit("tests/features/")
        .await;
}
