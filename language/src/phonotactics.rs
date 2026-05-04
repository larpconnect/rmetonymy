use crate::sound_class::{SoundClassKey, SoundClassKeyError};
use ipa::IpaString;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::Display;
use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum PhonotacticError {
    #[error("Invalid phonotactic pattern: {0}")]
    InvalidPattern(String),
    #[error("Unmatched parenthesis in pattern")]
    UnmatchedParenthesis,
    #[error("Invalid percentage value: {0}")]
    InvalidPercentage(String),
    #[error("Invalid sound class key: {0}")]
    InvalidSoundClass(#[from] SoundClassKeyError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Token {
    SoundClass(SoundClassKey),
    Ipa(IpaString),
    Group {
        tokens: Vec<Token>,
        probability: u8,
    },
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::SoundClass(s) => write!(f, "{s}"),
            Token::Ipa(i) => write!(f, "{i}"),
            Token::Group { tokens, probability } => {
                write!(f, "(")?;
                for token in tokens {
                    write!(f, "{token}")?;
                }
                write!(f, ")")?;
                if *probability != 20 {
                    write!(f, "{probability}%")?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhonotacticPattern(pub Vec<Token>);

impl Display for PhonotacticPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for token in &self.0 {
            write!(f, "{token}")?;
        }
        Ok(())
    }
}

impl Serialize for PhonotacticPattern {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for PhonotacticPattern {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<Self>().map_err(serde::de::Error::custom)
    }
}

impl FromStr for PhonotacticPattern {
    type Err = PhonotacticError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (tokens, remaining) = parse_tokens(s, false)?;
        if !remaining.is_empty() {
            return Err(PhonotacticError::InvalidPattern(remaining.to_string()));
        }
        Ok(PhonotacticPattern(tokens))
    }
}

#[expect(
    clippy::string_slice,
    reason = "Indexing is safe due to char bounds checking"
)]
#[expect(clippy::too_many_lines, reason = "Parser logic requires single pass")]
fn parse_tokens(s: &str, in_group: bool) -> Result<(Vec<Token>, &str), PhonotacticError> {
    let mut tokens = Vec::new();
    let mut char_indices = s.char_indices().peekable();
    let mut remaining = s;
    let mut offset = 0;

    while let Some((idx, c)) = char_indices.next() {
        if idx < offset {
            continue;
        }

        let slice = &s[idx..];

        if c == '(' {
            let (group_tokens, group_remaining) = parse_tokens(&slice[1..], true)?;

            let mut prob = 20;
            let mut after_prob_remaining = group_remaining;

            if let Some(first_char) = group_remaining.chars().next()
                && first_char.is_ascii_digit()
            {
                let mut prob_str = String::new();
                let prob_chars = group_remaining.chars();
                for pc in prob_chars {
                    if pc.is_ascii_digit() {
                        prob_str.push(pc);
                    } else if pc == '%' {
                        break;
                    } else {
                        return Err(PhonotacticError::InvalidPercentage(prob_str));
                    }
                }
                if let Ok(p) = prob_str.parse::<u8>() {
                    prob = p;
                    if !(1..=99).contains(&p) {
                        return Err(PhonotacticError::InvalidPercentage(prob_str));
                    }
                    after_prob_remaining = &group_remaining[prob_str.len() + 1..];
                } else {
                    return Err(PhonotacticError::InvalidPercentage(prob_str));
                }
            }

            tokens.push(Token::Group {
                tokens: group_tokens,
                probability: prob,
            });

            offset = s.len() - after_prob_remaining.len();
            remaining = after_prob_remaining;
            continue;
        } else if c == ')' {
            if !in_group {
                return Err(PhonotacticError::UnmatchedParenthesis);
            }
            return Ok((tokens, &slice[1..]));
        }

        // Try parsing as SoundClassKey first
        let mut parsed_token = None;

        // Allowed base letters for SoundClassKey
        let is_english = c.is_ascii_uppercase();
        let is_greek = ('\u{0391}'..='\u{03A9}').contains(&c);
        let is_hebrew = ('\u{05D0}'..='\u{05EA}').contains(&c);

        if is_english || is_greek || is_hebrew {
            // Might have a subscript
            let mut key_str = c.to_string();
            let mut next_offset = c.len_utf8();
            if let Some((_, next_c)) = char_indices.peek()
                && ('\u{2080}'..='\u{2089}').contains(next_c)
            {
                key_str.push(*next_c);
                next_offset += next_c.len_utf8();
            }
            if let Ok(sc) = SoundClassKey::from_str(&key_str) {
                parsed_token = Some(Token::SoundClass(sc));
                offset = idx + next_offset;
                remaining = &s[offset..];
            }
        }

        if parsed_token.is_none() {
            // Greedily match valid IPAString
            let char_indices_vec: Vec<(usize, char)> = slice.char_indices().collect();
            let char_len = char_indices_vec.len();

            let mut matched = false;
            for len in (1..=char_len).rev() {
                let start_idx = 0;
                let end_idx = char_indices_vec
                    .get(len)
                    .map_or(slice.len(), |(idx, _)| *idx);

                let substr = &slice[start_idx..end_idx];
                if substr.contains('(') || substr.contains(')') {
                    continue;
                }

                if let Ok(ipa_str) = IpaString::from_str(substr) {
                    parsed_token = Some(Token::Ipa(ipa_str));
                    offset = idx + end_idx;
                    remaining = &s[offset..];
                    matched = true;
                    break;
                }
            }
            if !matched {
                return Err(PhonotacticError::InvalidPattern(s.to_string()));
            }
        }

        if let Some(token) = parsed_token {
            tokens.push(token);
        }
    }

    if in_group {
        return Err(PhonotacticError::UnmatchedParenthesis);
    }

    Ok((tokens, remaining))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_patterns() {
        assert!("CVC".parse::<PhonotacticPattern>().is_ok());
        assert!("CV(C)".parse::<PhonotacticPattern>().is_ok());
        assert!("CD(L)C".parse::<PhonotacticPattern>().is_ok());
        assert!("CD(L)50%C".parse::<PhonotacticPattern>().is_ok());
        assert!("(C)VL".parse::<PhonotacticPattern>().is_ok());
        assert!("(C)50%V(C)15%".parse::<PhonotacticPattern>().is_ok());
        assert!("SS(S)".parse::<PhonotacticPattern>().is_ok());
        assert!("CaC".parse::<PhonotacticPattern>().is_ok());
    }
}
