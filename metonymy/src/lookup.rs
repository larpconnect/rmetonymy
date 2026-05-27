use anyhow::Context;
use ipa::IpaSystem;
use std::fs;
use std::path::PathBuf;

pub(crate) fn load_ipa_system(phone_config: Option<&PathBuf>) -> anyhow::Result<IpaSystem> {
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

pub(crate) fn handle_lookup(phoneme: &str, phone_config: Option<&PathBuf>) -> anyhow::Result<()> {
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
