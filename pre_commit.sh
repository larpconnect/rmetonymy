#!/bin/bash
set -e

echo "Running fmt..."
cargo fmt --all

echo "Running clippy..."
cargo clippy --workspace -- -D warnings

echo "Running tests..."
cargo test --workspace

echo "Running tarpaulin..."
cargo tarpaulin --out Json --out Html --output-dir tarpaulin-report

echo "Running audit..."
cargo audit

echo "All pre-commit checks passed!"
