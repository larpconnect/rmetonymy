with open("language/src/config.rs", "r") as f:
    text = f.read()
text = text.replace("use serde_json::json;", "")
with open("language/src/config.rs", "w") as f:
    f.write(text)
