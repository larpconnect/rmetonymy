use crate::{IpaString, get_phoneme_data};
use data::SpeFeature;
use data::feature::Feature;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Stress {
    Unstressed,
    PrimaryStress,
    SecondaryStress,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Syllable {
    pub stress: Stress,
    pub phonemes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IpaWord {
    pub syllables: Vec<Syllable>,
}

fn is_vowel(phoneme: &str) -> bool {
    if let Some(data) = get_phoneme_data(phoneme) {
        data.features.contains(&SpeFeature::Plus(Feature::Syllabic))
    } else {
        false
    }
}

fn is_long_vowel(phoneme: &str) -> bool {
    phoneme.contains('ː') || phoneme.contains('ˑ')
}

fn is_rhotic(phoneme: &str) -> bool {
    if let Some(data) = get_phoneme_data(phoneme) {
        data.features.contains(&SpeFeature::Plus(Feature::Rhotic))
    } else {
        false
    }
}

fn is_liquid(phoneme: &str) -> bool {
    if let Some(data) = get_phoneme_data(phoneme) {
        data.features.contains(&SpeFeature::Plus(Feature::Liquid))
            || data.features.contains(&SpeFeature::Plus(Feature::Rhotic))
            || data.features.contains(&SpeFeature::Plus(Feature::Lateral))
    } else {
        false
    }
}

fn get_sonority(phoneme: &str) -> i32 {
    if let Some(data) = crate::get_phoneme_data(phoneme) {
        data.sonority
    } else {
        0
    }
}

impl IpaWord {
    #[must_use]
    #[expect(clippy::too_many_lines, reason = "complex algorithmic logic")]
    pub fn syllabify(ipa: &IpaString) -> Self {
        let s = ipa.as_str();
        if s.is_empty() {
            return Self { syllables: vec![] };
        }

        let char_indices: Vec<(usize, char)> = s.char_indices().collect();
        let char_len = char_indices.len();
        let mut i = 0;

        let mut parsed_segments = Vec::new();

        while i < char_len {
            let Some(&(start_idx, _)) = char_indices.get(i) else {
                break;
            };

            if s.get(start_idx..).is_some_and(|s| s.starts_with('.'))
                || s.get(start_idx..)
                    .is_some_and(|s| s.starts_with('\u{200B}'))
            {
                parsed_segments.push(".".to_string());
                let offset = 1;
                i += offset;
                continue;
            } else if s.get(start_idx..).is_some_and(|s| s.starts_with('ˈ'))
                || s.get(start_idx..).is_some_and(|s| s.starts_with('\''))
            {
                parsed_segments.push("ˈ".to_string());
                i += 1;
                continue;
            } else if s.get(start_idx..).is_some_and(|s| s.starts_with('ˌ')) {
                parsed_segments.push("ˌ".to_string());
                i += 1;
                continue;
            }

            let mut matched = false;
            for len in (1..=char_len - i).rev() {
                let start_idx_bytes = char_indices.get(i).map_or(s.len(), |(idx, _)| *idx);
                let end_idx_bytes = char_indices.get(i + len).map_or(s.len(), |(idx, _)| *idx);

                let substr = s.get(start_idx_bytes..end_idx_bytes).unwrap_or_default();
                if crate::get_entry(substr).is_some()
                    || substr == "ː"
                    || substr == "ˑ"
                    || substr == "w"
                {
                    if let Some(data::IpaEntry::Modifier(_)) = crate::get_entry(substr) {
                        if let Some(last) = parsed_segments.last_mut() {
                            if !last.starts_with('.')
                                && !last.starts_with('ˈ')
                                && !last.starts_with('ˌ')
                            {
                                last.push_str(substr);
                            } else {
                                parsed_segments.push(substr.to_string());
                            }
                        } else {
                            parsed_segments.push(substr.to_string());
                        }
                    } else {
                        parsed_segments.push(substr.to_string());
                    }
                    i += len;
                    matched = true;
                    break;
                }
            }
            if !matched {
                i += 1;
            }
        }

        let mut explicit_syllables = Vec::new();
        let mut current_explicit = Vec::new();
        let mut explicit_stress = Stress::Unstressed;

        for seg in parsed_segments {
            if seg == "." || seg == "ˈ" || seg == "ˌ" {
                if !current_explicit.is_empty() {
                    explicit_syllables.push((explicit_stress.clone(), current_explicit));
                    current_explicit = Vec::new();
                }
                if seg == "ˈ" {
                    explicit_stress = Stress::PrimaryStress;
                } else if seg == "ˌ" {
                    explicit_stress = Stress::SecondaryStress;
                } else {
                    explicit_stress = Stress::Unstressed;
                }
            } else {
                current_explicit.push(seg);
            }
        }
        if !current_explicit.is_empty() {
            explicit_syllables.push((explicit_stress, current_explicit));
        }

        let mut final_syllables = Vec::new();

        for (stress, group) in explicit_syllables {
            let mut vowel_indices = Vec::new();
            for (idx, p) in group.iter().enumerate() {
                if is_vowel(p) {
                    vowel_indices.push(idx);
                }
            }

            if vowel_indices.len() <= 1 {
                final_syllables.push(Syllable {
                    stress,
                    phonemes: group,
                });
                continue;
            }

            let mut breaks = Vec::new();
            for v_idx in 0..vowel_indices.len() - 1 {
                let Some(&v1) = vowel_indices.get(v_idx) else {
                    continue;
                };
                let Some(&v2) = vowel_indices.get(v_idx + 1) else {
                    continue;
                };
                let consonants_between = v2 - v1 - 1;

                let b_idx = match consonants_between {
                    0 => v1 + 1, // V.V
                    1 => {
                        let is_short = !is_long_vowel(group.get(v1).map_or("", String::as_str));
                        let is_stressed = (stress == Stress::PrimaryStress
                            || stress == Stress::SecondaryStress)
                            && v_idx == 0;
                        let c = group.get(v1 + 1).map_or("", String::as_str);

                        // "Keep liquids (r and l etc) together with a preceding vowel. So farmer becomes far.mer"
                        if is_liquid(c) || is_rhotic(c) || (is_short && is_stressed) {
                            v1 + 2 // VC.V
                        } else {
                            v1 + 1 // V.CV
                        }
                    }
                    2 => {
                        let c1_sonority =
                            get_sonority(group.get(v1 + 1).map_or("", String::as_str));
                        let c2_sonority =
                            get_sonority(group.get(v1 + 2).map_or("", String::as_str));

                        if c1_sonority < c2_sonority {
                            v1 + 1 // V.CCV (Sonority Sequencing Principle)
                        } else {
                            v1 + 2 // VC.CV
                        }
                    }
                    3 => {
                        v1 + 2 // VC.CCV
                    }
                    4 => {
                        v1 + 3 // VCC.CCV
                    }
                    _ => v1 + (consonants_between / 2) + 1,
                };
                breaks.push(b_idx);
            }

            let mut start = 0;
            for (i, &b) in breaks.iter().enumerate() {
                let sy_stress = if i == 0 {
                    stress.clone()
                } else {
                    Stress::Unstressed
                };
                final_syllables.push(Syllable {
                    stress: sy_stress,
                    phonemes: group.get(start..b).unwrap_or_default().to_vec(),
                });
                start = b;
            }
            let sy_stress = if breaks.is_empty() {
                stress
            } else {
                Stress::Unstressed
            };
            final_syllables.push(Syllable {
                stress: sy_stress,
                phonemes: group.get(start..).unwrap_or_default().to_vec(),
            });
        }

        Self {
            syllables: final_syllables,
        }
    }
}

impl Display for IpaWord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, syl) in self.syllables.iter().enumerate() {
            if i > 0 && syl.stress == Stress::Unstressed {
                write!(f, ".")?;
            }
            match syl.stress {
                Stress::PrimaryStress => write!(f, "ˈ")?,
                Stress::SecondaryStress => write!(f, "ˌ")?,
                Stress::Unstressed => {}
            }
            for p in &syl.phonemes {
                write!(f, "{p}")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_syllabification(word: &str, expected: &str) {
        let ipa_str = word.parse::<IpaString>().unwrap();
        let ipa_word = IpaWord::syllabify(&ipa_str);
        assert_eq!(ipa_word.to_string(), expected);
    }

    #[test]
    fn test_farmer() {
        test_syllabification("ˈfɑɹmɚ", "ˈfɑɹ.mɚ");
    }

    #[test]
    fn test_dance() {
        test_syllabification("dɑːns", "dɑːns");
    }

    #[test]
    fn test_walking() {
        test_syllabification("wɔkɪŋ", "wɔ.kɪŋ");
    }

    #[test]
    fn test_sleep() {
        test_syllabification("ˈsliːp", "ˈsliːp");
    }

    #[test]
    fn test_sleepless() {
        test_syllabification("sliːpləs", "sliː.pləs");
    }

    #[test]
    fn test_ai() {
        test_syllabification("ai", "a.i");
    }

    #[test]
    fn test_api() {
        test_syllabification("api", "a.pi");
    }
}
