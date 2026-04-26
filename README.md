# metonymy

A multi-module project.

## Structure

* `metonymy` (the application proper, the entry point for the system)
* `data` (for processing files and working with JSON)
* `ipa` (for working with and parsing the international phonetic alphabet)
* `language` (for representing individual language structures, models)
* `soundchange` (for parsing the sound change language)

Dependency flow: `data` -> `ipa` -> `language` -> `soundchange` -> `metonymy`.

## Code Quality Tools

We use standard Rust tools like `rustfmt` and `clippy`.

Additional code quality checks:
* `cargo-audit` for dependency vulnerability scanning.
* `cargo-tarpaulin` for test coverage.
