use crate::config::SoundClass;
use crate::matcher::ast::*;
use crate::sound_class::SoundClassKey;
use data::IpaEntry;
use ipa::{get_entry, get_phoneme_data};
use std::collections::{BTreeMap, HashSet};

impl SoundMatcherPattern {
    /// Tokenizes the word, pulling out syllable boundaries and phonemes.
    fn tokenize(word: &str) -> Vec<Token> {
        let mut tokens = vec![Token::Boundary("#".to_string())];

        let mut chars = word.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '.' || c == 'ˌ' || c == 'ˈ' || c == '\'' {
                tokens.push(Token::Boundary("$".to_string()));
            } else {
                let mut phoneme = c.to_string();
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
                tokens.push(Token::Phoneme(phoneme));
            }
        }

        tokens.push(Token::Boundary("#".to_string()));
        tokens
    }

    #[must_use]
    pub fn matches(&self, word: &str, classes: &BTreeMap<SoundClassKey, SoundClass>) -> bool {
        let tokens = Self::tokenize(word);

        for i in 0..tokens.len() {
            if let Some(tokens_slice) = tokens.get(i..) {
                if self.match_at(tokens_slice, &self.elements, classes) {
                    return true;
                }
            }
        }

        false
    }

    fn match_at(
        &self,
        tokens: &[Token],
        pattern: &[PatternElement],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
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

        let mut match_lengths = Vec::new();

        match el.quantifier {
            Quantifier::None => {
                if let Some(len) = self.match_base(&el.base, tokens, classes) {
                    match_lengths.push(len);
                }
            }
            Quantifier::ZeroOrMore => {
                match_lengths.push(0);
                self.find_repeated_matches(
                    &el.base,
                    tokens,
                    classes,
                    1,
                    usize::MAX,
                    0,
                    &mut match_lengths,
                );
            }
            Quantifier::OneOrMore => {
                self.find_repeated_matches(
                    &el.base,
                    tokens,
                    classes,
                    1,
                    usize::MAX,
                    0,
                    &mut match_lengths,
                );
            }
        }

        match_lengths.sort_unstable_by(|a, b| b.cmp(a));
        match_lengths.dedup();

        for len in match_lengths {
            if let Some(tokens_slice) = tokens.get(len..) {
                if self.match_at(tokens_slice, rest_pattern, classes) {
                    return true;
                }
            }
        }

        false
    }

    fn find_repeated_matches(
        &self,
        base: &BaseElement,
        tokens: &[Token],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
        min: usize,
        max: usize,
        current_len: usize,
        results: &mut Vec<usize>,
    ) {
        if min == 0 {
            results.push(current_len);
        }

        if max > 0 {
            if let Some(tokens_slice) = tokens.get(current_len..) {
                if let Some(len) = self.match_base(base, tokens_slice, classes) {
                    if len > 0 {
                        let next_min = min.saturating_sub(1);
                        self.find_repeated_matches(
                            base,
                            tokens,
                            classes,
                            next_min,
                            max - 1,
                            current_len + len,
                            results,
                        );
                    }
                }
            }
        }
    }

    fn match_base(
        &self,
        base: &BaseElement,
        tokens: &[Token],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> Option<usize> {
        let first_token = tokens.first()?;

        match base {
            BaseElement::WordBoundary => {
                if let Token::Boundary(b) = first_token {
                    if b == "#" {
                        return Some(1);
                    }
                }
            }
            BaseElement::SyllableBoundary => {
                if let Token::Boundary(b) = first_token {
                    if b == "$" {
                        return Some(1);
                    }
                }
            }
            BaseElement::SoundClass(key) => {
                if let Token::Phoneme(p) = first_token {
                    if Self::phoneme_in_class(p, key, classes) {
                        return Some(1);
                    }
                }
            }
            BaseElement::IpaSequence(ipa) => {
                let target = ipa.as_str();
                let mut accumulated = String::new();
                let mut len = 0;
                for t in tokens {
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
            }
            BaseElement::FeatureClass(sc_opt, features) => {
                if let Token::Phoneme(p) = first_token {
                    if let Some(sc) = sc_opt {
                        if !Self::phoneme_in_class(p, sc, classes) {
                            return None;
                        }
                    }

                    if let Some(phoneme_data) = get_phoneme_data(p) {
                        let mut has_all_features = true;
                        let phoneme_features: HashSet<_> = phoneme_data
                            .features
                            .iter()
                            .filter_map(|sf| {
                                // Extract the underlying feature
                                match sf {
                                    data::SpeFeature::Plus(f) => Some((*f, true)),
                                    data::SpeFeature::Minus(f) => Some((*f, false)),
                                }
                            })
                            .collect();

                        for fd in features {
                            let mut found_match = false;
                            for &(ref feat, sign) in &phoneme_features {
                                if *feat == fd.feature && sign == fd.sign {
                                    found_match = true;
                                    break;
                                }
                            }
                            // Also fallback if feature is just present positively.
                            if !found_match {
                                has_all_features = false;
                                break;
                            }
                        }

                        if has_all_features {
                            return Some(1);
                        }
                    }
                }
            }
            BaseElement::Set(els) => {
                for el in els {
                    if let Some(len) = self.match_base(el, tokens, classes) {
                        return Some(len);
                    }
                }
            }
            BaseElement::OptionalGroup(pat) => {
                let mut lengths = vec![];
                self.find_group_match_lengths(tokens, &pat.elements, classes, 0, &mut lengths);
                lengths.sort_unstable_by(|a, b| b.cmp(a));
                if let Some(&l) = lengths.first() {
                    return Some(l);
                }
            }
        }
        None
    }

    fn find_group_match_lengths(
        &self,
        tokens: &[Token],
        pattern: &[PatternElement],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
        current_len: usize,
        results: &mut Vec<usize>,
    ) {
        if pattern.is_empty() {
            results.push(current_len);
            return;
        }

        let el = &pattern[0];
        let Some(rest_pattern) = pattern.get(1..) else {
            return;
        };

        let mut match_lengths = Vec::new();
        match el.quantifier {
            Quantifier::None => {
                if let Some(tokens_slice) = tokens.get(current_len..) {
                    if let Some(len) = self.match_base(&el.base, tokens_slice, classes) {
                        match_lengths.push(len);
                    }
                }
            }
            Quantifier::ZeroOrMore => {
                match_lengths.push(0);
                if let Some(tokens_slice) = tokens.get(current_len..) {
                    self.find_repeated_matches(
                        &el.base,
                        tokens_slice,
                        classes,
                        1,
                        usize::MAX,
                        0,
                        &mut match_lengths,
                    );
                }
            }
            Quantifier::OneOrMore => {
                if let Some(tokens_slice) = tokens.get(current_len..) {
                    self.find_repeated_matches(
                        &el.base,
                        tokens_slice,
                        classes,
                        1,
                        usize::MAX,
                        0,
                        &mut match_lengths,
                    );
                }
            }
        }

        for len in match_lengths {
            self.find_group_match_lengths(
                tokens,
                rest_pattern,
                classes,
                current_len + len,
                results,
            );
        }
    }

    fn phoneme_in_class(
        p: &str,
        key: &SoundClassKey,
        classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> bool {
        if key.as_str() == "V" {
            if let Some(entry) = get_entry(p) {
                return matches!(entry, IpaEntry::Vowel(_));
            }
            return false;
        }
        if key.as_str() == "C" {
            if let Some(entry) = get_entry(p) {
                return matches!(entry, IpaEntry::Consonant(_));
            }
            return false;
        }
        if key.as_str() == "D" {
            if let Some(_entry) = get_entry(p) {
                // Approximate diphthongs if tie bar is present
                return p.contains('\u{0361}') || p.contains('\u{035C}');
            }
            return false;
        }

        if let Some(sc) = classes.get(key) {
            // For efficiency as requested, we could build a HashSet, but let's just do an iter any here
            // since building a hashset per phoneme_in_class invocation is slower than iter over a few items.
            return sc.values.iter().any(|val| val == p);
        }
        false
    }
}
