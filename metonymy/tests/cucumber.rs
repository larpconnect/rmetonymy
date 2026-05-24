use assert_cmd::Command;
use cucumber::{World, given, then, when};

#[derive(Debug, Default, World)]
pub struct MetonymyWorld {
    output: String,
}

#[given("I have a basic setup")]
fn i_have_a_basic_setup(_world: &mut MetonymyWorld) {
    // Setup logic
}

#[when("I run metonymy")]
fn i_run_metonymy(world: &mut MetonymyWorld) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("metonymy")?;
    let assert = cmd.assert();
    world.output = String::from_utf8(assert.get_output().stdout.clone())?;
    Ok(())
}

#[when(expr = "I run metonymy with {string}")]
fn i_run_metonymy_with(
    world: &mut MetonymyWorld,
    args_str: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("metonymy")?;
    let args: Vec<String> = match shlex::split(&args_str) {
        Some(a) => a,
        None => args_str
            .split_whitespace()
            .map(std::string::ToString::to_string)
            .collect(),
    };
    cmd.args(&args);
    let assert = cmd.assert();
    world.output = String::from_utf8(assert.get_output().stdout.clone())?;
    drop(args_str);
    Ok(())
}

#[then("it should execute successfully")]
fn it_should_execute_successfully(world: &mut MetonymyWorld) {
    assert!(!world.output.is_empty());
}

#[then(expr = "the output should contain {string}")]
fn the_output_should_contain(
    world: &mut MetonymyWorld,
    expected: String,
) {
    assert!(
        world.output.contains(&expected),
        "Expected output to contain '{}', but it was:\n{}",
        expected,
        world.output
    );
    drop(expected);
}

#[then(expr = "the output should contain a generated word for {string} as {string}")]
fn the_output_should_contain_generated_word(
    world: &mut MetonymyWorld,
    definition: String,
    word_type: String,
) {
    let prefix = format!("{} : {} =", definition, word_type);
    let index = world.output.find(&prefix).unwrap_or_else(|| {
        panic!("Expected output to contain '{}', but was:\n{}", prefix, world.output);
    });
    let after = &world.output[index + prefix.len()..];
    let word = after.split_whitespace().next().unwrap_or("");
    assert!(
        !word.is_empty(),
        "Expected a generated word after '{}', but found none. Output:\n{}",
        prefix,
        world.output
    );
}

#[tokio::main]
async fn main() {
    MetonymyWorld::cucumber()
        .run_and_exit("tests/features/")
        .await;
}
