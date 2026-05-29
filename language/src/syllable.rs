use crate::config::LanguageConfig;
use ipa::IpaString;
use ipa::sequence::{IpaSequence, Phoneme, PhonemeSequence, ProsodyMarker, SequenceElement};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use thiserror::Error;

const DEFAULT_ZIPF_PARAM: f64 = 1.0;

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

impl SyllableStructure {
    /// Retrieve flat elements for the syllable structure.
    #[must_use]
    pub fn elements(&self) -> Vec<SequenceElement> {
        match self {
            Self::Standard {
                onset,
                nucleus,
                coda,
            } => {
                let mut elems = Vec::new();
                if let Some(onset_seq) = onset {
                    elems.extend(onset_seq.elements.iter().filter_map(|el| match el {
                        SequenceElement::Phoneme(p) => Some(SequenceElement::Phoneme(p.clone())),
                        _ => None,
                    }));
                }
                elems.extend(nucleus.elements.iter().filter_map(|el| match el {
                    SequenceElement::Phoneme(p) => Some(SequenceElement::Phoneme(p.clone())),
                    _ => None,
                }));
                if let Some(coda_seq) = coda {
                    elems.extend(coda_seq.elements.iter().filter_map(|el| match el {
                        SequenceElement::Phoneme(p) => Some(SequenceElement::Phoneme(p.clone())),
                        _ => None,
                    }));
                }
                elems
            }
            Self::Root(p) => vec![SequenceElement::Phoneme(p.clone())],
            Self::Arbitrary(seq) => seq
                .elements
                .iter()
                .filter_map(|el| match el {
                    SequenceElement::Phoneme(p) => Some(SequenceElement::Phoneme(p.clone())),
                    _ => None,
                })
                .collect(),
        }
    }
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
            elems.extend(syl.structure.elements());
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

impl IpaWord {
    /// Generates a flat `PhonemeSequence` from the word, retaining stress markers but omitting syllable breaks.
    #[must_use]
    pub fn to_flat_sequence(&self) -> PhonemeSequence {
        let mut elements = Vec::new();
        for syl in &self.syllables {
            match syl.stress {
                SyllableStress::PrimaryStress => {
                    elements.push(SequenceElement::Prosody(ProsodyMarker::PrimaryStress));
                }
                SyllableStress::SecondaryStress => {
                    elements.push(SequenceElement::Prosody(ProsodyMarker::SecondaryStress));
                }
                _ => {}
            }
            elements.extend(syl.structure.elements());
        }
        PhonemeSequence { elements }
    }

    /// Syllabifies a `PhonemeSequence` given the language configuration and applies prosody.
    ///
    /// # Errors
    /// Returns `Err` if the sequence has invalid syllable boundaries or adjacent prosody markers.
    pub fn try_from_sequence(
        seq: &PhonemeSequence,
        config: &LanguageConfig,
    ) -> Result<Self, SyllabificationError> {
        let initial_word = crate::syllabifier::syllabify_sequence(seq, config)?;
        let prosody = config.phonology.prosody.as_ref().unwrap_or(
            &crate::prosody::ProsodicConfig::NoFixedStress {
                config: crate::config::ZipfConfig {
                    a: DEFAULT_ZIPF_PARAM,
                    b: DEFAULT_ZIPF_PARAM,
                },
            },
        );
        let updated_word = prosody.apply_prosody(&initial_word, config);

        let propagated = initial_word.syllables.len() != updated_word.syllables.len()
            || initial_word
                .syllables
                .iter()
                .zip(&updated_word.syllables)
                .any(|(s1, s2)| s1.stress != s2.stress);

        if propagated {
            let flat_seq = updated_word.to_flat_sequence();
            crate::syllabifier::syllabify_sequence(&flat_seq, config)
        } else {
            Ok(updated_word)
        }
    }
}
