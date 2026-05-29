use crate::matcher::ast::{SoundMatcherPattern, Token};
use data::IpaEntry;
use ipa::get_entry;

impl SoundMatcherPattern {
    pub(crate) fn parse_phoneme_integration(
        chars: &mut std::iter::Peekable<std::str::Chars>,
    ) -> String {
        Self::parse_phoneme_op(chars, |s| get_entry(s).cloned())
    }

    fn parse_phoneme_op<F>(
        chars: &mut std::iter::Peekable<std::str::Chars>,
        mut get_entry_fn: F,
    ) -> String
    where
        F: FnMut(&str) -> Option<IpaEntry>,
    {
        let Some(first_char) = chars.next() else {
            return String::new();
        };
        let mut phoneme = first_char.to_string();
        while let Some(&next_c) = chars.peek() {
            if next_c == '.' || next_c == 'ˌ' || next_c == 'ˈ' || next_c == '\'' {
                break;
            }
            let combined = format!("{phoneme}{next_c}");
            if get_entry_fn(&combined).is_some() {
                phoneme = combined;
                chars.next();
            } else if let Some(IpaEntry::Modifier(_)) = get_entry_fn(&next_c.to_string()) {
                phoneme = combined;
                chars.next();
            } else {
                break;
            }
        }
        phoneme
    }

    /// Tokenizes the word, pulling out syllable boundaries and phonemes.
    pub(crate) fn tokenize(word: &str) -> Vec<Token> {
        let mut tokens = vec![Token::Boundary("#".to_string())];
        let mut chars = word.chars().peekable();
        Self::tokenize_loop_op(&mut chars, &mut tokens, |c| {
            Self::parse_phoneme_integration(c)
        });
        tokens.push(Token::Boundary("#".to_string()));
        tokens
    }

    fn tokenize_loop_op<F>(
        chars: &mut std::iter::Peekable<std::str::Chars>,
        tokens: &mut Vec<Token>,
        mut parse_phoneme_fn: F,
    ) where
        F: FnMut(&mut std::iter::Peekable<std::str::Chars>) -> String,
    {
        while let Some(&c) = chars.peek() {
            if c == '.' || c == 'ˌ' || c == 'ˈ' || c == '\'' {
                tokens.push(Token::Boundary("$".to_string()));
                chars.next();
            } else {
                tokens.push(Token::Phoneme(parse_phoneme_fn(chars)));
            }
        }
    }
}
