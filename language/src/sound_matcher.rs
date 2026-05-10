use crate::sound_class::SoundClassKey;

use data::SpeFeature;
use ipa::IpaString;
use pest::Parser;
use pest_derive::Parser;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

#[derive(Parser)]
#[grammar = "parser/sound_matcher.pest"]
pub struct SoundMatcherParser;

#[derive(Error, Debug, PartialEq)]
pub enum SoundMatcherError {
    #[error("Failed to parse pattern: {0}")]
    ParseError(String),
    #[error("Invalid feature name: {0}")]
    InvalidFeature(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Quantifier {
    ZeroOrMore,
    OneOrMore,
}

impl Display for Quantifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroOrMore => write!(f, "*"),
            Self::OneOrMore => write!(f, "+"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MatcherElement {
    WordBoundary,
    SyllableBoundary,
    SoundClass(SoundClassKey),
    Descriptor(Option<SoundClassKey>, Vec<SpeFeature>),
    IpaSequence(IpaString),
    Set(Vec<MatcherElement>),
    OptionalGroup(Vec<QuantifiedElement>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QuantifiedElement {
    pub element: MatcherElement,
    pub quantifier: Option<Quantifier>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SoundMatcherPattern {
    pub elements: Vec<QuantifiedElement>,
}

// Display logic ...

impl FromStr for SoundMatcherPattern {
    type Err = SoundMatcherError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut pairs = SoundMatcherParser::parse(Rule::main, s)
            .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?;

        let main_pair = pairs
            .next()
            .ok_or_else(|| SoundMatcherError::ParseError("Empty input".into()))?;
        let mut pattern_pair = None;
        for pair in main_pair.into_inner() {
            if pair.as_rule() == Rule::pattern {
                pattern_pair = Some(pair);
                break;
            }
        }

        let Some(pattern_pair) = pattern_pair else {
            return Err(SoundMatcherError::ParseError("Empty pattern".into()));
        };

        Ok(SoundMatcherPattern {
            elements: parse_pattern(pattern_pair)?,
        })
    }
}

fn parse_pattern(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Vec<QuantifiedElement>, SoundMatcherError> {
    let mut elements = Vec::new();
    for element_pair in pair.into_inner() {
        if element_pair.as_rule() == Rule::element {
            let mut inner = element_pair.into_inner();
            let base_pair = inner
                .next()
                .ok_or_else(|| SoundMatcherError::ParseError("Missing base pair".into()))?;
            let mut quantifier = None;
            if let Some(q_pair) = inner.next() {
                quantifier = match q_pair.as_str() {
                    "*" => Some(Quantifier::ZeroOrMore),
                    "+" => Some(Quantifier::OneOrMore),
                    _ => None,
                };
            }

            let element = parse_base_element(base_pair)?;
            elements.push(QuantifiedElement {
                element,
                quantifier,
            });
        }
    }
    Ok(elements)
}

fn parse_base_element(
    pair: pest::iterators::Pair<Rule>,
) -> Result<MatcherElement, SoundMatcherError> {
    match pair.as_rule() {
        Rule::word_boundary => Ok(MatcherElement::WordBoundary),
        Rule::syllable_boundary => Ok(MatcherElement::SyllableBoundary),
        Rule::sound_class => {
            let key = pair
                .as_str()
                .parse::<SoundClassKey>()
                .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?;
            Ok(MatcherElement::SoundClass(key))
        }
        Rule::ipa_sequence => {
            let ipa = pair
                .as_str()
                .parse::<IpaString>()
                .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?;
            Ok(MatcherElement::IpaSequence(ipa))
        }
        Rule::descriptor => {
            let mut sound_class = None;
            let mut features = Vec::new();
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::sound_class {
                    sound_class = Some(
                        inner
                            .as_str()
                            .parse::<SoundClassKey>()
                            .map_err(|e| SoundMatcherError::ParseError(e.to_string()))?,
                    );
                } else if inner.as_rule() == Rule::feature {
                    let mut sign = "+";
                    let mut name = "";
                    for f_inner in inner.into_inner() {
                        if f_inner.as_rule() == Rule::sign {
                            sign = f_inner.as_str();
                        } else if f_inner.as_rule() == Rule::feature_name {
                            name = f_inner.as_str();
                        }
                    }
                    // Handle aliasing `voiced` -> `voice` as per requirements
                    let mapped_name = if name == "voiced" { "voice" } else { name };
                    let feature_str = format!("{sign}{mapped_name}");
                    let spe_feature = feature_str
                        .parse::<SpeFeature>()
                        .map_err(SoundMatcherError::InvalidFeature)?;
                    features.push(spe_feature);
                }
            }
            Ok(MatcherElement::Descriptor(sound_class, features))
        }
        Rule::set => {
            let mut elements = Vec::new();
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::set_element {
                    let set_inner = inner
                        .into_inner()
                        .next()
                        .ok_or_else(|| SoundMatcherError::ParseError("Missing set inner".into()))?;
                    elements.push(parse_base_element(set_inner)?);
                }
            }
            Ok(MatcherElement::Set(elements))
        }
        Rule::optional_group => {
            let pattern_pair = pair
                .into_inner()
                .next()
                .ok_or_else(|| SoundMatcherError::ParseError("Missing group inner".into()))?;
            let elements = parse_pattern(pattern_pair)?;
            Ok(MatcherElement::OptionalGroup(elements))
        }
        _ => Err(SoundMatcherError::ParseError(format!(
            "Unexpected rule: {:?}",
            pair.as_rule()
        ))),
    }
}

// TODO Display, Serde logic, MatcherEngine...

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

impl Display for MatcherElement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WordBoundary => write!(f, "#"),
            Self::SyllableBoundary => write!(f, "$"),
            Self::SoundClass(sc) => write!(f, "{sc}"),
            Self::Descriptor(sc, features) => {
                write!(f, "[")?;
                if let Some(sc) = sc {
                    write!(f, "{sc} ")?;
                }
                for (i, feat) in features.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{feat}")?;
                }
                write!(f, "]")
            }
            Self::IpaSequence(ipa) => write!(f, "{ipa}"),
            Self::Set(elements) => {
                write!(f, "{{")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{elem}")?;
                }
                write!(f, "}}")
            }
            Self::OptionalGroup(elements) => {
                write!(f, "(")?;
                for elem in elements {
                    write!(f, "{elem}")?;
                }
                write!(f, ")")
            }
        }
    }
}

impl Display for QuantifiedElement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.element)?;
        if let Some(q) = &self.quantifier {
            write!(f, "{q}")?;
        }
        Ok(())
    }
}

impl Display for SoundMatcherPattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for elem in &self.elements {
            write!(f, "{elem}")?;
        }
        Ok(())
    }
}

use crate::config::SoundClass;
use ipa::{get_entry, get_phoneme_data};
use std::collections::BTreeMap;

impl SoundMatcherPattern {
    #[must_use]
    pub fn matches(
        &self,
        word: &IpaString,
        sound_classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> bool {
        let phonemes = extract_phonemes(word.as_str());

        for start_idx in 0..=phonemes.len() {
            let mut results = Vec::new();
            self.collect_subpattern_paths(
                &self.elements,
                0,
                &phonemes,
                start_idx,
                sound_classes,
                &mut results,
            );
            if !results.is_empty() {
                return true;
            }
        }
        false
    }

    #[expect(clippy::too_many_lines, reason = "Matching logic is complex")]
    fn match_single_element(
        &self,
        element: &MatcherElement,
        phonemes: &[String],
        idx: usize,
        sound_classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> Option<usize> {
        match element {
            MatcherElement::WordBoundary => {
                if idx == 0 || idx == phonemes.len() {
                    Some(idx) // Consumes 0 tokens
                } else {
                    None
                }
            }
            MatcherElement::SyllableBoundary => {
                if idx == 0 || idx == phonemes.len() {
                    Some(idx)
                } else {
                    let current = phonemes.get(idx)?;
                    if current == "." || current == "'" || current == "ˌ" || current == "ˈ" {
                        Some(idx + 1)
                    } else {
                        None
                    }
                }
            }
            MatcherElement::SoundClass(key) => {
                if idx >= phonemes.len() {
                    return None;
                }
                let current = phonemes.get(idx)?;

                if let Some(sc) = sound_classes.get(key) {
                    if check_builtin_class(key.as_str(), current) {
                        return Some(idx + 1);
                    }
                    if sc.values.contains(current) {
                        return Some(idx + 1);
                    }
                } else if check_builtin_class(key.as_str(), current) {
                    return Some(idx + 1);
                }
                None
            }
            MatcherElement::Descriptor(sc_opt, features) => {
                if idx >= phonemes.len() {
                    return None;
                }
                let current = phonemes.get(idx)?;

                if let Some(key) = sc_opt {
                    let mut matched_class = false;
                    if let Some(sc) = sound_classes.get(key) {
                        if check_builtin_class(key.as_str(), current) || sc.values.contains(current)
                        {
                            matched_class = true;
                        }
                    } else if check_builtin_class(key.as_str(), current) {
                        matched_class = true;
                    }
                    if !matched_class {
                        return None;
                    }
                }

                if let Some(phoneme_data) = get_phoneme_data(current) {
                    let mut satisfies = true;
                    for required_feat in features {
                        match required_feat {
                            SpeFeature::Plus(_) => {
                                if !phoneme_data.features.contains(required_feat) {
                                    satisfies = false;
                                    break;
                                }
                            }
                            SpeFeature::Minus(feat) => {
                                // A phoneme satisfies a minus feature if it DOES NOT contain the plus version.
                                // We also should ensure it doesn't contain the minus version (though usually negative features aren't explicit).
                                // Actually, if it contains the minus explicitly, it's fine.
                                // But if it contains the PLUS explicitly, it fails.
                                let plus_feat = SpeFeature::Plus(*feat);
                                if phoneme_data.features.contains(&plus_feat) {
                                    satisfies = false;
                                    break;
                                }
                            }
                        }
                    }
                    if satisfies {
                        return Some(idx + 1);
                    }
                }
                None
            }
            MatcherElement::IpaSequence(ipa) => {
                let ipa_str = ipa.as_str();
                let ipa_phonemes = extract_phonemes(ipa_str);

                if idx + ipa_phonemes.len() > phonemes.len() {
                    return None;
                }

                for i in 0..ipa_phonemes.len() {
                    if phonemes.get(idx + i) != ipa_phonemes.get(i) {
                        return None;
                    }
                }
                Some(idx + ipa_phonemes.len())
            }
            MatcherElement::Set(elements) => {
                for elem in elements {
                    if let Some(next_idx) =
                        self.match_single_element(elem, phonemes, idx, sound_classes)
                    {
                        return Some(next_idx);
                    }
                }
                None
            }
            MatcherElement::OptionalGroup(group_elements) => {
                let lengths = self.match_subpattern(group_elements, phonemes, idx, sound_classes);
                lengths.into_iter().next()
            }
        }
    }

    fn match_subpattern(
        &self,
        elements: &[QuantifiedElement],
        phonemes: &[String],
        idx: usize,
        sound_classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> Vec<usize> {
        let mut results = Vec::new();
        self.collect_subpattern_paths(elements, 0, phonemes, idx, sound_classes, &mut results);
        results.sort_by(|a, b| b.cmp(a));
        results.dedup();
        results
    }

    fn collect_subpattern_paths(
        &self,
        elements: &[QuantifiedElement],
        elem_idx: usize,
        phonemes: &[String],
        phoneme_idx: usize,
        sound_classes: &BTreeMap<SoundClassKey, SoundClass>,
        results: &mut Vec<usize>,
    ) {
        if elem_idx == elements.len() {
            results.push(phoneme_idx);
            return;
        }

        let Some(current_elem) = elements.get(elem_idx) else {
            return;
        };

        let get_single_matches = |p_idx: usize| -> Vec<usize> {
            match &current_elem.element {
                MatcherElement::OptionalGroup(inner) => {
                    self.match_subpattern(inner, phonemes, p_idx, sound_classes)
                }
                _ => {
                    if let Some(idx) = self.match_single_element(
                        &current_elem.element,
                        phonemes,
                        p_idx,
                        sound_classes,
                    ) {
                        vec![idx]
                    } else {
                        vec![]
                    }
                }
            }
        };

        match current_elem.quantifier {
            None => {
                for next_idx in get_single_matches(phoneme_idx) {
                    self.collect_subpattern_paths(
                        elements,
                        elem_idx + 1,
                        phonemes,
                        next_idx,
                        sound_classes,
                        results,
                    );
                }
            }
            Some(Quantifier::ZeroOrMore) => {
                self.collect_subpattern_paths(
                    elements,
                    elem_idx + 1,
                    phonemes,
                    phoneme_idx,
                    sound_classes,
                    results,
                );

                let mut queue = vec![phoneme_idx];
                let mut visited = std::collections::HashSet::new();
                visited.insert(phoneme_idx);

                while let Some(curr) = queue.pop() {
                    for next_idx in get_single_matches(curr) {
                        if next_idx > curr && !visited.contains(&next_idx) {
                            visited.insert(next_idx);
                            queue.push(next_idx);
                            self.collect_subpattern_paths(
                                elements,
                                elem_idx + 1,
                                phonemes,
                                next_idx,
                                sound_classes,
                                results,
                            );
                        }
                    }
                }
            }
            Some(Quantifier::OneOrMore) => {
                let mut queue = vec![phoneme_idx];
                let mut visited = std::collections::HashSet::new();
                visited.insert(phoneme_idx);

                while let Some(curr) = queue.pop() {
                    for next_idx in get_single_matches(curr) {
                        if next_idx > curr && !visited.contains(&next_idx) {
                            visited.insert(next_idx);
                            queue.push(next_idx);
                            self.collect_subpattern_paths(
                                elements,
                                elem_idx + 1,
                                phonemes,
                                next_idx,
                                sound_classes,
                                results,
                            );
                        }
                    }
                }
            }
        }
    }
}

fn check_builtin_class(key: &str, phoneme: &str) -> bool {
    let entry = get_entry(phoneme);
    match key {
        "C" => {
            if let Some(data::IpaEntry::Consonant(_)) = entry {
                return true;
            }
            if let Some(p) = get_phoneme_data(phoneme) {
                return p
                    .features
                    .contains(&SpeFeature::Plus(data::feature::Feature::Consonantal));
            }
            false
        }
        "V" => {
            if let Some(data::IpaEntry::Vowel(_)) = entry {
                return true;
            }
            if let Some(p) = get_phoneme_data(phoneme) {
                return p
                    .features
                    .contains(&SpeFeature::Plus(data::feature::Feature::Syllabic));
            }
            false
        }
        _ => false,
    }
}

fn extract_phonemes(s: &str) -> Vec<String> {
    let mut phonemes = Vec::new();
    let mut i = 0;
    let char_indices: Vec<(usize, char)> = s.char_indices().collect();
    let char_len = char_indices.len();

    while i < char_len {
        let mut matched = false;
        for len in (1..=char_len - i).rev() {
            let start_idx_bytes = char_indices.get(i).map_or(s.len(), |(idx, _)| *idx);
            let end_idx_bytes = char_indices.get(i + len).map_or(s.len(), |(idx, _)| *idx);

            let Some(substr) = s.get(start_idx_bytes..end_idx_bytes) else {
                break;
            };

            if substr == "." || substr == "'" || substr == "ˌ" || substr == "ˈ" {
                phonemes.push(substr.to_string());
                i += len;
                matched = true;
                break;
            }

            if get_entry(substr).is_some() {
                phonemes.push(substr.to_string());
                i += len;
                matched = true;
                break;
            }
        }
        if !matched {
            let start_idx_bytes = char_indices.get(i).map_or(0, |(idx, _)| *idx);
            let end_idx_bytes = char_indices.get(i + 1).map_or(s.len(), |(idx, _)| *idx);
            if let Some(sub) = s.get(start_idx_bytes..end_idx_bytes) {
                phonemes.push(sub.to_string());
            }
            i += 1;
        }
    }

    phonemes
}

#[cfg(test)]
mod tests {
    use crate::config::SoundClass;
    use crate::sound_class::SoundClassKey;
    use crate::sound_matcher::SoundMatcherPattern;
    use std::collections::BTreeMap;

    fn get_classes() -> BTreeMap<SoundClassKey, SoundClass> {
        let mut map = BTreeMap::new();
        map.insert(
            "C".parse().unwrap(),
            SoundClass {
                values: vec![],
                generator: None,
            },
        );
        map.insert(
            "V".parse().unwrap(),
            SoundClass {
                values: vec![],
                generator: None,
            },
        );
        map.insert(
            "F".parse().unwrap(),
            SoundClass {
                values: vec!["f".to_string(), "v".to_string()],
                generator: None,
            },
        );
        map
    }

    #[test]
    fn test_sound_matcher() {
        let sc = get_classes();

        let pattern1: SoundMatcherPattern = "aCa".parse().unwrap();
        assert!(pattern1.matches(&"alabama".parse().unwrap(), &sc));

        let pattern2: SoundMatcherPattern = "#aCa".parse().unwrap();
        assert!(pattern2.matches(&"alabama".parse().unwrap(), &sc));
        assert!(!pattern2.matches(&"balabama".parse().unwrap(), &sc));

        let pattern3: SoundMatcherPattern = "C+".parse().unwrap();
        assert!(pattern3.matches(&"str".parse().unwrap(), &sc));

        let pattern4: SoundMatcherPattern = "$ba".parse().unwrap();
        assert!(pattern4.matches(&"a.ba".parse().unwrap(), &sc)); // Matches after syllable boundary
        assert!(pattern4.matches(&"ba.a".parse().unwrap(), &sc)); // Matches start of word
        assert!(!pattern4.matches(&"aba".parse().unwrap(), &sc)); // Doesn't match middle

        let pattern5: SoundMatcherPattern = "[+voice]".parse().unwrap();
        assert!(pattern5.matches(&"b".parse().unwrap(), &sc));
        assert!(!pattern5.matches(&"p".parse().unwrap(), &sc));

        let pattern6: SoundMatcherPattern = "[F -voice]".parse().unwrap();
        assert!(pattern6.matches(&"f".parse().unwrap(), &sc));
        assert!(!pattern6.matches(&"v".parse().unwrap(), &sc));

        let pattern7: SoundMatcherPattern = "{a, b}".parse().unwrap();
        assert!(pattern7.matches(&"a".parse().unwrap(), &sc));
        assert!(pattern7.matches(&"b".parse().unwrap(), &sc));
        assert!(!pattern7.matches(&"c".parse().unwrap(), &sc));
    }
}
