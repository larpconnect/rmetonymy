use super::{Phoneme, ProsodyMarker, SequenceElement, check_is_modifier};
use crate::ipa_string::IpaStringError;

pub struct PhonemeSegmentContext<'a> {
    pub chars: &'a [char],
    pub s: &'a str,
}

#[inline]
pub fn is_modifier(c: char) -> bool {
    check_is_modifier(c)
}

pub fn parse_phoneme_segment_op<F, G>(
    ctx: &PhonemeSegmentContext<'_>,
    start: usize,
    idx: &mut usize,
    mut is_base_phoneme: F,
    mut is_mod: G,
) -> Result<SequenceElement, IpaStringError>
where
    F: FnMut(&str) -> bool,
    G: FnMut(char) -> bool,
{
    let c = *ctx
        .chars
        .get(start)
        .ok_or_else(|| IpaStringError::InvalidSequence("Index out of bounds".to_string()))?;
    let len = (1..=(ctx.chars.len() - start))
        .rev()
        .find(|&len| {
            let slice = ctx.chars.get(start..(start + len)).unwrap_or(&[]);
            is_base_phoneme(&slice.iter().collect::<String>())
        })
        .ok_or_else(|| {
            IpaStringError::InvalidSequence(format!(
                "Unrecognized base phoneme starting with '{c}' at index {start} in string \"{}\"",
                ctx.s
            ))
        })?;
    let slice = ctx
        .chars
        .get(start..(start + len))
        .ok_or_else(|| IpaStringError::InvalidSequence("Slice out of bounds".to_string()))?;
    let base: String = slice.iter().collect();
    *idx += len;
    let mut modifiers = Vec::new();
    while *idx < ctx.chars.len() {
        let current_c = *ctx
            .chars
            .get(*idx)
            .ok_or_else(|| IpaStringError::InvalidSequence("Index out of bounds".to_string()))?;
        if is_mod(current_c) && !matches!(current_c, '\'' | 'ˈ' | 'ˌ' | '.') {
            modifiers.push(current_c.to_string());
            *idx += 1;
        } else {
            break;
        }
    }
    Ok(SequenceElement::Phoneme(Phoneme { base, modifiers }))
}

pub fn parse_single_element_op<F, G>(
    c: char,
    ctx: &PhonemeSegmentContext<'_>,
    idx: &mut usize,
    is_base_phoneme: &mut F,
    is_mod: &mut G,
) -> Result<Option<SequenceElement>, IpaStringError>
where
    F: FnMut(&str) -> bool,
    G: FnMut(char) -> bool,
{
    if let Some(stress) = match c {
        '\'' | 'ˈ' => Some(ProsodyMarker::PrimaryStress),
        'ˌ' => Some(ProsodyMarker::SecondaryStress),
        _ => None,
    } {
        *idx += 1;
        return Ok(Some(SequenceElement::Prosody(stress)));
    }
    if c == '.' {
        *idx += 1;
        return Ok(Some(SequenceElement::SyllableBreak));
    }
    if is_mod(c) {
        return Err(IpaStringError::InvalidSequence(format!(
            "Modifier '{c}' found without a preceding base phoneme at index {idx} in string \"{}\"",
            ctx.s
        )));
    }
    let elem = parse_phoneme_segment_op(ctx, *idx, idx, is_base_phoneme, is_mod)?;
    Ok(Some(elem))
}

pub fn parse_elements_op<F, G>(
    s: &str,
    mut is_base_phoneme: F,
    mut is_mod: G,
) -> Result<Vec<SequenceElement>, IpaStringError>
where
    F: FnMut(&str) -> bool,
    G: FnMut(char) -> bool,
{
    use unicode_normalization::UnicodeNormalization;
    if s.is_empty() {
        return Ok(Vec::new());
    }
    let chars: Vec<char> = s.nfd().collect::<String>().chars().collect();
    let mut elements = Vec::new();
    let mut idx = 0;
    let ctx = PhonemeSegmentContext { chars: &chars, s };
    while idx < chars.len() {
        let c = *chars
            .get(idx)
            .ok_or_else(|| IpaStringError::InvalidSequence("Index out of bounds".to_string()))?;
        if let Some(elem) =
            parse_single_element_op(c, &ctx, &mut idx, &mut is_base_phoneme, &mut is_mod)?
        {
            elements.push(elem);
        }
    }
    Ok(elements)
}
