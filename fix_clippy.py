with open("ipa/src/string.rs", "r") as f:
    text = f.read()

text = text.replace("let substr: String = chars[i..i+len].iter().collect();", """if let Some(slice) = chars.get(i..i+len) {
                    let substr: String = slice.iter().collect();
                    if get_entry(&substr).is_some() {
                        i += len;
                        matched = true;
                        break;
                    }
                }
                continue;""")
text = text.replace("""                if get_entry(&substr).is_some() {
                    i += len;
                    matched = true;
                    break;
                }""", "") # Remove the duplicate old lines.

with open("ipa/src/string.rs", "w") as f:
    f.write(text)
