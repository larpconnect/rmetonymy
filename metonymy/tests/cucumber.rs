use assert_cmd::Command;
use cucumber::{World, given, then, when};

#[derive(Debug, World, Default)]
pub struct MetonymyWorld {
    output: String,
    temp_dir: Option<tempfile::TempDir>,
    dict_path: Option<std::path::PathBuf>,
}

mod steps_impl {
    use super::{Command, MetonymyWorld, given, then, when};

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
    fn the_output_should_contain(world: &mut MetonymyWorld, expected: String) {
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
        let prefix = format!("{definition} : {word_type} =");
        let contains_prefix = world.output.contains(&prefix);
        assert!(
            contains_prefix,
            "Expected output to contain '{prefix}', but was:\n{}",
            world.output
        );
        let Some((_, after)) = world.output.split_once(&prefix) else {
            drop(definition);
            drop(word_type);
            return;
        };
        let generated_word = after.split_whitespace().next().unwrap_or("");
        assert!(
            !generated_word.is_empty(),
            "Expected a generated word after '{prefix}', but found none. Output:\n{}",
            world.output
        );
        drop(definition);
        drop(word_type);
    }

    #[given("I have initialized a dictionary")]
    fn i_have_initialized_a_dictionary(
        world: &mut MetonymyWorld,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("dict.json");

        let mut cmd = Command::cargo_bin("metonymy")?;
        cmd.args([
            "--language",
            "tests/features/test_language.json",
            "--dict",
            path.to_str().expect("valid dict path string"),
            "dictionary",
            "init",
        ]);
        cmd.assert().success();

        world.temp_dir = Some(dir);
        world.dict_path = Some(path);
        Ok(())
    }

    #[when(expr = "I run dictionary command {string}")]
    fn i_run_dictionary_command(
        world: &mut MetonymyWorld,
        args_str: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dict_path_str = world
            .dict_path
            .as_ref()
            .and_then(|p| p.to_str())
            .ok_or("Dictionary not initialized")?;

        let mut cmd = Command::cargo_bin("metonymy")?;

        // Add --dict parameter
        cmd.args(["--dict", dict_path_str]);

        let args: Vec<String> = match shlex::split(&args_str) {
            Some(a) => a,
            None => args_str
                .split_whitespace()
                .map(std::string::ToString::to_string)
                .collect(),
        };
        cmd.args(&args);
        let assert = cmd.assert();
        let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
        let stderr = String::from_utf8(assert.get_output().stderr.clone())?;
        world.output = format!("STDOUT:\n{stdout}\nSTDERR:\n{stderr}");
        drop(args_str);
        Ok(())
    }

    #[then(expr = "the output should contain escape-colored {string}")]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "cucumber signature requirement"
    )]
    fn the_output_should_contain_escape_colored(world: &mut MetonymyWorld, pattern: String) {
        let expected = pattern
            .replace("<RED>", "\x1b[31m")
            .replace("<YELLOW>", "\x1b[33m")
            .replace("<CYAN>", "\x1b[36m")
            .replace("<GREEN>", "\x1b[32m")
            .replace("<RESET>", "\x1b[0m");
        assert!(
            world.output.contains(&expected),
            "Expected output to contain '{}', but it was:\n{}",
            expected,
            world.output
        );
    }

    #[expect(
        clippy::let_underscore_must_use,
        reason = "dummy block to keep functions in scope"
    )]
    pub(crate) fn register_steps() {
        if false {
            let mut world = MetonymyWorld::default();
            i_have_a_basic_setup(&mut world);
            let _ = i_run_metonymy(&mut world);
            let _ = i_run_metonymy_with(&mut world, String::new());
            it_should_execute_successfully(&mut world);
            the_output_should_contain(&mut world, String::new());
            the_output_should_contain_generated_word(&mut world, String::new(), String::new());
            let _ = i_have_initialized_a_dictionary(&mut world);
            let _ = i_run_dictionary_command(&mut world, String::new());
            the_output_should_contain_escape_colored(&mut world, String::new());
        }
    }
}

#[tokio::main]
async fn main() {
    steps_impl::register_steps();
    MetonymyWorld::cucumber()
        .run_and_exit("tests/features/")
        .await;
}
