---
name: rust-quality
description: Format Rust files and run Clippy with warnings denied in this project. Use when asked to format Rust code, check or fix Clippy diagnostics, or validate Rust changes before committing or opening a PR.
---

# Rust quality

Run from the project root. Follow [AGENTS.md](../../../AGENTS.md), [rustfmt.toml](../../../rustfmt.toml), [Cargo.toml](../../../Cargo.toml), and [clippy.toml](../../../clippy.toml). Keep the existing test exceptions and lint settings intact.

## Format and Clippy

1. Run `cargo fmt --all` to format Rust files using the project configuration.
2. Run `cargo clippy --all-targets --all-features -- -D warnings`. This denies both compiler and Clippy warnings for this invocation and includes test targets. If future features are mutually exclusive, run each supported combination separately with the same warning policy.
3. When fixes are requested, resolve diagnostics without weakening lint levels, adding blanket allowances, or changing unrelated behavior. Reformat changed code and rerun Clippy after fixes.

This workflow runs only formatting and Clippy. Do not add `cargo fmt --all -- --check`, `cargo check`, tests, or other validation commands.

Rustfmt formats Rust source files; it does not format Markdown or TOML. Do not claim those files were formatted by this workflow. Do not commit or push unless the user requested it.

## Report

State which files changed, which commands ran, and their results. Include unresolved diagnostics or concrete blockers. Only report success when formatting and Clippy completed successfully. Clippy includes test targets but does not run the test suite.
