use anyhow::Context;
use std::fs;
use std::path::Path;
use language::generator::validation::is_valid_derivation_name;

fn load_dictionary(dict_path: &Path) -> anyhow::Result<language::Dictionary> {
    let dict_json = fs::read_to_string(dict_path).with_context(|| {
        format!("Failed to read dictionary file from {}", dict_path.display())
    })?;
    dict_json
        .parse::<language::Dictionary>()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to parse dictionary")
}

fn load_language_config(
    language_path: Option<&Path>,
) -> anyhow::Result<Option<language::config::LanguageConfig>> {
    let Some(path) = language_path else {
        return Ok(None);
    };
    let lang_json = fs::read_to_string(path).with_context(|| {
        format!("Failed to read language config from {}", path.display())
    })?;
    let config: language::config::LanguageConfig =
        serde_json::from_str(&lang_json).context("Failed to parse language config JSON")?;
    config
        .validate()
        .context("Language configuration validation failed")?;
    Ok(Some(config))
}

fn parse_lookup_string(s: &str) -> (String, Vec<String>) {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() <= 1 {
        return (s.to_string(), Vec::new());
    }

    let mut parts = parts;
    let mut derivations = Vec::new();
    while parts.len() > 1 {
        if let Some(last) = parts.last() {
            if is_valid_derivation_name(last) {
                derivations.push((*last).to_string());
                parts.pop();
            } else {
                break;
            }
        }
    }
    derivations.reverse();
    let base_meaning = parts.join("-");
    (base_meaning, derivations)
}

fn type_matches(entry_type: &str, filter_type: &str) -> bool {
    let (w_base, w_sub) = entry_type.split_once('.').unwrap_or((entry_type, ""));
    let (f_base, f_sub) = filter_type.split_once('.').unwrap_or((filter_type, ""));

    if w_base != f_base {
        return false;
    }
    if !f_sub.is_empty() && w_sub != f_sub {
        return false;
    }
    true
}

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

    let (sc_word, _) = soundchange::apply_sound_changes(
        ipa_word,
        &compiled_sc,
        era,
        u32::MAX,
        config,
        false,
    )
    .map_err(|e| anyhow::anyhow!("Failed to apply sound changes: {e}"))?;
    println!("{sc_word}");

    let ortho_rules = config.orthography.as_deref().unwrap_or(&[]);
    let compiled_ortho = soundchange::compile_ortho_rules(ortho_rules)
        .map_err(|e| anyhow::anyhow!("Failed to compile orthography rules: {e}"))?;
    let (ortho_res, _) =
        soundchange::apply_orthography(&sc_word, &compiled_ortho, config, false)
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

fn print_lookup_with_derivations(
    ipa_word: &language::syllable::IpaWord,
    entry_type: &str,
    derivation_names: &[String],
    era: u32,
    meaning: &str,
    config: &language::config::LanguageConfig,
) -> anyhow::Result<()> {
    let res =
        soundchange::apply_derivations(ipa_word, entry_type, derivation_names, config, era)
            .map_err(|e| anyhow::anyhow!("Failed to apply derivations: {e}"))?;
    let (final_sc_word, sc_tags) =
        apply_post_derivation_sound_changes(&res.word, &res.tags, res.final_era, config)?;

    let ortho_rules = config.orthography.as_deref().unwrap_or(&[]);
    let compiled_ortho = soundchange::compile_ortho_rules(ortho_rules)
        .map_err(|e| anyhow::anyhow!("Failed to compile orthography rules: {e}"))?;
    let (ortho_res, _) =
        soundchange::apply_orthography(&final_sc_word, &compiled_ortho, config, false)
            .map_err(|e| anyhow::anyhow!("Failed to apply orthography: {e}"))?;

    let colored_derived = format_colored_word(&res.word, &res.tags);
    let colored_sc = format_colored_word(&final_sc_word, &sc_tags);

    println!("{meaning}:{}", res.final_type);
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
                    let color = match t % 3 {
                        1 => "\x1b[31m", // Red
                        2 => "\x1b[33m", // Yellow
                        0 => "\x1b[32m", // Green
                        _ => "\x1b[0m",
                    };
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

pub(crate) fn handle_dict_lookup(
    dict_path: &Path,
    language_path: Option<&Path>,
    meaning: &str,
    filter_type: Option<&str>,
) -> anyhow::Result<()> {
    let config_opt = load_language_config(language_path)?;
    let dict = load_dictionary(dict_path)?;
    let (base_meaning, derivation_names) = parse_lookup_string(meaning);

    let entry = dict
        .entries
        .iter()
        .find(|e| {
            if e.meaning.to_string() != base_meaning {
                return false;
            }
            if let Some(ft) = filter_type {
                let matches = type_matches(&e.r#type, ft);
                if !matches {
                    return false;
                }
            }
            true
        })
        .ok_or_else(|| {
            if let Some(ft) = filter_type {
                anyhow::anyhow!(
                    "Word with meaning '{base_meaning}' and type '{ft}' not found in dictionary"
                )
            } else {
                anyhow::anyhow!("Word with meaning '{base_meaning}' not found in dictionary")
            }
        })?;

    let config = config_opt.context("Language configuration file (--language) is required for lookup")?;
    let ipa_word = config
        .syllabify(&entry.definition)
        .map_err(|e| anyhow::anyhow!("Failed to syllabify definition: {e}"))?;

    if derivation_names.is_empty() {
        print_lookup_without_derivations(&ipa_word, entry.era, &config)?;
    } else {
        print_lookup_with_derivations(
            &ipa_word,
            &entry.r#type,
            &derivation_names,
            entry.era,
            meaning,
            &config,
        )?;
    }

    Ok(())
}
