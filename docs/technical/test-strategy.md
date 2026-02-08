# Test Strategy: Feature-Gated Test Tiers

## Overview

Tests are organized into tiers based on what external resources they require. Tiers are controlled by Cargo feature flags, so CI and local runs can express what's available without relying on `#[ignore]` or knowing individual test names.

Unit tests (no external dependencies) always run. Integration tests requiring external resources are gated behind feature flags that compile the test code in or out.

## Test Strata

| Feature | Gate | What it enables | Crate |
|---------|------|-----------------|-------|
| *(none)* | always | Unit tests — no external deps | all |
| `test-ml-model` | ONNX model on disk | `CharacterPredictor` inference tests | `storyteller-engine` |
| `test-llm` | Running Ollama or cloud API key | `ExternalServerProvider` integration test | `storyteller-engine` |
| `test-database` | PostgreSQL + AGE | *(future — ledger, graph queries)* | — |
| `test-messaging` | RabbitMQ | *(future — tasker-core dispatch)* | — |
| `test-services` | All of the above | *(future — end-to-end turn cycle)* | — |

Only `test-ml-model` and `test-llm` are implemented. The others document the intended growth path.

## Usage

```bash
# Unit tests only (default — no external deps required)
cargo test --workspace

# With ML model available (STORYTELLER_MODEL_PATH or STORYTELLER_DATA_PATH set)
cargo test --workspace --features test-ml-model

# With Ollama running
cargo test --workspace --features test-llm

# Everything
cargo test --workspace --features test-ml-model,test-llm
```

Features forward from the workspace root to `storyteller-engine`, so `--workspace --features test-ml-model` works without specifying `-p storyteller-engine`.

## How It Works

### Feature Definitions

Features are defined in `storyteller-engine/Cargo.toml` with no dependency implications — they only gate `#[cfg(test)]` code:

```toml
[features]
test-ml-model = []
test-llm = []
```

The workspace root `Cargo.toml` forwards them:

```toml
[features]
test-ml-model = ["storyteller-engine/test-ml-model"]
test-llm = ["storyteller-engine/test-llm"]
```

### Test Code Patterns

**Single gated test** (when only one test needs the feature):

```rust
#[cfg(feature = "test-llm")]
#[tokio::test]
async fn ollama_integration() { ... }
```

**Multiple gated tests with shared helpers** (use a submodule):

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn always_runs() { ... }

    #[cfg(feature = "test-ml-model")]
    mod with_model {
        use super::*;
        // helpers + gated tests here
    }
}
```

## Adding a New Tier

1. Add the feature to `storyteller-engine/Cargo.toml` (or the relevant crate):
   ```toml
   test-database = []
   ```

2. Add a forwarding feature to root `Cargo.toml`:
   ```toml
   test-database = ["storyteller-engine/test-database"]
   ```

3. Gate test code with `#[cfg(feature = "test-database")]`.

4. Update this document's strata table.

## Test Fixtures

Small binary fixtures (ONNX models, descriptor files) are committed directly to the repository under `tests/fixtures/`. This avoids MLOps overhead for artifacts that are only a few MB.

```
tests/fixtures/
└── models/
    ├── character_predictor.onnx       # 38KB — ONNX graph
    └── character_predictor.onnx.data  # 1.6MB — external weight tensor
```

`.gitattributes` marks `*.onnx` and `*.onnx.data` as binary to prevent diff/merge/line-ending issues.

When the model is retrained, copy the new files into `tests/fixtures/models/` and commit. If the model grows beyond ~10MB, evaluate Git LFS or an artifact registry.

## CI Integration

CI jobs can selectively enable tiers based on available infrastructure:

- **PR checks**: `cargo test --workspace` (unit tests only)
- **Nightly / integration**: `cargo test --workspace --features test-ml-model,test-llm`
- **Full environment**: `cargo test --workspace --features test-ml-model,test-llm,test-database,test-messaging`

Environment variables needed per tier:
- `test-ml-model`: `STORYTELLER_MODEL_PATH=$GITHUB_WORKSPACE/tests/fixtures/models` (committed to repo, must be absolute path)
- `test-llm`: Ollama running at `localhost:11434` with `mistral` model pulled
- `test-database`: PostgreSQL + AGE connection string *(future)*
- `test-messaging`: RabbitMQ connection string *(future)*

## Design Rationale

**Why features instead of `#[ignore]`?**

`#[ignore]` requires knowing specific test names or running all ignored tests indiscriminately. Feature flags let you express "I have a model available" as a single coordination signal, and the compiler includes/excludes the right tests. Tests that don't compile can't regress silently.

**Why not `cfg(test)` environment variables?**

Environment variables are checked at runtime — the test binary compiles the test, starts it, then panics when the resource is missing. Feature flags exclude the code at compile time, so `cargo test` reports an accurate count with no phantom failures.
