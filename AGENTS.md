# Metonymy Agent Directives

## System Context & Architecture
* **Project Type:** Rust Command-Line Interface (CLI) application.
* **Execution Environment:** Strictly local.
* **Network & State Constraints:** Zero network egress or ingress is permitted. No external database connections are used. 
* **Core I/O:** The application interfaces exclusively via standard streams (`stdin`, `stdout`, `stderr`) and the local filesystem, specifically interacting with structured data files.

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

## Rust Development Standards
* **Error Handling:** Utilize `Result<T, E>` with robust custom error types (e.g., via `thiserror`). Failures during file reads or deserialization must provide detailed context (file path, line number, schema violation). Do not use `.unwrap()` or `.expect()` outside of test modules.
* **Validation:** All data read from JSON or YAML files must be treated as untrusted and validated against strict schemas before internal processing.
* **Linting:** All code must pass `cargo clippy -- -D warnings`.
* **Formatting:** All code must be formatted using `cargo fmt`.
