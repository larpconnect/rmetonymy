import sys

def main():
    with open('language/src/phonotactics.rs', 'r') as f:
        rs = f.read()

    rs = rs.replace(
        '#[derive(Debug, Clone, PartialEq, Eq, Hash)]\npub enum PhonotacticPattern {',
        'pub const DEFAULT_OPTIONAL_PROBABILITY: u8 = 20;\n\n#[derive(Debug, Clone, PartialEq, Eq, Hash)]\npub enum PhonotacticPattern {'
    )
    rs = rs.replace('if *prob != 20 {', 'if *prob != DEFAULT_OPTIONAL_PROBABILITY {')
    rs = rs.replace('let mut prob = 20; // Default is 20%', 'let mut prob = DEFAULT_OPTIONAL_PROBABILITY;')
    rs = rs.replace('prob = s.parse::<u8>().unwrap_or(20);', 'prob = s.parse::<u8>().unwrap_or(DEFAULT_OPTIONAL_PROBABILITY);')

    # Fix let chains
    rs = rs.replace(
"""    if elements.len() == 1
        && let Some(el) = elements.pop()
    {
        Ok(el)
    } else {
        Ok(PhonotacticPattern::Sequence(elements))
    }""",
"""    if elements.len() == 1 {
        Ok(elements.pop().expect("elements is guaranteed to have 1 item"))
    } else {
        Ok(PhonotacticPattern::Sequence(elements))
    }"""
    )

    with open('language/src/phonotactics.rs', 'w') as f:
        f.write(rs)

if __name__ == '__main__':
    main()
