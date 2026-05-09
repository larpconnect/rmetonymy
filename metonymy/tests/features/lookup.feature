Feature: Lookup Subcommand

  Scenario: Looking up a valid base phoneme
    Given I have a basic setup
    When I run metonymy with "lookup --phoneme p"
    Then the output should contain "Base: p"
    And the output should contain "Features:"
    And the output should contain "Place:"
    And the output should contain "Manner:"

  Scenario: Looking up an invalid phoneme
    Given I have a basic setup
    When I run metonymy with "lookup --phoneme xyz"
    Then the output should contain "not found"
