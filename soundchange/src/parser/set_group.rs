use super::pattern::convert_pattern;
use crate::ast::MatchBase;
use crate::parser::Rule;
use crate::parser::error::SoundChangeParseError;
use ipa::IpaString;
use pest::iterators::Pair;

pub(crate) fn convert_set(pair: Pair<'_, Rule>) -> Result<MatchBase, SoundChangeParseError> {
    let mut elements = Vec::new();
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::sound_class => {
                let key = inner
                    .as_str()
                    .parse::<language::sound_class::SoundClassKey>()
                    .map_err(|e| {
                        SoundChangeParseError::ConversionError(format!(
                            "Invalid sound class key: {e:?}"
                        ))
                    })?;
                elements.push(MatchBase::SoundClass { key, marker: None });
            }
            Rule::ipa_sequence => {
                let ipa = inner.as_str().parse::<IpaString>().map_err(|e| {
                    SoundChangeParseError::ConversionError(format!("Invalid IPA: {e:?}"))
                })?;
                elements.push(MatchBase::IpaSequence(ipa));
            }
            _ => {}
        }
    }
    Ok(MatchBase::Set(elements))
}

pub(crate) fn convert_optional_group(
    pair: Pair<'_, Rule>,
) -> Result<MatchBase, SoundChangeParseError> {
    let inner = pair.into_inner().next().ok_or_else(|| {
        SoundChangeParseError::ConversionError("Empty optional group".to_string())
    })?;
    let pattern = convert_pattern(inner)?;
    Ok(MatchBase::OptionalGroup(pattern))
}
