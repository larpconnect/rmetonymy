use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Feature {
    Syllabic,
    Consonantal,
    Approximant,
    Sonorant,
    Continuant,
    DelayedRelease,
    Trill,
    Tap,
    Lateral,
    Nasal,
    Voice,
    SpreadGlottis,
    ConstrictedGlottis,
    Labial,
    Round,
    Labiodental,
    Coronal,
    Anterior,
    Distributed,
    Strident,
    Dorsal,
    High,
    Low,
    Front,
    Back,
    Tense,
    Pharyngeal,
    Radical,
    Aspirated,
    Bilabial,
    Stop,
    Nasalized,
    // Add any others found in tests
}
