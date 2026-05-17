use crate::config::SoundClass;
use crate::sound_class::SoundClassKey;
use data::{Feature, IpaEntry, SpeFeature};
use ipa::{IpaString, IpaSystem, combine_with_modifier, get_entry, get_phoneme_data};
use pest::Parser;
use pest_derive::Parser;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, HashSet};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

#[derive(Parser)]
#[grammar = "parser/matcher.pest"]
pub struct SoundMatcherParser;

#[derive(Error, Debug, PartialEq)]
pub enum SoundMatcherError {
    #[error("Failed to parse pattern: {0}")]
    ParseError(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Quantifier {
    ZeroOrMore,
    OneOrMore,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FeatureDescriptor {
    pub sign: bool, // true for +, false for -
    pub feature: Feature,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BaseElement {
    WordBoundary,
    SyllableBoundary,
    SoundClass(SoundClassKey),
    IpaSequence(IpaString),
    FeatureClass(Option<SoundClassKey>, Vec<FeatureDescriptor>),
    Set(Vec<BaseElement>), // Can only contain SoundClass or IpaSequence based on the pest grammar
    OptionalGroup(Box<SoundMatcherPattern>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PatternElement {
    pub base: BaseElement,
    pub quantifier: Quantifier,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SoundMatcherPattern {
    pub elements: Vec<PatternElement>,
}

impl FromStr for SoundMatcherPattern {
    type Err = SoundMatcherError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut pairs = SoundMatcherParser::parse(Rule::main, s)
            .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?;

        let main_pair = pairs
            .next()
            .ok_or_else(|| SoundMatcherError::ParseError("Empty input".to_string()))?;

        let mut pattern_pair = None;
        for pair in main_pair.into_inner() {
            if pair.as_rule() == Rule::pattern {
                pattern_pair = Some(pair);
                break;
            }
        }

        let Some(pattern_pair) = pattern_pair else {
            return Err(SoundMatcherError::ParseError("Empty pattern".to_string()));
        };

        parse_pattern(pattern_pair)
    }
}

fn parse_pattern(
    pair: pest::iterators::Pair<Rule>,
) -> Result<SoundMatcherPattern, SoundMatcherError> {
    let mut elements = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::pattern_element {
            elements.push(parse_pattern_element(inner)?);
        }
    }

    Ok(SoundMatcherPattern { elements })
}

fn parse_pattern_element(
    pair: pest::iterators::Pair<Rule>,
) -> Result<PatternElement, SoundMatcherError> {
    let mut quantifier = Quantifier::None;
    let mut base_element = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::zero_or_more => quantifier = Quantifier::ZeroOrMore,
            Rule::one_or_more => quantifier = Quantifier::OneOrMore,
            Rule::word_boundary => base_element = Some(BaseElement::WordBoundary),
            Rule::syllable_boundary => base_element = Some(BaseElement::SyllableBoundary),
            Rule::sound_class => {
                let key = inner
                    .as_str()
                    .parse::<SoundClassKey>()
                    .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?;
                base_element = Some(BaseElement::SoundClass(key));
            }
            Rule::ipa_sequence => {
                let ipa = inner
                    .as_str()
                    .parse::<IpaString>()
                    .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?;
                base_element = Some(BaseElement::IpaSequence(ipa));
            }
            Rule::feature_class => {
                let mut class_key = None;
                let mut features = Vec::new();
                for fc_inner in inner.into_inner() {
                    match fc_inner.as_rule() {
                        Rule::sound_class => {
                            class_key = Some(
                                fc_inner
                                    .as_str()
                                    .parse::<SoundClassKey>()
                                    .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?,
                            );
                        }
                        Rule::feature_descriptor => {
                            let mut sign = true;
                            let mut feature_name = "";
                            for fd_inner in fc_inner.into_inner() {
                                if fd_inner.as_rule() == Rule::feature_sign {
                                    sign = fd_inner.as_str() == "+";
                                } else if fd_inner.as_rule() == Rule::feature_name {
                                    feature_name = fd_inner.as_str();
                                }
                            }
                            let feature = Feature::from_str(feature_name).map_err(|e| {
                                SoundMatcherError::ParseError(format!(
                                    "Unknown feature: {}",
                                    feature_name
                                ))
                            })?;
                            features.push(FeatureDescriptor { sign, feature });
                        }
                        _ => {}
                    }
                }
                base_element = Some(BaseElement::FeatureClass(class_key, features));
            }
            Rule::set => {
                let mut set_elements = Vec::new();
                for set_inner in inner.into_inner() {
                    if set_inner.as_rule() == Rule::sound_class {
                        let key = set_inner
                            .as_str()
                            .parse::<SoundClassKey>()
                            .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?;
                        set_elements.push(BaseElement::SoundClass(key));
                    } else if set_inner.as_rule() == Rule::ipa_sequence {
                        let ipa = set_inner
                            .as_str()
                            .parse::<IpaString>()
                            .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?;
                        set_elements.push(BaseElement::IpaSequence(ipa));
                    }
                }
                base_element = Some(BaseElement::Set(set_elements));
            }
            Rule::optional_group => {
                for opt_inner in inner.into_inner() {
                    if opt_inner.as_rule() == Rule::pattern {
                        let pat = parse_pattern(opt_inner)?;
                        base_element = Some(BaseElement::OptionalGroup(Box::new(pat)));
                    }
                }
            }
            _ => {}
        }
    }

    let base = base_element
        .ok_or_else(|| SoundMatcherError::ParseError("Missing base element".to_string()))?;
    Ok(PatternElement { base, quantifier })
}

impl Display for SoundMatcherPattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for el in &self.elements {
            match &el.base {
                BaseElement::WordBoundary => write!(f, "#")?,
                BaseElement::SyllableBoundary => write!(f, "$")?,
                BaseElement::SoundClass(key) => write!(f, "{}", key)?,
                BaseElement::IpaSequence(ipa) => write!(f, "{}", ipa)?,
                BaseElement::FeatureClass(sc, features) => {
                    write!(f, "[")?;
                    if let Some(sc) = sc {
                        write!(f, "{} ", sc)?;
                    }
                    for (i, feat) in features.iter().enumerate() {
                        if i > 0 {
                            write!(f, " ")?;
                        }
                        let sign = if feat.sign { "+" } else { "-" };
                        write!(f, "{}{}", sign, feat.feature)?;
                    }
                    write!(f, "]")?;
                }
                BaseElement::Set(els) => {
                    write!(f, "{{")?;
                    for (i, set_el) in els.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        match set_el {
                            BaseElement::SoundClass(key) => write!(f, "{}", key)?,
                            BaseElement::IpaSequence(ipa) => write!(f, "{}", ipa)?,
                            _ => {}
                        }
                    }
                    write!(f, "}}")?;
                }
                BaseElement::OptionalGroup(pat) => write!(f, "({})", pat)?,
            }
            match el.quantifier {
                Quantifier::ZeroOrMore => write!(f, "*")?,
                Quantifier::OneOrMore => write!(f, "+")?,
                Quantifier::None => {}
            }
        }
        Ok(())
    }
}

impl Serialize for SoundMatcherPattern {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for SoundMatcherPattern {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<Self>().map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Boundary(String), // word boundary "#", syllable boundary "$", etc
    Phoneme(String),
}

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
                    // Quick check if next is a modifier or diacritic or part of phoneme sequence
                    if next_c == '.' || next_c == 'ˌ' || next_c == 'ˈ' || next_c == '\'' {
                        break;
                    }
                    let combined = format!("{}{}", phoneme, next_c);
                    if get_entry(&combined).is_some() {
                        phoneme = combined;
                        chars.next();
                    } else {
                        // Check if it's a known modifier
                        if get_entry(&next_c.to_string()).is_some() {
                            phoneme = combined;
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }
                // Even if not a valid phoneme by itself, just push it
                tokens.push(Token::Phoneme(phoneme));
            }
        }

        tokens.push(Token::Boundary("#".to_string()));
        tokens
    }

    pub fn matches(&self, word: &str, classes: &BTreeMap<SoundClassKey, SoundClass>) -> bool {
        let tokens = Self::tokenize(word);

        // We want to find any substring that matches. We can just test starting at every position.
        // The word boundary token is at the start and end of the `tokens` array.

        for i in 0..tokens.len() {
            if self.match_at(&tokens[i..], &self.elements, classes) {
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
            return true; // We successfully matched the pattern
        }

        let el = &pattern[0];
        let rest_pattern = &pattern[1..];

        // How many times can `el.base` match at the start of `tokens`?
        // We find all possible match lengths, then backtrack.
        let mut match_lengths = Vec::new();

        match el.quantifier {
            Quantifier::None => {
                if let Some(len) = self.match_base(&el.base, tokens, classes) {
                    match_lengths.push(len);
                }
            }
            Quantifier::ZeroOrMore => {
                match_lengths.push(0); // 0 times
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

        // Sort match_lengths descending for greedy matching
        match_lengths.sort_unstable_by(|a, b| b.cmp(a));
        match_lengths.dedup();

        for len in match_lengths {
            if self.match_at(&tokens[len..], rest_pattern, classes) {
                return true;
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
            if let Some(len) = self.match_base(base, &tokens[current_len..], classes) {
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

    fn match_base(
        &self,
        base: &BaseElement,
        tokens: &[Token],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> Option<usize> {
        if tokens.is_empty() {
            return None;
        }

        match base {
            BaseElement::WordBoundary => {
                if let Token::Boundary(ref b) = tokens[0] {
                    if b == "#" {
                        return Some(1);
                    }
                }
            }
            BaseElement::SyllableBoundary => {
                if let Token::Boundary(ref b) = tokens[0] {
                    if b == "$" {
                        return Some(1);
                    }
                }
            }
            BaseElement::SoundClass(key) => {
                if let Token::Phoneme(ref p) = tokens[0] {
                    if self.phoneme_in_class(p, key, classes) {
                        return Some(1);
                    }
                }
            }
            BaseElement::IpaSequence(ipa) => {
                // An IpaSequence might span multiple tokens or just one.
                // It's simpler if we check if the ipa sequence starts with our phoneme,
                // but actually IpaSequence in pattern is exact match string.
                // Let's reconstruct the sequence from tokens to match length.
                let target = ipa.as_str();
                let mut accumulated = String::new();
                let mut len = 0;
                for t in tokens {
                    if let Token::Phoneme(ref p) = t {
                        accumulated.push_str(p);
                        len += 1;
                        if accumulated == target {
                            return Some(len);
                        } else if target.starts_with(&accumulated) {
                            continue;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
            BaseElement::FeatureClass(sc_opt, features) => {
                if let Token::Phoneme(ref p) = tokens[0] {
                    // Check sound class first if present
                    if let Some(sc) = sc_opt {
                        if !self.phoneme_in_class(p, sc, classes) {
                            return None;
                        }
                    }

                    // Then check features
                    if let Some(phoneme_data) = get_phoneme_data(p) {
                        let mut has_all_features = true;

                        let mut p_features = Vec::new();
                        for feat in &phoneme_data.features {
                            p_features.push(feat.to_string().to_lowercase());
                        }

                        for fd in features {
                            let feat_name = fd.feature.to_string().to_lowercase();
                            let sign_str = if fd.sign { "+" } else { "-" };

                            // Check if the exact positive or negative feature exists.
                            // Some datasets only store positive features explicitly.
                            let mut found_positive = false;
                            for pf in &p_features {
                                if pf == &feat_name || pf.contains(&feat_name) {
                                    found_positive = true;
                                    break;
                                }
                            }

                            if fd.sign && !found_positive {
                                has_all_features = false;
                                break;
                            }
                            if !fd.sign && found_positive {
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
                if self.match_at(tokens, &pat.elements, classes) {
                    // Finding the length is tricky because match_at returns bool.
                    // But optional group shouldn't be matched like this; we should do full NFA or backtracking.
                    // A simple approximation is recursively using a helper that returns length.
                    // For now, let's just do a hacky length check.
                    let mut max_len = 0;
                    // Let's find all possible match lengths of pat against tokens.
                    let mut lengths = vec![];
                    self.find_group_match_lengths(tokens, &pat.elements, classes, 0, &mut lengths);
                    lengths.sort_unstable_by(|a, b| b.cmp(a));
                    if let Some(&l) = lengths.first() {
                        return Some(l);
                    }
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
        let rest_pattern = &pattern[1..];

        let mut match_lengths = Vec::new();
        match el.quantifier {
            Quantifier::None => {
                if let Some(len) = self.match_base(&el.base, &tokens[current_len..], classes) {
                    match_lengths.push(len);
                }
            }
            Quantifier::ZeroOrMore => {
                match_lengths.push(0);
                self.find_repeated_matches(
                    &el.base,
                    &tokens[current_len..],
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
                    &tokens[current_len..],
                    classes,
                    1,
                    usize::MAX,
                    0,
                    &mut match_lengths,
                );
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
        &self,
        p: &str,
        key: &SoundClassKey,
        classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> bool {
        // Handle defaults V, C, D
        if key.as_str() == "V" {
            // Is vowel?
            if let Some(entry) = get_entry(p) {
                return matches!(entry, IpaEntry::Vowel(_));
            }
            return ["a", "e", "i", "o", "u"].contains(&p); // Fallback
        }
        if key.as_str() == "C" {
            // Is consonant?
            if let Some(entry) = get_entry(p) {
                return matches!(entry, IpaEntry::Consonant(_));
            }
            return !["a", "e", "i", "o", "u"].contains(&p); // Fallback
        }
        if key.as_str() == "D" {
            // Is diphthong? (No clear IpaEntry type for diphthongs? Often they are two vowels)
            // Just assume not.
        }

        if let Some(sc) = classes.get(key) {
            return sc.values.contains(&p.to_string());
        }
        false
    }
}
