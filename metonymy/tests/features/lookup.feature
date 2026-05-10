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

  Scenario: Looking up an affricate phoneme
    Given I have a basic setup
    When I run metonymy with "lookup --phoneme t͡s"
    Then the output should contain "Base: t͡s"
    And the output should contain "Manner: ["
    And the output should contain "affricate"

  Scenario: Looking up another affricate phoneme
    Given I have a basic setup
    When I run metonymy with "lookup --phoneme d͡ʒ"
    Then the output should contain "Base: d͡ʒ"
    And the output should contain "Manner: ["
    And the output should contain "affricate"

  Scenario: Looking up a trill phoneme
    Given I have a basic setup
    When I run metonymy with "lookup --phoneme ɽr"
    Then the output should contain "Base: ɽr"
    And the output should contain "Manner: ["
    And the output should contain "trill"
