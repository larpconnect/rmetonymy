// qual:allow(srp) - Base engine implementation
use crate::config::SoundClass;
use crate::matcher::ast::{
    BaseElement, FeatureDescriptor, SoundMatcherPattern, Token,
};
use crate::sound_class::SoundClassKey;
use data::IpaEntry;
use ipa::{get_entry, get_phoneme_data};
use std::collections::BTreeMap;

pub(crate) struct BaseContext<'a> {
    pub(crate) base: &'a BaseElement,
    pub(crate) m: u8,
    pub(crate) tokens: &'a [Token],
    pub(crate) skip: usize,
    pub(crate) classes: &'a BTreeMap<SoundClassKey, SoundClass>,
}

impl SoundMatcherPattern {
    pub(crate) fn calculate_skip(base: &BaseElement, tokens: &[Token]) -> usize {
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

    pub(crate) fn match_unbound_base(
        &self,
        ctx: &BaseContext<'_>,
        temp_bindings: &mut BTreeMap<u8, Vec<Token>>,
    ) -> Option<(usize, BTreeMap<u8, Vec<Token>>)> {
        let len = self.match_base(ctx.base, ctx.tokens, ctx.classes, temp_bindings)?;
        Self::build_unbound_result_op(ctx.tokens, ctx.skip, len, ctx.m, temp_bindings)
    }

    fn build_unbound_result_op(
        tokens: &[Token],
        skip: usize,
        len: usize,
        marker: u8,
        temp_bindings: &mut BTreeMap<u8, Vec<Token>>,
    ) -> Option<(usize, BTreeMap<u8, Vec<Token>>)> {
        let matched_tokens = tokens.get(skip..len)?.to_vec();
        temp_bindings.insert(marker, matched_tokens);
        Some((len, temp_bindings.clone()))
    }

    pub(crate) fn match_marked_base(
        &self,
        ctx: &BaseContext<'_>,
        temp_bindings: &mut BTreeMap<u8, Vec<Token>>,
    ) -> Option<(usize, BTreeMap<u8, Vec<Token>>)> {
        let bound_opt = temp_bindings.get(&ctx.m).cloned();
        self.dispatch_marked_base_integration(ctx, temp_bindings, bound_opt)
    }

    fn dispatch_marked_base_integration(
        &self,
        ctx: &BaseContext<'_>,
        temp_bindings: &mut BTreeMap<u8, Vec<Token>>,
        bound_opt: Option<Vec<Token>>,
    ) -> Option<(usize, BTreeMap<u8, Vec<Token>>)> {
        match bound_opt {
            Some(bound) => {
                let slice = Self::check_bound_tokens_op(ctx.tokens, ctx.skip, &bound)?;
                let len = self.match_base(ctx.base, slice, ctx.classes, temp_bindings)?;
                Self::check_match_len_op(ctx.skip, &bound, len, temp_bindings)
            }
            None => self.match_unbound_base(ctx, temp_bindings),
        }
    }

    fn check_bound_tokens_op<'a>(
        tokens: &'a [Token],
        skip: usize,
        bound: &'a [Token],
    ) -> Option<&'a [Token]> {
        let tokens_to_check = tokens.get(skip..)?;
        if tokens_to_check.get(..bound.len()) == Some(bound) {
            Some(bound)
        } else {
            None
        }
    }

    fn check_match_len_op(
        skip: usize,
        bound: &[Token],
        len: usize,
        temp_bindings: &BTreeMap<u8, Vec<Token>>,
    ) -> Option<(usize, BTreeMap<u8, Vec<Token>>)> {
        if len == bound.len() {
            Some((skip + bound.len(), temp_bindings.clone()))
        } else {
            None
        }
    }

    pub(crate) fn match_base_with_bindings(
        &self,
        base: &BaseElement,
        marker: Option<u8>,
        tokens: &[Token],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
        bindings: &BTreeMap<u8, Vec<Token>>,
    ) -> Option<(usize, BTreeMap<u8, Vec<Token>>)> {
        let skip = Self::calculate_skip(base, tokens);
        let mut temp_bindings = bindings.clone();
        self.dispatch_base_with_bindings_integration(
            base,
            marker,
            (tokens, skip),
            classes,
            &mut temp_bindings,
        )
    }

    fn dispatch_base_with_bindings_integration(
        &self,
        base: &BaseElement,
        marker: Option<u8>,
        tokens_skip: (&[Token], usize),
        classes: &BTreeMap<SoundClassKey, SoundClass>,
        temp_bindings: &mut BTreeMap<u8, Vec<Token>>,
    ) -> Option<(usize, BTreeMap<u8, Vec<Token>>)> {
        let (tokens, skip) = tokens_skip;
        match marker {
            Some(m) => {
                let ctx = BaseContext {
                    base,
                    m,
                    tokens,
                    skip,
                    classes,
                };
                self.match_marked_base(&ctx, temp_bindings)
            }
            None => {
                let len = self.match_base(base, tokens, classes, temp_bindings)?;
                Some((len, temp_bindings.clone()))
            }
        }
    }

    pub(crate) fn match_base(
        &self,
        base: &BaseElement,
        tokens: &[Token],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
        bindings: &mut BTreeMap<u8, Vec<Token>>,
    ) -> Option<usize> {
        let skip = Self::calculate_skip(base, tokens);
        self.match_base_dispatch_integration(base, tokens, skip, classes, bindings)
    }

    fn match_base_dispatch_integration(
        &self,
        base: &BaseElement,
        tokens: &[Token],
        skip: usize,
        classes: &BTreeMap<SoundClassKey, SoundClass>,
        bindings: &mut BTreeMap<u8, Vec<Token>>,
    ) -> Option<usize> {
        let tokens_to_check = Self::get_slice_op(tokens, skip)?;
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
        Self::add_skip_op(len, skip)
    }

    #[inline]
    fn add_skip_op(len: Option<usize>, skip: usize) -> Option<usize> {
        len.map(|l| skip + l)
    }

    #[inline]
    fn get_slice_op(tokens: &[Token], start: usize) -> Option<&[Token]> {
        tokens.get(start..)
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
        let p_opt = Self::get_phoneme_str_op(first_token);
        Self::match_sound_class_integration(p_opt, key, classes)
    }

    fn match_sound_class_integration(
        p_opt: Option<&str>,
        key: &SoundClassKey,
        classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> Option<usize> {
        let p = p_opt?;
        let in_class = Self::phoneme_in_class(p, key, classes);
        Self::bool_to_option_len_op(in_class)
    }

    #[inline]
    fn get_phoneme_str_op(token: &Token) -> Option<&str> {
        if let Token::Phoneme(p) = token {
            Some(p)
        } else {
            None
        }
    }

    #[inline]
    fn bool_to_option_len_op(b: bool) -> Option<usize> {
        if b { Some(1) } else { None }
    }

    fn match_ipa_sequence(tokens: &[Token], ipa: &ipa::IpaString) -> Option<usize> {
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
        let p = Self::get_phoneme_str_op(first_token)?;
        Self::match_feature_class_integration(p, sc_opt, features, classes)
    }

    fn match_feature_class_integration(
        p: &str,
        sc_opt: Option<&SoundClassKey>,
        features: &[FeatureDescriptor],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> Option<usize> {
        let in_class = Self::match_sc_opt_integration(p, sc_opt, classes);
        let phoneme_data_opt = Self::get_phoneme_data_op(in_class, p);
        Self::match_features_from_data_integration(phoneme_data_opt, features)
    }

    #[inline]
    fn get_phoneme_data_op(in_class: bool, p: &str) -> Option<data::PhonemeData> {
        if !in_class {
            None
        } else {
            get_phoneme_data(p).cloned()
        }
    }

    fn match_features_from_data_integration(
        phoneme_data_opt: Option<data::PhonemeData>,
        features: &[FeatureDescriptor],
    ) -> Option<usize> {
        let has_all = Self::check_phoneme_features_from_opt_op(phoneme_data_opt, features);
        Self::bool_to_option_len_op(has_all)
    }

    fn check_phoneme_features_from_opt_op(
        phoneme_data_opt: Option<data::PhonemeData>,
        features: &[FeatureDescriptor],
    ) -> bool {
        let Some(phoneme_data) = phoneme_data_opt else {
            return false;
        };
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
                return false;
            }
        }
        true
    }

    fn match_sc_opt_integration(
        p: &str,
        sc_opt: Option<&SoundClassKey>,
        classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> bool {
        Self::match_sc_opt_dispatch_op(sc_opt, |sc| {
            Self::phoneme_in_class(p, sc, classes)
        })
    }

    fn match_sc_opt_dispatch_op<F>(sc_opt: Option<&SoundClassKey>, mut check_fn: F) -> bool
    where
        F: FnMut(&SoundClassKey) -> bool,
    {
        match sc_opt {
            Some(sc) => check_fn(sc),
            None => true,
        }
    }

    fn match_set(
        &self,
        els: &[BaseElement],
        tokens: &[Token],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
        bindings: &mut BTreeMap<u8, Vec<Token>>,
    ) -> Option<usize> {
        Self::match_set_op(els, |el| {
            self.match_base(el, tokens, classes, bindings)
        })
    }

    fn match_set_op<F>(els: &[BaseElement], mut match_fn: F) -> Option<usize>
    where
        F: FnMut(&BaseElement) -> Option<usize>,
    {
        for el in els {
            if let Some(len) = match_fn(el) {
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
        let ctx = crate::matcher::engine::pattern::GroupMatchContext {
            tokens,
            pattern: &pat.elements,
            classes,
        };
        self.find_group_match_lengths(&ctx, 0, bindings, &mut lengths);
        Self::extract_best_group_match_op(lengths, bindings)
    }

    fn extract_best_group_match_op(
        mut lengths: Vec<(usize, BTreeMap<u8, Vec<Token>>)>,
        bindings: &mut BTreeMap<u8, Vec<Token>>,
    ) -> Option<usize> {
        lengths.sort_unstable_by_key(|b| std::cmp::Reverse(b.0));
        if let Some((l, next_bindings)) = lengths.first() {
            *bindings = next_bindings.clone();
            return Some(*l);
        }
        None
    }

    pub(crate) fn phoneme_in_class(
        p: &str,
        key: &SoundClassKey,
        classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> bool {
        let entry = get_entry(p);
        Self::phoneme_in_class_op(p, key, classes, entry)
    }

    fn phoneme_in_class_op(
        p: &str,
        key: &SoundClassKey,
        classes: &BTreeMap<SoundClassKey, SoundClass>,
        entry: Option<&IpaEntry>,
    ) -> bool {
        match key.as_str() {
            "V" => entry.map_or(false, |e| matches!(e, IpaEntry::Vowel(_))),
            "C" => entry.map_or(false, |e| matches!(e, IpaEntry::Consonant(_))),
            "D" => entry.is_some() && (p.contains('\u{0361}') || p.contains('\u{035C}')),
            _ => {
                if let Some(sc) = classes.get(key) {
                    sc.values.iter().any(|val| val == p)
                } else {
                    false
                }
            }
        }
    }
}
