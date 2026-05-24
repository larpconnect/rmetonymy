Feature: IPA Sequence Lexer

  Scenario: Parse stress marks and syllable breaks
    When I parse the IPA string "'sliːp.les"
    Then the parsed sequence should have 9 elements
    And the element at index 0 should be PrimaryStress
    And the element at index 5 should be SyllableBreak

  Scenario: Parse base phoneme with modifiers
    When I parse the IPA string "kʰʰɑʰːpː"
    Then the parsed sequence should have 3 elements
    And the phoneme at index 0 should have base "k" and modifiers "ʰ, ʰ"
    And the phoneme at index 1 should have base "ɑ" and modifiers "ʰ, ː"
    And the phoneme at index 2 should have base "p" and modifiers "ː"

  Scenario: Unrecognized base phoneme produces an error
    When I parse the invalid IPA string "p1a"
    Then parsing should fail with an error containing "Unrecognized base phoneme"

  Scenario: Modifier without base produces an error
    When I parse the invalid IPA string "ʰp"
    Then parsing should fail with an error containing "without a preceding base phoneme"
