use crate::ast::{ConditionExpr, MatchPattern, Operator, ParsedMatchPart};
use crate::compiler::CompiledConditionExpr;
use crate::evaluator::{EvalContext, MatchState, WorkingWord};
use crate::parser::error::SoundChangeParseError;
use crate::parser::{Rule, SoundChangeParserInternal};
use ipa::sequence::IpaSequence;
use ipa::sequence::Phoneme;
use language::config::LanguageConfig;
use language::sound_class::SoundClassKey;
use language::syllable::IpaWord;
use pest::Parser;
use std::collections::BTreeSet;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrthoTransformElement {
    Empty,
    Literal {
        val: String,
        copy_modifiers: bool,
        append_modifiers: Vec<String>,
    },
    Ref {
        marker: Option<u8>,
        class_key: Option<SoundClassKey>,
        repeat: usize,
        copy_modifiers: bool,
        append_modifiers: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedOrthoRule {
    pub original_string: String,
    pub match_part: MatchPattern,
    pub operator: Operator,
    pub transform_part: Vec<OrthoTransformElement>,
    pub condition: Option<ConditionExpr>,
}

#[derive(Debug, Clone)]
pub struct CompiledOrthoRule {
    pub original_string: String,
    pub match_part: MatchPattern,
    pub operator: Operator,
    pub transform_part: Vec<OrthoTransformElement>,
    pub condition: Option<CompiledConditionExpr>,
}

/// Parses a single orthography rule string.
///
/// # Errors
/// Returns an error if parsing or structure validation fails.
pub fn parse_ortho_rule(s: &str) -> Result<ParsedOrthoRule, SoundChangeParseError> {
    use unicode_normalization::UnicodeNormalization;
    let s_normalized = s.nfd().collect::<String>();
    let s_trimmed = s_normalized.trim();
    if s_trimmed.is_empty() {
        return Err(SoundChangeParseError::ConversionError(
            "Empty input".to_string(),
        ));
    }

    let mut pairs = SoundChangeParserInternal::parse(Rule::sound_change, s_trimmed)
        .map_err(|e| SoundChangeParseError::PestError(e.to_string()))?;
    let main_pair = pairs
        .next()
        .ok_or_else(|| SoundChangeParseError::ConversionError("Empty input".to_string()))?;
    let inner = main_pair
        .into_inner()
        .next()
        .ok_or_else(|| SoundChangeParseError::ConversionError("No rules found".to_string()))?;

    match inner.as_rule() {
        Rule::reference_rule => Err(SoundChangeParseError::ValidationError(
            "Preamble references are not supported in orthography rules".to_string(),
        )),
        Rule::standard_rule => convert_standard_ortho_rule(inner, s_trimmed),
        _ => Err(SoundChangeParseError::ConversionError(format!(
            "Unexpected rule type {:?}",
            inner.as_rule()
        ))),
    }
}

fn convert_standard_ortho_rule(
    pair: pest::iterators::Pair<'_, Rule>,
    original: &str,
) -> Result<ParsedOrthoRule, SoundChangeParseError> {
    let mut match_part = None;
    let mut operator = Operator::RightMultipleTransparent;
    let mut transform_part = Vec::new();
    let mut condition = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::match_part => {
                match_part = Some(convert_ortho_match_part(inner)?);
            }
            Rule::arrow => {
                operator = crate::parser::parse_operator(inner.as_str())?;
            }
            Rule::transform_part => {
                transform_part = convert_ortho_transform_part(inner)?;
            }
            Rule::condition_expr => {
                condition = Some(crate::parser::condition::convert_condition_expr(inner)?);
            }
            _ => {}
        }
    }

    let match_part = match_part
        .map(|m| match m {
            ParsedMatchPart::Pattern(p) => Ok(p),
            ParsedMatchPart::Reference(_) => Err(SoundChangeParseError::ValidationError(
                "Preamble references not supported in orthography match part".to_string(),
            )),
        })
        .transpose()?
        .unwrap_or_else(|| MatchPattern {
            elements: Vec::new(),
        });

    Ok(ParsedOrthoRule {
        original_string: original.to_string(),
        match_part,
        operator,
        transform_part,
        condition,
    })
}

fn convert_ortho_match_part(
    pair: pest::iterators::Pair<'_, Rule>,
) -> Result<ParsedMatchPart, SoundChangeParseError> {
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| SoundChangeParseError::ConversionError("Empty match part".to_string()))?;
    match inner.as_rule() {
        Rule::reference_rule => {
            let name = crate::parser::pattern::convert_reference_rule(inner)?;
            Ok(ParsedMatchPart::Reference(name))
        }
        Rule::pattern => {
            let pattern = convert_ortho_pattern(inner)?;
            Ok(ParsedMatchPart::Pattern(pattern))
        }
        Rule::empty_symbol => Ok(ParsedMatchPart::Pattern(MatchPattern {
            elements: Vec::new(),
        })),
        _ => Err(SoundChangeParseError::ConversionError(format!(
            "Invalid match part rule: {:?}",
            inner.as_rule()
        ))),
    }
}

fn convert_ortho_pattern(
    pair: pest::iterators::Pair<'_, Rule>,
) -> Result<MatchPattern, SoundChangeParseError> {
    let mut elements = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::pattern_element {
            elements.push(convert_ortho_pattern_element(inner)?);
        }
    }
    Ok(MatchPattern { elements })
}

fn convert_ortho_pattern_element(
    pair: pest::iterators::Pair<'_, Rule>,
) -> Result<crate::ast::MatchElement, SoundChangeParseError> {
    let mut base = None;
    let mut modifiers_wildcard = false;
    let mut quantifier = crate::ast::MatchQuantifier::None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::modifier_wildcard => {
                modifiers_wildcard = true;
            }
            Rule::quantifier => {
                quantifier = crate::parser::pattern::convert_quantifier(inner)?;
            }
            rule => {
                base = Some(convert_ortho_base_element(inner, rule)?);
            }
        }
    }

    let base = base.ok_or_else(|| {
        SoundChangeParseError::ConversionError("Pattern element missing base".to_string())
    })?;

    Ok(crate::ast::MatchElement {
        base,
        modifiers_wildcard,
        quantifier,
    })
}

fn convert_ortho_base_element(
    pair: pest::iterators::Pair<'_, Rule>,
    rule: Rule,
) -> Result<crate::ast::MatchBase, SoundChangeParseError> {
    match rule {
        Rule::ipa_sequence => {
            let s = pair.as_str();
            let ipa = parse_ortho_ipa_string(s);
            Ok(crate::ast::MatchBase::IpaSequence(ipa))
        }
        _ => crate::parser::pattern::convert_base_element(pair, rule),
    }
}

fn parse_ortho_ipa_string(s: &str) -> ipa::IpaString {
    use std::str::FromStr;
    if let Ok(seq) = ipa::sequence::PhonemeSequence::from_str(s) {
        return ipa::IpaString::from(seq);
    }
    let mut elements = Vec::new();
    for c in s.chars() {
        elements.push(ipa::sequence::SequenceElement::Phoneme(Phoneme {
            base: c.to_string(),
            modifiers: Vec::new(),
        }));
    }
    ipa::IpaString::from(ipa::sequence::PhonemeSequence { elements })
}

fn convert_ortho_transform_part(
    pair: pest::iterators::Pair<'_, Rule>,
) -> Result<Vec<OrthoTransformElement>, SoundChangeParseError> {
    let inner = pair.into_inner().next().ok_or_else(|| {
        SoundChangeParseError::ConversionError("Empty transform part".to_string())
    })?;

    match inner.as_rule() {
        Rule::reference_rule => Err(SoundChangeParseError::ValidationError(
            "Preamble references are not supported in orthography rules".to_string(),
        )),
        Rule::transform_pattern => {
            let mut elements = Vec::new();
            for item in inner.into_inner() {
                if item.as_rule() == Rule::transform_element {
                    elements.push(convert_ortho_transform_element(item)?);
                }
            }
            Ok(elements)
        }
        Rule::empty_symbol => Ok(vec![OrthoTransformElement::Empty]),
        _ => Err(SoundChangeParseError::ConversionError(format!(
            "Invalid transform part rule: {:?}",
            inner.as_rule()
        ))),
    }
}

fn convert_ortho_ref_symbol(
    inner: pest::iterators::Pair<'_, Rule>,
    wildcard: bool,
    appends: Vec<String>,
) -> Result<OrthoTransformElement, SoundChangeParseError> {
    let (marker, class_key, repeat) =
        crate::parser::pattern::parse_transform_reference_symbol(inner)?;
    if let (None, Some(key)) = (marker, class_key.as_ref()) {
        let class_str = key.as_str();
        if class_str != "C" && class_str != "D" && class_str != "L" && class_str != "V" {
            return Err(SoundChangeParseError::ValidationError(format!(
                "Capital letters are banned in orthography rule transforms: '{class_str}'"
            )));
        }
    }
    Ok(OrthoTransformElement::Ref {
        marker,
        class_key,
        repeat,
        copy_modifiers: wildcard,
        append_modifiers: appends,
    })
}

fn convert_ortho_ipa_sequence(
    inner: &pest::iterators::Pair<'_, Rule>,
    wildcard: bool,
    appends: Vec<String>,
) -> Result<OrthoTransformElement, SoundChangeParseError> {
    let val = inner.as_str().to_string();
    if val.chars().any(|c| c.is_ascii_uppercase()) {
        return Err(SoundChangeParseError::ValidationError(format!(
            "Capital letters are banned in orthography rule transforms: '{val}'"
        )));
    }
    Ok(OrthoTransformElement::Literal {
        val,
        copy_modifiers: wildcard,
        append_modifiers: appends,
    })
}

fn convert_ortho_transform_element(
    pair: pest::iterators::Pair<'_, Rule>,
) -> Result<OrthoTransformElement, SoundChangeParseError> {
    let mut inner_pairs = pair.into_inner();
    let inner = inner_pairs.next().ok_or_else(|| {
        SoundChangeParseError::ConversionError("Empty transform element".to_string())
    })?;

    let (modifier_wildcard, append_modifiers) =
        crate::parser::transform::parse_transform_modifiers(inner_pairs);

    match inner.as_rule() {
        Rule::feature_class => Err(SoundChangeParseError::ValidationError(
            "Distinctive feature transforms are not allowed in orthography rules".to_string(),
        )),
        Rule::reference_symbol => {
            convert_ortho_ref_symbol(inner, modifier_wildcard, append_modifiers)
        }
        Rule::ipa_sequence => {
            convert_ortho_ipa_sequence(&inner, modifier_wildcard, append_modifiers)
        }
        _ => Err(SoundChangeParseError::ConversionError(format!(
            "Invalid transform element type: {:?}",
            inner.as_rule()
        ))),
    }
}

/// Compiles a set of orthography rules.
///
/// # Errors
/// Returns an error if any rule fails to parse or validate.
pub fn compile_ortho_rules(
    rules: &[String],
) -> Result<Vec<CompiledOrthoRule>, SoundChangeParseError> {
    let mut compiled = Vec::new();
    for rule_str in rules {
        let parsed = parse_ortho_rule(rule_str)?;
        let compiled_rule = compile_ortho_rule(parsed)?;
        compiled.push(compiled_rule);
    }
    Ok(compiled)
}

fn compile_ortho_rule(parsed: ParsedOrthoRule) -> Result<CompiledOrthoRule, SoundChangeParseError> {
    let cond = resolve_ortho_condition(parsed.condition)?;
    let compiled = CompiledOrthoRule {
        original_string: parsed.original_string,
        match_part: parsed.match_part,
        operator: parsed.operator,
        transform_part: parsed.transform_part,
        condition: cond,
    };
    validate_compiled_ortho_rule(&compiled)?;
    Ok(compiled)
}

fn resolve_ortho_condition(
    cond_opt: Option<ConditionExpr>,
) -> Result<Option<CompiledConditionExpr>, SoundChangeParseError> {
    let Some(cond) = cond_opt else {
        return Ok(None);
    };
    match cond {
        ConditionExpr::Reference(name) => Err(SoundChangeParseError::ReferenceError(format!(
            "Preamble reference '{name}' is not supported in orthography conditions"
        ))),
        ConditionExpr::Term { negated, pattern } => {
            Ok(Some(CompiledConditionExpr::Term { negated, pattern }))
        }
        ConditionExpr::Binary { left, op, right } => {
            let l = resolve_ortho_condition(Some(*left))?.ok_or_else(|| {
                SoundChangeParseError::ReferenceError(
                    "Empty left condition binary branch".to_string(),
                )
            })?;
            let r = resolve_ortho_condition(Some(*right))?.ok_or_else(|| {
                SoundChangeParseError::ReferenceError(
                    "Empty right condition binary branch".to_string(),
                )
            })?;
            Ok(Some(CompiledConditionExpr::Binary {
                left: Box::new(l),
                op,
                right: Box::new(r),
            }))
        }
    }
}

fn validate_compiled_ortho_rule(rule: &CompiledOrthoRule) -> Result<(), SoundChangeParseError> {
    validate_ortho_transform_bindings(rule)?;

    if let Some(ref cond) = rule.condition {
        crate::compiler::validation::validate_condition_has_placeholder(cond)?;
    }

    if rule.match_part.elements.is_empty() && rule.condition.is_none() {
        return Err(SoundChangeParseError::ValidationError(format!(
            "Null match (∅) in '{}' requires at least one condition.",
            rule.original_string
        )));
    }

    if matches!(
        rule.operator,
        Operator::RightSingleTransparent | Operator::LeftSingleTransparent
    ) && (rule.original_string.contains("-:>") || rule.original_string.contains("<-:"))
    {
        return Err(SoundChangeParseError::ValidationError(format!(
            "Opaque modifier (:) cannot be used with a single-change operator in '{}'.",
            rule.original_string
        )));
    }

    Ok(())
}

fn validate_ortho_transform_bindings(
    rule: &CompiledOrthoRule,
) -> Result<(), SoundChangeParseError> {
    let match_markers = crate::compiler::validation::get_match_markers(&rule.match_part);
    for el in &rule.transform_part {
        if let OrthoTransformElement::Ref {
            marker,
            class_key,
            repeat: _,
            ..
        } = el
        {
            if class_key.is_some() && marker.is_none() {
                return Err(SoundChangeParseError::ValidationError(format!(
                    "Unbound sound class in transform of '{}': all sound classes must have markers.",
                    rule.original_string
                )));
            }
            if let Some(m) = marker.filter(|m| !match_markers.contains(m)) {
                return Err(SoundChangeParseError::ValidationError(format!(
                    "Transform refers to marker '{m}' which is not bound in the match of '{}'.",
                    rule.original_string
                )));
            }
        }
    }
    Ok(())
}

fn flatten_phonemes_and_modifiers(phonemes: Vec<Phoneme>) -> Vec<Phoneme> {
    let mut flat_phonemes = Vec::new();
    for p in phonemes {
        flat_phonemes.push(Phoneme {
            base: p.base.clone(),
            modifiers: Vec::new(),
        });
        for m in p.modifiers {
            flat_phonemes.push(Phoneme {
                base: m,
                modifiers: Vec::new(),
            });
        }
    }
    flat_phonemes
}

/// Applies compiled orthography rules to an IPA word.
///
/// # Errors
/// Returns an error string if evaluation fails.
pub fn apply_orthography(
    word: &IpaWord,
    compiled_rules: &[CompiledOrthoRule],
    config: &LanguageConfig,
    verbose: bool,
) -> Result<(String, Vec<String>), String> {
    use ipa::sequence::IpaSequence;
    let flat_phonemes = flatten_phonemes_and_modifiers(word.phonemes());

    let mut working = WorkingWord {
        phonemes: flat_phonemes,
        syllable_boundaries: BTreeSet::new(),
        stress_index: None,
    };

    let ctx = EvalContext {
        classes: &config.phonology.sound_classes,
        system: ipa::DEFAULT_SYSTEM
            .as_ref()
            .map_err(|e| format!("Failed to load default IPA system: {e:?}"))?,
    };

    let mut trace_logs = Vec::new();
    if verbose {
        trace_logs.push("--- Orthography Transform ---".to_string());
    }

    for rule in compiled_rules {
        let before = working.clone();
        apply_ortho_rule(&mut working, rule, &ctx);
        if verbose && working != before {
            trace_logs.push(format!(
                "Ortho Rule: {}\n  In : {}\n  Out: {}",
                rule.original_string,
                before
                    .phonemes
                    .iter()
                    .map(ToString::to_string)
                    .collect::<String>(),
                working
                    .phonemes
                    .iter()
                    .map(ToString::to_string)
                    .collect::<String>()
            ));
        }
    }

    let flat: String = working.phonemes.iter().map(ToString::to_string).collect();

    Ok((flat, trace_logs))
}

fn apply_ortho_rule(word: &mut WorkingWord, rule: &CompiledOrthoRule, ctx: &EvalContext<'_>) {
    let is_leftward = matches!(
        rule.operator,
        Operator::LeftMultipleTransparent
            | Operator::LeftSingleTransparent
            | Operator::LeftMultipleOpaque
    );
    let is_opaque = matches!(
        rule.operator,
        Operator::RightMultipleOpaque | Operator::LeftMultipleOpaque
    );
    let is_single = matches!(
        rule.operator,
        Operator::RightSingleTransparent | Operator::LeftSingleTransparent
    );

    if is_opaque {
        apply_ortho_opaque_change(word, rule, is_leftward, ctx);
    } else {
        apply_ortho_transparent_change(word, rule, is_leftward, is_single, ctx);
    }
}

fn apply_ortho_opaque_change(
    word: &mut WorkingWord,
    rule: &CompiledOrthoRule,
    is_leftward: bool,
    ctx: &EvalContext<'_>,
) {
    let original_word = word.clone();
    let matches = crate::evaluator::engine::find_all_matches(
        &original_word,
        &rule.match_part,
        rule.condition.as_ref(),
        is_leftward,
        ctx,
    );

    let mut sorted_matches = matches;
    sorted_matches.sort_by_key(|b| std::cmp::Reverse(b.0.start));

    for (range, state) in sorted_matches {
        replace_ortho_range(word, range, &state, &rule.transform_part);
    }
}

fn apply_ortho_transparent_change(
    word: &mut WorkingWord,
    rule: &CompiledOrthoRule,
    is_leftward: bool,
    is_single: bool,
    ctx: &EvalContext<'_>,
) {
    let mut scan_idx = if is_leftward { word.phonemes.len() } else { 0 };

    loop {
        if is_leftward && scan_idx > word.phonemes.len() {
            scan_idx = word.phonemes.len();
        }

        let match_opt = crate::evaluator::engine::find_next_match(
            word,
            &rule.match_part,
            rule.condition.as_ref(),
            scan_idx,
            is_leftward,
            ctx,
        );
        let Some((range, state)) = match_opt else {
            break;
        };

        let new_range = replace_ortho_range(word, range.clone(), &state, &rule.transform_part);

        if is_single {
            break;
        }

        if is_leftward {
            if range.start == 0 {
                break;
            }
            scan_idx = range.start;
        } else {
            scan_idx = new_range.end;
            if scan_idx > word.phonemes.len() {
                break;
            }
        }
    }
}

fn replace_ortho_range(
    word: &mut WorkingWord,
    range: std::ops::Range<usize>,
    state: &MatchState,
    transform: &[OrthoTransformElement],
) -> std::ops::Range<usize> {
    let new_phonemes = build_ortho_transform_phonemes(transform, word, &range, state);
    let new_len = new_phonemes.len();

    word.phonemes.splice(range.clone(), new_phonemes);

    let original_len = range.end - range.start;
    let mut updated_boundaries = BTreeSet::new();
    for &b in &word.syllable_boundaries {
        if b < range.start {
            updated_boundaries.insert(b);
        } else if b >= range.end {
            updated_boundaries.insert(b - original_len + new_len);
        }
    }
    word.syllable_boundaries = updated_boundaries;

    range.start..range.start + new_len
}

fn eval_ortho_literal(
    el: &OrthoTransformElement,
    state: &MatchState,
    word: &WorkingWord,
) -> Vec<Phoneme> {
    let OrthoTransformElement::Literal {
        val,
        copy_modifiers,
        append_modifiers,
    } = el
    else {
        return Vec::new();
    };

    let parsed_seq = ipa::sequence::PhonemeSequence::from_str(val);
    let phonemes = if let Ok(seq) = parsed_seq {
        seq.phonemes().clone()
    } else {
        let mut phs = Vec::new();
        for c in val.chars() {
            phs.push(Phoneme {
                base: c.to_string(),
                modifiers: Vec::new(),
            });
        }
        phs
    };

    let mut result = Vec::new();
    for mut p in phonemes {
        if *copy_modifiers {
            for m in crate::evaluator::transform::get_captured_modifiers_for_element(state, 0, word)
            {
                if !p.modifiers.contains(&m) {
                    p.modifiers.push(m);
                }
            }
        }
        for m in append_modifiers {
            if !p.modifiers.contains(m) {
                p.modifiers.push(m.clone());
            }
        }
        result.push(p);
    }
    result
}

fn eval_ortho_ref(
    el: &OrthoTransformElement,
    word: &WorkingWord,
    range: &std::ops::Range<usize>,
    state: &MatchState,
) -> Vec<Phoneme> {
    let OrthoTransformElement::Ref {
        marker,
        class_key,
        repeat,
        copy_modifiers,
        append_modifiers,
    } = el
    else {
        return Vec::new();
    };

    let source_phonemes = crate::evaluator::transform::get_referenced_phonemes(
        word,
        *marker,
        class_key.as_ref(),
        state,
        range,
    );
    let mut result = Vec::new();
    for _ in 0..*repeat {
        for sp in &source_phonemes {
            let mut p = sp.clone();
            if *copy_modifiers {
                for m in
                    crate::evaluator::transform::get_captured_modifiers_for_element(state, 0, word)
                {
                    if !p.modifiers.contains(&m) {
                        p.modifiers.push(m);
                    }
                }
            }
            for m in append_modifiers {
                if !p.modifiers.contains(m) {
                    p.modifiers.push(m.clone());
                }
            }
            result.push(p);
        }
    }
    result
}

fn build_ortho_transform_phonemes(
    transform: &[OrthoTransformElement],
    word: &WorkingWord,
    range: &std::ops::Range<usize>,
    state: &MatchState,
) -> Vec<Phoneme> {
    let mut new_phonemes = Vec::new();

    for el in transform {
        match el {
            OrthoTransformElement::Empty => {}
            OrthoTransformElement::Literal { .. } => {
                new_phonemes.extend(eval_ortho_literal(el, state, word));
            }
            OrthoTransformElement::Ref { .. } => {
                new_phonemes.extend(eval_ortho_ref(el, word, range, state));
            }
        }
    }

    new_phonemes
}
