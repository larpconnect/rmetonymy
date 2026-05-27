Feature: Sound Matcher
  As a language creator
  I want to be able to match words against phonological patterns
  So that I can identify invalid sequences

  Background:
    Given the following sound classes exist:
      | Class | Values |
      | C     | p,t,k  |
      | V     | a,e,i  |
      | F     | f,v,s,z|

  Scenario: Match literal characters
    When I check the pattern "aCa" against the word "alabama"
    Then the pattern should match

  Scenario: Match using word boundary
    When I check the pattern "#aCa" against the word "alabama"
    Then the pattern should match

    When I check the pattern "#aCa" against the word "balabama"
    Then the pattern should not match

  Scenario: Match using syllable boundary
    When I check the pattern "$ba" against the word "a.ba"
    Then the pattern should match

    When I check the pattern "$ba" against the word "ba.a"
    Then the pattern should match

    When I check the pattern "$ba" against the word "aba"
    Then the pattern should not match

  Scenario: Match using feature descriptors
    When I check the pattern "[+voiced]V" against the word "be"
    Then the pattern should match

    When I check the pattern "[-voiced]V" against the word "pe"
    Then the pattern should match

  Scenario: Match using feature descriptors with sound class
    When I check the pattern "[F -voiced]" against the word "f"
    Then the pattern should match

    When I check the pattern "[F -voiced]" against the word "v"
    Then the pattern should not match

  Scenario: Match using sets
    When I check the pattern "{a, b}" against the word "a"
    Then the pattern should match

    When I check the pattern "{a, b}" against the word "b"
    Then the pattern should match

    When I check the pattern "{a, b}" against the word "c"
    Then the pattern should not match

  Scenario: Match using quantifiers
    When I check the pattern "C+" against the word "str"
    Then the pattern should match

    When I check the pattern "(ta)*" against the word "tatata"
    Then the pattern should match

    When I check the pattern "V*" against the word "str"
    Then the pattern should match

  Scenario: Another syllable check
    When I check the pattern "aba" against the word "a.ba"
    Then the pattern should match

  Scenario: Match using marked items (backreferences)
    When I check the pattern "C1VC1" against the word "kak"
    Then the pattern should match

    When I check the pattern "C1VC1" against the word "kal"
    Then the pattern should not match

    When I check the pattern "C1C2VC2C1" against the word "ktatk"
    Then the pattern should match

    When I check the pattern "C1C2VC2C1" against the word "ktatp"
    Then the pattern should not match

    When I check the pattern "[C1 -voiced]V[C1 -voiced]" against the word "kak"
    Then the pattern should match

