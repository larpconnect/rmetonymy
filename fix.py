import sys

def main():
    # Fix pest file
    with open('language/src/parser/phonotactics.pest', 'r') as f:
        pest = f.read()

    pest = pest.replace('optional_group = { "(" ~ pattern+ ~ ")" ~ probability? }', 'optional_group = { "(" ~ pattern ~ ")" ~ probability? }')
    pest = pest.replace('ipa_char = _{ !( "(" | ")" | "%" | digit | sound_class_base ) ~ ANY }', 'ipa_char = _{ !( "(" | ")" | "%" | digit | sound_class_base | " " | "\\t" | "\\r" | "\\n" ) ~ ANY }')

    with open('language/src/parser/phonotactics.pest', 'w') as f:
        f.write(pest)

    # Fix rust file
    with open('language/src/phonotactics.rs', 'r') as f:
        rs = f.read()

    rs = rs.replace('pub enum PhonotacticPattern {', 'pub const DEFAULT_OPTIONAL_PROBABILITY: u8 = 20;\n\n#[derive(Debug, Clone, PartialEq, Eq, Hash)]\npub enum PhonotacticPattern {')
    rs = rs.replace('if *prob != 20 {', 'if *prob != DEFAULT_OPTIONAL_PROBABILITY {')
    rs = rs.replace('let mut prob = 20; // Default is 20%', 'let mut prob = DEFAULT_OPTIONAL_PROBABILITY;')
    rs = rs.replace('prob = s.parse::<u8>().unwrap_or(20);', 'prob = s.parse::<u8>().unwrap_or(DEFAULT_OPTIONAL_PROBABILITY);')
    rs = rs.replace('if elements.len() == 1\n        && let Some(el) = elements.pop()\n    {', 'if elements.len() == 1 {\n        Ok(elements.pop().expect("elements is guaranteed to have 1 item"))\n    } else if false {')

    with open('language/src/phonotactics.rs', 'w') as f:
        f.write(rs)

if __name__ == '__main__':
    main()
