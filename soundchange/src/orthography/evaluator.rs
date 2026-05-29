use super::compiler::CompiledOrthoRule;
use super::parser::OrthoTransformElement;
use crate::ast::Operator;
use crate::evaluator::{EvalContext, MatchState, WorkingWord};
use ipa::IpaSequence;
use ipa::sequence::Phoneme;
use language::config::LanguageConfig;
use std::collections::BTreeSet;
use std::str::FromStr;

fn flatten_phonemes_and_modifiers(phonemes: &[Phoneme]) -> Vec<Phoneme> {
    let mut flat_phonemes = Vec::new();
    for p in phonemes {
        flat_phonemes.push(Phoneme {
            // Clone base string because Phoneme is not Copy
            base: p.base.clone(),
            modifiers: Vec::new(),
        });
        for m in &p.modifiers {
            flat_phonemes.push(Phoneme {
                // Clone modifier string because Phoneme is not Copy
                base: m.clone(),
                modifiers: Vec::new(),
            });
        }
    }
    flat_phonemes
}

/// Applies compiled orthography rules to an IPA word.
///
/// # Errors
/// Returns an error string if evaluation fails.
pub fn apply_orthography(
    word: &language::syllable::IpaWord,
    compiled_rules: &[CompiledOrthoRule],
    config: &LanguageConfig,
    verbose: bool,
) -> Result<(String, Vec<String>), String> {
    let flat_phonemes = flatten_phonemes_and_modifiers(&word.phonemes());
    let flat_len = flat_phonemes.len();
    let mut working = WorkingWord {
        phonemes: flat_phonemes,
        syllable_boundaries: BTreeSet::new(),
        stress_index: None,
        tags: vec![None; flat_len],
    };

    let ctx = EvalContext {
        classes: &config.phonology.sound_classes,
        system: ipa::DEFAULT_SYSTEM
            .as_ref()
            .map_err(|e| format!("Failed to load default IPA system: {e:?}"))?,
        active_tag: None,
    };

    let mut trace_logs = Vec::new();
    if verbose {
        trace_logs.push("--- Orthography Transform ---".to_string());
    }

    for rule in compiled_rules {
        // Clone WorkingWord to check if rule changed the word
        let before = working.clone();
        apply_ortho_rule(&mut working, rule, &ctx);
        if verbose && working != before {
            trace_logs.push(format!(
                "Ortho Rule: {}\n  In : {}\n  Out: {}",
                rule.original_string,
                before
                    .phonemes
                    .iter()
                    .map(ToString::to_string)
                    .collect::<String>(),
                working
                    .phonemes
                    .iter()
                    .map(ToString::to_string)
                    .collect::<String>()
            ));
        }
    }

    let flat: String = working.phonemes.iter().map(ToString::to_string).collect();

    Ok((flat, trace_logs))
}

fn apply_ortho_rule(word: &mut WorkingWord, rule: &CompiledOrthoRule, ctx: &EvalContext<'_>) {
    let is_leftward = matches!(
        rule.operator,
        Operator::LeftMultipleTransparent
            | Operator::LeftSingleTransparent
            | Operator::LeftMultipleOpaque
    );
    let is_opaque = matches!(
        rule.operator,
        Operator::RightMultipleOpaque | Operator::LeftMultipleOpaque
    );
    let is_single = matches!(
        rule.operator,
        Operator::RightSingleTransparent | Operator::LeftSingleTransparent
    );

    if is_opaque {
        apply_ortho_opaque_change(word, rule, is_leftward, ctx);
    } else {
        apply_ortho_transparent_change(word, rule, is_leftward, is_single, ctx);
    }
}

fn apply_ortho_opaque_change(
    word: &mut WorkingWord,
    rule: &CompiledOrthoRule,
    is_leftward: bool,
    ctx: &EvalContext<'_>,
) {
    // Clone WorkingWord to read unmodified word during match
    let original_word = word.clone();
    let matches = crate::evaluator::engine::find_all_matches(
        &original_word,
        &rule.match_part,
        rule.condition.as_ref(),
        is_leftward,
        ctx,
    );

    let mut sorted_matches = matches;
    sorted_matches.sort_by_key(|b| std::cmp::Reverse(b.0.start));

    for (range, state) in sorted_matches {
        replace_ortho_range(word, range, &state, &rule.transform_part);
    }
}

fn apply_ortho_transparent_change(
    word: &mut WorkingWord,
    rule: &CompiledOrthoRule,
    is_leftward: bool,
    is_single: bool,
    ctx: &EvalContext<'_>,
) {
    let mut scan_idx = if is_leftward { word.phonemes.len() } else { 0 };

    loop {
        if is_leftward && scan_idx > word.phonemes.len() {
            scan_idx = word.phonemes.len();
        }

        let match_opt = crate::evaluator::engine::find_next_match(
            word,
            &rule.match_part,
            rule.condition.as_ref(),
            scan_idx,
            is_leftward,
            ctx,
        );
        let Some((range, state)) = match_opt else {
            break;
        };

        // Clone range because splice takes ownership of range in next step
        let orig_range = range.clone();
        let new_range = replace_ortho_range(word, orig_range, &state, &rule.transform_part);

        if is_single {
            break;
        }

        if is_leftward {
            if range.start == 0 {
                break;
            }
            scan_idx = range.start;
        } else {
            scan_idx = new_range.end;
            if scan_idx > word.phonemes.len() {
                break;
            }
        }
    }
}

fn replace_ortho_range(
    word: &mut WorkingWord,
    range: std::ops::Range<usize>,
    state: &MatchState,
    transform: &[OrthoTransformElement],
) -> std::ops::Range<usize> {
    let new_phonemes = build_ortho_transform_phonemes(transform, word, &range, state);
    let new_len = new_phonemes.len();

    let start = range.start;
    let end = range.end;

    word.phonemes.splice(range.clone(), new_phonemes);
    let new_tags = vec![None; new_len];
    word.tags.splice(range, new_tags);

    let original_len = end - start;
    let mut updated_boundaries = BTreeSet::new();
    for &b in &word.syllable_boundaries {
        if b < start {
            updated_boundaries.insert(b);
        } else if b >= end {
            updated_boundaries.insert(b - original_len + new_len);
        }
    }
    word.syllable_boundaries = updated_boundaries;

    start..start + new_len
}

fn apply_captured_and_append_modifiers(
    p: &mut Phoneme,
    copy_modifiers: bool,
    append_modifiers: &[String],
    state: &MatchState,
    word: &WorkingWord,
) {
    if copy_modifiers {
        for m in crate::evaluator::transform::get_captured_modifiers_for_element(state, 0, word) {
            if !p.modifiers.contains(&m) {
                // Clone modifier string because modifiers list owns its strings
                p.modifiers.push(m.clone());
            }
        }
    }
    for m in append_modifiers {
        if !p.modifiers.contains(m) {
            // Clone modifier string because append_modifiers is a slice and we need to push a String
            p.modifiers.push(m.clone());
        }
    }
}

fn parse_literal_phonemes(val: &str) -> Vec<Phoneme> {
    if let Ok(seq) = ipa::sequence::PhonemeSequence::from_str(val) {
        // Clone phonemes vector because PhonemeSequence is parsed and owns them
        seq.phonemes().clone()
    } else {
        val.chars()
            .map(|c| Phoneme {
                base: c.to_string(),
                modifiers: Vec::new(),
            })
            .collect()
    }
}

fn eval_ortho_literal(
    el: &OrthoTransformElement,
    state: &MatchState,
    word: &WorkingWord,
) -> Vec<Phoneme> {
    let OrthoTransformElement::Literal {
        val,
        copy_modifiers,
        append_modifiers,
    } = el
    else {
        return Vec::new();
    };

    let mut phonemes = parse_literal_phonemes(val);

    for p in &mut phonemes {
        apply_captured_and_append_modifiers(p, *copy_modifiers, append_modifiers, state, word);
    }
    phonemes
}

fn eval_ortho_ref(
    el: &OrthoTransformElement,
    word: &WorkingWord,
    range: &std::ops::Range<usize>,
    state: &MatchState,
) -> Vec<Phoneme> {
    let OrthoTransformElement::Ref {
        marker,
        class_key,
        repeat,
        copy_modifiers,
        append_modifiers,
    } = el
    else {
        return Vec::new();
    };

    let source_phonemes = crate::evaluator::transform::get_referenced_phonemes(
        word,
        *marker,
        class_key.as_ref(),
        state,
        range,
    );
    let mut result = Vec::new();
    for _ in 0..*repeat {
        for sp in &source_phonemes {
            // Clone Phoneme to create a modified copy
            let mut p = sp.clone();
            apply_captured_and_append_modifiers(
                &mut p,
                *copy_modifiers,
                append_modifiers,
                state,
                word,
            );
            result.push(p);
        }
    }
    result
}

fn build_ortho_transform_phonemes(
    transform: &[OrthoTransformElement],
    word: &WorkingWord,
    range: &std::ops::Range<usize>,
    state: &MatchState,
) -> Vec<Phoneme> {
    let mut new_phonemes = Vec::new();

    for el in transform {
        match el {
            OrthoTransformElement::Empty => {}
            OrthoTransformElement::Literal { .. } => {
                new_phonemes.extend(eval_ortho_literal(el, state, word));
            }
            OrthoTransformElement::Ref { .. } => {
                new_phonemes.extend(eval_ortho_ref(el, word, range, state));
            }
        }
    }

    new_phonemes
}
