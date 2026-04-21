# Metonymy Agent Directives

## System Context & Architecture
* **Project Type:** Rust Command-Line Interface (CLI) application.
* **Execution Environment:** Strictly local.
* **Network & State Constraints:** Zero network egress or ingress is permitted. No external database connections are used. 
* **Core I/O:** The application interfaces exclusively via standard streams (`stdin`, `stdout`, `stderr`) and the local filesystem, specifically interacting with structured data files.

---

## Core Principles
- **Safety First**: Prioritize borrow checker rules, strict lifetime management, and avoiding `unsafe` unless strictly necessary.
- **Idiomatic Rust**: Use modern Rust idioms (Edition 2021+, 1.9+), `Option`/`Result` for error handling, and prefer iterators over loops.
- **Maintainability**: Keep methods small and focused.
- **Performance**: Focus on zero-cost abstractions.

---

## Technical Setup
- **Toolchain**: Use `+nightly` only if required; otherwise, default to stable.
- **Project Structure**: Cargo workspace.
- **Dependency Management**: Use `cargo add` for new dependencies.
- **Use rust**: Project is written in rust and may use gherkin for testing. Other generation 3 (e.g., python, javascript) or 5 (e.g., prolog) programming languages need to be approved by a human before being used.

---

## Coding Standards & Linting
- **Formatting**: Run `cargo fmt`.
- **Linting**: Use `cargo clippy` with `#[deny(clippy::all)]`.
- **Warnings**: Use `#[expect(...)]` instead of `#[allow(...)]` to resolve warnings later.
- **Documentation**: Add doc comments (`///`) to new public types and functions unless the purpose is obvious. Do not add documentation for things that can be easily inferred from the function name.

---

## Testing Discipline
- **TDD**: Write tests before implementing logic. Strive to have no more than one integration test and one unit test failing at one time.
  - If given more than one example or test case in order to complete a task then approach the set of cases systematically, solving one at a time rather than implementing them all as tests at once. 
- **Unit Tests**: Place in the same file within a `mod tests` block. Write tests before implementing logic.
- **Integration Tests**: Place in the `tests/` directory. Write tests before implementing logic. Use BDD-style Given-When-Then for integration tests and use cucumber.
- **Snapshot Tests**: Use `cargo insta` and prefer `cargo insta accept`. Use `insta` for testing all CLI output and parser results.

---

## File Encoding & Data Formats
* **Encoding:** All files (source code, configuration, JSON, YAML, and skill files) MUST be read and written in **UTF-8** without a Byte Order Mark (BOM). Agents must strictly enforce this encoding on file creation or modification.
* **Serialization/Deserialization:** Use `serde`, `serde_json`, and `serde_yaml` for all structured file I/O operations.
* **Atomic Operations:** When performing write operations on JSON/YAML files, ensure safe writes (e.g., writing to a temporary file and renaming) to prevent data corruption during unexpected terminations.

---

## MCP (Model Context Protocol) Integration
* **Provider:** `context7`
* **Usage:** Agents must utilize the `context7` MCP connection to query and retrieve project documentation, internal architectural decisions (ADRs), and specific crate documentation (e.g., `clap`, `serde` semantics).
* **Workflow:** Before generating novel solutions for data parsing or system architecture, query `context7` to ensure alignment with established repository patterns.

---

## Skill Files Integration Protocol
This repository implements the standardized Agent Skill Files protocol to modularize and dictate specific sub-task execution.
* **Location:** All skill files are located in the `.agents/skills/` directory.
* **Format:** Skill files are Markdown documents containing YAML frontmatter describing the `trigger` conditions, `context_requirements`, and `tool_dependencies`. All skill files must be strictly **UTF-8**.
* **Invocation:** Agents must scan `.agents/skills/` when beginning a new task. If a task matches a skill's trigger condition (e.g., "Refactoring YAML parsing logic"), the agent must load and abide by the specialized instructions defined in that skill file.
* **Maintenance:** When an agent develops a reusable workflow or resolves a complex edge case regarding local file I/O, they should generate or update a skill file to persist that operational knowledge.

---

## Specific Directives
- **Ownership/Memory**: Prefer `&str` for reading, `String` for modifying, and `Cow<&str>` for conditional modification.
  - Prefer `Cow<'a, str>` for types that are frequently passed between the parser and the database layer to minimize allocations
- **Async/Await**: Use `tokio` as the default runtime unless specified.
- **Errors**: Prefer `thiserror` for libraries and `anyhow` for applications.
  - **Application Level**: Use `anyhow` (or `miette` for pretty printing) in `main.rs` and within the top level of the repl and batch systems to handle errors.
  - **Library Level**: Use `thiserror` for any library or sub-module. 
  - **Panic Policy**: Never panic. Avoid `.unwrap()` and `.expect()`. Use `?` propagation or distinct error handling blocks.
  - Use anyhow::Result for fallible operations; use thiserror for domain-specific error types.
- **Type System**: 
  - **Type Safety**: Prefer enums or algebraic data types over bools or strings when dealing with features of sounds or other domain-specific values.
  - **Newtypes**: Use "Newtypes" (`struct UserId(String)`) to prevent primitive obsession.
  - **Enums**: Use rich Enums to represent state transitions and command variants.
  - **Traits**: Prefer generic functions with Trait bounds (`fn execute<T: Runnable>(...)`) over concrete types when decoupled logic allows.
- **Error Handling:** Use `thiserror`. 
- **Pattern Matching:** Must be exhaustive, avoid `_` where possible.
- **Lints:** NO new warnings are permitted. All `clippy` checks must pass and `cargo clippy --fix` should be run before submitting.
- **Git Flow:** Only work on feature branches. Never push to `main` or `develop`. When asked to push, push to a branch other than `main` or `develop` and open a pull request.
  - **Rebase**: Always ensure that changes are rebased on top of and up to date with the `main` branch before pushing them. Avoid the need for merge commits.
  - **PRs**: Always check to see if a PR has already been merged before continuing to work on it. If it has been merged, create a new PR for the commit. Do not open a new PR if one has already been opened for a given conversation.
  - **Jules Branches**: Branches created by `jules` should be prefix the branch name with `jules/`.
  - **PR Units**: Favor small units of work as individual PRs which are easier to independently review.
- **Scope:** Do not introduce new features that are outside the current prompt's explicit scope.
- **Ambiguity:** Stop and ask for human input if the requirements are unclear or conflicting.
- **Plan Review**: The agent must output a structured "Action Plan" before execution
- For collections that are usually small (e.g., CLI arguments or tags), prefer the `smallvec` or `tinyvec` crates to keep data on the stack
- Prefer a Context or State struct that implements Send + Sync. This struct should be passed to command executors rather than using global statics
- The core logic must reside in a library crate (lib.rs), with the REPL and Batch logic acting as thin wrappers (consumers) of that library
- Feature Gating: Use Cargo features to separate REPL-only dependencies (like rustyline) and Batch-only dependencies. The core library should remain as lightweight as possible
- Avoid 'Macro-heavy' crates unless they provide significant value (like `serde` or `clap`).
- Fix any test or type errors until the whole suite is green. Do not submit code with broken tests or that does not compile. 
- Do not modify files in `/generated` directories.
- Do not bypass CI/CD checks.
- Background jobs go in /src/worker/jobs/ and must implement the Job trait from crates_io_worker. Jobs must be idempotent.
- Never ignore deprecation warnings; fix them immediately or use #[expect(deprecated)].
- All configuration files should be UTF-8. `unicode-normalization` should be used for unicode processing.
- Prefer `IndexMap` (from the `indexmap` crate) to `HashMap` to provide deterministic ordering for tests.

---

# CLI Architecture
- Use clap's Subcommand enum to unify Batch and REPL entry points. Example: myapp run <file> (batch) vs myapp repl (interactive)

---

## Batch-specific rules
- Input: Must accept input via file path arguments OR `stdin`.
- Exit Codes: Return 0 for success, non-zero for failures.
- Logging: Use the `tracing` crate. Logs go to `stderr`.
- Persistence: Use atomic file writes (write to temp file, then rename) to prevent data corruption during batch processing.

---


## File Formats

- The "lingua franca" of the system is JSON. Both for input and for output. It MAY be in JSON5 or JSON with comments.
- JSON should map to a JSON-schema file. This file may be either generated from the code or written out and submitted. It SHOULD be used to validate JSON files that are read or written. JSON-schema files must be kept up to date with the JSON files that are used both for configuration and examples.
- Other file formats that may optionally be included so long as the features are fully represented in JSON:
  - Protobuf (and specifically textproto)
  - Cypher
  - Yaml
  - HOCON
  - Other JSON formats (e.g., vulgarlang).
  - CSV formats

---

## Performance Rules
- **Allocations:** Zero-copy is preferred. Use `&str` or `Cow<'a, str>` for parsing.
- **Clones:** Any `.clone()` on a non-Copy type MUST be accompanied by a comment explaining why it is necessary.
- **Async:** Never block the executor. Use `spawn_blocking` for CPU-intensive or synchronous I/O.
- **Data Layout:** Prefer `Box<[T]>` over `Vec<T>` if size is fixed after creation. Avoid `Vec<Vec<T>>`.
- **Inlining:** Use `#[inline]` for small public helpers (< 5 lines).

---

## Security & Privacy
- **Secrets:** NEVER write secrets to code. Use `std::env` or the `secrecy` crate for sensitive types.
- **Logging:** Use `tracing::error!` but NEVER log `PII`, `SPII`, `tokens`, or `passwords`.
- **Inputs:** All external data from APIs or users MUST be validated via `serde` with `#[serde(deny_unknown_fields)]`.
- **Tools:** The agent is restricted from using tools that navigate outside the project directory or access the network unless explicitly permitted.
- **Supply Chain:** Run `cargo audit` after adding any new crate. This is mandatory after `cargo add`.
- If the agent logs database results for debugging, it must use a redaction wrapper. Wrap sensitive fields in `secrecy::SecretString` so they are masked as *** in logs.

---


## Project Specific

- Separate code, configuration, and data. Do not write out configuration in code and instead prefer to use JSON files as the source of truth. Data should go in `data/`.
- If a file will be too large additional files may be created. This should be done in preference to truncating the amount of information.
- Prefer standard notation for linguistic terms and concepts.
- Prefer "old style" ligatures for IPA (e.g., ʦ over t͡s) but accept both styles and be able to work with both styles.
- Feature-Based Logic: Prefer bitflags or enums for phonological features (e.g., Place, Manner, Voicing) rather than string-matching "bilabial" or "plosive."

---

## Command Reference
- `cargo check`
- `cargo clippy -- -D warnings`
- `cargo test`

---

## Critical Crates
- `clap`: Argument parsing (derive pattern).
- `cargo-deny`: Prevent unwated licenses.
- `schemars`: Keep JSON schema in sync with rust structures.
- `anyhow`/`thiserror`: Error handling.
- `tracing`: Structured logging.
- `serde`: Serialization.
- `unicode-normalization`: For working with unicode. 
- `tokio`: Async runtime (if applicable).
- `insta`: Snapshot testing.
