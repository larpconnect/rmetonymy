import re

with open("ipa/src/string.rs", "r") as f:
    text = f.read()

# Let's cleanly rewrite the FromStr function to be safe.
# Find `fn from_str` and replace it.

start_idx = text.find("    fn from_str(s: &str) -> Result<Self, Self::Err> {")
end_idx = text.find("    }\n}\n\nimpl Serialize")

replacement = """    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(IpaString(s.to_string()));
        }

        let mut i = 0;
        let chars: Vec<char> = s.chars().collect();
        while i < chars.len() {
            let mut matched = false;
            for len in (1..=chars.len() - i).rev() {
                if let Some(slice) = chars.get(i..i + len) {
                    let substr: String = slice.iter().collect();
                    if get_entry(&substr).is_some() {
                        i += len;
                        matched = true;
                        break;
                    }
                }
            }
            if !matched {
                return Err(IpaStringError::InvalidSequence(s.to_string()));
            }
        }

        Ok(IpaString(s.to_string()))"""

text = text[:start_idx] + replacement + text[end_idx:]

with open("ipa/src/string.rs", "w") as f:
    f.write(text)
