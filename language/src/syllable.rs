use crate::config::LanguageConfig;
use ipa::IpaString;
use ipa::sequence::{IpaSequence, Phoneme, PhonemeSequence, ProsodyMarker, SequenceElement};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use thiserror::Error;

/// Stress variants for a syllable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SyllableStress {
    PrimaryStress,
    SecondaryStress,
    Unstressed,
    UnknownStress,
}

/// The internal structure of a Syllable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyllableStructure {
    Standard {
        onset: Option<PhonemeSequence>,
        nucleus: PhonemeSequence,
        coda: Option<PhonemeSequence>,
    },
    Root(Phoneme),
    Arbitrary(PhonemeSequence),
}

/// A syllable containing structure and stress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Syllable {
    pub structure: SyllableStructure,
    pub stress: SyllableStress,
}

impl Syllable {
    /// Creates a standard syllable structure.
    #[must_use]
    pub fn standard(
        onset: Option<PhonemeSequence>,
        nucleus: PhonemeSequence,
        coda: Option<PhonemeSequence>,
        stress: SyllableStress,
    ) -> Self {
        Self {
            structure: SyllableStructure::Standard {
                onset,
                nucleus,
                coda,
            },
            stress,
        }
    }

    /// Creates a root syllable structure.
    #[must_use]
    pub fn root(root: Phoneme, stress: SyllableStress) -> Self {
        Self {
            structure: SyllableStructure::Root(root),
            stress,
        }
    }

    /// Creates an arbitrary syllable structure.
    #[must_use]
    pub fn arbitrary(seq: PhonemeSequence, stress: SyllableStress) -> Self {
        Self {
            structure: SyllableStructure::Arbitrary(seq),
            stress,
        }
    }
}

/// A word represented as a sequence of syllables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpaWord {
    pub syllables: Vec<Syllable>,
}

impl IpaWord {
    /// Creates a new `IpaWord` from a sequence of syllables.
    #[must_use]
    pub fn new(syllables: Vec<Syllable>) -> Self {
        Self { syllables }
    }
}

#[derive(Debug, Error)]
pub enum SyllabificationError {
    #[error("Syllable break at start or end is not allowed")]
    BoundarySyllableBreak,
    #[error("Double syllable breaks are not allowed")]
    DoubleSyllableBreak,
    #[error("Prosodic marker combined with syllable break is not allowed")]
    ProsodicWithSyllableBreak,
    #[error("Adjacent prosodic markers are not allowed")]
    AdjacentProsody,
    #[error("IPA parsing failed: {0}")]
    IpaError(#[from] ipa::ipa_string::IpaStringError),
}

impl IpaSequence for IpaWord {
    fn elements(&self) -> Vec<SequenceElement> {
        let mut elems = Vec::new();
        for (i, syl) in self.syllables.iter().enumerate() {
            if i > 0
                && matches!(
                    syl.stress,
                    SyllableStress::UnknownStress | SyllableStress::Unstressed
                )
            {
                elems.push(SequenceElement::SyllableBreak);
            }
            match syl.stress {
                SyllableStress::PrimaryStress => {
                    elems.push(SequenceElement::Prosody(ProsodyMarker::PrimaryStress));
                }
                SyllableStress::SecondaryStress => {
                    elems.push(SequenceElement::Prosody(ProsodyMarker::SecondaryStress));
                }
                _ => {}
            }

            match &syl.structure {
                SyllableStructure::Standard {
                    onset,
                    nucleus,
                    coda,
                } => {
                    if let Some(onset_seq) = onset {
                        for el in &onset_seq.elements {
                            if let SequenceElement::Phoneme(p) = el {
                                elems.push(SequenceElement::Phoneme(p.clone()));
                            }
                        }
                    }
                    for el in &nucleus.elements {
                        if let SequenceElement::Phoneme(p) = el {
                            elems.push(SequenceElement::Phoneme(p.clone()));
                        }
                    }
                    if let Some(coda_seq) = coda {
                        for el in &coda_seq.elements {
                            if let SequenceElement::Phoneme(p) = el {
                                elems.push(SequenceElement::Phoneme(p.clone()));
                            }
                        }
                    }
                }
                SyllableStructure::Root(p) => {
                    elems.push(SequenceElement::Phoneme(p.clone()));
                }
                SyllableStructure::Arbitrary(seq) => {
                    for el in &seq.elements {
                        if let SequenceElement::Phoneme(p) = el {
                            elems.push(SequenceElement::Phoneme(p.clone()));
                        }
                    }
                }
            }
        }
        elems
    }
}

impl Display for IpaWord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for el in self.elements() {
            write!(f, "{el}")?;
        }
        Ok(())
    }
}

impl From<IpaWord> for PhonemeSequence {
    fn from(word: IpaWord) -> Self {
        PhonemeSequence {
            elements: word.elements(),
        }
    }
}

impl From<IpaWord> for IpaString {
    fn from(word: IpaWord) -> Self {
        let seq: PhonemeSequence = word.into();
        IpaString::from(seq)
    }
}

// --- Pre-segmenter ---

struct PreSegment {
    phonemes: Vec<Phoneme>,
    stress: SyllableStress,
}

fn validate_sequence(seq: &PhonemeSequence) -> Result<(), SyllabificationError> {
    if seq.elements.is_empty() {
        return Ok(());
    }

    if let Some(SequenceElement::SyllableBreak) = seq.elements.first() {
        return Err(SyllabificationError::BoundarySyllableBreak);
    }
    if let Some(SequenceElement::SyllableBreak) = seq.elements.last() {
        return Err(SyllabificationError::BoundarySyllableBreak);
    }

    for i in 0..seq.elements.len().saturating_sub(1) {
        let Some(el1) = seq.elements.get(i) else {
            continue;
        };
        let Some(el2) = seq.elements.get(i + 1) else {
            continue;
        };

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

// --- Nucleus Finder ---

#[derive(Debug, Clone)]
struct NucleusRange {
    start: usize,
    end: usize,
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

// --- Helper Parser ---

fn parse_syllable(
    phonemes: &[Phoneme],
    stress: SyllableStress,
    nucleus_range: &NucleusRange,
) -> Syllable {
    let onset = if nucleus_range.start > 0 {
        let elements = phonemes
            .get(..nucleus_range.start)
            .unwrap_or(&[])
            .iter()
            .cloned()
            .map(SequenceElement::Phoneme)
            .collect();
        Some(PhonemeSequence { elements })
    } else {
        None
    };

    let nucleus = PhonemeSequence {
        elements: phonemes
            .get(nucleus_range.start..nucleus_range.end)
            .unwrap_or(&[])
            .iter()
            .cloned()
            .map(SequenceElement::Phoneme)
            .collect(),
    };

    let coda = if nucleus_range.end < phonemes.len() {
        let elements = phonemes
            .get(nucleus_range.end..)
            .unwrap_or(&[])
            .iter()
            .cloned()
            .map(SequenceElement::Phoneme)
            .collect();
        Some(PhonemeSequence { elements })
    } else {
        None
    };

    Syllable::standard(onset, nucleus, coda, stress)
}

// --- Split Point Logic ---

fn is_valid_onset(onset: &[Phoneme], is_word_initial: bool, config: &LanguageConfig) -> bool {
    let k = onset.len();
    for a in 0..k.saturating_sub(1) {
        let Some(o_curr) = onset.get(a) else {
            continue;
        };
        let Some(o_next) = onset.get(a + 1) else {
            continue;
        };
        let is_sibilant_start =
            a == 0 && crate::phonology::has_feature(o_curr, data::feature::Feature::Strident);
        if !is_sibilant_start
            && crate::phonology::get_sonority(o_curr) >= crate::phonology::get_sonority(o_next)
        {
            return false;
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
    let m = consonants.len();

    if m == 0 {
        return 0;
    }

    let mut min_s = 0;

    if let Some(first_c) = consonants.first()
        && crate::phonology::has_feature(first_c, data::feature::Feature::Liquid)
    {
        min_s = 1;
    }

    let is_vi_stressed = vowel_idx == 0
        && matches!(
            seg.stress,
            SyllableStress::PrimaryStress | SyllableStress::SecondaryStress
        );
    let vi_phonemes = seg.phonemes.get(n_curr.start..n_curr.end).unwrap_or(&[]);
    let is_vi_single_short_vowel = vi_phonemes.len() == 1
        && vi_phonemes
            .first()
            .is_some_and(crate::phonology::can_vowel_capture);

    if is_vi_stressed
        && is_vi_single_short_vowel
        && let Some(first_c) = consonants.first()
        && !crate::phonology::has_feature(first_c, data::feature::Feature::Liquid)
        && !crate::phonology::has_feature(first_c, data::feature::Feature::Glide)
    {
        min_s = min_s.max(1);
    }

    let mut geminated_s = None;
    for (idx, c) in consonants.iter().enumerate() {
        if crate::phonology::has_feature(c, data::feature::Feature::Long) {
            geminated_s = Some(idx + 1);
            break;
        }
    }

    if let Some(g_s) = geminated_s {
        g_s
    } else {
        find_optimal_split_point(consonants, min_s, vowel_idx == 0, config)
    }
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

// --- Reconstruct Syllables ---

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

        let onset = if onset_start < n_curr.start {
            let elements = seg
                .phonemes
                .get(onset_start..n_curr.start)
                .unwrap_or(&[])
                .iter()
                .cloned()
                .map(SequenceElement::Phoneme)
                .collect();
            Some(PhonemeSequence { elements })
        } else {
            None
        };

        let nucleus = PhonemeSequence {
            elements: seg
                .phonemes
                .get(n_curr.start..n_curr.end)
                .unwrap_or(&[])
                .iter()
                .cloned()
                .map(SequenceElement::Phoneme)
                .collect(),
        };

        let coda_end = if i == nuclei.len() - 1 {
            seg.phonemes.len()
        } else {
            let curr_split = split_points.get(i).copied().unwrap_or(0);
            n_curr.end + curr_split
        };

        let coda = if n_curr.end < coda_end {
            let elements = seg
                .phonemes
                .get(n_curr.end..coda_end)
                .unwrap_or(&[])
                .iter()
                .cloned()
                .map(SequenceElement::Phoneme)
                .collect();
            Some(PhonemeSequence { elements })
        } else {
            None
        };

        let stress = if i == 0 {
            seg.stress
        } else {
            SyllableStress::UnknownStress
        };

        syllables.push(Syllable::standard(onset, nucleus, coda, stress));
    }
    syllables
}

// --- Segment Syllabifiers ---

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

impl IpaWord {
    /// Syllabifies a `PhonemeSequence` given the language configuration.
    ///
    /// # Errors
    /// Returns `Err` if the sequence has invalid syllable boundaries or adjacent prosody markers.
    pub fn try_from_sequence(
        seq: &PhonemeSequence,
        config: &LanguageConfig,
    ) -> Result<Self, SyllabificationError> {
        validate_sequence(seq)?;

        if seq.elements.is_empty() {
            return Ok(Self {
                syllables: Vec::new(),
            });
        }

        let has_any_vowels = seq.elements.iter().any(|el| {
            if let SequenceElement::Phoneme(p) = el {
                crate::phonology::is_vowel(p)
            } else {
                false
            }
        });

        let segments = pre_segment(seq);
        let mut syllables = Vec::new();

        for seg in &segments {
            let mut seg_syllables = syllabify_segment(seg, config, has_any_vowels);
            syllables.append(&mut seg_syllables);
        }

        Ok(Self { syllables })
    }
}
