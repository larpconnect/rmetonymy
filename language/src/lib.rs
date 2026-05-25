pub mod config;
pub mod dictionary;
pub mod generator;
pub mod matcher;
pub mod phonology;
pub mod phonotactics;
pub mod prosody;
pub mod sound_class;
pub mod syllabifier;
pub mod syllable;

pub use dictionary::{Dictionary, DictionaryEntry, NewEntry};
pub use prosody::{
    AlternatingConfig, FootSize, MainStress, PatternedConfig, ProsodicConfig, StressLocation,
};
pub use syllable::{IpaWord, SyllabificationError, Syllable, SyllableStress, SyllableStructure};

pub fn load_language() {
    // Basic module for representing individual language structures
}
