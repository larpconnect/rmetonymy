with open("language/src/config.rs", "r") as f:
    text = f.read()

# We need to add custom deserialization for LanguageConfig or PhonologyConfig to ensure default classes are present.
# It's easier to add it on PhonologyConfig or define a default for `sound_classes` that populates C, D, L, V.
# A custom deserialize on PhonologyConfig is best.

replacement = """
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
        let key = default_key.parse::<SoundClassKey>().unwrap();
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
"""

import re
text = text.replace("use serde::{Deserialize, Serialize};", "")
text = re.sub(r'#\[derive\(Debug, Clone, Serialize, Deserialize, PartialEq\)]\npub struct PhonologyConfig \{\n    pub sound_classes: BTreeMap<SoundClassKey, SoundClass>,\n\}', replacement, text)

with open("language/src/config.rs", "w") as f:
    f.write(text)
