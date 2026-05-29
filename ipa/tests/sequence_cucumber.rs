use cucumber::{World, then, when};
use ipa::sequence::{PhonemeSequence, ProsodyMarker, SequenceElement};
use std::str::FromStr;

#[derive(Debug, Default, World)]
pub struct SequenceWorld {
    pub parsed: Option<Result<PhonemeSequence, ipa::ipa_string::IpaStringError>>,
}

#[when(expr = "I parse the IPA string {string}")]
fn when_parse_ipa_string(world: &mut SequenceWorld, s_param: String) {
    let s = s_param;
    world.parsed = Some(PhonemeSequence::from_str(&s));
}

#[when(expr = "I parse the invalid IPA string {string}")]
fn when_parse_invalid_ipa_string(world: &mut SequenceWorld, s_param: String) {
    let s = s_param;
    world.parsed = Some(PhonemeSequence::from_str(&s));
}

fn get_parsed_sequence(world: &SequenceWorld) -> Result<&PhonemeSequence, String> {
    let parsed_opt = world.parsed.as_ref();
    let res = parsed_opt.ok_or_else(|| "No parsed sequence found".to_string())?;
    res.as_ref().map_err(std::string::ToString::to_string)
}

#[then(expr = "the parsed sequence should have {int} elements")]
fn then_sequence_should_have_elements(
    world: &mut SequenceWorld,
    count: usize,
) -> Result<(), String> {
    let seq = get_parsed_sequence(world)?;
    if seq.elements.len() != count {
        return Err(format!(
            "Expected {count} elements, got {}",
            seq.elements.len()
        ));
    }
    Ok(())
}

fn get_element(seq: &PhonemeSequence, idx: usize) -> Result<&SequenceElement, String> {
    seq.elements
        .get(idx)
        .ok_or_else(|| format!("Index {idx} out of bounds"))
}

#[then(expr = "the element at index {int} should be PrimaryStress")]
fn then_element_primary_stress(world: &mut SequenceWorld, idx: usize) -> Result<(), String> {
    let seq = get_parsed_sequence(world)?;
    let el = get_element(seq, idx)?;
    if !matches!(el, SequenceElement::Prosody(ProsodyMarker::PrimaryStress)) {
        return Err(format!("Expected PrimaryStress, got {el:?}"));
    }
    Ok(())
}

#[then(expr = "the element at index {int} should be SyllableBreak")]
fn then_element_syllable_break(world: &mut SequenceWorld, idx: usize) -> Result<(), String> {
    let seq = get_parsed_sequence(world)?;
    let el = get_element(seq, idx)?;
    if !matches!(el, SequenceElement::SyllableBreak) {
        return Err(format!("Expected SyllableBreak, got {el:?}"));
    }
    Ok(())
}

#[then(expr = "the phoneme at index {int} should have base {string} and modifiers {string}")]
fn then_phoneme_has_base_modifiers(
    world: &mut SequenceWorld,
    idx: usize,
    base_param: String,
    modifiers_str_param: String,
) -> Result<(), String> {
    let base = base_param;
    let modifiers_str = modifiers_str_param;
    let seq = get_parsed_sequence(world)?;
    let el = get_element(seq, idx)?;
    let SequenceElement::Phoneme(p) = el else {
        return Err(format!("Element at index {idx} is not a Phoneme"));
    };
    if p.base != base {
        return Err(format!("Expected base {base}, got {}", p.base));
    }
    let expected_modifiers: Vec<String> = modifiers_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if p.modifiers != expected_modifiers {
        return Err(format!(
            "Expected modifiers {expected_modifiers:?}, got {:?}",
            p.modifiers
        ));
    }
    Ok(())
}

#[then(expr = "parsing should fail with an error containing {string}")]
fn then_parsing_should_fail(
    world: &mut SequenceWorld,
    err_part_param: String,
) -> Result<(), String> {
    let err_part = err_part_param;
    let parsed_opt = world.parsed.as_ref();
    let parsed_res = parsed_opt.ok_or_else(|| "No parsed sequence found".to_string())?;
    let Err(err) = parsed_res else {
        return Err("Expected parsing to fail but it succeeded".to_string());
    };
    if !err.to_string().contains(&err_part) {
        return Err(format!("Error '{err}' did not contain '{err_part}'"));
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if false {
        let mut world = SequenceWorld::default();
        when_parse_ipa_string(&mut world, String::new());
        when_parse_invalid_ipa_string(&mut world, String::new());
        let _ = then_sequence_should_have_elements(&mut world, 0);
        let _ = then_element_primary_stress(&mut world, 0);
        let _ = then_element_syllable_break(&mut world, 0);
        let _ = then_phoneme_has_base_modifiers(&mut world, 0, String::new(), String::new());
        let _ = then_parsing_should_fail(&mut world, String::new());
    }
    SequenceWorld::cucumber()
        .run_and_exit("tests/features/sequence.feature")
        .await;
}
