use soundchange::ast::ParsedSoundChange;
use soundchange::parse_rule_string;

#[test]
fn test_integration_parse_soundchange() {
    let parsed = parse_rule_string("a => o").expect("valid parse");
    assert!(matches!(parsed, ParsedSoundChange::Rule { .. }));
}
