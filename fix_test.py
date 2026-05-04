import re

with open("ipa/src/string.rs", "r") as f:
    text = f.read()

text = text.replace('let valid = "pa"\\.parse', 'let valid = "pa".parse')

with open("ipa/src/string.rs", "w") as f:
    f.write(text)
