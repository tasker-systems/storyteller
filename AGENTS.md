# AGENTS.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

**storyteller** is a world building and storytelling engine. This is a greenfield pre-alpha project where breaking changes are expected.

**Status**: Pre-alpha (no code yet)

---

## Development Commands

Once the project has code:

```bash
# Build
cargo build --all-features
cargo check --all-features

# Test
cargo test --all-features
cargo test <test_name>                    # Single test

# Lint and format
cargo clippy --all-targets --all-features
cargo fmt
cargo fmt --check                         # CI check

# Documentation
cargo doc --all-features --open
```

---

## Rust Standards

This project follows the tasker-systems Rust conventions:

- Use `#[expect(lint_name, reason = "...")]` instead of `#[allow]`
- All public types must implement `Debug`
- All MPSC channels must be bounded (no `unbounded_channel()`)
- Microsoft Universal Guidelines + Rust API Guidelines apply

---

## Related Repositories

| Repository | Description |
|------------|-------------|
| tasker-core | Workflow orchestration engine (Rust) |
| tasker-contrib | Framework integrations and deployment tooling |
