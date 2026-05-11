use crate::config::SoundClass;
use crate::sound_class::SoundClassKey;
use crate::sound_matcher::ast::{MatcherElement, QuantifiedElement, Quantifier, SoundMatcherPattern};
use data::SpeFeature;
use ipa::{get_entry, get_phoneme_data, IpaString};
use std::collections::BTreeMap;

impl SoundMatcherPattern {
    #[must_use]
    pub fn matches(
        &self,
        word: &IpaString,
        sound_classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> bool {
        let phonemes = extract_phonemes(word);

        for start_idx in 0..=phonemes.len() {
            if self.has_any_match(&self.elements, 0, &phonemes, start_idx, sound_classes) {
                return true;
            }
        }
        false
    }

    fn has_any_match(
        &self,
        elements: &[QuantifiedElement],
        elem_idx: usize,
        phonemes: &[&str],
        phoneme_idx: usize,
        sound_classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> bool {
        if elem_idx == elements.len() {
            return true;
        }
        let Some(current_elem) = elements.get(elem_idx) else {
            return false;
        };

        match current_elem.quantifier {
            None => self.match_no_quantifier(elements, elem_idx, phonemes, phoneme_idx, sound_classes),
            Some(Quantifier::ZeroOrMore) => self.match_zero_or_more(elements, elem_idx, phonemes, phoneme_idx, sound_classes),
            Some(Quantifier::OneOrMore) => self.match_one_or_more(elements, elem_idx, phonemes, phoneme_idx, sound_classes),
        }
    }

    fn get_single_matches(
        &self,
        current_elem: &QuantifiedElement,
        p_idx: usize,
        phonemes: &[&str],
        sound_classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> Vec<usize> {
        match &current_elem.element {
            MatcherElement::OptionalGroup(inner) => {
                self.match_subpattern(inner, phonemes, p_idx, sound_classes)
            }
            _ => {
                if let Some(idx) = self.match_single_element(&current_elem.element, phonemes, p_idx, sound_classes) {
                    vec![idx]
                } else {
                    vec![]
                }
            }
        }
    }

    fn match_no_quantifier(
        &self,
        elements: &[QuantifiedElement],
        elem_idx: usize,
        phonemes: &[&str],
        phoneme_idx: usize,
        sound_classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> bool {
        let Some(current_elem) = elements.get(elem_idx) else { return false; };
        for next_idx in self.get_single_matches(current_elem, phoneme_idx, phonemes, sound_classes) {
            if self.has_any_match(elements, elem_idx + 1, phonemes, next_idx, sound_classes) {
                return true;
            }
        }
        false
    }

    fn match_zero_or_more(
        &self,
        elements: &[QuantifiedElement],
        elem_idx: usize,
        phonemes: &[&str],
        phoneme_idx: usize,
        sound_classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> bool {
        if self.has_any_match(elements, elem_idx + 1, phonemes, phoneme_idx, sound_classes) {
            return true;
        }
        self.match_one_or_more(elements, elem_idx, phonemes, phoneme_idx, sound_classes)
    }

    fn match_one_or_more(
        &self,
        elements: &[QuantifiedElement],
        elem_idx: usize,
        phonemes: &[&str],
        phoneme_idx: usize,
        sound_classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> bool {
        let Some(current_elem) = elements.get(elem_idx) else { return false; };
        let mut queue = vec![phoneme_idx];
        let mut visited = vec![false; phonemes.len() + 1];
        if let Some(v) = visited.get_mut(phoneme_idx) { *v = true; }

        while let Some(curr) = queue.pop() {
            for next_idx in self.get_single_matches(current_elem, curr, phonemes, sound_classes) {
                if next_idx > curr && !visited.get(next_idx).copied().unwrap_or(false) {
                    if let Some(v) = visited.get_mut(next_idx) { *v = true; }
                    queue.push(next_idx);
                    if self.has_any_match(elements, elem_idx + 1, phonemes, next_idx, sound_classes) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn match_single_element(
        &self,
        element: &MatcherElement,
        phonemes: &[&str],
        idx: usize,
        sound_classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> Option<usize> {
        match element {
            MatcherElement::WordBoundary => Self::match_word_boundary(idx, phonemes.len()),
            MatcherElement::SyllableBoundary => Self::match_syllable_boundary(idx, phonemes),
            MatcherElement::SoundClass(key) => Self::match_sound_class(key, idx, phonemes, sound_classes),
            MatcherElement::Descriptor(sc_opt, features) => Self::match_descriptor(sc_opt.as_ref(), features, idx, phonemes, sound_classes),
            MatcherElement::IpaSequence(ipa) => Self::match_ipa_sequence(ipa, idx, phonemes),
            MatcherElement::Set(elements) => self.match_set(elements, idx, phonemes, sound_classes),
            MatcherElement::OptionalGroup(group_elements) => {
                let lengths = self.match_subpattern(group_elements, phonemes, idx, sound_classes);
                lengths.into_iter().next()
            }
        }
    }

    fn match_word_boundary(idx: usize, len: usize) -> Option<usize> {
        if idx == 0 || idx == len {
            Some(idx)
        } else {
            None
        }
    }

    fn match_syllable_boundary(idx: usize, phonemes: &[&str]) -> Option<usize> {
        if idx == 0 || idx == phonemes.len() {
            Some(idx)
        } else {
            let current = phonemes.get(idx)?;
            if matches!(*current, "." | "'" | "ˌ" | "ˈ") {
                Some(idx + 1)
            } else {
                None
            }
        }
    }

    fn match_sound_class(
        key: &SoundClassKey,
        idx: usize,
        phonemes: &[&str],
        sound_classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> Option<usize> {
        let current = phonemes.get(idx)?;
        if let Some(sc) = sound_classes.get(key) {
            if check_builtin_class(key.as_str(), current) || sc.values.iter().any(|v| v == current) {
                return Some(idx + 1);
            }
        } else if check_builtin_class(key.as_str(), current) {
            return Some(idx + 1);
        }
        None
    }

    fn match_descriptor(
        sc_opt: Option<&SoundClassKey>,
        features: &[SpeFeature],
        idx: usize,
        phonemes: &[&str],
        sound_classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> Option<usize> {
        let current = phonemes.get(idx)?;

        if let Some(key) = sc_opt {
            let matched_class = if let Some(sc) = sound_classes.get(key) {
                check_builtin_class(key.as_str(), current) || sc.values.iter().any(|v| v == current)
            } else {
                check_builtin_class(key.as_str(), current)
            };
            if !matched_class {
                return None;
            }
        }

        let phoneme_data = get_phoneme_data(current)?;
        for required_feat in features {
            match required_feat {
                SpeFeature::Plus(_) => {
                    if !phoneme_data.features.contains(required_feat) {
                        return None;
                    }
                }
                SpeFeature::Minus(feat) => {
                    let plus_feat = SpeFeature::Plus(*feat);
                    if phoneme_data.features.contains(&plus_feat) {
                        return None;
                    }
                }
            }
        }
        Some(idx + 1)
    }

    fn match_ipa_sequence(ipa: &IpaString, idx: usize, phonemes: &[&str]) -> Option<usize> {
        let ipa_phonemes = extract_phonemes_internal(ipa.as_str());
        if idx + ipa_phonemes.len() > phonemes.len() {
            return None;
        }
        for i in 0..ipa_phonemes.len() {
            if phonemes.get(idx + i).copied() != ipa_phonemes.get(i).copied() {
                return None;
            }
        }
        Some(idx + ipa_phonemes.len())
    }

    fn match_set(
        &self,
        elements: &[MatcherElement],
        idx: usize,
        phonemes: &[&str],
        sound_classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> Option<usize> {
        for elem in elements {
            if let Some(next_idx) = self.match_single_element(elem, phonemes, idx, sound_classes) {
                return Some(next_idx);
            }
        }
        None
    }

    fn match_subpattern(
        &self,
        elements: &[QuantifiedElement],
        phonemes: &[&str],
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
        phonemes: &[&str],
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
                let mut visited = vec![false; phonemes.len() + 1];
                if let Some(v) = visited.get_mut(phoneme_idx) { *v = true; }

                while let Some(curr) = queue.pop() {
                    for next_idx in get_single_matches(curr) {
                        if next_idx > curr && !visited.get(next_idx).copied().unwrap_or(false) {
                            if let Some(v) = visited.get_mut(next_idx) { *v = true; }
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
                let mut visited = vec![false; phonemes.len() + 1];
                if let Some(v) = visited.get_mut(phoneme_idx) { *v = true; }

                while let Some(curr) = queue.pop() {
                    for next_idx in get_single_matches(curr) {
                        if next_idx > curr && !visited.get(next_idx).copied().unwrap_or(false) {
                            if let Some(v) = visited.get_mut(next_idx) { *v = true; }
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

fn extract_phonemes(s: &IpaString) -> Vec<&str> {
    extract_phonemes_internal(s.as_str())
}

fn extract_phonemes_internal(s: &str) -> Vec<&str> {
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
                phonemes.push(substr);
                i += len;
                matched = true;
                break;
            }

            if get_entry(substr).is_some() {
                phonemes.push(substr);
                i += len;
                matched = true;
                break;
            }
        }
        if !matched {
            let start_idx_bytes = char_indices.get(i).map_or(0, |(idx, _)| *idx);
            let end_idx_bytes = char_indices.get(i + 1).map_or(s.len(), |(idx, _)| *idx);
            if let Some(sub) = s.get(start_idx_bytes..end_idx_bytes) {
                phonemes.push(sub);
            }
            i += 1;
        }
    }

    phonemes
}
