use crate::sound_class::SoundClassKey;
use ipa::IpaString;

use std::collections::BTreeMap;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LanguageConfig {
    pub id: Uuid,
    pub name: NameConfig,
    pub metadata: MetadataConfig,
    pub phonology: PhonologyConfig,
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

use serde::{Deserialize, Deserializer, Serialize};

fn ensure_default_sound_classes<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<SoundClassKey, SoundClass>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut map = BTreeMap::<SoundClassKey, SoundClass>::deserialize(deserializer)?;

    let defaults = ["C", "D", "L", "V"];
    for default_key in defaults {
        // We unwrap here for parsing the hardcoded default keys, which are known to be valid
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SoundClass {
    pub values: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generator: Option<GeneratorConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum GeneratorConfig {
    Zipf { a: f64, b: f64 },
    Equiprobable,
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    use uuid::Uuid;

    #[test]
    fn test_language_config_deserialization() {
        let json_str = r#"{
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
                            "type": "Zipf",
                            "a": 1.0,
                            "b": 2.0
                        }
                    }
                }
            }
        }"#;

        let config: LanguageConfig = serde_json::from_str(json_str).expect("Should deserialize");

        assert_eq!(
            config.id,
            Uuid::parse_str("018f4a3e-6b9f-7a1a-9b1a-2b3c4d5e6f7a").unwrap()
        );
        assert_eq!(config.name.endonym.as_str(), "p");
        assert!(config.name.exonym.is_none());
        assert_eq!(
            config.metadata.created_at,
            datetime!(2024-05-04 00:12:00 +00:00)
        );

        let phonology = config.phonology.sound_classes;

        // Ensure explicit class is parsed
        let class_a = phonology
            .get(&"A".parse::<SoundClassKey>().unwrap())
            .unwrap();
        assert_eq!(class_a.values, vec!["p", "t", "k"]);
        assert_eq!(
            class_a.generator,
            Some(GeneratorConfig::Zipf { a: 1.0, b: 2.0 })
        );

        // Ensure default classes are inserted automatically
        assert!(phonology.contains_key(&"C".parse::<SoundClassKey>().unwrap()));
        assert!(phonology.contains_key(&"D".parse::<SoundClassKey>().unwrap()));
        assert!(phonology.contains_key(&"L".parse::<SoundClassKey>().unwrap()));
        assert!(phonology.contains_key(&"V".parse::<SoundClassKey>().unwrap()));

        let class_c = phonology
            .get(&"C".parse::<SoundClassKey>().unwrap())
            .unwrap();
        assert!(class_c.values.is_empty());
        assert!(class_c.generator.is_none());
    }

    #[test]
    fn test_generator_equiprobable() {
        let json_str = r#"{
            "values": ["a", "e", "i"],
            "generator": {
                "type": "Equiprobable"
            }
        }"#;

        let sound_class: SoundClass = serde_json::from_str(json_str).expect("Should deserialize");
        assert_eq!(sound_class.generator, Some(GeneratorConfig::Equiprobable));
    }
}
