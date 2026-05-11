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

fn print_modified_phoneme_info(base: &str, modifier: &str, system: &IpaSystem) {
    if let Some(combined_features) = system.combine_with_modifier(base, modifier) {
        if let Some(base_data) = system.get_phoneme_data(base) {
            println!("Base: {base}");
            println!("Modifiers: {modifier}");
            println!("Original Features: {:?}", base_data.features);
            println!("Modified Features: {combined_features:?}");
            println!("Place: {:?}", base_data.place);
            println!("Manner: {:?}", base_data.manner);
        }
    }
}

fn find_base_and_modifier<'a>(system: &IpaSystem, phoneme: &'a str) -> Option<(&'a str, &'a str)> {
    let char_indices: Vec<(usize, char)> = phoneme.char_indices().collect();

    for len in (1..=char_indices.len()).rev() {
        let end_idx = char_indices.get(len).map_or(phoneme.len(), |&(idx, _)| idx);
        let prefix = phoneme.get(0..end_idx).unwrap_or("");
        if system.get_phoneme_data(prefix).is_some() {
            let start_idx = char_indices.get(len).map_or(phoneme.len(), |&(idx, _)| idx);
            let modifier = phoneme.get(start_idx..).unwrap_or("");
            return Some((prefix, modifier));
        }
    }
    None
}

fn handle_lookup(phoneme: &str, phone_config: Option<&PathBuf>) -> anyhow::Result<()> {
    let system = load_ipa_system(phone_config)?;

    println!("Looking up phoneme: {phoneme}");

    // Look up the exact string as a single phoneme/symbol first
    if system.get_phoneme_data(phoneme).is_some() {
        print_phoneme_info(phoneme, &system);
        return Ok(());
    }

    // Fallback logic for affricates and diacritics
    match find_base_and_modifier(&system, phoneme) {
        Some((base, "")) => {
            print_phoneme_info(base, &system);
        }
        Some((base, modifier)) => {
            if system.combine_with_modifier(base, modifier).is_some() {
                print_modified_phoneme_info(base, modifier, &system);
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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

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
        }
    } else {
        // Wire in the submodules eventually
        soundchange::parse_soundchange();
    }

    Ok(())
}
