import sys

def main():
    # Fix rust file
    with open('language/src/phonotactics.rs', 'r') as f:
        rs = f.read()

    rs = rs.replace('pub const DEFAULT_OPTIONAL_PROBABILITY: u8 = 20;\n\n#[derive(Debug, Clone, PartialEq, Eq, Hash)]\npub enum PhonotacticPattern {', 'pub const DEFAULT_OPTIONAL_PROBABILITY: u8 = 20;\n\n#[derive(Debug, Clone, PartialEq, Eq, Hash)]\npub enum PhonotacticPattern {')
    rs = rs.replace('if elements.len() == 1 {\n        Ok(elements.pop().expect("elements is guaranteed to have 1 item"))\n    } else if false {\n        Ok(el)', 'if elements.len() == 1 {\n        Ok(elements.pop().expect("elements is guaranteed to have 1 item"))')

    with open('language/src/phonotactics.rs', 'w') as f:
        f.write(rs)

if __name__ == '__main__':
    main()
