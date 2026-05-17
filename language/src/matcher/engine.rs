use crate::config::SoundClass;
use crate::matcher::ast::{
    BaseElement, FeatureDescriptor, PatternElement, Quantifier, SoundMatcherPattern, Token,
};
use crate::sound_class::SoundClassKey;
use data::IpaEntry;
use ipa::{IpaString, get_entry, get_phoneme_data};
use std::collections::{BTreeMap, HashSet};

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

        for i in 0..tokens.len() {
            if let Some(tokens_slice) = tokens.get(i..)
                && self.match_at(tokens_slice, &self.elements, classes)
            {
                return true;
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

        let mut match_lengths = self.get_match_lengths(el, tokens, classes);
        match_lengths.sort_unstable_by(|a, b| b.cmp(a));
        match_lengths.dedup();

        for len in match_lengths {
            if let Some(tokens_slice) = tokens.get(len..)
                && self.match_at(tokens_slice, rest_pattern, classes)
            {
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
    ) -> Vec<usize> {
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
        match_lengths
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Recursive logic dictates numerous arguments passed"
    )]
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

        if max > 0
            && let Some(tokens_slice) = tokens.get(current_len..)
            && let Some(len) = self.match_base(base, tokens_slice, classes)
            && len > 0
        {
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

    fn match_base(
        &self,
        base: &BaseElement,
        tokens: &[Token],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> Option<usize> {
        let first_token = tokens.first()?;

        match base {
            BaseElement::WordBoundary => Self::match_word_boundary(first_token),
            BaseElement::SyllableBoundary => Self::match_syllable_boundary(first_token),
            BaseElement::SoundClass(key) => Self::match_sound_class(first_token, key, classes),
            BaseElement::IpaSequence(ipa) => Self::match_ipa_sequence(tokens, ipa),
            BaseElement::FeatureClass(sc_opt, features) => {
                Self::match_feature_class(first_token, sc_opt.as_ref(), features, classes)
            }
            BaseElement::Set(els) => self.match_set(els, tokens, classes),
            BaseElement::OptionalGroup(pat) => self.match_optional_group(pat, tokens, classes),
        }
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
            && b == "$"
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
                let phoneme_features: HashSet<_> = phoneme_data
                    .features
                    .iter()
                    .map(|sf| match sf {
                        data::SpeFeature::Plus(f) => (*f, true),
                        data::SpeFeature::Minus(f) => (*f, false),
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
        None
    }

    fn match_set(
        &self,
        els: &[BaseElement],
        tokens: &[Token],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> Option<usize> {
        for el in els {
            if let Some(len) = self.match_base(el, tokens, classes) {
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
    ) -> Option<usize> {
        let mut lengths = vec![];
        self.find_group_match_lengths(tokens, &pat.elements, classes, 0, &mut lengths);
        lengths.sort_unstable_by(|a, b| b.cmp(a));
        if let Some(&l) = lengths.first() {
            return Some(l);
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

        let Some(el) = pattern.first() else {
            return;
        };
        let Some(rest_pattern) = pattern.get(1..) else {
            return;
        };

        let match_lengths = self.get_group_match_lengths(el, tokens, classes, current_len);

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

    fn get_group_match_lengths(
        &self,
        el: &PatternElement,
        tokens: &[Token],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
        current_len: usize,
    ) -> Vec<usize> {
        let mut match_lengths = Vec::new();
        match el.quantifier {
            Quantifier::None => {
                if let Some(tokens_slice) = tokens.get(current_len..)
                    && let Some(len) = self.match_base(&el.base, tokens_slice, classes)
                {
                    match_lengths.push(len);
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
        match_lengths
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
