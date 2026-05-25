//! Prosody module for managing stress configurations and applying them to words.

use crate::config::{LanguageConfig, ZipfConfig};
use crate::generator::rng::{Rng, RngExt};
use crate::generator::validation::ValidationError;
use crate::syllable::{IpaWord, Syllable, SyllableStress, SyllableStructure};
use serde::{Deserialize, Serialize};

/// Options for alternating stress placement.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AlternatingConfig {
    FirstSyllable,
    SecondSyllable,
    Antepenultimate,
    Penultimate,
    Ultimate,
}

/// Foot size variants (either 2 or 3).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FootSize {
    #[serde(rename = "2", alias = "two")]
    Two = 2,
    #[serde(rename = "3", alias = "three")]
    Three = 3,
}

/// Stress location inside a foot.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum StressLocation {
    #[serde(rename = "1st", alias = "first")]
    First,
    #[serde(rename = "2nd", alias = "second")]
    Second,
    #[serde(rename = "3rd", alias = "third")]
    Third,
}

/// Primary stress anchor (first or last foot).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MainStress {
    First,
    Last,
}

/// Patterned stress configuration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct PatternedConfig {
    pub foot: FootSize,
    pub stress_location: StressLocation,
    pub main_stress: MainStress,
}

/// Helper function to provide default parameters for No Fixed Stress Zipf configuration.
fn default_no_fixed_stress_zipf() -> ZipfConfig {
    ZipfConfig { a: 1.0, b: 1.0 }
}

/// Prosodic stress system configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ProsodicConfig {
    NoFixedStress {
        #[serde(default = "default_no_fixed_stress_zipf")]
        config: ZipfConfig,
    },
    Unstressed,
    Alternating {
        option: AlternatingConfig,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stress_open_monosyllables: Option<bool>,
    },
    Patterned(PatternedConfig),
}

impl ProsodicConfig {
    /// Validates the prosodic configuration invariants.
    ///
    /// # Errors
    /// Returns `ValidationError` if the configuration is invalid.
    pub fn validate(&self) -> Result<(), ValidationError> {
        match self {
            Self::Patterned(pat) => {
                let limit = match pat.foot {
                    FootSize::Two => 2,
                    FootSize::Three => 3,
                };
                let loc_val = match pat.stress_location {
                    StressLocation::First => 1,
                    StressLocation::Second => 2,
                    StressLocation::Third => 3,
                };
                if loc_val > limit {
                    return Err(ValidationError::InvalidProsodyConfig(format!(
                        "Stress location {:?} is invalid for foot size {:?}",
                        pat.stress_location, pat.foot
                    )));
                }
            }
            Self::NoFixedStress { config } if config.a < 0.0 || config.b < 0.0 => {
                return Err(ValidationError::InvalidProsodyConfig(
                    "Zipf parameters a and b must be non-negative".to_string(),
                ));
            }
            _ => {}
        }
        Ok(())
    }

    /// Applies the stress configuration to a word using thread-local RNG.
    #[must_use]
    pub fn apply_prosody(&self, word: &IpaWord, config: &LanguageConfig) -> IpaWord {
        let mut rng = crate::generator::rng::thread_rng();
        self.apply_prosody_with_rng(word, config, &mut rng)
    }

    /// Applies the stress configuration using a given random number generator.
    #[must_use]
    pub fn apply_prosody_with_rng<R: Rng + ?Sized>(
        &self,
        word: &IpaWord,
        _config: &LanguageConfig,
        rng: &mut R,
    ) -> IpaWord {
        let mut syllables = word.syllables.clone();
        let num_syllables = syllables.len();
        if num_syllables == 0 {
            return word.clone();
        }

        match self {
            Self::Unstressed => {
                // If unstressed, do not add any stress marks at all.
                // Existing stress is kept (Application Rule 2).
                word.clone()
            }
            Self::NoFixedStress { config: zipf } => {
                if let Some(p_idx) = find_stress_anchor(word) {
                    apply_alternating_secondary_stresses(&mut syllables, p_idx);
                    IpaWord::new(syllables)
                } else {
                    let p_idx = sample_zipf_index(num_syllables, zipf.a, zipf.b, rng);
                    apply_alternating_secondary_stresses(&mut syllables, p_idx);
                    IpaWord::new(syllables)
                }
            }
            Self::Alternating {
                option,
                stress_open_monosyllables,
            } => {
                apply_alternating(
                    word,
                    *option,
                    *stress_open_monosyllables,
                    &mut syllables,
                    num_syllables,
                );
                IpaWord::new(syllables)
            }
            Self::Patterned(pat) => {
                apply_patterned(word, *pat, &mut syllables, num_syllables);
                IpaWord::new(syllables)
            }
        }
    }
}

fn apply_alternating_monosyllable(
    syllables: &mut [Syllable],
    stress_open_monosyllables: Option<bool>,
) {
    let stress_open = stress_open_monosyllables.unwrap_or(true);
    if let Some(first) = syllables
        .first_mut()
        .filter(|s| is_closed_syllable(s) || stress_open)
    {
        first.stress = SyllableStress::PrimaryStress;
    }
}

fn get_alternating_target_index(option: AlternatingConfig, num_syllables: usize) -> usize {
    let idx = match option {
        AlternatingConfig::FirstSyllable => 0,
        AlternatingConfig::SecondSyllable => 1,
        AlternatingConfig::Antepenultimate => num_syllables.saturating_sub(3),
        AlternatingConfig::Penultimate => num_syllables.saturating_sub(2),
        AlternatingConfig::Ultimate => num_syllables.saturating_sub(1),
    };
    idx.clamp(0, num_syllables - 1)
}

fn apply_alternating(
    word: &IpaWord,
    option: AlternatingConfig,
    stress_open_monosyllables: Option<bool>,
    syllables: &mut [Syllable],
    num_syllables: usize,
) {
    if let Some(p_idx) = find_stress_anchor(word) {
        apply_alternating_secondary_stresses(syllables, p_idx);
    } else if num_syllables == 1 {
        apply_alternating_monosyllable(syllables, stress_open_monosyllables);
    } else {
        let target_idx = get_alternating_target_index(option, num_syllables);
        apply_alternating_secondary_stresses(syllables, target_idx);
    }
}

fn apply_patterned_first(
    syllables: &mut [Syllable],
    num_complete_feet: usize,
    foot_size: usize,
    stress_loc: usize,
) {
    for i in 0..num_complete_feet {
        let start = i * foot_size;
        let stress_idx = start + stress_loc;
        let syl_opt = if stress_idx < start + foot_size {
            syllables.get_mut(stress_idx)
        } else {
            None
        };
        if let Some(syl) = syl_opt {
            if i == 0 {
                syl.stress = SyllableStress::PrimaryStress;
            } else {
                syl.stress = SyllableStress::SecondaryStress;
            }
        }
    }
}

fn apply_patterned_last(
    syllables: &mut [Syllable],
    num_complete_feet: usize,
    foot_size: usize,
    stress_loc: usize,
    remainder: usize,
) {
    for i in 0..num_complete_feet {
        let start = remainder + i * foot_size;
        let stress_idx = start + stress_loc;
        let syl_opt = if stress_idx < start + foot_size {
            syllables.get_mut(stress_idx)
        } else {
            None
        };
        if let Some(syl) = syl_opt {
            if i == num_complete_feet - 1 {
                syl.stress = SyllableStress::PrimaryStress;
            } else {
                syl.stress = SyllableStress::SecondaryStress;
            }
        }
    }
}

fn apply_patterned_short_word(
    syllables: &mut [Syllable],
    num_syllables: usize,
    stress_loc: usize,
    anchor_opt: Option<usize>,
) {
    if let Some(syl) = anchor_opt.and_then(|idx| syllables.get_mut(idx)) {
        syl.stress = SyllableStress::PrimaryStress;
        return;
    }
    if num_syllables == 1 {
        if let Some(first) = syllables.first_mut().filter(|s| is_closed_syllable(s)) {
            first.stress = SyllableStress::PrimaryStress;
        }
        return;
    }
    let stress_idx = stress_loc.clamp(0, num_syllables - 1);
    if let Some(syl) = syllables.get_mut(stress_idx) {
        syl.stress = SyllableStress::PrimaryStress;
    }
}

fn apply_patterned_main_stress_first_anchored(
    syllables: &mut [Syllable],
    num_complete_feet: usize,
    foot_size: usize,
    stress_loc: usize,
    p_idx: usize,
) {
    let primary_foot_idx = p_idx / foot_size;
    for i in 0..num_complete_feet {
        if i == primary_foot_idx {
            if let Some(syl) = syllables.get_mut(p_idx) {
                syl.stress = SyllableStress::PrimaryStress;
            }
        } else {
            let stress_idx = i * foot_size + stress_loc;
            if let Some(syl) = syllables.get_mut(stress_idx) {
                syl.stress = SyllableStress::SecondaryStress;
            }
        }
    }
    let syl_opt = if primary_foot_idx >= num_complete_feet {
        syllables.get_mut(p_idx)
    } else {
        None
    };
    if let Some(syl) = syl_opt {
        syl.stress = SyllableStress::PrimaryStress;
    }
}

fn apply_patterned_main_stress_last_remainder_anchored(
    syllables: &mut [Syllable],
    num_complete_feet: usize,
    foot_size: usize,
    stress_loc: usize,
    remainder: usize,
    p_idx: usize,
) {
    if let Some(syl) = syllables.get_mut(p_idx) {
        syl.stress = SyllableStress::PrimaryStress;
    }
    for i in 0..num_complete_feet {
        let stress_idx = remainder + i * foot_size + stress_loc;
        if let Some(syl) = syllables.get_mut(stress_idx) {
            syl.stress = SyllableStress::SecondaryStress;
        }
    }
}

fn apply_patterned_main_stress_last_foot_anchored(
    syllables: &mut [Syllable],
    num_complete_feet: usize,
    foot_size: usize,
    stress_loc: usize,
    remainder: usize,
    p_idx: usize,
) {
    let primary_foot_idx = (p_idx - remainder) / foot_size;
    for i in 0..num_complete_feet {
        if i == primary_foot_idx {
            if let Some(syl) = syllables.get_mut(p_idx) {
                syl.stress = SyllableStress::PrimaryStress;
            }
        } else {
            let stress_idx = remainder + i * foot_size + stress_loc;
            if let Some(syl) = syllables.get_mut(stress_idx) {
                syl.stress = SyllableStress::SecondaryStress;
            }
        }
    }
}

fn apply_patterned_main_stress_last_anchored(
    syllables: &mut [Syllable],
    num_complete_feet: usize,
    foot_size: usize,
    stress_loc: usize,
    remainder: usize,
    p_idx: usize,
) {
    if p_idx < remainder {
        apply_patterned_main_stress_last_remainder_anchored(
            syllables,
            num_complete_feet,
            foot_size,
            stress_loc,
            remainder,
            p_idx,
        );
    } else {
        apply_patterned_main_stress_last_foot_anchored(
            syllables,
            num_complete_feet,
            foot_size,
            stress_loc,
            remainder,
            p_idx,
        );
    }
}

fn apply_patterned(
    word: &IpaWord,
    pat: PatternedConfig,
    syllables: &mut [Syllable],
    num_syllables: usize,
) {
    let foot_size = pat.foot as usize;
    let stress_loc = match pat.stress_location {
        StressLocation::First => 0,
        StressLocation::Second => 1,
        StressLocation::Third => 2,
    };

    for syl in syllables.iter_mut() {
        syl.stress = SyllableStress::Unstressed;
    }

    let anchor_opt = find_stress_anchor(word);

    if num_syllables < foot_size {
        apply_patterned_short_word(syllables, num_syllables, stress_loc, anchor_opt);
        return;
    }

    let num_complete_feet = num_syllables / foot_size;
    match pat.main_stress {
        MainStress::First => {
            if let Some(p_idx) = anchor_opt {
                apply_patterned_main_stress_first_anchored(
                    syllables,
                    num_complete_feet,
                    foot_size,
                    stress_loc,
                    p_idx,
                );
            } else {
                apply_patterned_first(syllables, num_complete_feet, foot_size, stress_loc);
            }
        }
        MainStress::Last => {
            let remainder = num_syllables % foot_size;
            if let Some(p_idx) = anchor_opt {
                apply_patterned_main_stress_last_anchored(
                    syllables,
                    num_complete_feet,
                    foot_size,
                    stress_loc,
                    remainder,
                    p_idx,
                );
            } else {
                apply_patterned_last(
                    syllables,
                    num_complete_feet,
                    foot_size,
                    stress_loc,
                    remainder,
                );
            }
        }
    }
}

fn leftmost_primary_stress_index(word: &IpaWord) -> Option<usize> {
    word.syllables
        .iter()
        .position(|syl| matches!(syl.stress, SyllableStress::PrimaryStress))
}

fn leftmost_secondary_stress_index(word: &IpaWord) -> Option<usize> {
    word.syllables
        .iter()
        .position(|syl| matches!(syl.stress, SyllableStress::SecondaryStress))
}

fn find_stress_anchor(word: &IpaWord) -> Option<usize> {
    leftmost_primary_stress_index(word).or_else(|| leftmost_secondary_stress_index(word))
}

fn apply_alternating_secondary_stresses(syllables: &mut [Syllable], primary_idx: usize) {
    for (i, syl) in syllables.iter_mut().enumerate() {
        let diff = i.abs_diff(primary_idx);

        if i == primary_idx {
            syl.stress = SyllableStress::PrimaryStress;
        } else if diff % 2 == 0 {
            syl.stress = SyllableStress::SecondaryStress;
        } else {
            syl.stress = SyllableStress::Unstressed;
        }
    }
}

fn is_closed_syllable(syl: &Syllable) -> bool {
    match &syl.structure {
        SyllableStructure::Standard { coda, .. } => {
            if let Some(coda_seq) = coda {
                !coda_seq.elements.is_empty()
            } else {
                false
            }
        }
        SyllableStructure::Root(_) => false,
        SyllableStructure::Arbitrary(seq) => {
            if let Some(ipa::sequence::SequenceElement::Phoneme(p)) = seq.elements.last() {
                !crate::phonology::is_vowel(p)
            } else {
                false
            }
        }
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "RNG choices scaled to length of syllables"
)]
fn sample_zipf_index<R: Rng + ?Sized>(num_choices: usize, a: f64, b: f64, rng: &mut R) -> usize {
    if num_choices <= 1 {
        return 0;
    }
    let mut sum = 0.0;
    for i in 1..=num_choices {
        sum += 1.0 / (i as f64 + b).powf(a);
    }

    let r = rng.random::<f64>() * sum;
    let mut accum = 0.0;
    for i in 1..=num_choices {
        let w = 1.0 / (i as f64 + b).powf(a);
        accum += w;
        if accum >= r {
            return i - 1;
        }
    }
    num_choices - 1
}
