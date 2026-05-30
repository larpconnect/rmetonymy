//! Parser utilities for pest grammar parsing.

use pest::RuleType;
use pest::iterators::{Pair, Pairs};

/// Extracts the target pattern pair from a main pair.
///
/// # Errors
/// Returns an error if the input is empty or does not contain the expected rule.
pub fn extract_pattern_pair_op<Rule>(
    mut pairs: Pairs<'_, Rule>,
    pattern_rule: Rule,
) -> Result<Pair<'_, Rule>, String>
where
    Rule: RuleType,
{
    let main_pair = pairs.next().ok_or_else(|| "Empty input".to_string())?;

    let mut pattern_pair = None;
    for pair in main_pair.into_inner() {
        if pair.as_rule() == pattern_rule {
            pattern_pair = Some(pair);
            break;
        }
    }

    pattern_pair.ok_or_else(|| "Empty pattern".to_string())
}
