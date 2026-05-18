use cucumber::{World, given, then, when};
use language::config::SoundClass;
use language::matcher::SoundMatcherPattern;
use language::sound_class::SoundClassKey;
use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Debug, Default, World)]
pub struct SoundMatcherWorld {
    pub sound_classes: BTreeMap<SoundClassKey, SoundClass>,
    pub pattern: Option<SoundMatcherPattern>,
    pub word: String,
    pub matches: bool,
}

#[given(expr = "the following sound classes exist:")]
async fn given_sound_classes(world: &mut SoundMatcherWorld, step: &cucumber::gherkin::Step) {
    if let Some(table) = step.table.as_ref() {
        for row in table.rows.iter().skip(1) {
            // Skip header
            let class_key = SoundClassKey::from_str(&row[0]).unwrap();
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
async fn check_pattern(world: &mut SoundMatcherWorld, pattern_str: String, word: String) {
    let pattern = SoundMatcherPattern::from_str(&pattern_str).unwrap();
    world.matches = pattern.matches(&word, &world.sound_classes);
    world.pattern = Some(pattern);
    world.word = word;
}

#[then(expr = "the pattern should match")]
async fn pattern_should_match(world: &mut SoundMatcherWorld) {
    assert!(world.matches, "Expected pattern to match, but it didn't");
}

#[then(expr = "the pattern should not match")]
async fn pattern_should_not_match(world: &mut SoundMatcherWorld) {
    assert!(!world.matches, "Expected pattern to not match, but it did");
}

#[tokio::main]
async fn main() {
    SoundMatcherWorld::cucumber()
        .run_and_exit("tests/features/sound_matcher.feature")
        .await;
}
