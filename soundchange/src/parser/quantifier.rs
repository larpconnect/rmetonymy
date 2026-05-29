use crate::ast::MatchQuantifier;
use crate::parser::Rule;
use crate::parser::error::SoundChangeParseError;
use pest::iterators::Pair;

pub(crate) fn convert_quantifier(
    pair: Pair<'_, Rule>,
) -> Result<MatchQuantifier, SoundChangeParseError> {
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| SoundChangeParseError::ConversionError("Empty quantifier".to_string()))?;
    let text = inner.as_str();
    let (is_zero, num_str) = if let Some(stripped) = text.strip_prefix('*') {
        (true, stripped)
    } else if let Some(stripped) = text.strip_prefix('+') {
        (false, stripped)
    } else {
        return Err(SoundChangeParseError::ConversionError(format!(
            "Invalid quantifier: {text}"
        )));
    };
    let num_opt = if num_str.is_empty() {
        None
    } else {
        Some(num_str.parse::<u32>().map_err(|e| {
            SoundChangeParseError::ConversionError(format!("Invalid quantifier limit: {e}"))
        })?)
    };

    match (is_zero, num_opt) {
        (true, None) => Ok(MatchQuantifier::ZeroOrMore),
        (false, None) => Ok(MatchQuantifier::OneOrMore),
        (true, Some(n)) => Ok(MatchQuantifier::ZeroOrMoreBounded(n)),
        (false, Some(n)) => Ok(MatchQuantifier::OneOrMoreBounded(n)),
    }
}
