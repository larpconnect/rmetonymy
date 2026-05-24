use crate::config::LanguageConfig;
use crate::syllable::{IpaWord, SyllabificationError, Syllable, SyllableStress};
use ipa::sequence::{Phoneme, PhonemeSequence, ProsodyMarker, SequenceElement};

struct PreSegment {
    phonemes: Vec<Phoneme>,
    stress: SyllableStress,
}

#[derive(Debug, Clone)]
struct NucleusRange {
    start: usize,
    end: usize,
}

/// Validate sequence bounds and adjacent elements.
///
/// # Errors
/// Returns `Err` if syllable breaks are at boundaries, double, or adjacent to prosody.
pub fn validate_sequence(seq: &PhonemeSequence) -> Result<(), SyllabificationError> {
    if seq.elements.is_empty() {
        return Ok(());
    }

    if let Some(SequenceElement::SyllableBreak) = seq.elements.first() {
        return Err(SyllabificationError::BoundarySyllableBreak);
    }
    if let Some(SequenceElement::SyllableBreak) = seq.elements.last() {
        return Err(SyllabificationError::BoundarySyllableBreak);
    }

    for window in seq.elements.windows(2) {
        if let [el1, el2] = window {
            match (el1, el2) {
                (SequenceElement::SyllableBreak, SequenceElement::SyllableBreak) => {
                    return Err(SyllabificationError::DoubleSyllableBreak);
                }
                (SequenceElement::SyllableBreak, SequenceElement::Prosody(_))
                | (SequenceElement::Prosody(_), SequenceElement::SyllableBreak) => {
                    return Err(SyllabificationError::ProsodicWithSyllableBreak);
                }
                (SequenceElement::Prosody(_), SequenceElement::Prosody(_)) => {
                    return Err(SyllabificationError::AdjacentProsody);
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn pre_segment(seq: &PhonemeSequence) -> Vec<PreSegment> {
    let mut segments = Vec::new();
    let mut current_phonemes = Vec::new();
    let mut current_stress = SyllableStress::UnknownStress;

    for el in &seq.elements {
        match el {
            SequenceElement::Phoneme(p) => {
                current_phonemes.push(p.clone());
            }
            SequenceElement::SyllableBreak => {
                segments.push(PreSegment {
                    phonemes: std::mem::take(&mut current_phonemes),
                    stress: current_stress,
                });
                current_stress = SyllableStress::UnknownStress;
            }
            SequenceElement::Prosody(marker) => {
                segments.push(PreSegment {
                    phonemes: std::mem::take(&mut current_phonemes),
                    stress: current_stress,
                });
                current_stress = match marker {
                    ProsodyMarker::PrimaryStress => SyllableStress::PrimaryStress,
                    ProsodyMarker::SecondaryStress => SyllableStress::SecondaryStress,
                };
            }
        }
    }

    segments.push(PreSegment {
        phonemes: current_phonemes,
        stress: current_stress,
    });

    segments.retain(|seg| !seg.phonemes.is_empty());
    segments
}

fn find_nuclei(phonemes: &[Phoneme]) -> Vec<NucleusRange> {
    let mut nuclei = Vec::new();
    let mut idx = 0;
    while idx < phonemes.len() {
        let Some(p_curr) = phonemes.get(idx) else {
            break;
        };
        if crate::phonology::is_vowel(p_curr) {
            if let Some(p_next) = phonemes.get(idx + 1)
                && crate::phonology::are_diphthong(p_curr, p_next)
            {
                nuclei.push(NucleusRange {
                    start: idx,
                    end: idx + 2,
                });
                idx += 2;
            } else {
                nuclei.push(NucleusRange {
                    start: idx,
                    end: idx + 1,
                });
                idx += 1;
            }
        } else {
            idx += 1;
        }
    }
    nuclei
}

fn extract_phoneme_sequence(
    phonemes: &[Phoneme],
    start: usize,
    end: usize,
) -> Option<PhonemeSequence> {
    if start < end {
        let elements = phonemes
            .get(start..end)
            .unwrap_or(&[])
            .iter()
            .cloned()
            .map(SequenceElement::Phoneme)
            .collect();
        Some(PhonemeSequence { elements })
    } else {
        None
    }
}

fn parse_syllable(
    phonemes: &[Phoneme],
    stress: SyllableStress,
    nucleus_range: &NucleusRange,
) -> Syllable {
    let onset = extract_phoneme_sequence(phonemes, 0, nucleus_range.start);
    let nucleus = extract_phoneme_sequence(phonemes, nucleus_range.start, nucleus_range.end)
        .unwrap_or_else(|| PhonemeSequence {
            elements: Vec::new(),
        });
    let coda = extract_phoneme_sequence(phonemes, nucleus_range.end, phonemes.len());

    Syllable::standard(onset, nucleus, coda, stress)
}

fn is_valid_onset(onset: &[Phoneme], is_word_initial: bool, config: &LanguageConfig) -> bool {
    for (idx, window) in onset.windows(2).enumerate() {
        if let [o_curr, o_next] = window {
            let is_sibilant_start =
                idx == 0 && crate::phonology::has_feature(o_curr, data::feature::Feature::Strident);
            if !is_sibilant_start
                && crate::phonology::get_sonority(o_curr) >= crate::phonology::get_sonority(o_next)
            {
                return false;
            }
        }
    }

    !crate::phonology::is_illegal_onset(onset, is_word_initial, config)
}

fn find_optimal_split_point(
    consonants: &[Phoneme],
    min_s: usize,
    is_word_initial: bool,
    config: &LanguageConfig,
) -> usize {
    let m = consonants.len();
    let mut final_s = m;

    for s in min_s..=m {
        let proposed_onset = consonants.get(s..).unwrap_or(&[]);
        if is_valid_onset(proposed_onset, is_word_initial, config) {
            final_s = s;
            break;
        }
    }

    final_s
}

fn check_liquid_constraint(consonants: &[Phoneme]) -> usize {
    if let Some(first_c) = consonants.first()
        && crate::phonology::has_feature(first_c, data::feature::Feature::Liquid)
    {
        1
    } else {
        0
    }
}

fn check_stressed_capture_constraint(
    seg: &PreSegment,
    n_curr: &NucleusRange,
    consonants: &[Phoneme],
    vowel_idx: usize,
) -> usize {
    let is_vi_stressed = vowel_idx == 0
        && matches!(
            seg.stress,
            SyllableStress::PrimaryStress | SyllableStress::SecondaryStress
        );
    let vi_phonemes = seg.phonemes.get(n_curr.start..n_curr.end).unwrap_or(&[]);
    let is_vi_single_short_vowel =
        matches!(vi_phonemes, [v] if crate::phonology::can_vowel_capture(v));

    if is_vi_stressed
        && is_vi_single_short_vowel
        && let Some(first_c) = consonants.first()
        && !crate::phonology::has_feature(first_c, data::feature::Feature::Liquid)
        && !crate::phonology::has_feature(first_c, data::feature::Feature::Glide)
    {
        return 1;
    }
    0
}

fn find_geminated_consonant_split(consonants: &[Phoneme]) -> Option<usize> {
    for (idx, c) in consonants.iter().enumerate() {
        if crate::phonology::has_feature(c, data::feature::Feature::Long) {
            return Some(idx + 1);
        }
    }
    None
}

fn calculate_single_split_point(
    seg: &PreSegment,
    config: &LanguageConfig,
    n_curr: &NucleusRange,
    n_next: &NucleusRange,
    vowel_idx: usize,
) -> usize {
    let start_c = n_curr.end;
    let end_c = n_next.start;
    let consonants = seg.phonemes.get(start_c..end_c).unwrap_or(&[]);

    if consonants.is_empty() {
        return 0;
    }

    if let Some(g_s) = find_geminated_consonant_split(consonants) {
        return g_s;
    }

    let min_s = check_liquid_constraint(consonants).max(check_stressed_capture_constraint(
        seg, n_curr, consonants, vowel_idx,
    ));

    find_optimal_split_point(consonants, min_s, vowel_idx == 0, config)
}

fn find_split_points(
    seg: &PreSegment,
    config: &LanguageConfig,
    nuclei: &[NucleusRange],
) -> Vec<usize> {
    let mut split_points = Vec::new();
    for i in 0..nuclei.len().saturating_sub(1) {
        let Some(n_curr) = nuclei.get(i) else {
            continue;
        };
        let Some(n_next) = nuclei.get(i + 1) else {
            continue;
        };
        let split_idx = calculate_single_split_point(seg, config, n_curr, n_next, i);
        split_points.push(split_idx);
    }
    split_points
}

fn reconstruct_syllables(
    seg: &PreSegment,
    nuclei: &[NucleusRange],
    split_points: &[usize],
) -> Vec<Syllable> {
    let mut syllables = Vec::new();
    for (i, n_curr) in nuclei.iter().enumerate() {
        let onset_start = if i == 0 {
            0
        } else {
            let prev_end = nuclei.get(i - 1).map_or(0, |n| n.end);
            let prev_split = split_points.get(i - 1).copied().unwrap_or(0);
            prev_end + prev_split
        };

        let onset = extract_phoneme_sequence(&seg.phonemes, onset_start, n_curr.start);
        let nucleus = extract_phoneme_sequence(&seg.phonemes, n_curr.start, n_curr.end)
            .unwrap_or_else(|| PhonemeSequence {
                elements: Vec::new(),
            });

        let coda_end = if i == nuclei.len() - 1 {
            seg.phonemes.len()
        } else {
            let curr_split = split_points.get(i).copied().unwrap_or(0);
            n_curr.end + curr_split
        };

        let coda = extract_phoneme_sequence(&seg.phonemes, n_curr.end, coda_end);
        let stress = if i == 0 {
            seg.stress
        } else {
            SyllableStress::UnknownStress
        };

        syllables.push(Syllable::standard(onset, nucleus, coda, stress));
    }
    syllables
}

fn syllabify_segment_no_nuclei(seg: &PreSegment, has_any_vowels_in_word: bool) -> Vec<Syllable> {
    if has_any_vowels_in_word {
        return vec![Syllable::arbitrary(
            PhonemeSequence {
                elements: seg
                    .phonemes
                    .iter()
                    .cloned()
                    .map(SequenceElement::Phoneme)
                    .collect(),
            },
            seg.stress,
        )];
    }
    seg.phonemes
        .iter()
        .cloned()
        .map(|p| Syllable::root(p, seg.stress))
        .collect()
}

fn syllabify_segment_multiple_nuclei(
    seg: &PreSegment,
    config: &LanguageConfig,
    nuclei: &[NucleusRange],
) -> Vec<Syllable> {
    let split_points = find_split_points(seg, config, nuclei);
    reconstruct_syllables(seg, nuclei, &split_points)
}

fn syllabify_segment(
    seg: &PreSegment,
    config: &LanguageConfig,
    has_any_vowels_in_word: bool,
) -> Vec<Syllable> {
    let nuclei = find_nuclei(&seg.phonemes);

    if nuclei.is_empty() {
        return syllabify_segment_no_nuclei(seg, has_any_vowels_in_word);
    }

    if nuclei.len() == 1
        && let Some(n0) = nuclei.first()
    {
        return vec![parse_syllable(&seg.phonemes, seg.stress, n0)];
    }

    syllabify_segment_multiple_nuclei(seg, config, &nuclei)
}

/// Main entry point for the syllabification of a `PhonemeSequence`.
///
/// # Errors
/// Returns `Err` if the sequence has invalid syllable boundaries or adjacent prosody markers.
pub fn syllabify_sequence(
    seq: &PhonemeSequence,
    config: &LanguageConfig,
) -> Result<IpaWord, SyllabificationError> {
    validate_sequence(seq)?;

    if seq.elements.is_empty() {
        return Ok(IpaWord {
            syllables: Vec::new(),
        });
    }

    let has_any_vowels = seq
        .elements
        .iter()
        .any(|el| matches!(el, SequenceElement::Phoneme(p) if crate::phonology::is_vowel(p)));

    let segments = pre_segment(seq);
    let mut syllables = Vec::new();

    for seg in &segments {
        let mut seg_syllables = syllabify_segment(seg, config, has_any_vowels);
        syllables.append(&mut seg_syllables);
    }

    Ok(IpaWord { syllables })
}
