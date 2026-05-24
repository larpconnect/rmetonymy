Feature: Syllabification

  Scenario: Syllabify various IPA words
    Given the language configuration exists
    When I syllabify the IPA string "ˈfɑɹmɚ"
    Then the syllables should format to "ˈfɑɹ.mɚ"

    When I syllabify the IPA string "dɑːns"
    Then the syllables should format to "dɑːns"

    When I syllabify the IPA string "wɔkɪŋ"
    Then the syllables should format to "wɔ.kɪŋ"

    When I syllabify the IPA string "mankind"
    Then the syllables should format to "man.kind"

    When I syllabify the IPA string "ˈsliːp"
    Then the syllables should format to "ˈsliːp"

    When I syllabify the IPA string "ki̯el"
    Then the syllables should format to "ki̯el"

    When I syllabify the IPA string "kuo̯l"
    Then the syllables should format to "kuo̯l"

    When I syllabify the IPA string "kiel"
    Then the syllables should format to "ki.el"

    When I syllabify the IPA string "əmɛɹɪkən"
    Then the syllables should format to "ə.mɛɹ.ɪ.kən"

    When I syllabify the IPA string "ˈfɑːmə"
    Then the syllables should format to "ˈfɑː.mə"

    When I syllabify the IPA string "pəˈlɪtɪkəl"
    Then the syllables should format to "pəˈlɪt.ɪ.kəl"

    When I syllabify the IPA string "ˌæstrəˈnɒmɪkəl"
    Then the syllables should format to "ˌæs.trəˈnɒm.ɪ.kəl"

  Scenario: Syllabify with illegal onset patterns
    Given a language configuration with illegal onsets:
      | pattern |
      | $cz     |
    When I syllabify the IPA string "acza"
    Then the syllables should format to "ac.za"
