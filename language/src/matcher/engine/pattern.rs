// qual:allow(srp) - Pattern engine implementation
use crate::config::SoundClass;
use crate::matcher::ast::{BaseElement, PatternElement, Quantifier, SoundMatcherPattern, Token};
use crate::sound_class::SoundClassKey;
use std::collections::BTreeMap;

pub(crate) struct RepeatContext<'a> {
    pub(crate) base: &'a BaseElement,
    pub(crate) marker: Option<u8>,
    pub(crate) tokens: &'a [Token],
    pub(crate) classes: &'a BTreeMap<SoundClassKey, SoundClass>,
}

pub(crate) struct GroupMatchContext<'a> {
    pub(crate) tokens: &'a [Token],
    pub(crate) pattern: &'a [PatternElement],
    pub(crate) classes: &'a BTreeMap<SoundClassKey, SoundClass>,
}

#[derive(Clone, Copy)]
pub(crate) enum PatternStatus {
    Empty,
    NotEmpty,
}

impl SoundMatcherPattern {
    #[must_use]
    pub fn matches(&self, word: &str, classes: &BTreeMap<SoundClassKey, SoundClass>) -> bool {
        let tokens = Self::tokenize(word);
        self.matches_loop_integration(&tokens, classes)
    }

    fn matches_loop_integration(
        &self,
        tokens: &[Token],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
    ) -> bool {
        Self::matches_loop_op(tokens, |slice, bindings| {
            self.match_at(slice, &self.elements, classes, bindings)
        })
    }

    fn matches_loop_op<F>(tokens: &[Token], mut match_at_fn: F) -> bool
    where
        F: FnMut(&[Token], &mut BTreeMap<u8, Vec<Token>>) -> bool,
    {
        let mut bindings = BTreeMap::new();
        for i in 0..tokens.len() {
            if let Some(tokens_slice) = tokens.get(i..)
                && match_at_fn(tokens_slice, &mut bindings)
            {
                return true;
            }
            bindings.clear();
        }
        false
    }

    pub(crate) fn match_at(
        &self,
        tokens: &[Token],
        pattern: &[PatternElement],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
        bindings: &mut BTreeMap<u8, Vec<Token>>,
    ) -> bool {
        let Some((el, rest_pattern)) = Self::split_pattern_op(pattern) else {
            return true; // empty pattern is a match
        };
        let match_lengths = self.get_match_lengths(el, tokens, classes, bindings);
        self.match_at_loop_integration(tokens, rest_pattern, classes, bindings, match_lengths)
    }

    fn match_at_loop_integration(
        &self,
        tokens: &[Token],
        rest_pattern: &[PatternElement],
        classes: &BTreeMap<SoundClassKey, SoundClass>,
        bindings: &mut BTreeMap<u8, Vec<Token>>,
        match_lengths: Vec<(usize, BTreeMap<u8, Vec<Token>>)>,
    ) -> bool {
        let sorted_lengths = Self::sort_match_lengths_op(match_lengths);
        Self::match_at_loop_op(tokens, sorted_lengths, bindings, |slice, next_bindings| {
            self.match_at(slice, rest_pattern, classes, next_bindings)
        })
    }

    #[inline]
    fn split_pattern_op(
        pattern: &[PatternElement],
    ) -> Option<(&PatternElement, &[PatternElement])> {
        if pattern.is_empty() {
            None
        } else {
            let first = pattern.first()?;
            let rest = pattern.get(1..).unwrap_or(&[]);
            Some((first, rest))
        }
    }

    fn sort_match_lengths_op(
        mut match_lengths: Vec<(usize, BTreeMap<u8, Vec<Token>>)>,
    ) -> Vec<(usize, BTreeMap<u8, Vec<Token>>)> {
        match_lengths.sort_unstable_by_key(|b| std::cmp::Reverse(b.0));
        match_lengths.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
        match_lengths
    }

    fn match_at_loop_op<F>(
        tokens: &[Token],
        match_lengths: Vec<(usize, BTreeMap<u8, Vec<Token>>)>,
        bindings: &mut BTreeMap<u8, Vec<Token>>,
        mut match_at_fn: F,
    ) -> bool
    where
        F: FnMut(&[Token], &mut BTreeMap<u8, Vec<Token>>) -> bool,
    {
        for (len, next_bindings) in match_lengths {
            let mut temp_bindings = next_bindings;
            if let Some(tokens_slice) = tokens.get(len..)
                && match_at_fn(tokens_slice, &mut temp_bindings)
            {
                *bindings = temp_bindings;
                return true;
            }
        }
        false
    }

    pub(crate) fn get_match_lengths(
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
        self.dispatch_match_lengths_integration(&ctx, &el.quantifier, bindings, &mut match_lengths);
        match_lengths
    }

    fn dispatch_match_lengths_integration(
        &self,
        ctx: &RepeatContext<'_>,
        quantifier: &Quantifier,
        bindings: &BTreeMap<u8, Vec<Token>>,
        results: &mut Vec<(usize, BTreeMap<u8, Vec<Token>>)>,
    ) {
        match quantifier {
            Quantifier::None => {
                if let Some((len, next_bindings)) = self.match_base_with_bindings(
                    ctx.base,
                    ctx.marker,
                    ctx.tokens,
                    ctx.classes,
                    bindings,
                ) {
                    results.push((len, next_bindings));
                }
            }
            Quantifier::ZeroOrMore => {
                self.find_repeated_matches(ctx, (0, usize::MAX), 0, bindings, results);
            }
            Quantifier::OneOrMore => {
                self.find_repeated_matches(ctx, (1, usize::MAX), 0, bindings, results);
            }
        }
    }

    pub(crate) fn find_repeated_matches(
        &self,
        ctx: &RepeatContext<'_>,
        range: (usize, usize),
        current_len: usize,
        bindings: &BTreeMap<u8, Vec<Token>>,
        results: &mut Vec<(usize, BTreeMap<u8, Vec<Token>>)>,
    ) {
        let state = (range, current_len);
        let context = (ctx.tokens, bindings);
        Self::find_repeated_matches_op(
            state,
            context,
            results,
            |tokens_slice, current_bindings| {
                self.match_base_with_bindings(
                    ctx.base,
                    ctx.marker,
                    tokens_slice,
                    ctx.classes,
                    current_bindings,
                )
            },
            |next_range, next_len, next_bindings, res| {
                self.find_repeated_matches(ctx, next_range, next_len, next_bindings, res);
            },
        );
    }

    fn find_repeated_matches_op<F, G>(
        state: ((usize, usize), usize),
        context: (&[Token], &BTreeMap<u8, Vec<Token>>),
        results: &mut Vec<(usize, BTreeMap<u8, Vec<Token>>)>,
        mut match_base_fn: F,
        mut recurse_fn: G,
    ) where
        F: FnMut(&[Token], &BTreeMap<u8, Vec<Token>>) -> Option<(usize, BTreeMap<u8, Vec<Token>>)>,
        G: FnMut(
            (usize, usize),
            usize,
            &BTreeMap<u8, Vec<Token>>,
            &mut Vec<(usize, BTreeMap<u8, Vec<Token>>)>,
        ),
    {
        let ((min, max), current_len) = state;
        let (tokens, bindings) = context;
        if min == 0 {
            results.push((current_len, bindings.clone()));
        }

        if max > 0
            && let Some(tokens_slice) = tokens.get(current_len..)
            && let Some((len, next_bindings)) = match_base_fn(tokens_slice, bindings)
            && len > 0
        {
            let next_min = min.saturating_sub(1);
            recurse_fn(
                (next_min, max - 1),
                current_len + len,
                &next_bindings,
                results,
            );
        }
    }

    pub(crate) fn find_group_match_lengths(
        &self,
        ctx: &GroupMatchContext<'_>,
        current_len: usize,
        bindings: &BTreeMap<u8, Vec<Token>>,
        results: &mut Vec<(usize, BTreeMap<u8, Vec<Token>>)>,
    ) {
        let pattern_status =
            Self::check_pattern_empty_op(ctx.pattern, current_len, bindings, results);
        self.dispatch_group_match_integration(ctx, current_len, bindings, results, pattern_status);
    }

    fn dispatch_group_match_integration(
        &self,
        ctx: &GroupMatchContext<'_>,
        current_len: usize,
        bindings: &BTreeMap<u8, Vec<Token>>,
        results: &mut Vec<(usize, BTreeMap<u8, Vec<Token>>)>,
        pattern_status: PatternStatus,
    ) {
        Self::group_match_dispatch_op(pattern_status, || {
            let (el, rest_pattern) = Self::split_pattern_el_op(ctx.pattern);
            let match_lengths = self.get_match_lengths(el, ctx.tokens, ctx.classes, bindings);
            self.recurse_group_match_lengths_integration(
                ctx,
                rest_pattern,
                current_len,
                match_lengths,
                results,
            );
        });
    }

    fn group_match_dispatch_op<F>(pattern_status: PatternStatus, mut not_empty_fn: F)
    where
        F: FnMut(),
    {
        if let PatternStatus::NotEmpty = pattern_status {
            not_empty_fn();
        }
    }

    fn recurse_group_match_lengths_integration(
        &self,
        ctx: &GroupMatchContext<'_>,
        rest_pattern: &[PatternElement],
        current_len: usize,
        match_lengths: Vec<(usize, BTreeMap<u8, Vec<Token>>)>,
        results: &mut Vec<(usize, BTreeMap<u8, Vec<Token>>)>,
    ) {
        Self::recurse_group_match_op(
            ctx.tokens,
            match_lengths,
            |tokens_slice, len, next_bindings| {
                let sub_ctx = GroupMatchContext {
                    tokens: tokens_slice,
                    pattern: rest_pattern,
                    classes: ctx.classes,
                };
                self.find_group_match_lengths(&sub_ctx, current_len + len, next_bindings, results);
            },
        );
    }

    fn check_pattern_empty_op(
        pattern: &[PatternElement],
        current_len: usize,
        bindings: &BTreeMap<u8, Vec<Token>>,
        results: &mut Vec<(usize, BTreeMap<u8, Vec<Token>>)>,
    ) -> PatternStatus {
        if pattern.is_empty() {
            results.push((current_len, bindings.clone()));
            PatternStatus::Empty
        } else {
            PatternStatus::NotEmpty
        }
    }

    fn split_pattern_el_op(pattern: &[PatternElement]) -> (&PatternElement, &[PatternElement]) {
        let first = pattern.first().expect("pattern is not empty");
        let rest = pattern.get(1..).unwrap_or(&[]);
        (first, rest)
    }

    fn recurse_group_op<F>(
        tokens: &[Token],
        match_lengths: Vec<(usize, BTreeMap<u8, Vec<Token>>)>,
        mut recurse_fn: F,
    ) where
        F: FnMut(&[Token], usize, &BTreeMap<u8, Vec<Token>>),
    {
        for (len, next_bindings) in match_lengths {
            if let Some(tokens_slice) = tokens.get(len..) {
                recurse_fn(tokens_slice, len, &next_bindings);
            }
        }
    }

    fn recurse_group_match_op<F>(
        tokens: &[Token],
        match_lengths: Vec<(usize, BTreeMap<u8, Vec<Token>>)>,
        recurse_fn: F,
    ) where
        F: FnMut(&[Token], usize, &BTreeMap<u8, Vec<Token>>),
    {
        Self::recurse_group_op(tokens, match_lengths, recurse_fn);
    }
}
