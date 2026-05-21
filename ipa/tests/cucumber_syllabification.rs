use cucumber::{World, given, then, when};
use ipa::{IpaString, IpaWord};

#[derive(Debug, Default, World)]
pub struct SyllabificationWorld {
    input_word: Option<String>,
    syllabified_word: Option<String>,
}

#[given(expr = "the word {string}")]
async fn the_word(world: &mut SyllabificationWorld, word: String) {
    world.input_word = Some(word);
}

#[when(expr = "it is syllabified")]
async fn it_is_syllabified(world: &mut SyllabificationWorld) {
    let input = world.input_word.as_ref().unwrap();
    let ipa_string: IpaString = input.parse().unwrap();
    let ipa_word = IpaWord::syllabify(&ipa_string);
    world.syllabified_word = Some(ipa_word.to_string());
}

#[then(expr = "the syllables should be {string}")]
async fn the_syllables_should_be(world: &mut SyllabificationWorld, expected: String) {
    let actual = world.syllabified_word.as_ref().unwrap();
    assert_eq!(actual, &expected);
}

#[tokio::main]
async fn main() {
    SyllabificationWorld::cucumber()
        .run("tests/features/syllabification.feature")
        .await;
}
