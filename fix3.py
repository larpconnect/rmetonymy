import sys

def main():
    with open('language/src/phonotactics.rs', 'r') as f:
        rs = f.read()

    rs = rs.replace(
"""#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub const DEFAULT_OPTIONAL_PROBABILITY: u8 = 20;

pub const DEFAULT_OPTIONAL_PROBABILITY: u8 = 20;

pub const DEFAULT_OPTIONAL_PROBABILITY: u8 = 20;""", "pub const DEFAULT_OPTIONAL_PROBABILITY: u8 = 20;")

    with open('language/src/phonotactics.rs', 'w') as f:
        f.write(rs)

if __name__ == '__main__':
    main()
