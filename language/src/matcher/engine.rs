use crate::config::SoundClass;
use crate::matcher::ast::{
    BaseElement, FeatureDescriptor, PatternElement, Quantifier, SoundMatcherPattern, Token,
};
use crate::sound_class::SoundClassKey;
use data::IpaEntry;
use ipa::{IpaString, get_entry, get_phoneme_data};
use std::collections::BTreeMap;

struct RepeatContext<'a> {
    base: &'a BaseElement,
    marker: Option<u8>,
    tokens: &'a [Token],
    classes: &'a BTreeMap<SoundClassKey, SoundClass>,
}

impl SoundMatcherPattern {
    fn parse_phoneme(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
        let Some(first_char) = chars.next() else {
            return String::new();
        };
        let mut phoneme = first_char.to_string();
        while let Some(&next_c) = chars.peek() {
            if next_c == '.' || next_c == 'ˌ' || next_c == 'ˈ' || next_c == '\'' {
                break;
            }
            let combined = format!("{phoneme}{next_c}");
            if get_entry(&combined).is_some() {
                phoneme = combined;
                chars.next();
            } else if let Some(IpaEntry::Modifier(_)) = get_entry(&next_c.to_string()) {
                phoneme = combined;
                chars.next();
            } else {
                break;
            }
        }
        phoneme
    }

    /// Tokenizes the word, pulling out syllable boundaries and phonemes.
    fn tokenize(word: &str) -> Vec<Token> {
        let mut tokens = vec![Token::Boundary("#".to_string())];
        let mut chars = word.chars().peekable();

        while let Some(&c) = chars.peek() {
            if c == '.' || c == 'ˌ' || c == 'ˈ' || c == '\'' {
                tokens.push(Token::Boundary("$".to_string()));
                chars.next();
            } else {
                tokens.push(Token::Phoneme(Self::parse_phoneme(&mut chars)));
            }
        }

        tokens.push(Token::Boundary("#".to_string()));
        tokens
    }

    #[must_use]
    pub fn matches(&self, word: &str, classes: &BTreeMap<SoundClassKey, SoundClass>) -> bool {
        let tokens = Self::tokenize(word);
        let mut bindings = BTreeMap::new();

        for i in 0..tokens.len() {
            if let Some(tokens_slice) = tokens.get(i..)
                && self.match_at(tokens_slice, &self.elements, classes, &mut bindings)
            {
                return true;
            }
            bindings.clear();
        }

        false
    }

    fn match_at(
        &self,
        tokens: &[Token],
        pattern: &[PatternElement],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
        bindings: &mut BTreeMap<u8, Vec<Token>>,
    ) -> bool {
        if pattern.is_empty() {
            return true;
        }

        let Some(el) = pattern.first() else {
            return false;
        };
        let Some(rest_pattern) = pattern.get(1..) else {
            return false;
        };

        let mut match_lengths = self.get_match_lengths(el, tokens, classes, bindings);
        match_lengths.sort_unstable_by_key(|b| std::cmp::Reverse(b.0));
        match_lengths.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);

        for (len, next_bindings) in match_lengths {
            let mut temp_bindings = next_bindings;
            if let Some(tokens_slice) = tokens.get(len..)
                && self.match_at(tokens_slice, rest_pattern, classes, &mut temp_bindings)
            {
                *bindings = temp_bindings;
                return true;
            }
        }

        false
    }

    fn get_match_lengths(
        &self,
        el: &PatternElement,
        tokens: &[Token],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
        bindings: &BTreeMap<u8, Vec<Token>>,
    ) -> Vec<(usize, BTreeMap<u8, Vec<Token>>)> {
        let mut match_lengths = Vec::new();
        let ctx = RepeatContext {
            base: &el.base,
            marker: el.marker,
            tokens,
            classes,
        };
        match el.quantifier {
            Quantifier::None => {
                if let Some((len, next_bindings)) = self.match_base_with_bindings(
                    &el.base,
                    el.marker,
                    tokens,
                    classes,
                    bindings,
                ) {
                    match_lengths.push((len, next_bindings));
                }
            }
            Quantifier::ZeroOrMore => {
                self.find_repeated_matches(
                    &ctx,
                    0,
                    usize::MAX,
                    0,
                    bindings,
                    &mut match_lengths,
                );
            }
            Quantifier::OneOrMore => {
                self.find_repeated_matches(
                    &ctx,
                    1,
                    usize::MAX,
                    0,
                    bindings,
                    &mut match_lengths,
                );
            }
        }
        match_lengths
    }

    fn find_repeated_matches(
        &self,
        ctx: &RepeatContext<'_>,
        min: usize,
        max: usize,
        current_len: usize,
        bindings: &BTreeMap<u8, Vec<Token>>,
        results: &mut Vec<(usize, BTreeMap<u8, Vec<Token>>)>,
    ) {
        if min == 0 {
            results.push((current_len, bindings.clone()));
        }

        if max > 0
            && let Some(tokens_slice) = ctx.tokens.get(current_len..)
            && let Some((len, next_bindings)) = self.match_base_with_bindings(
                ctx.base,
                ctx.marker,
                tokens_slice,
                ctx.classes,
                bindings,
            )
            && len > 0
        {
            let next_min = min.saturating_sub(1);
            self.find_repeated_matches(
                ctx,
                next_min,
                max - 1,
                current_len + len,
                &next_bindings,
                results,
            );
        }
    }

    fn calculate_skip(base: &BaseElement, tokens: &[Token]) -> usize {
        let mut skip = 0;
        while let Some(Token::Boundary(b)) = tokens.get(skip) {
            if b == "$" && !matches!(base, BaseElement::SyllableBoundary) {
                skip += 1;
            } else {
                break;
            }
        }
        skip
    }

    fn match_unbound_base(
        &self,
        base: &BaseElement,
        m: u8,
        tokens: &[Token],
        skip: usize,
        classes: &BTreeMap<SoundClassKey, SoundClass>,
        temp_bindings: &mut BTreeMap<u8, Vec<Token>>,
    ) -> Option<(usize, BTreeMap<u8, Vec<Token>>)> {
        let len = self.match_base(base, tokens, classes, temp_bindings)?;
        let matched_tokens = tokens.get(skip..len)?.to_vec();
        temp_bindings.insert(m, matched_tokens);
        Some((len, temp_bindings.clone()))
    }

    fn match_marked_base(
        &self,
        base: &BaseElement,
        m: u8,
        tokens: &[Token],
        skip: usize,
        classes: &BTreeMap<SoundClassKey, SoundClass>,
        temp_bindings: &mut BTreeMap<u8, Vec<Token>>,
    ) -> Option<(usize, BTreeMap<u8, Vec<Token>>)> {
        if let Some(bound_tokens) = temp_bindings.get(&m).cloned() {
            let tokens_to_check = tokens.get(skip..)?;
            if tokens_to_check.get(..bound_tokens.len()) == Some(bound_tokens.as_slice())
                && self.match_base(base, &bound_tokens, classes, temp_bindings)
                    == Some(bound_tokens.len())
            {
                Some((skip + bound_tokens.len(), temp_bindings.clone()))
            } else {
                None
            }
        } else {
            self.match_unbound_base(base, m, tokens, skip, classes, temp_bindings)
        }
    }

    fn match_base_with_bindings(
        &self,
        base: &BaseElement,
        marker: Option<u8>,
        tokens: &[Token],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
        bindings: &BTreeMap<u8, Vec<Token>>,
    ) -> Option<(usize, BTreeMap<u8, Vec<Token>>)> {
        let skip = Self::calculate_skip(base, tokens);
        let mut temp_bindings = bindings.clone();

        if let Some(m) = marker {
            self.match_marked_base(base, m, tokens, skip, classes, &mut temp_bindings)
        } else {
            let len = self.match_base(base, tokens, classes, &mut temp_bindings)?;
            Some((len, temp_bindings))
        }
    }

    fn match_base(
        &self,
        base: &BaseElement,
        tokens: &[Token],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
        bindings: &mut BTreeMap<u8, Vec<Token>>,
    ) -> Option<usize> {
        let mut skip = 0;
        while let Some(Token::Boundary(b)) = tokens.get(skip) {
            if b == "$" && !matches!(base, BaseElement::SyllableBoundary) {
                skip += 1;
            } else {
                break;
            }
        }

        let tokens_to_check = tokens.get(skip..)?;
        let first_token = tokens_to_check.first()?;

        let len = match base {
            BaseElement::WordBoundary => Self::match_word_boundary(first_token),
            BaseElement::SyllableBoundary => Self::match_syllable_boundary(first_token),
            BaseElement::SoundClass(key) => Self::match_sound_class(first_token, key, classes),
            BaseElement::IpaSequence(ipa) => Self::match_ipa_sequence(tokens_to_check, ipa),
            BaseElement::FeatureClass(sc_opt, features) => {
                Self::match_feature_class(first_token, sc_opt.as_ref(), features, classes)
            }
            BaseElement::Set(els) => self.match_set(els, tokens_to_check, classes, bindings),
            BaseElement::OptionalGroup(pat) => {
                self.match_optional_group(pat, tokens_to_check, classes, bindings)
            }
        };
        len.map(|l| skip + l)
    }

    fn match_word_boundary(first_token: &Token) -> Option<usize> {
        if let Token::Boundary(b) = first_token
            && b == "#"
        {
            return Some(1);
        }
        None
    }

    fn match_syllable_boundary(first_token: &Token) -> Option<usize> {
        if let Token::Boundary(b) = first_token
            && (b == "$" || b == "#")
        {
            return Some(1);
        }
        None
    }

    fn match_sound_class(
        first_token: &Token,
        key: &SoundClassKey,
        classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> Option<usize> {
        if let Token::Phoneme(p) = first_token
            && Self::phoneme_in_class(p, key, classes)
        {
            return Some(1);
        }
        None
    }

    fn match_ipa_sequence(tokens: &[Token], ipa: &IpaString) -> Option<usize> {
        let target = ipa.as_str();
        let mut accumulated = String::new();
        let mut len = 0;
        for t in tokens {
            if let Token::Boundary(b) = t
                && b == "$"
            {
                len += 1;
                continue;
            }
            if let Token::Phoneme(p) = t {
                accumulated.push_str(p);
                len += 1;
                if accumulated == target {
                    return Some(len);
                } else if !target.starts_with(&accumulated) {
                    break;
                }
            } else {
                break;
            }
        }
        None
    }

    fn match_feature_class(
        first_token: &Token,
        sc_opt: Option<&SoundClassKey>,
        features: &[FeatureDescriptor],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> Option<usize> {
        if let Token::Phoneme(p) = first_token {
            if let Some(sc) = sc_opt
                && !Self::phoneme_in_class(p, sc, classes)
            {
                return None;
            }

            if let Some(phoneme_data) = get_phoneme_data(p) {
                let mut has_all_features = true;
                let phoneme_features: std::collections::HashMap<_, _> = phoneme_data
                    .features
                    .iter()
                    .map(|sf| match sf {
                        data::SpeFeature::Plus(f) => (*f, true),
                        data::SpeFeature::Minus(f) => (*f, false),
                    })
                    .collect();

                for fd in features {
                    let has_feature = if let Some(&sign) = phoneme_features.get(&fd.feature) {
                        sign
                    } else {
                        false // Default to minus if absent
                    };

                    if has_feature != fd.sign {
                        has_all_features = false;
                        break;
                    }
                }

                if has_all_features {
                    return Some(1);
                }
            }
        }
        None
    }

    fn match_set(
        &self,
        els: &[BaseElement],
        tokens: &[Token],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
        bindings: &mut BTreeMap<u8, Vec<Token>>,
    ) -> Option<usize> {
        for el in els {
            if let Some(len) = self.match_base(el, tokens, classes, bindings) {
                return Some(len);
            }
        }
        None
    }

    fn match_optional_group(
        &self,
        pat: &SoundMatcherPattern,
        tokens: &[Token],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
        bindings: &mut BTreeMap<u8, Vec<Token>>,
    ) -> Option<usize> {
        let mut lengths = vec![];
        self.find_group_match_lengths(tokens, &pat.elements, classes, 0, bindings, &mut lengths);
        lengths.sort_unstable_by_key(|b| std::cmp::Reverse(b.0));
        if let Some((l, next_bindings)) = lengths.first() {
            *bindings = next_bindings.clone();
            return Some(*l);
        }
        None
    }

    fn find_group_match_lengths(
        &self,
        tokens: &[Token],
        pattern: &[PatternElement],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
        current_len: usize,
        bindings: &BTreeMap<u8, Vec<Token>>,
        results: &mut Vec<(usize, BTreeMap<u8, Vec<Token>>)>,
    ) {
        if pattern.is_empty() {
            results.push((current_len, bindings.clone()));
            return;
        }

        let Some(el) = pattern.first() else {
            return;
        };
        let Some(rest_pattern) = pattern.get(1..) else {
            return;
        };

        let match_lengths = self.get_match_lengths(el, tokens, classes, bindings);

        for (len, next_bindings) in match_lengths {
            if let Some(tokens_slice) = tokens.get(len..) {
                self.find_group_match_lengths(
                    tokens_slice,
                    rest_pattern,
                    classes,
                    current_len + len,
                    &next_bindings,
                    results,
                );
            }
        }
    }


    fn phoneme_in_class(
        p: &str,
        key: &SoundClassKey,
        classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> bool {
        match key.as_str() {
            "V" => Self::is_vowel(p),
            "C" => Self::is_consonant(p),
            "D" => Self::is_diphthong(p),
            _ => {
                if let Some(sc) = classes.get(key) {
                    sc.values.iter().any(|val| val == p)
                } else {
                    false
                }
            }
        }
    }

    fn is_vowel(p: &str) -> bool {
        if let Some(entry) = get_entry(p) {
            return matches!(entry, IpaEntry::Vowel(_));
        }
        false
    }

    fn is_consonant(p: &str) -> bool {
        if let Some(entry) = get_entry(p) {
            return matches!(entry, IpaEntry::Consonant(_));
        }
        false
    }

    fn is_diphthong(p: &str) -> bool {
        if let Some(_entry) = get_entry(p) {
            return p.contains('\u{0361}') || p.contains('\u{035C}');
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_backreferences_basic() {
        let pattern = SoundMatcherPattern::from_str("C1VC1").unwrap();
        let mut classes = BTreeMap::new();
        classes.insert(
            "C".parse::<SoundClassKey>().unwrap(),
            SoundClass {
                values: vec!["k".to_string(), "l".to_string(), "t".to_string()],
                generator: None,
            },
        );

        assert!(pattern.matches("kak", &classes));
        assert!(!pattern.matches("kal", &classes));
        assert!(pattern.matches("tat", &classes));
        assert!(!pattern.matches("tal", &classes));
    }

    #[test]
    fn test_backreferences_multiple() {
        let pattern = SoundMatcherPattern::from_str("C1C2VC2C1").unwrap();
        let mut classes = BTreeMap::new();
        classes.insert(
            "C".parse::<SoundClassKey>().unwrap(),
            SoundClass {
                values: vec!["k".to_string(), "l".to_string(), "t".to_string(), "p".to_string()],
                generator: None,
            },
        );

        assert!(pattern.matches("ktatk", &classes));
        assert!(!pattern.matches("ktatp", &classes));
        assert!(pattern.matches("klalk", &classes));
    }

    #[test]
    fn test_backreferences_feature_class() {
        let pattern = SoundMatcherPattern::from_str("[C1 -voiced]V[C1 -voiced]").unwrap();
        let mut classes = BTreeMap::new();
        classes.insert(
            "C".parse::<SoundClassKey>().unwrap(),
            SoundClass {
                values: vec!["k".to_string(), "l".to_string(), "t".to_string()],
                generator: None,
            },
        );

        assert!(pattern.matches("kak", &classes));
        assert!(!pattern.matches("kal", &classes));
    }
}

