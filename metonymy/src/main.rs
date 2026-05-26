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
    Generate(GenerateCmd),
    /// Manage the conlang dictionary
    Dictionary(DictionaryCmd),
}

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
        r#type: String,
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

fn handle_generate(cmd: &GenerateCmd, language_path: Option<&PathBuf>) -> anyhow::Result<()> {
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

fn parse_etymology(raw: &[String]) -> anyhow::Result<std::collections::BTreeMap<u32, Vec<String>>> {
    let mut map: std::collections::BTreeMap<u32, Vec<String>> = std::collections::BTreeMap::new();
    for item in raw {
        let parts: Vec<&str> = item.splitn(2, ':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid etymology format: '{item}'. Expected 'era:word1,word2,...'");
        }
        let era_str = parts.first().context("Missing era in etymology")?;
        let words_str = parts.get(1).context("Missing words list in etymology")?;
        let era: u32 = era_str
            .trim()
            .parse()
            .with_context(|| format!("Invalid era number '{era_str}' in etymology"))?;
        let words: Vec<String> = words_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        map.entry(era).or_default().extend(words);
    }
    Ok(map)
}

fn handle_dict_init(
    dict_path: &std::path::Path,
    lang_path: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    let lang_path = lang_path.context(
        "Language configuration file (--language) is required to initialize a dictionary",
    )?;
    let lang_json = fs::read_to_string(lang_path).with_context(|| {
        format!(
            "Failed to read language config from {}",
            lang_path.display()
        )
    })?;
    let lang_config: language::config::LanguageConfig =
        serde_json::from_str(&lang_json).context("Failed to parse language config JSON")?;

    let new_dict = language::Dictionary::new(lang_config.id);
    new_dict
        .save_to_file(dict_path)
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to initialize dictionary file")?;

    println!(
        "Initialized blank dictionary for language ID {} at {}",
        lang_config.id,
        dict_path.display()
    );
    Ok(())
}

fn handle_dict_add(dict_path: &std::path::Path, entry: language::NewEntry) -> anyhow::Result<()> {
    let dict_json = fs::read_to_string(dict_path).with_context(|| {
        format!(
            "Failed to read dictionary file from {}",
            dict_path.display()
        )
    })?;
    let mut dict = dict_json
        .parse::<language::Dictionary>()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to parse dictionary")?;

    let definition = entry.definition.to_string();
    let meaning = entry.meaning.to_string();

    let entry_id = dict.add_entry(entry);

    dict.save_to_file(dict_path)
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to save dictionary")?;

    println!("Added word '{definition}' (meaning: '{meaning}') with ID {entry_id}");
    Ok(())
}

fn handle_dict_remove(dict_path: &std::path::Path, id: &str) -> anyhow::Result<()> {
    let dict_json = fs::read_to_string(dict_path).with_context(|| {
        format!(
            "Failed to read dictionary file from {}",
            dict_path.display()
        )
    })?;
    let mut dict = dict_json
        .parse::<language::Dictionary>()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to parse dictionary")?;

    if dict.remove_entry(id) {
        dict.save_to_file(dict_path)
            .map_err(|e| anyhow::anyhow!(e))
            .context("Failed to save dictionary")?;
        println!("Removed word with ID {id}");
    } else {
        println!("Word with ID {id} not found in dictionary");
    }
    Ok(())
}

fn print_entry(
    idx: usize,
    entry: &language::DictionaryEntry,
    eras: &std::collections::BTreeMap<u32, language::Era>,
) {
    let id = &entry.id;
    println!("{idx}. [{id}]");
    println!("   Definition : /{}/", entry.definition);
    println!("   Meaning    : /{}/", entry.meaning);
    let (word_type, word_subtype) = entry.r#type.split_once('.').unwrap_or((&entry.r#type, ""));
    if word_subtype.is_empty() {
        println!("   Type       : {word_type}");
    } else {
        println!("   Type       : {word_type} ({word_subtype})");
    }
    let era = entry.era;
    if let Some(era_meta) = eras.get(&era) {
        let name_str = era_meta
            .name
            .as_ref()
            .map_or(String::new(), |n| format!(" /{n}/"));
        let desc_str = era_meta
            .description
            .as_ref()
            .map_or(String::new(), |d| format!(" - {d}"));
        println!("   Era        : {era}{name_str}{desc_str}");
    } else {
        println!("   Era        : {era}");
    }
    if let Some(etymology) = entry.etymology.as_ref().filter(|e| !e.is_empty()) {
        println!("   Etymology  :");
        for (era, sources) in etymology {
            let joined_sources = sources.join(", ");
            println!("     Era {era}: {joined_sources}");
        }
    }
    if !entry.usage_notes.is_empty() {
        let notes = &entry.usage_notes;
        println!("   Usage Notes: {notes}");
    }
    println!("--------------------------------------------------------------------------------");
}

fn handle_dict_print(dict_path: &std::path::Path) -> anyhow::Result<()> {
    let dict_json = fs::read_to_string(dict_path).with_context(|| {
        format!(
            "Failed to read dictionary file from {}",
            dict_path.display()
        )
    })?;
    let dict = dict_json
        .parse::<language::Dictionary>()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to parse dictionary")?;

    println!("================================================================================");
    println!("Dictionary ID: {}", dict.id);
    if !dict.eras.is_empty() {
        println!("Total Eras   : {}", dict.eras.len());
    }
    println!("Total Entries: {}", dict.entries.len());
    println!("================================================================================");

    if !dict.eras.is_empty() {
        println!("Eras:");
        for (num, era) in &dict.eras {
            let name_str = era
                .name
                .as_ref()
                .map_or(String::new(), |n| format!(" /{n}/"));
            let desc_str = era
                .description
                .as_ref()
                .map_or(String::new(), |d| format!(" - {d}"));
            println!("  * Era {num} (ID: {}){}{}", era.id, name_str, desc_str);
        }
        println!(
            "================================================================================"
        );
    }

    for (i, entry) in dict.entries.iter().enumerate() {
        print_entry(i + 1, entry, &dict.eras);
    }
    Ok(())
}

fn handle_dict_add_era_cmd(
    dict_path: &std::path::Path,
    era: Option<u32>,
    name: Option<String>,
    description: Option<String>,
) -> anyhow::Result<()> {
    let dict_json = fs::read_to_string(dict_path).with_context(|| {
        format!(
            "Failed to read dictionary file from {}",
            dict_path.display()
        )
    })?;
    let mut dict = dict_json
        .parse::<language::Dictionary>()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to parse dictionary")?;

    let ipa_name = match name {
        Some(n) => Some(
            n.parse::<ipa::IpaString>()
                .context("Failed to parse era name as a valid IPA string")?,
        ),
        None => None,
    };

    let (assigned_era, era_id) = dict
        .add_era(era, ipa_name, description)
        .map_err(|e| anyhow::anyhow!(e))?;

    dict.save_to_file(dict_path)
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to save dictionary")?;

    println!("Added era {assigned_era} with ID {era_id}");
    Ok(())
}

fn generate_conlang_word(
    language_path: Option<&std::path::Path>,
    r#type: &str,
) -> anyhow::Result<ipa::IpaString> {
    let lang_path = language_path
        .context("Language configuration file (--language) is required to generate the word")?;
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

    let mut rng = language::generator::thread_rng();
    let mut warning_logged = false;

    let word = language::generator::generate_word(
        r#type,
        &config,
        &mut rng,
        8, // max attempts
        &mut warning_logged,
    )?;
    word.parse::<ipa::IpaString>()
        .context("Failed to parse generated word as a valid IPA string")
}

#[expect(clippy::too_many_arguments, reason = "Command line parameter wrapper")]
fn handle_dict_add_cmd(
    dict_path: &std::path::Path,
    language_path: Option<&std::path::Path>,
    meaning: &str,
    definition: Option<&str>,
    generate: bool,
    r#type: String,
    era: Option<u32>,
    etymology: &[String],
    usage_notes: String,
) -> anyhow::Result<()> {
    let ipa_meaning = meaning
        .parse::<ipa::IpaString>()
        .context("Failed to parse meaning as a valid IPA string")?;
    let ipa_definition = if generate {
        generate_conlang_word(language_path, &r#type)?
    } else {
        let def_str = definition.context("Definition must be provided when not generating")?;
        def_str
            .parse::<ipa::IpaString>()
            .context("Failed to parse definition as a valid IPA string")?
    };
    let ety_map = if etymology.is_empty() {
        None
    } else {
        Some(parse_etymology(etymology)?)
    };
    let entry = language::NewEntry {
        meaning: ipa_meaning,
        definition: ipa_definition,
        r#type,
        era,
        etymology: ety_map,
        usage_notes,
    };
    handle_dict_add(dict_path, entry)
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

fn handle_dictionary_cmd(
    dict_cmd: DictionaryCmd,
    language: Option<&PathBuf>,
    dict: Option<&PathBuf>,
) -> anyhow::Result<()> {
    let dict_path =
        dict.context("Dictionary file path (--dict) is required for dictionary command")?;
    match dict_cmd.subcommand {
        DictionarySubcommand::Init => {
            handle_dict_init(dict_path, language.map(PathBuf::as_path))?;
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
            handle_dict_add_cmd(
                dict_path,
                language.map(PathBuf::as_path),
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
            handle_dict_remove(dict_path, &id)?;
        }
        DictionarySubcommand::Print => {
            handle_dict_print(dict_path)?;
        }
        DictionarySubcommand::AddEra {
            era,
            name,
            description,
        } => {
            handle_dict_add_era_cmd(dict_path, era, name, description)?;
        }
    }
    Ok(())
}

fn run_command(
    cmd: Commands,
    phone_config: Option<&PathBuf>,
    language: Option<&PathBuf>,
    dict: Option<&PathBuf>,
) -> anyhow::Result<()> {
    match cmd {
        Commands::Lookup { phoneme } => {
            handle_lookup(&phoneme, phone_config)?;
        }
        Commands::Generate(gen_cmd) => {
            handle_generate(&gen_cmd, language)?;
        }
        Commands::Dictionary(dict_cmd) => {
            handle_dictionary_cmd(dict_cmd, language, dict)?;
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
    } else {
        soundchange::parse_soundchange();
    }

    Ok(())
}
