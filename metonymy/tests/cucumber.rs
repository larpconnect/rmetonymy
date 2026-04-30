use cucumber::{World, given, then, when};

#[derive(Debug, Default, World)]
pub struct MetonymyWorld {
    // Shared state
}

#[given("I have a basic setup")]
fn i_have_a_basic_setup(_world: &mut MetonymyWorld) {
    // Setup logic
}

#[when("I run metonymy")]
fn i_run_metonymy(_world: &mut MetonymyWorld) {
    // Action logic
}

#[then("it should execute successfully")]
fn it_should_execute_successfully(_world: &mut MetonymyWorld) {
    // Assertion logic
}

#[tokio::main]
async fn main() {
    MetonymyWorld::cucumber()
        .run_and_exit("tests/features/")
        .await;
}
