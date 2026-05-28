---
name: Features and Command-Line Interface Reference
description: Guide explaining Metonymy's features, algorithms, and CLI.
target_audience: Conlang builders and developers integrating Metonymy into their workflows.
---

# Features and Command-Line Interface Reference

Metonymy (`rmetonymy`) is a command-line utility and linguistic library designed to assist conlang
developers in creating, evolving, and organizing constructed languages. It models real-world
phonology, historical sound change, syllabification, and lexicographical development.

This document details the core modules, their theoretical mechanisms, and how to invoke their
capabilities via the command-line interface.

---

## 1. IPA Phoneme and Feature System

Metonymy represents phonological systems using the International Phonetic Alphabet (IPA). Rather
than treating phonemes as arbitrary strings, the system parses them into structured feature vectors
based on the Sound Pattern of English (SPE) distinctive feature system.

### How it Works

*   **Distinctive Features**: Every phoneme is mapped to binary feature attributes (e.g.,
    `[+voiced]`, `[-nasal]`, `[+coronal]`, `[-sonorant]`).
*   **Acoustic Properties**: Place and manner of articulation are classified (e.g., Bilabial, Stop).
*   **Base and Modifier Composition**: The parser splits phonemes into a base character and one or
    more modifiers (diacritics like aspiration `ʰ`, gemination `ː`, or nasalization `~`).
    The final feature set is computed dynamically by applying modifier feature transformations
    to the base phoneme.
*   **System Definitions**: The default phonology mappings are defined in
    [ipa.json](../ipa/ipa.json) and validated against the schema
    [ipa_schema.json](../data/ipa_schema.json).

### CLI Command

Use the `lookup` subcommand to inspect a phoneme's properties:

```bash
cargo run --bin metonymy -- \
  [--phone-config <config_json>] \
  lookup --phoneme <symbol>
```

#### Example

```bash
cargo run --bin metonymy -- lookup --phoneme tʰ
```

This output displays the base symbol, modifiers, place, manner, and the combined distinctive
features vector. If `--phone-config` is omitted, the system falls back to the default IPA database.

---

## 2. Language Configuration

A language's phonetic rules, generators, and historical changes are centralized in a language
configuration JSON file. It conforms to the schema defined in
[language.schema.json](../language/language.schema.json).

### Structure

The language configuration is composed of:

*   **Identifiers and Metadata**: A unique UUID, endonym/exonym names, and timestamps.
*   **Sound Classes**: Custom phoneme groupings (e.g., `C` for consonants, `V` for vowels, `L` for
    liquids) mapped to lists of IPA symbols. Each class can specify a probability generator.
*   **Phonotactics**: Structural patterns (such as `CV(C)`) that define word-type schemas
    (e.g., `noun.masculine`, `verb`).
*   **Illegal Patterns**: Sound sequences restricted by the language's phonotactics.
*   **Prosody**: Global stress rules governing primary and secondary stress assignment.
*   **Sound Changes**: Chronological lists of sound changes grouped by historical era.

---

## 3. Syllabification and Prosody

Metonymy includes an automatic syllabification engine that parses an unstructured stream of IPA
phonemes into structured syllables. It then applies metrical stress rules to place primary (`ˈ`)
and secondary (`ˌ`) stress marks.

### How it Works

*   **Linguistic Principles**: The syllabification pipeline applies the *Sonority Sequencing
    Principle* (SSP), the *Maximal Onset Principle* (MOP), *Stressed Vowel Capture*, *Liquid Coda
    Constraints*, *Geminate Splitting*, and language-specific phonotactic blocks.
*   For complete details on these rules, see the dedicated guide:
    [Phonological Syllabification Rules](file:///home/clementsd/rmetonymy/docs/syllabification.md).
*   **Prosodic Stress Configurations**:
    *   `Unstressed`: Preserves existing stress markers but does not generate new ones.
    *   `NoFixedStress`: Places primary stress randomly using a Zipfian distribution, propagating
        secondary stress outward.
    *   `Alternating`: Assigns primary and alternating secondary stress relative to a target
        syllable (`FirstSyllable`, `SecondSyllable`, `Penultimate`, `Antepenultimate`, `Ultimate`).
    *   `Patterned`: Applies metrical foot configurations with parameters for foot size (binary `2`
        or ternary `3`), stress location (`1st`, `2nd`, `3rd`), and directionality (`First` or
        `Last` foot dominates).

---

## 4. Word Generation Engine

Metonymy provides a generative engine that creates phonetically valid conlang roots based on
syllable templates and probability distributions.

### How it Works

1.  **Template Expansion**: The engine loads the phonotactic patterns defined for the requested
    grammatical type (e.g., `CV(C)` expands to `C V` or `C V C`).
2.  **Phoneme Selection**: Elements are sampled from sound classes. The selection follows either:
    *   `Equiprobable`: All class members have an equal probability of selection.
    *   `Zipf`: Frequencies decay exponentially, modeling natural frequency distributions.
3.  **Syllabification and Stress**: The resulting string is syllabified and stressed based on the
    prosody model.
4.  **Phonotactic Filtering**: The candidate word is matched against `illegal_patterns`. If it is
    matched, the candidate is discarded, and the engine retries up to `max_attempts` (default: 8).
    *   *Backreferences*: Illegal patterns support backreferences (e.g., `C1VC1`), which enforce
        that identical indices must match the exact same phoneme.

### CLI Command

```bash
cargo run --bin metonymy -- \
  --language <lang_config_path> \
  generate [--max-attempts <num>] \
  word <definition> <grammatical_type>
```

#### Example

```bash
cargo run --bin metonymy -- \
  --language examples/lang.json \
  generate --max-attempts 12 \
  word "water" noun.masculine
```

---

## 5. Conlang Dictionary Management

The `dictionary` subcommand manages a lexicographical database representing the vocabulary of the
language across different historical epochs.

### How it Works

*   **Dictionary Schema**: Dictionaries conform to the JSON schema in
    [dictionary.schema.json](file:///home/clementsd/rmetonymy/language/dictionary.schema.json).
*   **Historical Eras**: Dictionary entries are linked to historical eras. Adding an entry to a new
    era automatically registers that era inside the dictionary.
*   **Atomic Writes**: Saves to dictionary files are performed atomically (using a temporary
    file and rename) to prevent corruption during unexpected terminations.
*   **Base62 Identifiers**: Each dictionary entry is assigned a unique Base62 UUID.
*   **Etymological Roots**: Words can maintain historical etymology chains linking back to ancestral
    forms in earlier eras.

### CLI Commands

#### Initialize a Dictionary

Creates a blank dictionary file matching a specific language's ID:

```bash
cargo run --bin metonymy -- \
  --dict <dict_path> \
  --language <lang_config_path> \
  dictionary init
```

#### Add a Word (Manual Definition)

Adds a vocabulary item with a manually defined IPA representation:

```bash
cargo run --bin metonymy -- \
  --dict <dict_path> \
  dictionary add \
  --meaning <meaning> \
  --definition <ipa_definition> \
  --type <type> \
  [--era <era_number>] \
  [--etymology <era:source_word,...>] \
  [--usage-notes <notes>]
```

#### Add a Word (Auto-Generated Definition)

Queries the language configuration to generate a phonetically valid word on the fly, adding it
to the dictionary:

```bash
cargo run --bin metonymy -- \
  --dict <dict_path> \
  --language <lang_config_path> \
  dictionary add \
  --meaning <meaning> \
  --generate \
  --type <type> \
  [--era <era_number>]
```

#### Remove a Word

Removes an entry from the dictionary using its Base62 ID:

```bash
cargo run --bin metonymy -- \
  --dict <dict_path> \
  dictionary remove <entry_id>
```

#### Pretty-Print the Dictionary

Prints a formatted list of all eras and dictionary entries to standard output:

```bash
cargo run --bin metonymy -- \
  --dict <dict_path> \
  dictionary print
```

#### Add a Custom Era

Manually defines metadata for a historical era:

```bash
cargo run --bin metonymy -- \
  --dict <dict_path> \
  dictionary add-era \
  [--era <era_number>] \
  [--name <ipa_name>] \
  [--description <desc>]
```

---

## 6. Historical Sound Change Simulation

The `sound-change` command allows conlang developers to simulate the phonological evolution of words
across eras.

### Sound Change Rules Syntax

Sound change rules are parsed using a custom PEGs grammar defined in
[soundchange.pest](file:///home/clementsd/rmetonymy/soundchange/src/parser/soundchange.pest).
The general shape of a rule is:

$$\text{Match} \quad \text{Operator} \quad \text{Transform} \quad / \quad \text{Condition}$$

#### Operators

*   `=>` or `>` : Rightward multiple transparent replacement.
*   `->` : Rightward single transparent replacement.
*   `=:>` : Rightward multiple opaque replacement.
*   `<=` or `<` : Leftward multiple transparent replacement.
*   `<-` : Leftward single transparent replacement.
*   `<:=` : Leftward multiple opaque replacement.

#### Transparent vs. Opaque Rule Application

Metonymy differentiates between *transparent* and *opaque* operators, changing how rules feed or
bleed their own context during simulation.

*   **Transparent (`=>`, `->`, `<=`, `<-`)**: Process matches sequentially (left-to-right or
    right-to-left). When a match is found, it is replaced immediately. The engine then scans
    the remaining portion of the *newly modified* word. This allows for feeding (a change
    creates a context for a subsequent change) and bleeding (a change destroys a context).
*   **Opaque (`=:>`, `<:=`)**: Process matches simultaneously. The engine scans the word,
    identifying all matching locations based *strictly* on the original, unmodified state
    of the word. Once all match locations are mapped, it applies all replacements in a
    single parallel step. Thus, changes in one syllable cannot trigger or prevent changes
    in adjacent syllables during the same rule pass.

##### Trace Comparison: `colorado`

Consider `/colorado/` (`c o l o r a d o`) and a rule replacing Consonant-Vowel (`CV`) sequences
with `k` when preceded by a vowel (`/ V_`):

*   **Transparent (`CV => k / V_`)**:
    1.  `lo` matches (preceded by vowel `o`). First change applied: `c o k r a d o`.
    2.  `ra` is now preceded by `k` (a consonant, not a vowel), so it no longer matches.
    3.  `do` matches (preceded by vowel `a`). Second change applied: `c o k r a k`.
    4.  **Result**: `cokrak`
*   **Opaque (`CV =:> k / V_`)**:
    1.  Matches identified on the original word: `lo` (preceded by `o`), `ra` (preceded by `o`),
        and `do` (preceded by `a`).
    2.  All matched segments replaced simultaneously: `c o [lo][ra][do]` &rarr; `c o [k][k][k]`.
    3.  **Result**: `cokkk`

#### Pattern Features

*   **Distinctive Features**: Rules can target and modify specific phonetic features (e.g.,
    `[+voiced] => _ː` or `d => [-voiced]`).
*   **Deletions and Insertions**: Represented by the empty symbol `∅`. For example,
    `a => ∅` deletes `/a/`, and `∅ => i / C_V` inserts `/i/` between a consonant and a vowel.
*   **Boundaries**: `#` represents a word boundary, while `$`, `.`, `ˌ`, `ˈ`, or `'` represent
    syllable boundaries.
*   **Quantifiers and Groups**: Quantifiers (`*`, `+`) allow matching repeating patterns. Optional
    parentheses `()` group matches.
*   **Agreement (Alpha Notation)**: Greek letter variables capture and align features between
    different phonemes (e.g., `n => [_ α@place] / _[α@place]` matches nasal place assimilation).

### CLI Command

Evaluates sound changes against a word:

```bash
cargo run --bin metonymy -- \
  --language <lang_config_path> \
  sound-change \
  [--start <start_era>] \
  [--end <end_era>] \
  [-v | --verbose] \
  <word_in_ipa>
```

The `--verbose` flag prints the state of the word after every applied sound change rule.
