use cucumber::{World, given, then, when};
use ipa::IpaString;
use language::config::SoundClass;
use language::sound_class::SoundClassKey;
use language::sound_matcher::SoundMatcherPattern;
use std::collections::BTreeMap;

#[derive(Debug, Default, World)]
struct SoundMatcherWorld {
    sound_classes: BTreeMap<SoundClassKey, SoundClass>,
    pattern: Option<SoundMatcherPattern>,
    word: Option<IpaString>,
}

#[given(expr = "the following sound classes exist:")]
fn given_sound_classes(world: &mut SoundMatcherWorld, step: &cucumber::gherkin::Step) {
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
fn when_check_pattern(world: &mut SoundMatcherWorld, pattern_str: String, word_str: String) {
    let pattern: SoundMatcherPattern = pattern_str.parse().unwrap();
    let word: IpaString = word_str.parse().unwrap();

    world.pattern = Some(pattern);
    world.word = Some(word);
}

#[then(expr = "the pattern should match")]
fn then_pattern_matches(world: &mut SoundMatcherWorld) {
    let pattern = world.pattern.as_ref().unwrap();
    let word = world.word.as_ref().unwrap();
    assert!(pattern.matches(word, &world.sound_classes));
}

#[then(expr = "the pattern should not match")]
fn then_pattern_does_not_match(world: &mut SoundMatcherWorld) {
    let pattern = world.pattern.as_ref().unwrap();
    let word = world.word.as_ref().unwrap();
    assert!(!pattern.matches(word, &world.sound_classes));
}

#[tokio::main]
async fn main() {
    SoundMatcherWorld::run("tests/features/sound_matcher.feature").await;
}
