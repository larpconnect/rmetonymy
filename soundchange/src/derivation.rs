use crate::{EvalContext, WorkingWord, apply_rule, compile_single_rule_from_str};
use ipa::IpaSequence;
use ipa::sequence::{PhonemeSequence, ProsodyMarker, SequenceElement};
use language::config::LanguageConfig;
use language::syllable::IpaWord;
use std::collections::BTreeSet;
use std::str::FromStr;

fn extract_phonemes_and_boundaries(
    seq: &PhonemeSequence,
) -> (Vec<ipa::sequence::Phoneme>, BTreeSet<usize>) {
    let mut phonemes = Vec::new();
    let mut syllable_boundaries = BTreeSet::new();
    for el in &seq.elements {
        match el {
            SequenceElement::Phoneme(p) => {
                phonemes.push(p.clone());
            }
            SequenceElement::SyllableBreak => {
                syllable_boundaries.insert(phonemes.len());
            }
            SequenceElement::Prosody(_) => {}
        }
    }
    (phonemes, syllable_boundaries)
}

fn find_stress_index(seq: &PhonemeSequence, phonemes: &[ipa::sequence::Phoneme]) -> Option<usize> {
    let primary_stress_pos = seq
        .elements
        .iter()
        .position(|el| matches!(el, SequenceElement::Prosody(ProsodyMarker::PrimaryStress)))?;

    let mut phoneme_count_before = 0;
    if let Some(sub_slice) = seq.elements.get(..primary_stress_pos) {
        for el in sub_slice {
            if matches!(el, SequenceElement::Phoneme(_)) {
                phoneme_count_before += 1;
            }
        }
    }
    for (i, p) in phonemes.iter().enumerate().skip(phoneme_count_before) {
        if language::phonology::is_vowel(p) {
            return Some(i);
        }
    }
    None
}

/// Helper to convert a `PhonemeSequence` and a parallel `tags` vector into a `WorkingWord`.
#[must_use]
pub fn sequence_to_working_word(seq: &PhonemeSequence, tags: Vec<Option<usize>>) -> WorkingWord {
    let (phonemes, syllable_boundaries) = extract_phonemes_and_boundaries(seq);
    let stress_index = find_stress_index(seq, &phonemes);
    let mut tags = tags;
    if tags.len() != phonemes.len() {
        tags.resize(phonemes.len(), None);
    }

    WorkingWord {
        phonemes,
        syllable_boundaries,
        stress_index,
        tags,
    }
}

/// The result of successfully applying derivations to a word.
#[derive(Debug, Clone)]
pub struct DerivationResult {
    pub word: IpaWord,
    pub tags: Vec<Option<usize>>,
    pub final_type: String,
    pub final_era: u32,
    pub step_types: Vec<Option<String>>,
}

/// Applies a list of derivations to a word (and its type), tracking which derivation modified which phonemes.
///
/// # Errors
/// Returns an error string if type matching fails or transform application fails.
pub fn apply_derivations(
    definition: &IpaWord,
    start_type: &str,
    derivation_names: &[String],
    config: &LanguageConfig,
    word_era: u32,
) -> Result<DerivationResult, String> {
    let mut current_type = start_type.to_string();
    let mut current_era = word_era;
    let mut seq = PhonemeSequence::from(definition.clone());
    let mut tags = vec![None; seq.phonemes().len()];
    let mut step_types = Vec::new();

    for (idx, deriv_name) in derivation_names.iter().enumerate() {
        let deriv = get_derivation(config, deriv_name)?;
        step_types.push(deriv.to_type.clone());

        apply_single_derivation(
            &mut seq,
            &mut tags,
            &mut current_type,
            &mut current_era,
            idx + 1,
            deriv_name,
            config,
        )?;
    }

    let final_word = IpaWord::try_from_sequence(&seq, config)
        .map_err(|e| format!("Failed to build final resyllabified word: {e}"))?;

    Ok(DerivationResult {
        word: final_word,
        tags,
        final_type: current_type,
        final_era: current_era,
        step_types,
    })
}

fn filter_intermediate_eras(
    compiled_eras: &[(u32, Vec<crate::CompiledSoundChangeRule>)],
    start_era: u32,
    end_era: u32,
) -> Vec<&(u32, Vec<crate::CompiledSoundChangeRule>)> {
    let mut sorted_eras: Vec<_> = compiled_eras
        .iter()
        .filter(|(era, _)| *era >= start_era && *era < end_era)
        .collect();
    sorted_eras.sort_by_key(|(era, _)| *era);
    sorted_eras
}

fn eval_intermediate_rules(
    working: &mut WorkingWord,
    sorted_eras: &[&(u32, Vec<crate::CompiledSoundChangeRule>)],
    ctx: &EvalContext<'_>,
) -> Result<(), String> {
    for (_, rules) in sorted_eras {
        for rule in rules {
            apply_rule(working, rule, ctx)?;
        }
    }
    Ok(())
}

fn rebuild_intermediate_seq_and_tags(
    seq: &mut PhonemeSequence,
    tags: &mut Vec<Option<usize>>,
    working_tags: Vec<Option<usize>>,
    flat_elements: Vec<SequenceElement>,
    config: &LanguageConfig,
) -> Result<(), String> {
    let flat_seq = PhonemeSequence {
        elements: flat_elements,
    };
    let resyllabified = IpaWord::try_from_sequence(&flat_seq, config)
        .map_err(|e| format!("Failed to resyllabify word: {e}"))?;

    let mut next_tags = working_tags;
    if next_tags.len() != resyllabified.phonemes().len() {
        next_tags.resize(resyllabified.phonemes().len(), None);
    }
    *seq = PhonemeSequence::from(resyllabified);
    *tags = next_tags;
    Ok(())
}

fn apply_intermediate_sound_changes(
    seq: &mut PhonemeSequence,
    tags: &mut Vec<Option<usize>>,
    start_era: u32,
    end_era: u32,
    config: &LanguageConfig,
) -> Result<(), String> {
    let Some(ref sound_changes) = config.sound_changes else {
        return Ok(());
    };

    let compiled_eras = crate::compiler::compile_sound_changes(sound_changes)
        .map_err(|e| format!("Failed to compile sound changes: {e:?}"))?;

    let mut working = sequence_to_working_word(seq, tags.clone());
    let ctx = EvalContext {
        classes: &config.phonology.sound_classes,
        system: ipa::DEFAULT_SYSTEM
            .as_ref()
            .map_err(|e| format!("Failed to load default IPA system: {e:?}"))?,
        active_tag: None,
    };

    let sorted_eras = filter_intermediate_eras(&compiled_eras, start_era, end_era);
    eval_intermediate_rules(&mut working, &sorted_eras, &ctx)?;

    let flat_elements: Vec<SequenceElement> =
        std::mem::take(&mut working.to_flat_sequence().elements)
            .into_iter()
            .filter(|el| !matches!(el, SequenceElement::SyllableBreak))
            .collect();

    rebuild_intermediate_seq_and_tags(seq, tags, working.tags, flat_elements, config)
}

fn resyllabify_and_update_tags(
    seq: &mut PhonemeSequence,
    tags: &mut Vec<Option<usize>>,
    config: &LanguageConfig,
) -> Result<(), String> {
    let flat_elements: Vec<SequenceElement> = std::mem::take(&mut seq.elements)
        .into_iter()
        .filter(|el| !matches!(el, SequenceElement::SyllableBreak))
        .collect();
    let flat_seq = PhonemeSequence {
        elements: flat_elements,
    };

    let resyllabified = IpaWord::try_from_sequence(&flat_seq, config)
        .map_err(|e| format!("Failed to resyllabify word: {e}"))?;

    let phonemes_count = resyllabified.phonemes().len();
    if tags.len() != phonemes_count {
        tags.resize(phonemes_count, None);
    }

    *seq = PhonemeSequence::from(resyllabified);
    Ok(())
}

fn get_derivation<'a>(
    config: &'a LanguageConfig,
    deriv_name: &str,
) -> Result<&'a language::config::Derivation, String> {
    config
        .derivations
        .as_ref()
        .and_then(|list| list.iter().find(|d| d.name == deriv_name))
        .ok_or_else(|| format!("Derivation '{deriv_name}' not found in configuration"))
}

fn check_era_and_apply_changes(
    seq: &mut PhonemeSequence,
    tags: &mut Vec<Option<usize>>,
    current_era: &mut u32,
    deriv_era: Option<u32>,
    deriv_name: &str,
    config: &LanguageConfig,
) -> Result<(), String> {
    let Some(era_val) = deriv_era else {
        return Ok(());
    };
    if era_val < *current_era {
        return Err(format!(
            "Cannot apply derivation '{deriv_name}': word era {current_era} is after derivation era {era_val}"
        ));
    }
    if era_val > *current_era {
        apply_intermediate_sound_changes(seq, tags, *current_era, era_val, config)?;
        *current_era = era_val;
    }
    Ok(())
}

fn check_type_constraint(
    current_type: &str,
    from_type: Option<&String>,
    deriv_name: &str,
) -> Result<(), String> {
    let Some(from_t) = from_type else {
        return Ok(());
    };
    let matches = type_matches(current_type, from_t);
    if !matches {
        return Err(format!(
            "Cannot apply derivation '{deriv_name}': word type '{current_type}' does not match expected '{from_t}'"
        ));
    }
    Ok(())
}

fn apply_single_derivation(
    seq: &mut PhonemeSequence,
    tags: &mut Vec<Option<usize>>,
    current_type: &mut String,
    current_era: &mut u32,
    deriv_idx: usize,
    deriv_name: &str,
    config: &LanguageConfig,
) -> Result<(), String> {
    let deriv = get_derivation(config, deriv_name)?;

    check_era_and_apply_changes(seq, tags, current_era, deriv.era, deriv_name, config)?;

    check_type_constraint(current_type, deriv.from_type.as_ref(), deriv_name)?;

    for transform in &deriv.transforms {
        apply_derivation_transform(seq, tags, transform, deriv_idx, config)?;
    }

    if let Some(ref to_t) = deriv.to_type {
        current_type.clone_from(to_t);
    }

    resyllabify_and_update_tags(seq, tags, config)?;
    Ok(())
}

fn apply_prefix_transform(
    seq: &mut PhonemeSequence,
    tags: &mut Vec<Option<usize>>,
    pref_str: &str,
    deriv_idx: usize,
) -> Result<(), String> {
    if pref_str.is_empty() {
        return Err("Empty prefix is not allowed".to_string());
    }
    let pref_seq = PhonemeSequence::from_str(pref_str)
        .map_err(|e| format!("Invalid prefix '{pref_str}': {e:?}"))?;
    let pref_phonemes_len = pref_seq.phonemes().len();

    let mut new_elements = pref_seq.elements;
    new_elements.extend(seq.elements.iter().cloned());
    *seq = PhonemeSequence {
        elements: new_elements,
    };

    let mut new_tags = vec![Some(deriv_idx); pref_phonemes_len];
    new_tags.extend(tags.iter().copied());
    *tags = new_tags;
    Ok(())
}

fn apply_suffix_transform(
    seq: &mut PhonemeSequence,
    tags: &mut Vec<Option<usize>>,
    suff_str: &str,
    deriv_idx: usize,
) -> Result<(), String> {
    if suff_str.is_empty() {
        return Err("Empty suffix is not allowed".to_string());
    }
    let suff_seq = PhonemeSequence::from_str(suff_str)
        .map_err(|e| format!("Invalid suffix '{suff_str}': {e:?}"))?;
    let suff_phonemes_len = suff_seq.phonemes().len();

    seq.elements.extend(suff_seq.elements);
    tags.extend(vec![Some(deriv_idx); suff_phonemes_len]);
    Ok(())
}

fn apply_sound_change_transform(
    seq: &mut PhonemeSequence,
    tags: &mut Vec<Option<usize>>,
    transform: &str,
    deriv_idx: usize,
    config: &LanguageConfig,
) -> Result<(), String> {
    let compiled_rule = compile_single_rule_from_str(transform, config.sound_changes.as_ref())
        .map_err(|e| format!("Failed to compile derivation sound change '{transform}': {e}"))?;

    let mut working = sequence_to_working_word(seq, tags.clone());
    let ctx = EvalContext {
        classes: &config.phonology.sound_classes,
        system: ipa::DEFAULT_SYSTEM.as_ref().map_err(|e| format!("{e}"))?,
        active_tag: Some(deriv_idx),
    };

    apply_rule(&mut working, &compiled_rule, &ctx)
        .map_err(|e| format!("Failed to apply derivation sound change '{transform}': {e}"))?;

    *seq = working.to_flat_sequence();
    *tags = working.tags;
    Ok(())
}

fn apply_derivation_transform(
    seq: &mut PhonemeSequence,
    tags: &mut Vec<Option<usize>>,
    transform: &str,
    deriv_idx: usize,
    config: &LanguageConfig,
) -> Result<(), String> {
    if let Some(pref_str) = transform.strip_suffix('-') {
        apply_prefix_transform(seq, tags, pref_str, deriv_idx)
    } else if let Some(suff_str) = transform.strip_prefix('-') {
        apply_suffix_transform(seq, tags, suff_str, deriv_idx)
    } else {
        apply_sound_change_transform(seq, tags, transform, deriv_idx, config)
    }
}

fn type_matches(word_type: &str, filter_type: &str) -> bool {
    let (w_base, w_sub) = word_type.split_once('.').unwrap_or((word_type, ""));
    let (f_base, f_sub) = filter_type.split_once('.').unwrap_or((filter_type, ""));

    if w_base != f_base {
        return false;
    }
    if !f_sub.is_empty() && w_sub != f_sub {
        return false;
    }
    true
}
