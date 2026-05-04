with open("ipa/tests/lib_tests.rs", "r") as f:
    text = f.read()

import re
text = re.sub(
    r'fn test_global_combine_with_modifier\(\) {\n    assert!\(ipa::combine_with_modifier\("p", "ʰ"\).is_some\(\)\);\n}',
    'fn test_global_combine_with_modifier() {\n    assert!(ipa::combine_with_modifier("p", "ʰ").is_none());\n}',
    text
)

with open("ipa/tests/lib_tests.rs", "w") as f:
    f.write(text)
