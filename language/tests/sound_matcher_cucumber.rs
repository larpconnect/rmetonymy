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
fn given_sound_classes(
    c_world: &mut SoundMatcherWorld,
    step: &cucumber::gherkin::Step,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(table) = step.table.as_ref() {
        for row in table.rows.iter().skip(1) {
            // Skip header
            let class_key = SoundClassKey::from_str(row.first().ok_or("missing class key")?)?;
            let values: Vec<String> = row
                .get(1)
                .ok_or("missing values")?
                .split(',')
                .map(|s: &str| s.trim().to_string())
                .collect();
            c_world.sound_classes.insert(
                class_key,
                SoundClass {
                    values,
                    generator: None,
                },
            );
        }
    }
    Ok(())
}

#[when(expr = "I check the pattern {string} against the word {string}")]
fn check_pattern(
    c_world: &mut SoundMatcherWorld,
    pattern_str: String,
    input_word: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let pattern = SoundMatcherPattern::from_str(&pattern_str)?;
    c_world.matches = pattern.matches(&input_word, &c_world.sound_classes);
    c_world.pattern = Some(pattern);
    c_world.word = input_word;
    drop(pattern_str);
    Ok(())
}

#[then(expr = "the pattern should match")]
fn pattern_should_match(c_world: &mut SoundMatcherWorld) {
    assert!(c_world.matches, "Expected pattern to match, but it didn't");
}

#[then(expr = "the pattern should not match")]
fn pattern_should_not_match(c_world: &mut SoundMatcherWorld) {
    assert!(!c_world.matches, "Expected pattern to not match, but it did");
}

#[tokio::main]
async fn main() {
    SoundMatcherWorld::cucumber()
        .run_and_exit("tests/features/sound_matcher.feature")
        .await;
}
