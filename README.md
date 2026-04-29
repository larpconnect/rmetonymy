# Metonymy

Metonymy is a CLI tool to facilitate building conlangs. To do this it includes (or will include) a variety of features to help users with various parts of conlang construction, including analysis, structure definition, generation, and application.

**Note:** This tool is not ready for use and file formats may change erratically and unpredictably, but once we hit 1.0 we'll do structured, semantic versions.

## Core Concepts

Building languages will eventually involve at least three files:

1. A _phoneme database_. This represents the basic phoneme definitions and will largely be shared by languages. This can be multiple discrete files.
2. A _language definition_. This controls how the language is put together: phonology, phonotactics, grammar, etc.
3. A _dictionary_, which defines the actual words of the language. This can be multiple discrete files.

## Building and Running

Code is predominately in Rust. Tests are written in Rust and/or Gherkin.

To build the project, ensure you have Rust and Cargo installed, then run:

```bash
cargo build
```

To run the software:

```bash
cargo run -- [arguments]
```

To run tests across the workspace:

```bash
cargo test --workspace
```

## Installation

To install the CLI tool locally, run the following command from the root of the repository:

```bash
cargo install --path metonymy
```

## Development Layout

The repository is a multi-module Rust workspace with the following members:

* `metonymy` (the application proper, the entry point for the system)
* `data` (for processing files and working with JSON)
* `ipa` (for working with and parsing the international phonetic alphabet)
* `language` (for representing individual language structures, models)
* `soundchange` (for parsing the sound change language)

Dependency flow: `data` -> `ipa` -> `language` -> `soundchange` -> `metonymy`.

### Code Quality Tools

We use standard Rust tools like `rustfmt` and `clippy`. You can use `cargo fmt` for code formatting, and `cargo clippy --workspace` for linting.

Additional code quality checks:
* `cargo-audit` for dependency vulnerability scanning.
* `cargo-tarpaulin` for test coverage.
