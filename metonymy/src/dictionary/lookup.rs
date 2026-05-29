use anyhow::Context;
use std::path::Path;

const NUM_COLORS: usize = 3;

fn load_dictionary(dict_path: &Path) -> anyhow::Result<language::Dictionary> {
    super::load_dictionary(dict_path)
}

fn load_language_config(
    language_path: Option<&Path>,
) -> anyhow::Result<Option<language::config::LanguageConfig>> {
    let Some(path) = language_path else {
        return Ok(None);
    };
    super::load_language_config(path).map(Some)
}

use super::parse::parse_lookup_string;



fn print_lookup_without_derivations(
    ipa_word: &language::syllable::IpaWord,
    era: u32,
    config: &language::config::LanguageConfig,
) -> anyhow::Result<()> {
    println!("{ipa_word}");

    let sound_changes = config
        .sound_changes
        .clone()
        .unwrap_or(language::config::SoundChanges {
            preamble: Vec::new(),
            eras: Vec::new(),
        });
    let compiled_sc = soundchange::compile_sound_changes(&sound_changes)
        .map_err(|e| anyhow::anyhow!("Failed to compile sound changes: {e}"))?;

    let (sc_word, _) =
        soundchange::apply_sound_changes(ipa_word, &compiled_sc, (era, u32::MAX), config, false)
            .map_err(|e| anyhow::anyhow!("Failed to apply sound changes: {e}"))?;
    println!("{sc_word}");

    let ortho_rules = config.orthography.as_deref().unwrap_or(&[]);
    let compiled_ortho = soundchange::compile_ortho_rules(ortho_rules)
        .map_err(|e| anyhow::anyhow!("Failed to compile orthography rules: {e}"))?;
    let (ortho_res, _) = soundchange::apply_orthography(&sc_word, &compiled_ortho, config, false)
        .map_err(|e| anyhow::anyhow!("Failed to apply orthography: {e}"))?;
    println!("{ortho_res}");

    Ok(())
}

fn apply_post_derivation_sound_changes(
    derived_word: &language::syllable::IpaWord,
    tags: &[Option<usize>],
    final_era: u32,
    config: &language::config::LanguageConfig,
) -> anyhow::Result<(language::syllable::IpaWord, Vec<Option<usize>>)> {
    let sound_changes = config
        .sound_changes
        .clone()
        .unwrap_or(language::config::SoundChanges {
            preamble: Vec::new(),
            eras: Vec::new(),
        });
    let compiled_sc = soundchange::compile_sound_changes(&sound_changes)
        .map_err(|e| anyhow::anyhow!("Failed to compile sound changes: {e}"))?;

    let mut working = soundchange::derivation::sequence_to_working_word(
        &ipa::sequence::PhonemeSequence::from(derived_word.clone()),
        tags.to_vec(),
    );

    let ctx = soundchange::EvalContext {
        classes: &config.phonology.sound_classes,
        system: ipa::DEFAULT_SYSTEM
            .as_ref()
            .map_err(|e| anyhow::anyhow!("Failed to load default IPA system: {e:?}"))?,
        active_tag: None,
    };

    let mut sorted_eras: Vec<_> = compiled_sc
        .iter()
        .filter(|(era_num, _)| *era_num >= final_era)
        .collect();
    sorted_eras.sort_by_key(|(era_num, _)| *era_num);

    for (_era_num, rules) in sorted_eras {
        for rule in rules {
            soundchange::apply_rule(&mut working, rule, &ctx)
                .map_err(|e| anyhow::anyhow!("Failed to apply sound change: {e}"))?;
        }
    }

    let flat_seq = working.to_flat_sequence();
    let final_sc_word = language::syllable::IpaWord::try_from_sequence(&flat_seq, config)
        .map_err(|e| anyhow::anyhow!("Failed to resyllabify word after sound changes: {e}"))?;
    Ok((final_sc_word, working.tags))
}

fn get_derivation_color(idx: usize) -> &'static str {
    match idx % NUM_COLORS {
        1 => "\x1b[31m", // Red
        2 => "\x1b[36m", // Cyan
        0 => "\x1b[32m", // Green
        _ => "",
    }
}

fn format_colored_lookup_line(
    base_meaning: &str,
    derivation_names: &[String],
    step_types: &[Option<String>],
) -> String {
    let mut result = base_meaning.to_string();
    for (i, name) in derivation_names.iter().enumerate() {
        let idx = i + 1;
        let color = get_derivation_color(idx);
        result.push_str(color);
        result.push('-');
        result.push_str(name);
        if let Some(Some(to_type)) = step_types.get(i) {
            result.push(':');
            result.push_str(to_type);
        }
        result.push_str("\x1b[0m");
    }
    result
}

struct LookupDerivationsParams<'a> {
    ipa_word: &'a language::syllable::IpaWord,
    entry_type: &'a str,
    derivation_names: &'a [String],
    era: u32,
    base_meaning: &'a str,
    config: &'a language::config::LanguageConfig,
}

fn print_lookup_with_derivations(
    params: LookupDerivationsParams<'_>,
) -> anyhow::Result<()> {
    let res = soundchange::apply_derivations(
        params.ipa_word,
        params.entry_type,
        params.derivation_names,
        params.config,
        params.era,
    )
    .map_err(|e| anyhow::anyhow!("Failed to apply derivations: {e}"))?;
    let (final_sc_word, sc_tags) =
        apply_post_derivation_sound_changes(&res.word, &res.tags, res.final_era, params.config)?;

    let ortho_rules = params.config.orthography.as_deref().unwrap_or(&[]);
    let compiled_ortho = soundchange::compile_ortho_rules(ortho_rules)
        .map_err(|e| anyhow::anyhow!("Failed to compile orthography rules: {e}"))?;
    let (ortho_res, _) =
        soundchange::apply_orthography(&final_sc_word, &compiled_ortho, params.config, false)
            .map_err(|e| anyhow::anyhow!("Failed to apply orthography: {e}"))?;

    let colored_derived = format_colored_word(&res.word, &res.tags);
    let colored_sc = format_colored_word(&final_sc_word, &sc_tags);
    let colored_line = format_colored_lookup_line(params.base_meaning, params.derivation_names, &res.step_types);

    println!("{colored_line}");
    println!("{colored_derived}");
    println!("{colored_sc}");
    println!("{ortho_res}");

    Ok(())
}

fn format_colored_word(word: &language::syllable::IpaWord, tags: &[Option<usize>]) -> String {
    use ipa::IpaSequence;
    use ipa::sequence::SequenceElement;

    let mut result = String::new();
    let mut phoneme_idx = 0;

    for el in word.elements() {
        match el {
            SequenceElement::Phoneme(p) => {
                let tag = tags.get(phoneme_idx).copied().flatten();
                if let Some(t) = tag {
                    let color = get_derivation_color(t);
                    result.push_str(color);
                    result.push_str(&p.to_string());
                    result.push_str("\x1b[0m");
                } else {
                    result.push_str(&p.to_string());
                }
                phoneme_idx += 1;
            }
            SequenceElement::SyllableBreak => {
                result.push('.');
            }
            SequenceElement::Prosody(pm) => {
                result.push_str(&pm.to_string());
            }
        }
    }
    result
}

fn entry_matches(
    entry: &language::DictionaryEntry,
    base_meaning: &str,
    filter_type: Option<&str>,
) -> bool {
    if entry.meaning.to_string() != base_meaning {
        return false;
    }
    if let Some(ft) = filter_type {
        let matches = language::type_matches(&entry.r#type, ft);
        if !matches {
            return false;
        }
    }
    true
}

fn make_lookup_error(base_meaning: &str, filter_type: Option<&str>) -> anyhow::Error {
    if let Some(ft) = filter_type {
        anyhow::anyhow!(
            "Word with meaning '{base_meaning}' and type '{ft}' not found in dictionary"
        )
    } else {
        anyhow::anyhow!("Word with meaning '{base_meaning}' not found in dictionary")
    }
}

fn find_matching_entry<'a>(
    dict: &'a language::Dictionary,
    base_meaning: &str,
    filter_type: Option<&str>,
) -> anyhow::Result<&'a language::DictionaryEntry> {
    dict.entries
        .iter()
        .find(|e| entry_matches(e, base_meaning, filter_type))
        .ok_or_else(|| make_lookup_error(base_meaning, filter_type))
}

fn print_lookup_result(
    ipa_word: &language::syllable::IpaWord,
    entry: &language::DictionaryEntry,
    derivation_names: &[String],
    base_meaning: &str,
    config: &language::config::LanguageConfig,
) -> anyhow::Result<()> {
    if derivation_names.is_empty() {
        print_lookup_without_derivations(ipa_word, entry.era, config)?;
    } else {
        let params = LookupDerivationsParams {
            ipa_word,
            entry_type: &entry.r#type,
            derivation_names,
            era: entry.era,
            base_meaning,
            config,
        };
        print_lookup_with_derivations(params)?;
    }
    Ok(())
}

pub(crate) fn handle_dict_lookup(
    dict_path: &Path,
    language_path: Option<&Path>,
    meaning: &str,
    filter_type: Option<&str>,
) -> anyhow::Result<()> {
    let config_opt = load_language_config(language_path)?;
    let dict = load_dictionary(dict_path)?;
    let (base_meaning, derivation_names) = parse_lookup_string(meaning);

    let entry = find_matching_entry(&dict, &base_meaning, filter_type)?;

    let config =
        config_opt.context("Language configuration file (--language) is required for lookup")?;
    let ipa_word = config
        .syllabify(&entry.definition)
        .map_err(|e| anyhow::anyhow!("Failed to syllabify definition: {e}"))?;

    print_lookup_result(&ipa_word, entry, &derivation_names, &base_meaning, &config)
}
