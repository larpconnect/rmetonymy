Feature: Sound Change Subcommand
  Scenario: Applying sound changes with explicit start and end eras
    Given I have a basic setup
    When I run metonymy with "--language tests/features/test_language.json sound-change --start 1 --end 1 pi.te.ki"
    Then the output should contain "ˈpad.a.ga"

  Scenario: Applying sound changes with default start and end eras
    Given I have a basic setup
    When I run metonymy with "--language tests/features/test_language.json sound-change pi.te.ki"
    Then the output should contain "ˈpad.a.ga"

  Scenario: Applying sound changes with verbose trace output
    Given I have a basic setup
    When I run metonymy with "--language tests/features/test_language.json sound-change pi.te.ki --verbose"
    Then the output should contain "vowel-lowering"
    And the output should contain "apply-lenition"
    And the output should contain "stress-first-vowel"
    And the output should contain "ˈpad.a.ga"
