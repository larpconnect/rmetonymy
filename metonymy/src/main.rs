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
                let system = match &cli.phone_config {
                    Some(path) => {
                        let json = fs::read_to_string(path)?;
                        IpaSystem::new(&json)
                            .map_err(|e| anyhow::anyhow!("Failed to parse phone config: {e}"))?
                    }
                    None => IpaSystem::default(),
                };

                println!("Looking up phoneme: {phoneme}");

                // Right now IpaString just validates that it's a sequence of IPA symbols
                // Since IpaSystem doesn't have an IpaString -> Phoneme list parser out-of-the-box right now
                // We'll just look up the entire string as a single phoneme/symbol first.

                if let Some(data) = system.get_phoneme_data(&phoneme) {
                    println!("Base: {phoneme}");
                    println!("Features: {:?}", data.features);
                    println!("Place: {:?}", data.place);
                    println!("Manner: {:?}", data.manner);
                } else {
                    // Try looking up the first character as base, and the rest as modifiers
                    // Naive approach for the scope of the CLI command for now
                    let mut chars = phoneme.chars();
                    if let Some(base_char) = chars.next() {
                        let base = base_char.to_string();
                        let modifier = chars.collect::<String>();

                        if let Some(combined_features) =
                            system.combine_with_modifier(&base, &modifier)
                        {
                            if let Some(base_data) = system.get_phoneme_data(&base) {
                                println!("Base: {base}");
                                println!("Modifiers: {modifier}");
                                println!("Original Features: {:?}", base_data.features);
                                println!("Modified Features: {combined_features:?}");
                                println!("Place: {:?}", base_data.place);
                                println!("Manner: {:?}", base_data.manner);
                            }
                        } else if system.get_entry(&phoneme).is_some() {
                            println!("Found entry, but it is not a base phoneme.");
                        } else {
                            println!("Phoneme '{phoneme}' not found or could not combine.");
                        }
                    } else {
                        println!("Phoneme '{phoneme}' not found.");
                    }
                }
            }
        }
    } else {
        // Wire in the submodules eventually
        soundchange::parse_soundchange();
    }

    Ok(())
}
