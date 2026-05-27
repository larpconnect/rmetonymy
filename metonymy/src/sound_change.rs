use anyhow::Context;
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser)]
pub struct SoundChangeCmd {
    /// Start era (inclusive)
    #[arg(long, default_value_t = 0)]
    pub start: u32,

    /// End era (inclusive)
    #[arg(long)]
    pub end: Option<u32>,

    /// The word to apply changes to (in IPA)
    pub word: String,

    /// Show the results from every individual sound change
    #[arg(short, long)]
    pub verbose: bool,
}

pub(crate) fn handle_sound_change(
    cmd: &SoundChangeCmd,
    language_path: Option<&PathBuf>,
) -> anyhow::Result<()> {
    let lang_path = language_path
        .context("Language configuration file (--language) is required for sound change command")?;
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

    let parsed_word =
        ipa::sequence::PhonemeSequence::from_str(&cmd.word).context("Invalid IPA input word")?;

    let ipa_word = language::syllable::IpaWord::try_from_sequence(&parsed_word, &config)
        .context("Failed to syllabify input word")?;

    let sound_changes = config
        .sound_changes
        .clone()
        .unwrap_or(language::config::SoundChanges {
            preamble: Vec::new(),
            eras: Vec::new(),
        });

    let compiled = soundchange::compile_sound_changes(&sound_changes)
        .map_err(|e| anyhow::anyhow!("Failed to compile sound changes: {e}"))?;

    let end_era = cmd.end.unwrap_or(u32::MAX);
    let (result_word, trace_logs) = soundchange::evaluator::apply_sound_changes(
        &ipa_word,
        &compiled,
        cmd.start,
        end_era,
        &config,
        cmd.verbose,
    )
    .map_err(|e| anyhow::anyhow!("Failed to apply sound changes: {e}"))?;

    if cmd.verbose {
        for log in trace_logs {
            println!("{log}");
        }
    }

    println!("{result_word}");
    Ok(())
}
