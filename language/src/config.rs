use crate::sound_class::SoundClassKey;
use ipa::IpaString;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use time::OffsetDateTime;
use uuid::Uuid;

// qual:allow(srp) - Orchestrates deserialized language configurations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LanguageConfig {
    pub id: Uuid,
    pub name: NameConfig,
    pub metadata: MetadataConfig,
    pub phonology: PhonologyConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sound_changes: Option<SoundChanges>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orthography: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derivations: Option<Vec<Derivation>>,
}

impl LanguageConfig {
    /// Syllabifies an IPA string using this language configuration.
    ///
    /// # Errors
    /// Returns `Err` if parsing or syllabification fails.
    pub fn syllabify(
        &self,
        ipa_str: &ipa::IpaString,
    ) -> Result<crate::syllable::IpaWord, crate::syllable::SyllabificationError> {
        let system = ipa::DEFAULT_SYSTEM.as_ref().map_err(|e| {
            crate::syllable::SyllabificationError::IpaError(
                ipa::ipa_string::IpaStringError::InvalidSequence(format!(
                    "Failed to load default IPA system: {e}"
                )),
            )
        })?;
        let seq = ipa::sequence::PhonemeSequence::parse_with_system(ipa_str.as_str(), system)?;
        crate::syllable::IpaWord::try_from_sequence(&seq, self)
    }

    /// Validates the language configuration invariants.
    ///
    /// # Errors
    /// Returns `Err` if validation fails.
    pub fn validate(&self) -> Result<(), crate::generator::ValidationError> {
        crate::generator::validate_generator_keys(&self.phonology.phonotactics.generators)?;
        crate::generator::validate_sound_class_cycles(&self.phonology.sound_classes)?;
        let defined: std::collections::HashSet<_> =
            self.phonology.sound_classes.keys().cloned().collect();
        crate::generator::validate_pattern_sound_classes(
            &self.phonology.phonotactics.generators,
            &defined,
        )?;
        crate::generator::validate_generator_cycles(&self.phonology.phonotactics.generators)?;
        if let Some(prosody) = &self.phonology.prosody {
            prosody.validate()?;
        }
        if let Some(derivations) = &self.derivations {
            crate::generator::validation::validate_derivations(derivations)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NameConfig {
    pub endonym: IpaString,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exonym: Option<IpaString>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetadataConfig {
    #[serde(with = "time::serde::iso8601")]
    pub created_at: OffsetDateTime,
    #[serde(
        with = "time::serde::iso8601::option",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub updated_at: Option<OffsetDateTime>,
}

fn ensure_default_sound_classes<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<SoundClassKey, SoundClass>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut map = BTreeMap::<SoundClassKey, SoundClass>::deserialize(deserializer)?;

    let defaults = ["C", "D", "L", "V"];
    for default_key in defaults {
        // Parse the hardcoded default keys, which are known to be valid
        let key = default_key
            .parse::<SoundClassKey>()
            .map_err(serde::de::Error::custom)?;
        map.entry(key).or_insert_with(|| SoundClass {
            values: Vec::new(),
            generator: None,
        });
    }

    Ok(map)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhonologyConfig {
    #[serde(deserialize_with = "ensure_default_sound_classes")]
    pub sound_classes: BTreeMap<SoundClassKey, SoundClass>,
    #[serde(default)]
    pub phonotactics: PhonotacticsConfig,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub illegal_patterns: Vec<crate::matcher::SoundMatcherPattern>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prosody: Option<crate::prosody::ProsodicConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SoundClass {
    pub values: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generator: Option<GeneratorConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PhonotacticsConfig {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub generators: BTreeMap<String, crate::generator::WordGenerator>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZipfConfig {
    #[serde(deserialize_with = "deserialize_f64_or_str")]
    pub a: f64,
    #[serde(deserialize_with = "deserialize_f64_or_str")]
    pub b: f64,
}

impl Default for ZipfConfig {
    fn default() -> Self {
        Self { a: 1.0, b: 2.7 }
    }
}

fn default_zipf_config() -> ZipfConfig {
    ZipfConfig::default()
}

fn deserialize_f64_or_str<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct F64OrStrVisitor;

    impl Visitor<'_> for F64OrStrVisitor {
        type Value = f64;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a float or a string representing a float")
        }

        fn visit_f64<E>(self, val: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(val)
        }

        #[expect(
            clippy::cast_precision_loss,
            reason = "Casting deserialized integer value to f64"
        )]
        fn visit_i64<E>(self, val: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(val as f64)
        }

        #[expect(
            clippy::cast_precision_loss,
            reason = "Casting deserialized unsigned integer value to f64"
        )]
        fn visit_u64<E>(self, val: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(val as f64)
        }

        fn visit_str<E>(self, val: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            val.parse::<f64>().map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_any(F64OrStrVisitor)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum GeneratorConfig {
    #[serde(alias = "Zipf", alias = "zipf")]
    Zipf {
        #[serde(default = "default_zipf_config")]
        config: ZipfConfig,
    },
    #[serde(alias = "Equiprobable", alias = "equiprobable")]
    Equiprobable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SoundChanges {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preamble: Vec<PreambleItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub eras: Vec<EraRules>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PreambleItem {
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: PreambleType,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PreambleType {
    Full,
    Match,
    Transform,
    Condition,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EraRules {
    pub era: u32,
    pub rules: Vec<SoundChangeRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SoundChangeRule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub changes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Derivation {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub era: Option<u32>,
    pub transforms: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::WordPattern;
    use time::macros::datetime;
    use uuid::Uuid;

    fn get_test_config_json() -> &'static str {
        r#"{
            "id": "018f4a3e-6b9f-7a1a-9b1a-2b3c4d5e6f7a",
            "name": {
                "endonym": "p"
            },
            "metadata": {
                "created_at": "2024-05-04T00:12:00Z"
            },
            "phonology": {
                "sound_classes": {
                    "A": {
                        "values": ["p", "t", "k"],
                        "generator": {
                            "type": "zipf",
                            "config": {
                                "a": 1.0,
                                "b": 2.0
                            }
                        }
                    }
                },
                "phonotactics": {
                    "generators": {
                        "noun.masculine": {
                            "patterns": ["CV(C)", "CV(CV)"],
                            "type": "zipf",
                            "config": {
                                "a": "1.5",
                                "b": "1.0"
                            }
                        }
                    }
                }
            }
        }"#
    }

    fn assert_deserialized_phonology(phonology: &BTreeMap<SoundClassKey, SoundClass>) {
        let class_key_a = "A".parse::<SoundClassKey>().expect("parse A");
        let class_a = phonology.get(&class_key_a).expect("A should exist");
        assert_eq!(class_a.values, vec!["p", "t", "k"]);
        assert_eq!(
            class_a.generator,
            Some(GeneratorConfig::Zipf {
                config: ZipfConfig { a: 1.0, b: 2.0 }
            })
        );

        let class_key_c = "C".parse::<SoundClassKey>().expect("parse C");
        let class_key_d = "D".parse::<SoundClassKey>().expect("parse D");
        let class_key_l = "L".parse::<SoundClassKey>().expect("parse L");
        let class_key_v = "V".parse::<SoundClassKey>().expect("parse V");
        assert!(phonology.contains_key(&class_key_c));
        assert!(phonology.contains_key(&class_key_d));
        assert!(phonology.contains_key(&class_key_l));
        assert!(phonology.contains_key(&class_key_v));

        let class_c = phonology.get(&class_key_c).expect("C should exist");
        assert!(class_c.values.is_empty());
        assert!(class_c.generator.is_none());
    }

    fn assert_deserialized_generators(
        generators: &BTreeMap<String, crate::generator::WordGenerator>,
    ) {
        let noun_masculine = generators
            .get("noun.masculine")
            .expect("noun.masculine should exist");

        let pat1 = "CV(C)".parse::<WordPattern>().expect("parse pat1");
        let pat2 = "CV(CV)".parse::<WordPattern>().expect("parse pat2");
        assert_eq!(noun_masculine.patterns, vec![pat1, pat2]);
        assert_eq!(
            noun_masculine.generator,
            GeneratorConfig::Zipf {
                config: ZipfConfig { a: 1.5, b: 1.0 }
            }
        );
    }

    #[test]
    fn test_language_config_deserialization() {
        let json_str = get_test_config_json();
        let config: LanguageConfig = serde_json::from_str(json_str).expect("deserialize config");

        let expected_uuid =
            Uuid::parse_str("018f4a3e-6b9f-7a1a-9b1a-2b3c4d5e6f7a").expect("parse uuid");
        assert_eq!(config.id, expected_uuid);
        assert_eq!(config.name.endonym.as_str(), "p");
        assert!(config.name.exonym.is_none());
        assert_eq!(
            config.metadata.created_at,
            datetime!(2024-05-04 00:12:00 +00:00)
        );

        assert_deserialized_phonology(&config.phonology.sound_classes);
        assert_deserialized_generators(&config.phonology.phonotactics.generators);
    }

    #[test]
    fn test_generator_equiprobable() {
        let json_str = r#"{
            "values": ["a", "e", "i"],
            "generator": {
                "type": "Equiprobable"
            }
        }"#;

        let sound_class: SoundClass =
            serde_json::from_str(json_str).expect("deserialize sound class");
        assert_eq!(sound_class.generator, Some(GeneratorConfig::Equiprobable));
    }

    #[test]
    fn test_ensure_default_sound_classes() {
        let json_str = r#"{"A": {"values": ["a"]}}"#;
        let mut deserializer = serde_json::Deserializer::from_str(json_str);
        let res = ensure_default_sound_classes(&mut deserializer).expect("valid");
        assert!(res.contains_key(&"C".parse().expect("valid")));
        assert!(res.contains_key(&"A".parse().expect("valid")));
    }

    #[test]
    fn test_default_zipf_config() {
        let conf = default_zipf_config();
        assert!((conf.a - 1.0).abs() < f64::EPSILON);
        assert!((conf.b - 2.7).abs() < f64::EPSILON);
    }

    #[test]
    fn test_deserialize_f64_or_str() {
        let mut deserializer = serde_json::Deserializer::from_str("1.5");
        let val = deserialize_f64_or_str(&mut deserializer).expect("valid");
        assert!((val - 1.5).abs() < f64::EPSILON);
    }
}
