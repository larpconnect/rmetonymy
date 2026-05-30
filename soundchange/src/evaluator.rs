pub mod boundary;
pub mod boundary_adjust;
pub mod condition;
pub(crate) mod condition_match;
pub mod descriptor;
pub mod engine;
pub mod feature_changes;
pub mod features;
pub mod helper;
pub mod lengths;
pub mod match_base;
pub(crate) mod repeated;
pub mod transform;

use crate::ast::Operator;
use crate::compiler::{CompiledRuleChange, CompiledSoundChangeRule};
use ipa::sequence::{Phoneme, PhonemeSequence, ProsodyMarker, SequenceElement};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingWord {
    pub phonemes: Vec<Phoneme>,
    pub syllable_boundaries: BTreeSet<usize>,
    pub stress_index: Option<usize>,
    pub tags: Vec<Option<usize>>,
}

impl WorkingWord {
    #[must_use]
    pub fn from_ipa_word(word: &language::syllable::IpaWord) -> Self {
        let mut phonemes = Vec::new();
        let mut syllable_boundaries = BTreeSet::new();
        let mut stress_index = None;

        for (i, syl) in word.syllables.iter().enumerate() {
            if i > 0 {
                syllable_boundaries.insert(phonemes.len());
            }
            let start_idx = phonemes.len();
            let syl_elems = syl.structure.elements();
            let mut local_idx = 0;
            for el in &syl_elems {
                if let SequenceElement::Phoneme(p) = el {
                    phonemes.push(p.clone());
                    if stress_index.is_none()
                        && matches!(
                            syl.stress,
                            language::syllable::SyllableStress::PrimaryStress
                        )
                        && language::phonology::is_vowel(p)
                    {
                        stress_index = Some(start_idx + local_idx);
                    }
                    local_idx += 1;
                }
            }
        }

        let tags = vec![None; phonemes.len()];
        Self {
            phonemes,
            syllable_boundaries,
            stress_index,
            tags,
        }
    }

    #[must_use]
    pub fn to_flat_sequence(&self) -> PhonemeSequence {
        let mut elements = Vec::new();
        let stress_boundary_idx = self.stress_index.map(|val_idx| {
            self.syllable_boundaries
                .iter()
                .filter(|&&b| b <= val_idx)
                .max()
                .copied()
                .unwrap_or(0)
        });

        for (i, p) in self.phonemes.iter().enumerate() {
            if stress_boundary_idx == Some(i) {
                elements.push(SequenceElement::Prosody(ProsodyMarker::PrimaryStress));
            } else if self.syllable_boundaries.contains(&i) {
                elements.push(SequenceElement::SyllableBreak);
            }
            elements.push(SequenceElement::Phoneme(p.clone()));
        }
        PhonemeSequence { elements }
    }
}

#[derive(Clone, Debug)]
pub enum CapturedAlpha {
    Sign(bool),
    Strings(Vec<String>),
}

#[derive(Clone, Default, Debug)]
pub struct MatchState {
    pub alpha: HashMap<String, CapturedAlpha>,
    pub markers:
        HashMap<(Option<language::sound_class::SoundClassKey>, u8), std::ops::Range<usize>>,
    pub element_ranges: HashMap<usize, std::ops::Range<usize>>,
}

pub struct EvalContext<'a> {
    pub classes: &'a BTreeMap<language::sound_class::SoundClassKey, language::config::SoundClass>,
    pub system: &'a ipa::IpaSystem,
    pub active_tag: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StressUpdate {
    Set(usize),
    Clear,
    Keep,
}

pub struct TraceContext<'a> {
    pub verbose: bool,
    pub logs: &'a mut Vec<String>,
}

/// Internal helper to apply rules for a single era.
fn apply_era_rules(
    working: &mut WorkingWord,
    era: u32,
    rules: &[CompiledSoundChangeRule],
    ctx: &EvalContext<'_>,
    trace: &mut TraceContext<'_>,
) -> Result<(), String> {
    if trace.verbose {
        trace.logs.push(format!("--- Era {era} ---"));
    }
    for rule in rules {
        let before = working.clone();
        apply_rule(working, rule, ctx)?;
        if trace.verbose && *working != before {
            let rule_name = rule.name.as_deref().unwrap_or("<unnamed>");
            trace.logs.push(format!(
                "Rule: {rule_name}\n  In : {}\n  Out: {}",
                before.to_flat_sequence(),
                working.to_flat_sequence()
            ));
        }
    }
    Ok(())
}

/// Applies sound changes to a word.
///
/// # Errors
/// Returns an error if evaluation fails (e.g. invalid feature changes or failure to load the IPA system).
pub fn apply_sound_changes(
    word: &language::syllable::IpaWord,
    compiled_eras: &[(u32, Vec<CompiledSoundChangeRule>)],
    era_range: (u32, u32),
    config: &language::config::LanguageConfig,
    verbose: bool,
) -> Result<(language::syllable::IpaWord, Vec<String>), String> {
    let mut working = WorkingWord::from_ipa_word(word);
    let mut trace_logs = Vec::new();

    let ctx = EvalContext {
        classes: &config.phonology.sound_classes,
        system: ipa::DEFAULT_SYSTEM
            .as_ref()
            .map_err(|e| format!("Failed to load default IPA system: {e:?}"))?,
        active_tag: None,
    };

    // Filter and sort eras
    let (start_era, end_era) = era_range;
    let mut sorted_eras: Vec<_> = compiled_eras
        .iter()
        .filter(|(era, _)| *era >= start_era && *era <= end_era)
        .collect();
    sorted_eras.sort_by_key(|(era, _)| *era);

    let mut trace = TraceContext {
        verbose,
        logs: &mut trace_logs,
    };

    for (era, rules) in sorted_eras {
        apply_era_rules(&mut working, *era, rules, &ctx, &mut trace)?;
    }

    // Convert back to IpaWord by resyllabifying
    let flat_seq = working.to_flat_sequence();
    let resyllabified = language::syllable::IpaWord::try_from_sequence(&flat_seq, config)
        .map_err(|e| format!("Failed to resyllabify word: {e}"))?;

    Ok((resyllabified, trace_logs))
}

/// Applies a compiled sound change rule to a working word.
///
/// # Errors
/// Returns an error if the rule cannot be evaluated or feature updates fail.
pub fn apply_rule(
    word: &mut WorkingWord,
    rule: &CompiledSoundChangeRule,
    ctx: &EvalContext<'_>,
) -> Result<(), String> {
    for change in &rule.changes {
        apply_rule_change(word, change, ctx)?;
    }
    Ok(())
}

fn apply_rule_change(
    word: &mut WorkingWord,
    change: &CompiledRuleChange,
    ctx: &EvalContext<'_>,
) -> Result<(), String> {
    let is_leftward = matches!(
        change.operator,
        Operator::LeftMultipleTransparent
            | Operator::LeftSingleTransparent
            | Operator::LeftMultipleOpaque
    );
    let is_opaque = matches!(
        change.operator,
        Operator::RightMultipleOpaque | Operator::LeftMultipleOpaque
    );
    let is_single = matches!(
        change.operator,
        Operator::RightSingleTransparent | Operator::LeftSingleTransparent
    );

    if is_opaque {
        apply_opaque_change(word, change, is_leftward, ctx)?;
    } else {
        apply_transparent_change(word, change, is_leftward, is_single, ctx)?;
    }
    Ok(())
}

fn apply_opaque_change(
    word: &mut WorkingWord,
    change: &CompiledRuleChange,
    is_leftward: bool,
    ctx: &EvalContext<'_>,
) -> Result<(), String> {
    let original_word = word.clone();
    let matches = engine::find_all_matches(
        &original_word,
        &change.match_part,
        change.condition.as_ref(),
        is_leftward,
        ctx,
    );

    // Apply matches in reverse order of their start index to keep indices valid during modification
    let mut sorted_matches = matches;
    sorted_matches.sort_by_key(|b| std::cmp::Reverse(b.0.start));

    for (range, state) in sorted_matches {
        transform::replace_range(word, range, &state, &change.transform_part, ctx)?;
    }
    Ok(())
}

fn apply_transparent_change(
    word: &mut WorkingWord,
    change: &CompiledRuleChange,
    is_leftward: bool,
    is_single: bool,
    ctx: &EvalContext<'_>,
) -> Result<(), String> {
    let params = engine::TransparentLoopParams {
        match_part: &change.match_part,
        condition: change.condition.as_ref(),
        is_leftward,
        is_single,
        ctx,
    };
    engine::evaluate_transparent_loop(word, &params, |word, range, state| {
        transform::replace_range(word, range, state, &change.transform_part, ctx)
    })
}
