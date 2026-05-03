import json

data = {}

places = {
    'bilabial': {'features': ['+labial', '+bilabial', '-coronal', '-dorsal', '-pharyngeal'], 'place': ['bilabial']},
    'labiodental': {'features': ['+labial', '+labiodental', '-coronal', '-dorsal', '-pharyngeal'], 'place': ['labiodental']},
    'dental': {'features': ['-labial', '+coronal', '+anterior', '+distributed', '-dorsal', '-pharyngeal'], 'place': ['dental']},
    'alveolar': {'features': ['-labial', '+coronal', '+anterior', '-distributed', '-dorsal', '-pharyngeal'], 'place': ['alveolar']},
    'postalveolar': {'features': ['-labial', '+coronal', '-anterior', '+distributed', '-dorsal', '-pharyngeal'], 'place': ['postalveolar']},
    'retroflex': {'features': ['-labial', '+coronal', '-anterior', '-distributed', '-dorsal', '-pharyngeal'], 'place': ['retroflex']},
    'palatal': {'features': ['-labial', '-coronal', '+dorsal', '+high', '-low', '+front', '-back', '-pharyngeal'], 'place': ['palatal']},
    'velar': {'features': ['-labial', '-coronal', '+dorsal', '+high', '-low', '-front', '+back', '-pharyngeal'], 'place': ['velar']},
    'uvular': {'features': ['-labial', '-coronal', '+dorsal', '-high', '+low', '-front', '+back', '-pharyngeal'], 'place': ['uvular']},
    'pharyngeal': {'features': ['-labial', '-coronal', '-dorsal', '+pharyngeal', '+radical'], 'place': ['pharyngeal']},
    'glottal': {'features': ['-labial', '-coronal', '-dorsal', '-pharyngeal'], 'place': ['glottal']},
}

manners = {
    'plosive': {'features': ['-syllabic', '+consonantal', '-sonorant', '-continuant', '-delayed_release', '+stop'], 'manner': ['plosive']},
    'nasal': {'features': ['-syllabic', '+consonantal', '+sonorant', '-continuant', '+nasal'], 'manner': ['nasal']},
    'trill': {'features': ['-syllabic', '+consonantal', '+sonorant', '+continuant', '+trill'], 'manner': ['trill']},
    'tap': {'features': ['-syllabic', '+consonantal', '+sonorant', '+continuant', '+tap'], 'manner': ['tap']},
    'fricative': {'features': ['-syllabic', '+consonantal', '-sonorant', '+continuant', '+delayed_release'], 'manner': ['fricative']},
    'lateral_fricative': {'features': ['-syllabic', '+consonantal', '-sonorant', '+continuant', '+delayed_release', '+lateral'], 'manner': ['lateral', 'fricative']},
    'approximant': {'features': ['-syllabic', '-consonantal', '+sonorant', '+continuant', '+approximant'], 'manner': ['approximant']},
    'lateral_approximant': {'features': ['-syllabic', '+consonantal', '+sonorant', '+continuant', '+approximant', '+lateral'], 'manner': ['lateral', 'approximant']},

    # Non-pulmonic
    'click': {'features': ['-syllabic', '+consonantal', '-sonorant', '-continuant', '+delayed_release'], 'manner': ['click']},
    'implosive': {'features': ['-syllabic', '+consonantal', '-sonorant', '-continuant', '+stop'], 'manner': ['implosive']},
    'ejective': {'features': ['-syllabic', '+consonantal', '-sonorant', '-continuant', '+stop', '+constricted_glottis'], 'manner': ['ejective']},
    'ejective_fricative': {'features': ['-syllabic', '+consonantal', '-sonorant', '+continuant', '+delayed_release', '+constricted_glottis'], 'manner': ['ejective', 'fricative']},
}

def make_consonant(symbol, place, manner, voiced, extra_features=None, aliases=None):
    if aliases is None: aliases = []
    if extra_features is None: extra_features = []

    f = []
    f.extend(places[place]['features'])
    f.extend(manners[manner]['features'])

    # Voicing
    if voiced is True:
        f.append('+voice')
    elif voiced is False:
        f.append('-voice')

    for ef in extra_features:
        # replace existing feature if there's a conflict
        base_f = ef[1:]
        f = [x for x in f if x[1:] != base_f]
        f.append(ef)

    return {
        'type': 'consonant',
        'features': f,
        'place': places[place]['place'],
        'manner': manners[manner]['manner'],
        'aliases': aliases
    }

pulmonic = [
    # Plosives
    ('p', 'bilabial', 'plosive', False),
    ('b', 'bilabial', 'plosive', True),
    ('t', 'alveolar', 'plosive', False),
    ('d', 'alveolar', 'plosive', True),
    ('ʈ', 'retroflex', 'plosive', False),
    ('ɖ', 'retroflex', 'plosive', True),
    ('c', 'palatal', 'plosive', False),
    ('ɟ', 'palatal', 'plosive', True),
    ('k', 'velar', 'plosive', False),
    ('ɡ', 'velar', 'plosive', True, [], ['g']),
    ('q', 'uvular', 'plosive', False),
    ('ɢ', 'uvular', 'plosive', True),
    ('ʔ', 'glottal', 'plosive', False, ['+constricted_glottis']),

    # Nasals
    ('m', 'bilabial', 'nasal', True),
    ('ɱ', 'labiodental', 'nasal', True),
    ('n', 'alveolar', 'nasal', True),
    ('ɳ', 'retroflex', 'nasal', True),
    ('ɲ', 'palatal', 'nasal', True),
    ('ŋ', 'velar', 'nasal', True),
    ('ɴ', 'uvular', 'nasal', True),

    # Trills
    ('ʙ', 'bilabial', 'trill', True),
    ('r', 'alveolar', 'trill', True),
    ('ʀ', 'uvular', 'trill', True),

    # Taps or Flaps
    ('ⱱ', 'labiodental', 'tap', True),
    ('ɾ', 'alveolar', 'tap', True),
    ('ɽ', 'retroflex', 'tap', True),

    # Fricatives
    ('ɸ', 'bilabial', 'fricative', False),
    ('β', 'bilabial', 'fricative', True),
    ('f', 'labiodental', 'fricative', False),
    ('v', 'labiodental', 'fricative', True),
    ('θ', 'dental', 'fricative', False),
    ('ð', 'dental', 'fricative', True),
    ('s', 'alveolar', 'fricative', False, ['+strident']),
    ('z', 'alveolar', 'fricative', True, ['+strident']),
    ('ʃ', 'postalveolar', 'fricative', False, ['+strident']),
    ('ʒ', 'postalveolar', 'fricative', True, ['+strident']),
    ('ʂ', 'retroflex', 'fricative', False, ['+strident']),
    ('ʐ', 'retroflex', 'fricative', True, ['+strident']),
    ('ç', 'palatal', 'fricative', False),
    ('ʝ', 'palatal', 'fricative', True),
    ('x', 'velar', 'fricative', False),
    ('ɣ', 'velar', 'fricative', True),
    ('χ', 'uvular', 'fricative', False),
    ('ʁ', 'uvular', 'fricative', True),
    ('ħ', 'pharyngeal', 'fricative', False),
    ('ʕ', 'pharyngeal', 'fricative', True),
    ('h', 'glottal', 'fricative', False, ['+spread_glottis']),
    ('ɦ', 'glottal', 'fricative', True, ['+spread_glottis']),

    # Lateral Fricatives
    ('ɬ', 'alveolar', 'lateral_fricative', False),
    ('ɮ', 'alveolar', 'lateral_fricative', True),

    # Approximants
    ('ʋ', 'labiodental', 'approximant', True),
    ('ɹ', 'alveolar', 'approximant', True),
    ('ɻ', 'retroflex', 'approximant', True),
    ('j', 'palatal', 'approximant', True),
    ('ɰ', 'velar', 'approximant', True),

    # Lateral Approximants
    ('l', 'alveolar', 'lateral_approximant', True),
    ('ɭ', 'retroflex', 'lateral_approximant', True),
    ('ʎ', 'palatal', 'lateral_approximant', True),
    ('ʟ', 'velar', 'lateral_approximant', True),
]

non_pulmonic = [
    # Clicks
    ('ʘ', 'bilabial', 'click', False),
    ('ǀ', 'dental', 'click', False),
    ('ǃ', 'alveolar', 'click', False),
    ('ǂ', 'palatal', 'click', False),
    ('ǁ', 'alveolar', 'click', False, ['+lateral']),

    # Voiced implosives
    ('ɓ', 'bilabial', 'implosive', True, ['+constricted_glottis']),
    ('ɗ', 'alveolar', 'implosive', True, ['+constricted_glottis']),
    ('ᶑ', 'retroflex', 'implosive', True, ['+constricted_glottis']),
    ('ʄ', 'palatal', 'implosive', True, ['+constricted_glottis']),
    ('ɠ', 'velar', 'implosive', True, ['+constricted_glottis']),
    ('ʛ', 'uvular', 'implosive', True, ['+constricted_glottis']),

    # Ejectives (Plosives)
    ('pʼ', 'bilabial', 'ejective', False),
    ('tʼ', 'alveolar', 'ejective', False),
    ('kʼ', 'velar', 'ejective', False),
    ('sʼ', 'alveolar', 'ejective_fricative', False, ['+strident']),
]

for p in pulmonic + non_pulmonic:
    sym = p[0]
    aliases = p[5] if len(p) > 5 else []
    extra_features = p[4] if len(p) > 4 else []
    data[sym] = make_consonant(sym, p[1], p[2], p[3], extra_features, aliases)


# Vowels
# Height: close, near-close, close-mid, mid, open-mid, near-open, open
# Backness: front, central, back
# Roundedness: unrounded, rounded
vowel_heights = {
    'close': ['+high', '-low', '+tense'],
    'near_close': ['+high', '-low', '-tense'],
    'close_mid': ['-high', '-low', '+tense'],
    'mid': ['-high', '-low', '-tense'],
    'open_mid': ['-high', '-low', '-tense'],
    'near_open': ['-high', '+low', '-tense'],
    'open': ['-high', '+low', '+tense'],
}

vowel_backness = {
    'front': ['+front', '-back'],
    'central': ['-front', '-back'],
    'back': ['-front', '+back'],
}

vowel_roundness = {
    'unrounded': ['-round'],
    'rounded': ['+round'],
}

def make_vowel(symbol, height, backness, roundness, aliases=None):
    if aliases is None: aliases = []

    f = ['+syllabic', '-consonantal', '+sonorant', '+continuant', '+voice']
    f.extend(vowel_heights[height])
    f.extend(vowel_backness[backness])
    f.extend(vowel_roundness[roundness])

    # remove duplicate features just in case
    seen = set()
    f_dedup = []
    for feat in f:
        core = feat[1:]
        if core not in seen:
            seen.add(core)
            f_dedup.append(feat)

    return {
        'type': 'vowel',
        'features': f_dedup,
        'place': [backness],
        'manner': [height],
        'aliases': aliases
    }

vowels = [
    # Close
    ('i', 'close', 'front', 'unrounded'),
    ('y', 'close', 'front', 'rounded'),
    ('ɨ', 'close', 'central', 'unrounded'),
    ('ʉ', 'close', 'central', 'rounded'),
    ('ɯ', 'close', 'back', 'unrounded'),
    ('u', 'close', 'back', 'rounded'),

    # Near-close
    ('ɪ', 'near_close', 'front', 'unrounded'),
    ('ʏ', 'near_close', 'front', 'rounded'),
    ('ʊ', 'near_close', 'back', 'rounded'),

    # Close-mid
    ('e', 'close_mid', 'front', 'unrounded'),
    ('ø', 'close_mid', 'front', 'rounded'),
    ('ɘ', 'close_mid', 'central', 'unrounded'),
    ('ɵ', 'close_mid', 'central', 'rounded'),
    ('ɤ', 'close_mid', 'back', 'unrounded'),
    ('o', 'close_mid', 'back', 'rounded'),

    # Mid
    ('ə', 'mid', 'central', 'unrounded'),

    # Open-mid
    ('ɛ', 'open_mid', 'front', 'unrounded'),
    ('œ', 'open_mid', 'front', 'rounded'),
    ('ɜ', 'open_mid', 'central', 'unrounded'),
    ('ɞ', 'open_mid', 'central', 'rounded'),
    ('ʌ', 'open_mid', 'back', 'unrounded'),
    ('ɔ', 'open_mid', 'back', 'rounded'),

    # Near-open
    ('æ', 'near_open', 'front', 'unrounded'),
    ('ɐ', 'near_open', 'central', 'unrounded'),

    # Open
    ('a', 'open', 'front', 'unrounded'),
    ('ɶ', 'open', 'front', 'rounded'),
    ('ɑ', 'open', 'back', 'unrounded'),
    ('ɒ', 'open', 'back', 'rounded'),
]

for v in vowels:
    aliases = v[4] if len(v) > 4 else []
    data[v[0]] = make_vowel(v[0], v[1], v[2], v[3], aliases)

print(json.dumps(data, indent=2, ensure_ascii=False))
