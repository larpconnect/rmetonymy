Feature: Syllabification Algorithms
  As a language creator
  I want to syllabify IPA strings
  So that I can analyze words according to phonetic rules

  Scenario Outline: Standard word syllabification
    Given the word "<word>"
    When it is syllabified
    Then the syllables should be "<syllables>"

    Examples:
      | word | syllables |
      | ˈfɑɹmɚ | ˈfɑɹ.mɚ |
      | dɑːns | dɑːns |
      | wɔkɪŋ | wɔ.kɪŋ |
      | ˈsliːp | ˈsliːp |
      | sliːpləs | sliː.pləs |
      | ai | a.i |
      | api | a.pi |
