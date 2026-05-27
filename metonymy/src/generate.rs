use anyhow::Context;
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
pub struct GenerateCmd {
    /// Maximum number of attempts to generate a word without illegal patterns
    #[arg(long, default_value_t = 8)]
    pub max_attempts: usize,

    #[command(subcommand)]
    pub subcommand: GenerateSubcommand,
}

#[derive(Subcommand)]
pub enum GenerateSubcommand {
    /// Generate a word for a definition and type
    Word {
        /// The definition of the word (e.g. red)
        definition: String,

        /// The grammatical type (e.g. adjective or adjective.masculine)
        r#type: String,
    },
}

pub(crate) fn handle_generate(
    cmd: &GenerateCmd,
    language_path: Option<&PathBuf>,
) -> anyhow::Result<()> {
    let lang_path = language_path
        .context("Language configuration file (--language) is required for generation")?;
    let lang_json = fs::read_to_string(lang_path).with_context(|| {
        format!(
            "Failed to read language config from {}",
            lang_path.display()
        )
    })?;

    let config: language::config::LanguageConfig =
        serde_json::from_str(&lang_json).context("Failed to parse language config JSON")?;

    config
        .validate()
        .context("Language configuration validation failed")?;

    match &cmd.subcommand {
        GenerateSubcommand::Word {
            definition,
            r#type: type_name,
        } => {
            let mut rng = language::generator::thread_rng();
            let mut warning_logged = false;
            let word = language::generator::generate_word(
                type_name,
                &config,
                &mut rng,
                cmd.max_attempts,
                &mut warning_logged,
            )?;
            println!("{definition} : {type_name} = {word}");
        }
    }

    Ok(())
}
