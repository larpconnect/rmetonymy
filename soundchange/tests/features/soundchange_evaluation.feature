Feature: Sound Change Library Evaluation

  Scenario Outline: Evaluating basic sound changes
    Given a default language configuration
    When I apply sound change rule "<rule>" to the word "<input>"
    Then the result should be "<output>"

    Examples:
      | input       | rule                      | output       |
      | colorado    | a => o                    | colorodo     |
      | colorãdo    | a => o                    | colorãdo     |
      | colorãdo    | ã => õ                    | colorõdo     |
      | colorãdo    | aᴴ => o                   | colorodo     |
      | colorãdo    | aᴴ => oᴴ                  | colorõdo     |
      | colorado    | a => ∅                    | colordo      |
      | colorado    | C => r                    | rororaro     |
      | colorado    | C => __                   | ccollorraddo |
      | colorado    | CV => ka                  | kakakaka     |
      | colorado    | [+liquid] => _ː           | colːorːado   |
      | colorado    | [C +liquid] => _ː         | colːorːado   |
      | colorado    | [C -liquid] => _ː         | cːoloradːo   |
      | colorado    | C1V1 => V1C1              | ocolarod     |
      | colorado    | X => a                    | aaaaaaaa     |
      | colorado    | C(V) => i                 | iiii         |
      | colorado    | C{o,i} => i               | iirai        |
      | colorado    | d => [-voiced]            | colorato     |
      | colorado    | d => [_ -voiced]          | colorato     |
      | dg          | CC => [-voiced]           | tk           |

  Scenario Outline: Evaluating conditional sound changes
    Given a default language configuration
    When I apply sound change rule "<rule>" to the word "<input>"
    Then the result should be "<output>"

    Examples:
      | input       | rule                      | output       |
      | colorado    | C => k / _o               | kokorako     |
      | colorado    | V => i / _[+liquid]       | cilirado     |
      | colorado    | V => i / ~_[+liquid]      | coloridi     |
      | colorado    | C1V1 => __ / o_           | cololorarado |
      | colorado    | C => k / _                | kokokako     |
      | colorado    | ∅ => i / C_V              | ciolioriadio |

  Scenario Outline: Evaluating alpha notation sound changes
    Given a default language configuration
    When I apply sound change rule "<rule>" to the word "<input>"
    Then the result should be "<output>"

    Examples:
      | input       | rule                             | output       |
      | nk          | n => [_ α@place] / _[α@place]    | ŋk           |
      | dk          | d => [_ α@voiced] / _[α@voiced]  | tk           |
      | dtk         | d => i / _[α@voiced][α@voiced]   | itk          |
      | tk          | t => [_ -α@voiced] / _[α@voiced] | dk           |

  Scenario Outline: Evaluating advanced sound changes
    Given a default language configuration
    When I apply sound change rule "<rule>" to the word "<input>"
    Then the result should be "<output>"

    Examples:
      | input       | rule                      | output       |
      | mississippi | (C)CV => k                | kkkk         |
      | mississippi | (C)+V => k                | kkkk         |
      | mississippi | ((C)CV)+3 => k            | kk           |
      | colorado    | [^L +liquid] => k         | colokado     |
      | colorado    | CV => k / V_              | cokrak       |
      | colorado    | CV =:> k / V_             | cokkk        |
      | colorado    | o => t / C_C & _C*o       | ctlorado     |
      | colorado    | o => t / _C*o \| _C*a     | ctltrado     |
      | colorado    | o -> i                    | cilorado     |
      | colorado    | o <- i                    | coloradi     |

  Scenario Outline: Evaluating sound changes with syllable boundaries and stress
    Given a default language configuration
    When I apply sound change rule "<rule>" to the word "<input>" showing boundaries
    Then the boundary result should be "<output>"

    Examples:
      | input      | rule                                 | output      |
      | pa.ta.ka   | a => o / _$                          | po.to.ko    |
      | paˈta.ka   | [V +stress] => o                     | paˈto.ka    |
      | paˈta.ka   | [V -stress] => o                     | poˈta.ko    |
      | pa.ta.ka   | [V1] => [V1 +stress] / #C*_          | ˈpa.ta.ka   |
      | paˈta.ka   | [V1 +stress] => [V1 -stress]         | pa.ta.ka    |

  Scenario Outline: Validating syntax and parsing errors
    Given a default language configuration
    When I compile sound change rule "<rule>"
    Then it should fail validation with message containing "<error>"

    Examples:
      | rule               | error                                                       |
      | C => C / C_        | Unbound sound class                                         |
      | C => t / tk        | No use of the match                                         |
      | t => [_ -α@voiced] | used in transform but never captured                        |
      | ∅ => t             | Null match                                                  |
      | C -:> t            | Pest parse error                                            |
      | C <> t             | Pest parse error                                            |

  Scenario: Rule name cannot be a distinctive feature name
    Given a default language configuration
    When I compile a sound change rule named "nasal" with rule "a => o"
    Then it should fail validation with message containing "Rule name 'nasal' is a distinctive feature name"

  Scenario Outline: Evaluating orthography transforms
    Given a default language configuration
    When I apply orthography rule "<rule>" to the word "<input>"
    Then the orthography result should be "<output>"

    Examples:
      | input   | rule                                     | output |
      | ʐ       | ʐ => ẓ                                   | ẓ      |
      | j       | j => y                                   | y      |
      | gb      | g͡b => gb                                 | gb     |
      | kp      | k͡p => kp                                 | kp     |
      | ph      | pʰ => ph                                 | ph     |
      | aː      | V1ː => V1V1                              | aa     |
      | ki      | k => c / _i                              | ci     |
      | ka      | k => c / _i                              | ka     |

  Scenario Outline: Validating orthography syntax errors
    When I compile orthography rule "<rule>"
    Then it should fail orthography validation with message containing "<error>"

    Examples:
      | rule               | error                                                               |
      | C => C / C_        | Unbound sound class                                                 |
      | C => t / tk        | No use of the match                                                 |
      | ʐ => Ẓ             | Capital letters are banned                                          |
      | C => [+voiced]     | Distinctive feature transforms are not allowed                      |

  Scenario: Evaluating multiple orthography rules sequentially
    Given a default language configuration
    When I apply orthography rules "V1ː => V1̈" and "V1ᴴ => V1ᴴo" to the word "aː"
    Then the orthography result should be "äo"

  Scenario: Evaluating empty orthography configuration
    Given a default language configuration
    When I apply empty orthography to the word "pa.ta.ka"
    Then the orthography result should be "pataka"


