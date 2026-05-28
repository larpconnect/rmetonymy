pub mod dictionary;
pub mod generate;
pub mod lookup;
pub mod sound_change;

use clap::{Parser, Subcommand};
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

    /// The dictionary file to load/save
    #[arg(long)]
    dict: Option<PathBuf>,

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
    Generate(generate::GenerateCmd),
    /// Manage the conlang dictionary
    Dictionary(dictionary::DictionaryCmd),
    /// Apply sound changes to a word
    SoundChange(sound_change::SoundChangeCmd),
}

fn init_logging(verbose: bool) {
    let level = if verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_writer(std::io::stderr)
        .init();

    if verbose {
        println!("Metonymy is running in verbose mode...");
    } else {
        println!("Metonymy is running...");
    }
}

fn run_command(
    cmd: Commands,
    phone_config: Option<&PathBuf>,
    language: Option<&PathBuf>,
    dict: Option<&PathBuf>,
) -> anyhow::Result<()> {
    match cmd {
        Commands::Lookup { phoneme } => {
            lookup::handle_lookup(&phoneme, phone_config)?;
        }
        Commands::Generate(gen_cmd) => {
            generate::handle_generate(&gen_cmd, language)?;
        }
        Commands::Dictionary(dict_cmd) => {
            dictionary::handle_dictionary_cmd(dict_cmd, language, dict)?;
        }
        Commands::SoundChange(sc_cmd) => {
            sound_change::handle_sound_change(&sc_cmd, language)?;
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    init_logging(cli.verbose);

    let phone_config = cli.phone_config.as_ref();
    let language = cli.language.as_ref();
    let dict = cli.dict.as_ref();

    if let Some(cmd) = cli.command {
        run_command(cmd, phone_config, language, dict)?;
    }

    Ok(())
}
