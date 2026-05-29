pub mod lookup;
pub mod ops;

use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
pub struct DictionaryCmd {
    #[command(subcommand)]
    pub subcommand: DictionarySubcommand,
}

#[derive(Subcommand)]
pub enum DictionarySubcommand {
    /// Create a blank dictionary for a language
    Init,

    /// Add a word to the dictionary
    Add {
        /// The meaning of the word (`IpaString`)
        #[arg(long)]
        meaning: String,

        /// The conlang definition of the word (`IpaString`)
        #[arg(
            long,
            required_unless_present = "generate",
            conflicts_with = "generate"
        )]
        definition: Option<String>,

        /// Generate the word automatically based on the language configuration
        #[arg(
            long,
            required_unless_present = "definition",
            conflicts_with = "definition"
        )]
        generate: bool,

        /// The type of the word (optionally with a subtype separated by a period, e.g. noun.masculine)
        #[arg(long)]
        r#type: String,

        /// The era of the word
        #[arg(long)]
        era: Option<u32>,

        /// Etymology entry: `era:source_word1,source_word2`... (can be specified multiple times)
        #[arg(long)]
        etymology: Vec<String>,

        /// Usage notes
        #[arg(long, default_value = "")]
        usage_notes: String,
    },

    /// Remove a word from the dictionary
    Remove {
        /// The Base62 ID of the word to remove
        id: String,
    },

    /// Pretty print the dictionary to stdout
    Print,

    /// Add an era to the dictionary
    #[command(name = "add-era")]
    AddEra {
        /// The era number. Increments if not specified
        #[arg(long)]
        era: Option<u32>,

        /// Optional era name
        #[arg(long)]
        name: Option<String>,

        /// Optional description of the era
        #[arg(long)]
        description: Option<String>,
    },

    /// Lookup a word in the dictionary by meaning and apply derivations
    Lookup {
        /// The meaning label, optionally with derivations (e.g. meaning-DERIV1-DERIV2)
        meaning: String,

        /// Filter by the type/subtype (e.g. noun.masculine)
        #[arg(long)]
        r#type: Option<String>,
    },
}

fn handle_init(dict_path: &Path, language: Option<&PathBuf>) -> anyhow::Result<()> {
    ops::handle_dict_init(dict_path, language.map(PathBuf::as_path))
}

#[expect(
    clippy::too_many_arguments,
    reason = "Internal subcommand dispatcher helper"
)]
fn handle_add(
    dict_path: &Path,
    language: Option<&PathBuf>,
    meaning: &str,
    definition: Option<&str>,
    generate: bool,
    r#type: String,
    era: Option<u32>,
    etymology: &[String],
    usage_notes: String,
) -> anyhow::Result<()> {
    ops::handle_dict_add_cmd(
        dict_path,
        language.map(PathBuf::as_path),
        meaning,
        definition,
        generate,
        r#type,
        era,
        etymology,
        usage_notes,
    )
}

fn handle_remove(dict_path: &Path, id: &str) -> anyhow::Result<()> {
    ops::handle_dict_remove(dict_path, id)
}

fn handle_print(dict_path: &Path) -> anyhow::Result<()> {
    ops::handle_dict_print(dict_path)
}

fn handle_add_era(
    dict_path: &Path,
    era: Option<u32>,
    name: Option<String>,
    description: Option<String>,
) -> anyhow::Result<()> {
    ops::handle_dict_add_era_cmd(dict_path, era, name, description)
}

fn handle_lookup(
    dict_path: &Path,
    language: Option<&PathBuf>,
    meaning: &str,
    r#type: Option<&str>,
) -> anyhow::Result<()> {
    lookup::handle_dict_lookup(dict_path, language.map(PathBuf::as_path), meaning, r#type)
}

fn dispatch_subcommand(
    subcommand: DictionarySubcommand,
    dict_path: &Path,
    language: Option<&PathBuf>,
) -> anyhow::Result<()> {
    match subcommand {
        DictionarySubcommand::Init => {
            handle_init(dict_path, language)?;
        }
        DictionarySubcommand::Add {
            meaning,
            definition,
            generate,
            r#type,
            era,
            etymology,
            usage_notes,
        } => {
            handle_add(
                dict_path,
                language,
                &meaning,
                definition.as_deref(),
                generate,
                r#type,
                era,
                &etymology,
                usage_notes,
            )?;
        }
        DictionarySubcommand::Remove { id } => {
            handle_remove(dict_path, &id)?;
        }
        DictionarySubcommand::Print => {
            handle_print(dict_path)?;
        }
        DictionarySubcommand::AddEra {
            era,
            name,
            description,
        } => {
            handle_add_era(dict_path, era, name, description)?;
        }
        DictionarySubcommand::Lookup { meaning, r#type } => {
            handle_lookup(dict_path, language, &meaning, r#type.as_deref())?;
        }
    }
    Ok(())
}

pub(crate) fn handle_dictionary_cmd(
    dict_cmd: DictionaryCmd,
    language: Option<&PathBuf>,
    dict: Option<&PathBuf>,
) -> anyhow::Result<()> {
    let dict_path =
        dict.context("Dictionary file path (--dict) is required for dictionary command")?;
    dispatch_subcommand(dict_cmd.subcommand, dict_path, language)
}
