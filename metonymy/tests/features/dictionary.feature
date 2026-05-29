Feature: Dictionary Subcommand

  Scenario: Initializing a dictionary
    Given I have initialized a dictionary
    When I run dictionary command "dictionary print"
    Then the output should contain "Total Entries: 0"

  Scenario: Adding eras and words
    Given I have initialized a dictionary
    When I run dictionary command "dictionary add-era --era 1 --name a --description 'First Era'"
    Then the output should contain "Added era 1 with ID"
    When I run dictionary command "dictionary add --meaning red --definition pat --type adjective --era 1"
    Then the output should contain "Added word 'pat' (meaning: 'red') with ID"
    When I run dictionary command "dictionary print"
    Then the output should contain "Total Eras   : 1"
    And the output should contain "* Era 1 (ID:"
    And the output should contain "First Era"
    And the output should contain "Total Entries: 1"
    And the output should contain "Definition : /pat/"

  Scenario: Auto-creating eras when adding a word with unseen era
    Given I have initialized a dictionary
    When I run dictionary command "dictionary add --meaning red --definition pat --type adjective --era 5"
    Then the output should contain "Added word 'pat' (meaning: 'red') with ID"
    When I run dictionary command "dictionary print"
    Then the output should contain "Total Eras   : 1"
    And the output should contain "* Era 5 (ID:"

  Scenario: Looking up a word with and without derivations
    Given I have initialized a dictionary
    When I run dictionary command "dictionary add --meaning red --definition pat --type adjective --era 1"
    Then the output should contain "Added word 'pat' (meaning: 'red') with ID"
    When I run dictionary command "--language tests/features/test_language.json dictionary lookup red"
    Then the output should contain "pat"
    When I run dictionary command "--language tests/features/test_language.json dictionary lookup red-PLURAL"
    Then the output should contain escape-colored "red<RED>-PLURAL:adjective<RESET>"
    And the output should contain escape-colored "<RED>a<RESET>.pa.t<RED>i<RESET>"
    And the output should contain escape-colored "ˈ<RED>a<RESET>b.a.d<RED>a<RESET>"
    And the output should contain "abada"
