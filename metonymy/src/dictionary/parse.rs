use language::generator::validation::is_valid_derivation_name;

pub(crate) fn parse_lookup_string(s: &str) -> (String, Vec<String>) {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() <= 1 {
        return (s.to_string(), Vec::new());
    }

    let mut parts = parts;
    let mut derivations = Vec::new();
    while parts.len() > 1 {
        if let Some(last) = parts.last() {
            if is_valid_derivation_name(last) {
                derivations.push((*last).to_string());
                parts.pop();
            } else {
                break;
            }
        }
    }
    derivations.reverse();
    let base_meaning = parts.join("-");
    (base_meaning, derivations)
}
