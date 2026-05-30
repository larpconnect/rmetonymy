use anyhow::Context;
use std::path::Path;

const MAX_GENERATION_ATTEMPTS: usize = 8;

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

pub(crate) fn handle_dict_init(dict_path: &Path, lang_path: Option<&Path>) -> anyhow::Result<()> {
    let lang_path = lang_path.context(
        "Language configuration file (--language) is required to initialize a dictionary",
    )?;
    let lang_config = super::load_language_config(lang_path)?;

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

fn read_and_parse_dict(dict_path: &Path) -> anyhow::Result<language::Dictionary> {
    super::load_dictionary(dict_path)
}

fn save_dict(dict: &language::Dictionary, dict_path: &Path) -> anyhow::Result<()> {
    dict.save_to_file(dict_path)
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to save dictionary")
}

pub(crate) fn handle_dict_add(dict_path: &Path, entry: language::NewEntry) -> anyhow::Result<()> {
    let mut dict = read_and_parse_dict(dict_path)?;

    let definition = entry.definition.to_string();
    let meaning = entry.meaning.to_string();

    let entry_id = dict.add_entry(entry);

    save_dict(&dict, dict_path)?;

    println!("Added word '{definition}' (meaning: '{meaning}') with ID {entry_id}");
    Ok(())
}

pub(crate) fn handle_dict_remove(dict_path: &Path, id: &str) -> anyhow::Result<()> {
    let mut dict = read_and_parse_dict(dict_path)?;

    if dict.remove_entry(id) {
        save_dict(&dict, dict_path)?;
        println!("Removed word with ID {id}");
    } else {
        println!("Word with ID {id} not found in dictionary");
    }
    Ok(())
}

fn print_border() {
    println!("================================================================================");
}

pub(crate) fn handle_dict_print(dict_path: &Path) -> anyhow::Result<()> {
    let dict = read_and_parse_dict(dict_path)?;

    print_border();
    println!("Dictionary ID: {}", dict.id);
    if !dict.eras.is_empty() {
        println!("Total Eras   : {}", dict.eras.len());
    }
    println!("Total Entries: {}", dict.entries.len());
    print_border();

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
        print_border();
    }

    for (i, entry) in dict.entries.iter().enumerate() {
        print_entry(i + 1, entry, &dict.eras);
    }
    Ok(())
}

pub(crate) fn handle_dict_add_era_cmd(
    dict_path: &Path,
    era: Option<u32>,
    name: Option<String>,
    description: Option<String>,
) -> anyhow::Result<()> {
    let mut dict = read_and_parse_dict(dict_path)?;

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

    save_dict(&dict, dict_path)?;

    println!("Added era {assigned_era} with ID {era_id}");
    Ok(())
}

fn generate_conlang_word(
    language_path: Option<&Path>,
    r#type: &str,
) -> anyhow::Result<ipa::IpaString> {
    let lang_path = language_path
        .context("Language configuration file (--language) is required to generate the word")?;
    let config = super::load_language_config(lang_path)?;

    let mut rng = language::generator::thread_rng();
    let mut warning_logged = false;

    let word = language::generator::generate_word(
        r#type,
        &config,
        &mut rng,
        MAX_GENERATION_ATTEMPTS,
        &mut warning_logged,
    )?;
    word.parse::<ipa::IpaString>()
        .context("Failed to parse generated word as a valid IPA string")
}

pub struct DictAddParams<'a> {
    pub dict_path: &'a Path,
    pub language_path: Option<&'a Path>,
    pub meaning: &'a str,
    pub definition: Option<&'a str>,
    pub generate: bool,
    pub r#type: String,
    pub era: Option<u32>,
    pub etymology: &'a [String],
    pub usage_notes: String,
}

pub(crate) fn handle_dict_add_cmd(params: DictAddParams<'_>) -> anyhow::Result<()> {
    let ipa_meaning = params
        .meaning
        .parse::<ipa::IpaString>()
        .context("Failed to parse meaning as a valid IPA string")?;
    let mut ipa_definition = if params.generate {
        generate_conlang_word(params.language_path, &params.r#type)?
    } else {
        let def_str = params
            .definition
            .context("Definition must be provided when not generating")?;
        def_str
            .parse::<ipa::IpaString>()
            .context("Failed to parse definition as a valid IPA string")?
    };

    if let Some(path) = params.language_path {
        let config = super::load_language_config(path)?;
        if let Ok(syllabified) = config.syllabify(&ipa_definition) {
            ipa_definition = syllabified.to_string().parse::<ipa::IpaString>()?;
        }
    }

    let ety_map = if params.etymology.is_empty() {
        None
    } else {
        Some(parse_etymology(params.etymology)?)
    };
    let entry = language::NewEntry {
        meaning: ipa_meaning,
        definition: ipa_definition,
        r#type: params.r#type,
        era: params.era,
        etymology: ety_map,
        usage_notes: params.usage_notes,
    };
    handle_dict_add(params.dict_path, entry)
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
