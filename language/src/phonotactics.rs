use crate::sound_class::SoundClassKey;
use ipa::IpaString;
use pest::Parser;
use pest_derive::Parser;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

#[derive(Parser)]
#[grammar = "parser/phonotactics.pest"]
pub struct PhonotacticsParser;

#[derive(Error, Debug, PartialEq)]
pub enum PhonotacticsError {
    #[error("Failed to parse pattern: {0}")]
    ParseError(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PhonotacticPattern {
    Sequence(Vec<PhonotacticPattern>),
    SoundClass(SoundClassKey),
    IpaSequence(IpaString),
    OptionalGroup(Box<PhonotacticPattern>, u8), // pattern, probability
}

impl Display for PhonotacticPattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sequence(seq) => {
                for p in seq {
                    write!(f, "{p}")?;
                }
                Ok(())
            }
            Self::SoundClass(sc) => write!(f, "{sc}"),
            Self::IpaSequence(ipa) => write!(f, "{ipa}"),
            Self::OptionalGroup(pat, prob) => {
                write!(f, "({pat})")?;
                if *prob != 20 {
                    write!(f, "{prob}%")?;
                }
                Ok(())
            }
        }
    }
}

impl FromStr for PhonotacticPattern {
    type Err = PhonotacticsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut pairs = PhonotacticsParser::parse(Rule::main, s)
            .map_err(|e| PhonotacticsError::ParseError(e.to_string()))?;

        // main -> pattern -> EOI
        let main_pair = pairs
            .next()
            .ok_or_else(|| PhonotacticsError::ParseError("Empty input".to_string()))?;
        let mut pattern_pair = None;
        for pair in main_pair.into_inner() {
            if pair.as_rule() == Rule::pattern {
                pattern_pair = Some(pair);
                break;
            }
        }

        let Some(pattern_pair) = pattern_pair else {
            return Err(PhonotacticsError::ParseError("Empty pattern".to_string()));
        };

        parse_pattern(pattern_pair)
    }
}

fn parse_pattern(
    pair: pest::iterators::Pair<Rule>,
) -> Result<PhonotacticPattern, PhonotacticsError> {
    let mut elements = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::sound_class => {
                let key = inner
                    .as_str()
                    .parse::<SoundClassKey>()
                    .map_err(|e| PhonotacticsError::ParseError(e.to_string()))?;
                elements.push(PhonotacticPattern::SoundClass(key));
            }
            Rule::ipa_sequence => {
                let ipa = inner
                    .as_str()
                    .parse::<IpaString>()
                    .map_err(|e| PhonotacticsError::ParseError(e.to_string()))?;
                elements.push(PhonotacticPattern::IpaSequence(ipa));
            }
            Rule::optional_group => {
                let mut prob = 20; // Default is 20%
                let mut inner_pattern = None;

                for opt_inner in inner.into_inner() {
                    match opt_inner.as_rule() {
                        Rule::pattern => {
                            inner_pattern = Some(parse_pattern(opt_inner)?);
                        }
                        Rule::probability => {
                            let s = opt_inner.as_str();
                            let s = s.strip_suffix('%').unwrap_or(s);
                            prob = s.parse::<u8>().unwrap_or(20);
                        }
                        _ => {}
                    }
                }

                if let Some(pat) = inner_pattern {
                    elements.push(PhonotacticPattern::OptionalGroup(Box::new(pat), prob));
                }
            }
            _ => {}
        }
    }

    if elements.len() == 1
        && let Some(el) = elements.pop()
    {
        Ok(el)
    } else {
        Ok(PhonotacticPattern::Sequence(elements))
    }
}

impl Serialize for PhonotacticPattern {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for PhonotacticPattern {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<Self>().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_patterns() {
        let valid_cases = [
            "CVC",
            "CV(C)",
            "CD(L)C",
            "CD(L)50%C",
            "(C)VL",
            "(C)50%V(C)15%",
            "CV(CV)",
            "CV(C(V))",
            "SS(S)",
            "SSS",
            "CaC",
            "Cã(F)",
            "CV(C(V)10%)50%",
            "CtsC",
            "Ct͡sC",
            "CiːC",
            "(Ct)V",
        ];

        for case in valid_cases {
            let res = PhonotacticsParser::parse(Rule::main, case);
            assert!(res.is_ok(), "Failed to parse: {}", case);
        }
    }

    #[test]
    fn test_phonotactic_pattern_parsing() {
        let p = "CV(C)50%".parse::<PhonotacticPattern>().unwrap();
        if let PhonotacticPattern::Sequence(seq) = p {
            assert_eq!(seq.len(), 3);
            assert_eq!(seq[0], PhonotacticPattern::SoundClass("C".parse().unwrap()));
            assert_eq!(seq[1], PhonotacticPattern::SoundClass("V".parse().unwrap()));

            if let PhonotacticPattern::OptionalGroup(inner, prob) = &seq[2] {
                assert_eq!(
                    **inner,
                    PhonotacticPattern::SoundClass("C".parse().unwrap())
                );
                assert_eq!(*prob, 50);
            } else {
                panic!("Expected OptionalGroup");
            }
        } else {
            panic!("Expected Sequence");
        }
    }

    #[test]
    fn test_invalid_patterns() {
        let invalid_cases = [
            "C V C",     // whitespace not allowed
            "CV(C)100%", // % cannot be > 99 (digits limited to 2)
            "()",        // empty optional group
            "CV(C)%",    // missing digits before %
            "12C",       // unexpected digits outside probability
            "50%C",      // % applied to something without parens
            "C@",        // invalid ipa character
            "C(V))",     // unbalanced parens
        ];

        for case in invalid_cases {
            let res = case.parse::<PhonotacticPattern>();
            assert!(res.is_err(), "Should have failed to parse: {}", case);
        }
    }
}
