Feature: Generate Subcommand

  Scenario: Generating a noun word
    Given I have a basic setup
    When I run metonymy with "--language tests/features/test_language.json generate word red noun"
    Then the output should contain a generated word for "red" as "noun"

  Scenario: Generating a verb word with default fallback
    Given I have a basic setup
    When I run metonymy with "--language tests/features/test_language.json generate word run verb"
    Then the output should contain a generated word for "run" as "verb"
