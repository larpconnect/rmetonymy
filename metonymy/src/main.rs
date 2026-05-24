use anyhow::Context;
use clap::{Parser, Subcommand};
use ipa::IpaSystem;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Verbose logging output
    #[arg(short, long)]
    verbose: bool,

    /// The phoneme configuration file
    #[arg(long)]
    phone_config: Option<PathBuf>,

    /// The language configuration file
    #[arg(long)]
    language: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Lookup a phoneme and report its information
    Lookup {
        /// The phoneme to look up
        #[arg(long)]
        phoneme: String,
    },
    /// Generate words or other linguistic constructs
    Generate(GenerateCmd),
}

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
        word_type: String,
    },
}

fn load_ipa_system(phone_config: Option<&PathBuf>) -> anyhow::Result<IpaSystem> {
    match phone_config {
        Some(path) => {
            let json = fs::read_to_string(path)
                .with_context(|| format!("Failed to read phone config from {}", path.display()))?;
            IpaSystem::new(&json).context("Failed to parse phone config")
        }
        None => Ok(IpaSystem::default()),
    }
}

fn print_phoneme_info(symbol: &str, system: &IpaSystem) {
    if let Some(data) = system.get_phoneme_data(symbol) {
        println!("Base: {symbol}");
        println!("Features: {:?}", data.features);
        println!("Place: {:?}", data.place);
        println!("Manner: {:?}", data.manner);
    }
}

fn print_modified_phoneme_info(
    base: &str,
    modifier: &str,
    combined_features: &[data::SpeFeature],
    system: &IpaSystem,
) {
    if let Some(base_data) = system.get_phoneme_data(base) {
        println!("Base: {base}");
        println!("Modifiers: {modifier}");
        println!("Original Features: {:?}", base_data.features);
        println!("Modified Features: {combined_features:?}");
        println!("Place: {:?}", base_data.place);
        println!("Manner: {:?}", base_data.manner);
    }
}

fn find_base_and_modifier<'a>(system: &IpaSystem, phoneme: &'a str) -> Option<(&'a str, &'a str)> {
    let char_indices: Vec<(usize, char)> = phoneme.char_indices().collect();

    for len in (1..=char_indices.len()).rev() {
        let split_idx = char_indices.get(len).map_or(phoneme.len(), |&(idx, _)| idx);
        let prefix = phoneme.get(0..split_idx).unwrap_or("");
        if system.get_phoneme_data(prefix).is_some() {
            let modifier = phoneme.get(split_idx..).unwrap_or("");
            return Some((prefix, modifier));
        }
    }
    None
}

fn handle_lookup(phoneme: &str, phone_config: Option<&PathBuf>) -> anyhow::Result<()> {
    let system = load_ipa_system(phone_config)?;

    println!("Looking up phoneme: {phoneme}");

    match find_base_and_modifier(&system, phoneme) {
        Some((base, "")) => {
            print_phoneme_info(base, &system);
        }
        Some((base, modifier)) => {
            if let Some(combined_features) = system.combine_with_modifier(base, modifier) {
                print_modified_phoneme_info(base, modifier, &combined_features, &system);
            } else if system.get_entry(phoneme).is_some() {
                println!("Found entry, but it is not a base phoneme.");
            } else {
                println!("Phoneme '{phoneme}' not found or could not combine.");
            }
        }
        None => {
            println!("Phoneme '{phoneme}' not found.");
        }
    }

    Ok(())
}

fn handle_generate(
    cmd: &GenerateCmd,
    language_path: Option<&PathBuf>,
) -> anyhow::Result<()> {
    let lang_path = language_path.context("Language configuration file (--language) is required for generation")?;
    let lang_json = fs::read_to_string(lang_path)
        .with_context(|| format!("Failed to read language config from {}", lang_path.display()))?;
    
    let config: language::config::LanguageConfig = serde_json::from_str(&lang_json)
        .context("Failed to parse language config JSON")?;

    config.validate().context("Language configuration validation failed")?;

    match &cmd.subcommand {
        GenerateSubcommand::Word { definition, word_type } => {
            let mut rng = language::generator::thread_rng();
            let mut warning_logged = false;
            let word = language::generator::generate_word(
                word_type,
                &config,
                &mut rng,
                cmd.max_attempts,
                &mut warning_logged,
            )?;
            println!("{definition} : {word_type} = {word}");
        }
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let level = if cli.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_writer(std::io::stderr)
        .init();

    if cli.verbose {
        println!("Metonymy is running in verbose mode...");
    } else {
        println!("Metonymy is running...");
    }

    if let Some(cmd) = cli.command {
        match cmd {
            Commands::Lookup { phoneme } => {
                handle_lookup(&phoneme, cli.phone_config.as_ref())?;
            }
            Commands::Generate(gen_cmd) => {
                handle_generate(&gen_cmd, cli.language.as_ref())?;
            }
        }
    } else {
        soundchange::parse_soundchange();
    }

    Ok(())
}
