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

    /// Apply orthography transform as a final step
    #[arg(long)]
    pub orthography: bool,
}

fn load_config(
    language_path: Option<&PathBuf>,
) -> anyhow::Result<language::config::LanguageConfig> {
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

    Ok(config)
}

fn parse_word(
    word: &str,
    config: &language::config::LanguageConfig,
) -> anyhow::Result<language::syllable::IpaWord> {
    let parsed_word =
        ipa::sequence::PhonemeSequence::from_str(word).context("Invalid IPA input word")?;

    let ipa_word = language::syllable::IpaWord::try_from_sequence(&parsed_word, config)
        .context("Failed to syllabify input word")?;

    Ok(ipa_word)
}

fn apply_orthography_op(
    word: &language::syllable::IpaWord,
    config: &language::config::LanguageConfig,
    verbose: bool,
) -> anyhow::Result<(String, Vec<String>)> {
    let ortho_rules = config.orthography.as_deref().unwrap_or(&[]);
    let compiled_ortho = soundchange::compile_ortho_rules(ortho_rules)
        .map_err(|e| anyhow::anyhow!("Failed to compile orthography rules: {e}"))?;
    let (ortho_res, logs) = soundchange::apply_orthography(word, &compiled_ortho, config, verbose)
        .map_err(|e| anyhow::anyhow!("Failed to apply orthography: {e}"))?;
    Ok((ortho_res, logs))
}

pub(crate) fn handle_sound_change(
    cmd: &SoundChangeCmd,
    language_path: Option<&PathBuf>,
) -> anyhow::Result<()> {
    let config = load_config(language_path)?;
    let ipa_word = parse_word(&cmd.word, &config)?;

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
        (cmd.start, end_era),
        &config,
        cmd.verbose,
    )
    .map_err(|e| anyhow::anyhow!("Failed to apply sound changes: {e}"))?;

    let mut result_str = result_word.to_string();
    let mut ortho_logs = Vec::new();

    if cmd.orthography {
        let (ortho_res, logs) = apply_orthography_op(&result_word, &config, cmd.verbose)?;
        result_str = ortho_res;
        ortho_logs = logs;
    }

    if cmd.verbose {
        for log in trace_logs {
            println!("{log}");
        }
        for log in ortho_logs {
            println!("{log}");
        }
    }

    println!("{result_str}");
    Ok(())
}
