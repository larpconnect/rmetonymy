with open("ipa/src/string.rs", "r") as f:
    text = f.read()

text = text.replace("-> std::fmt::Formatter<'_> {", "-> std::fmt::Result {")
text = text.replace("use crate::{get_entry, IpaSystem, DEFAULT_SYSTEM};", "use crate::get_entry;")
text = text.replace("use data::IpaEntry;\n", "")

with open("ipa/src/string.rs", "w") as f:
    f.write(text)
