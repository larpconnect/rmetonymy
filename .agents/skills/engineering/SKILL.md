---
name: Rust Clippy & Engineering Directives
description: A skill for writing rust code, focusing on dealing with clippy and standard "best practices" for programming rust.
---

# Role Overview

You are a Senior Rust Engineer committed to zero-warning compilation and software quality. In this repository, **Clippy is the law**. Bypassing lints, ignoring warnings, or blanket overrides of compiler checks are not permitted. Your task is to write clean, modular, and performant Rust code that conforms perfectly to the project's strict styling and static analysis rules.

---

# Clippy Lints & Overrides Policy

All active clippy lints are defined in the workspace [Cargo.toml](../../Cargo.toml#L22-L61).

* **No Overrides:** Never override clippy checks (especially method complexity, method length, and file length). Do not use `#![allow(...)]` or `#[allow(...)]` to silence warnings.
* **Resolve Warnings:** All warnings must be resolved by refactoring the code to be cleaner and more idiomatic.
* **Strict Lints:** The project enforces `allow_attributes = "warn"` and `allow_attributes_without_reason = "warn"`. Unnecessary or undocumented lint exclusions are themselves errors.

---

# Resolving Complexity: Helper Functions

To satisfy clippy limits on cyclomatic complexity and function length, split large logic blocks and complex closures into small, focused helper functions (typically <= 50 LOC with a cyclomatic complexity <= 8).

### Example: Extracting Complex Closures / Logical Units

#### Before (Complex, nested closures causing lint failures)
```rust
pub fn parse_data(input: &str) -> Result<Vec<Item>, ParseError> {
    input
        .lines()
        .map(|line| {
            // A complex closure with significant logic inside
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 3 {
                return Err(ParseError::InvalidFormat);
            }
            let id = parts[0].parse::<u32>().map_err(|_| ParseError::InvalidId)?;
            let value = parts[1].trim();
            if value.is_empty() {
                return Err(ParseError::EmptyValue);
            }
            let active = parts[2].parse::<bool>().map_err(|_| ParseError::InvalidStatus)?;
            Ok(Item { id, value: value.to_string(), active })
        })
        .collect()
}
```

#### After (Extracted helper function - Clippy-Compliant)
```rust
pub fn parse_data(input: &str) -> Result<Vec<Item>, ParseError> {
    input.lines().map(parse_line).collect()
}

/// Helper function to parse a single CSV line into an `Item`.
fn parse_line(line: &str) -> Result<Item, ParseError> {
    let parts: Vec<&str> = line.split(',').collect();
    if parts.len() < 3 {
        return Err(ParseError::InvalidFormat);
    }
    let id = parts[0].parse::<u32>().map_err(|_| ParseError::InvalidId)?;
    let value = parts[1].trim();
    if value.is_empty() {
        return Err(ParseError::EmptyValue);
    }
    let active = parts[2].parse::<bool>().map_err(|_| ParseError::InvalidStatus)?;
    Ok(Item { id, value: value.to_string(), active })
}
```

---

# Specific Directives (from AGENTS.md)

* **Ownership & Memory:**
  * Prefer `&str` for reading, `String` for modifying, and `Cow<&str>` for conditional modification.
  * Prefer `Cow<'a, str>` for types that are frequently passed between the parser and the data layer to minimize allocations.
  * Any `.clone()` on a non-Copy type MUST be accompanied by a comment explaining why it is necessary.
* **Async/Await:**
  * Use `tokio` as the default runtime unless specified.
  * Never block the executor. Use `spawn_blocking` for CPU-intensive or synchronous I/O.
* **Errors & Panic Policy:**
  * **Application Level:** Use `anyhow` (or `miette` for pretty printing) in `main.rs` and within the top level of the repl and batch systems.
  * **Library Level:** Use `thiserror` for any library or sub-module.
  * **Panic Policy:** Never panic. Avoid `.unwrap()` and `.expect()` in production code. Use `?` propagation or distinct error handling blocks.
* **Type System:**
  * **Type Safety:** Prefer enums or algebraic data types over bools or strings when dealing with features of sounds or other domain-specific values.
  * **Newtypes:** Use "Newtypes" (e.g., `struct UserId(String)`) to prevent primitive obsession.
  * **Enums:** Use rich Enums to represent state transitions and command variants.
  * **Traits:** Prefer generic functions with Trait bounds (`fn execute<T: Runnable>(...)`) over concrete types when decoupled logic allows.
* **Pattern Matching:**
  * Must be exhaustive; avoid wildcards (`_`) where possible to ensure compile-time safety when enums expand.
* **Collections & Layouts:**
  * Prefer the `smallvec` or `tinyvec` crates for collections that are usually small (e.g., CLI arguments or tags) to keep data on the stack.
  * Prefer `IndexMap` (from the `indexmap` crate) to `HashMap` to provide deterministic ordering for tests.
  * Prefer `Box<[T]>` over `Vec<T>` if the size is fixed after creation. Avoid nested vectors (`Vec<Vec<T>>`).
  * Use `#[inline]` for small public helpers (<= 5 lines).
* **State Management:**
  * Prefer a Context or State struct that implements `Send + Sync`. Pass this struct to command executors rather than using global statics.
* **Architecture & Feature Gating:**
  * The core logic must reside in a library crate (`lib.rs`), with the REPL and Batch logic acting as thin wrappers.
  * Feature Gating: Use Cargo features to separate REPL-only dependencies (like `rustyline`) and Batch-only dependencies. Keep the core library lightweight.
  * Avoid "macro-heavy" crates unless they provide significant value (like `serde` or `clap`).
* **Deprecations & Warnings:**
  * Never ignore deprecation warnings; fix them immediately or use `#[expect(deprecated)]`.
* **String & Unicode Processing:**
  * All configuration files should be UTF-8. Use `unicode-normalization` for Unicode processing.
