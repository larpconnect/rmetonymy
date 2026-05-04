with open("language/src/config.rs", "r") as f:
    text = f.read()

text = text.replace("let key = default_key.parse::<SoundClassKey>().unwrap();", """let key = default_key.parse::<SoundClassKey>().map_err(serde::de::Error::custom)?;""")

with open("language/src/config.rs", "w") as f:
    f.write(text)
