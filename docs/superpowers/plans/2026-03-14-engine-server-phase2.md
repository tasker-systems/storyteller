# Engine Server Phase 2: Client Library, CLI, and Pipeline Port

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a typed gRPC client library, CLI playtest harness, and port the existing turn pipeline from the workshop into the server.

**Architecture:** Vertical slice — get one end-to-end path working (compose → playtest with real pipeline), then widen with composition cache and CLI ergonomics. Server and CLI are fully decoupled: the CLI depends only on `storyteller-client`, never on `storyteller-server`.

**Tech Stack:** Rust, tonic (gRPC), clap (CLI), tokio, serde. Existing crates: storyteller-core, storyteller-engine, storyteller-composer, storyteller-ml.

**Spec:** `docs/superpowers/specs/2026-03-14-engine-server-phase2-design.md`

---

## File Structure

### New Files

| File | Responsibility |
|------|---------------|
| `crates/storyteller-server/Cargo.toml` | Server crate manifest (renamed from storyteller-api) |
| `crates/storyteller-server/src/bin/main.rs` | Server binary entry point |
| `crates/storyteller-core/src/types/health.rs` | `ServerHealth`, `SubsystemHealth`, `HealthStatus` types |
| `crates/storyteller-client/Cargo.toml` | Client crate manifest |
| `crates/storyteller-client/build.rs` | Proto compilation (client stubs) |
| `crates/storyteller-client/src/lib.rs` | `StorytellerClient`, `ClientConfig`, `ClientError` |
| `crates/storyteller-cli/src/playtest.rs` | Playtest subcommand implementation |
| `crates/storyteller-cli/src/compose.rs` | Compose subcommand implementation |
| `crates/storyteller-cli/src/composer_cache.rs` | Local descriptor cache logic |
| `crates/storyteller-cli/src/player_simulation.rs` | Player simulation LLM for playtest |

### Modified Files

| File | Changes |
|------|---------|
| `Cargo.toml` (root) | Add `storyteller-client` to workspace members |
| `crates/storyteller-core/src/types/mod.rs` | Add `pub mod health` |
| `proto/storyteller/v1/common.proto` | Add `SubsystemHealth`, `HealthResponse` messages |
| `proto/storyteller/v1/engine.proto` | Replace `CheckLlmStatus`/`LlmStatus` with `CheckHealth`/`HealthResponse` |
| `crates/storyteller-server/src/server.rs` | Wire structured LLM + predictor, add binary entry point logic |
| `crates/storyteller-server/src/grpc/engine_service.rs` | Replace SubmitInput stubs with real pipeline, wire opening narration |
| `crates/storyteller-server/src/engine/providers.rs` | Update `EngineProviders` (predictor field type change) |
| `crates/storyteller-cli/Cargo.toml` | Drop server/engine/core deps, add client + reqwest deps |
| `crates/storyteller-cli/src/main.rs` | Remove `Serve`, add `Playtest`/`Compose`/`Composer` subcommands |

### Removed Files

| File | Reason |
|------|--------|
| `crates/storyteller-cli/src/bin/server.rs` | Bevy binary, superseded by server's own binary |
| `crates/storyteller-cli/src/bin/play_scene_context.rs` | Deprecated, superseded by `playtest` subcommand |

---

## Chunk 1: Foundation (Rename + Health Types)

### Task 1: Rename `storyteller-api` → `storyteller-server`

**Files:**
- Rename: `crates/storyteller-api/` → `crates/storyteller-server/`
- Modify: `Cargo.toml` (root workspace, line 7)
- Modify: `crates/storyteller-server/Cargo.toml` (package name)
- Modify: `crates/storyteller-cli/Cargo.toml` (dependency name)
- Modify: `crates/storyteller-cli/src/main.rs` (use statement, line 5)
- Modify: `crates/storyteller-server/tests/grpc_integration.rs` (use statements)

- [ ] **Step 1: Rename the directory**

```bash
mv crates/storyteller-api crates/storyteller-server
```

- [ ] **Step 2: Update package name in crate Cargo.toml**

In `crates/storyteller-server/Cargo.toml`, change:
```toml
[package]
name = "storyteller-server"
```

- [ ] **Step 3: Update root workspace members**

In root `Cargo.toml`, change the workspace member from `"crates/storyteller-api"` to `"crates/storyteller-server"`.

- [ ] **Step 4: Update CLI dependency**

In `crates/storyteller-cli/Cargo.toml`, change the dependency from:
```toml
storyteller-api = { path = "../storyteller-api" }
```
to:
```toml
storyteller-server = { path = "../storyteller-server" }
```

- [ ] **Step 5: Update CLI import**

In `crates/storyteller-cli/src/main.rs`, change:
```rust
use storyteller_api::server::{run_server, ServerConfig};
```
to:
```rust
use storyteller_server::server::{run_server, ServerConfig};
```

- [ ] **Step 6: Update integration test imports**

In `crates/storyteller-server/tests/grpc_integration.rs`, update any `use storyteller_api::` to `use storyteller_server::`.

- [ ] **Step 7: Verify compilation**

Run: `cargo check --workspace --exclude storyteller-workshop`
Expected: Clean compilation with no errors.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "refactor: rename storyteller-api to storyteller-server"
```

---

### Task 2: Add server binary target

**Files:**
- Create: `crates/storyteller-server/src/bin/main.rs`
- Modify: `crates/storyteller-server/Cargo.toml` (add `[[bin]]` section)

- [ ] **Step 1: Create the server binary**

Create `crates/storyteller-server/src/bin/main.rs`:

```rust
use storyteller_server::server::{run_server, ServerConfig};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    dotenvy::dotenv().ok();

    let config = ServerConfig::from_env();
    run_server(config).await?;

    Ok(())
}
```

- [ ] **Step 2: Add binary target and dependencies to Cargo.toml**

In `crates/storyteller-server/Cargo.toml`, add the `[[bin]]` section and any missing dependencies (`tracing-subscriber`, `dotenvy`):

```toml
[[bin]]
name = "storyteller-server"
path = "src/bin/main.rs"
```

Add to `[dependencies]`:
```toml
tracing-subscriber = { workspace = true, features = ["env-filter"] }
dotenvy = { workspace = true }
```

- [ ] **Step 3: Verify the binary builds**

Run: `cargo build -p storyteller-server --bin storyteller-server`
Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-server/src/bin/main.rs crates/storyteller-server/Cargo.toml
git commit -m "feat: add standalone binary target for storyteller-server"
```

---

### Task 3: Remove server code from CLI

**Files:**
- Remove: `crates/storyteller-cli/src/bin/server.rs`
- Remove: `crates/storyteller-cli/src/bin/play_scene_context.rs`
- Modify: `crates/storyteller-cli/Cargo.toml` (remove `[[bin]]` sections, drop server/engine deps)
- Modify: `crates/storyteller-cli/src/main.rs` (remove `Serve` subcommand)

- [ ] **Step 1: Delete the old binary files**

```bash
rm crates/storyteller-cli/src/bin/server.rs
rm crates/storyteller-cli/src/bin/play_scene_context.rs
rmdir crates/storyteller-cli/src/bin 2>/dev/null || true
```

- [ ] **Step 2: Remove `[[bin]]` sections from CLI Cargo.toml**

In `crates/storyteller-cli/Cargo.toml`, remove the two `[[bin]]` entries for `storyteller-server` and `play-scene`.

- [ ] **Step 3: Drop server dependencies from CLI**

In `crates/storyteller-cli/Cargo.toml`, remove these dependencies:
- `storyteller-server` (was `storyteller-api`)
- `storyteller-engine`
- `storyteller-core`

The CLI will depend only on `storyteller-client` (added in a later task). For now, keep minimal deps: `clap`, `tokio`, `tracing-subscriber`, `dotenvy`.

- [ ] **Step 4: Gut the CLI main.rs**

Replace `crates/storyteller-cli/src/main.rs` with a minimal shell that has no subcommands yet (they'll be added in Chunk 4):

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "storyteller-cli", about = "Storyteller engine CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    // Subcommands will be added in later tasks
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {}
}
```

- [ ] **Step 5: Verify compilation**

Run: `cargo check --workspace --exclude storyteller-workshop`
Expected: Clean compilation.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor: remove server code from CLI, decouple from storyteller-server"
```

---

### Task 4: Add health types to `storyteller-core`

**Files:**
- Create: `crates/storyteller-core/src/types/health.rs`
- Modify: `crates/storyteller-core/src/types/mod.rs` (line 21, add `pub mod health`)

- [ ] **Step 1: Write the health types**

Create `crates/storyteller-core/src/types/health.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Rollup health status for a subsystem or the server as a whole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unavailable,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded => write!(f, "degraded"),
            Self::Unavailable => write!(f, "unavailable"),
        }
    }
}

impl HealthStatus {
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "healthy" => Self::Healthy,
            "degraded" => Self::Degraded,
            _ => Self::Unavailable,
        }
    }
}

/// Health of an individual server subsystem (e.g., narrator_llm, predictor).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemHealth {
    pub name: String,
    pub status: HealthStatus,
    pub message: Option<String>,
}

/// Aggregate server health with per-subsystem breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerHealth {
    pub status: HealthStatus,
    pub subsystems: Vec<SubsystemHealth>,
}

impl ServerHealth {
    /// Compute rollup status from subsystems: worst status wins.
    pub fn from_subsystems(subsystems: Vec<SubsystemHealth>) -> Self {
        let status = subsystems
            .iter()
            .map(|s| &s.status)
            .fold(HealthStatus::Healthy, |worst, current| {
                match (&worst, current) {
                    (HealthStatus::Unavailable, _) | (_, HealthStatus::Unavailable) => {
                        HealthStatus::Unavailable
                    }
                    (HealthStatus::Degraded, _) | (_, HealthStatus::Degraded) => {
                        HealthStatus::Degraded
                    }
                    _ => HealthStatus::Healthy,
                }
            });
        Self { status, subsystems }
    }
}
```

- [ ] **Step 2: Export the module**

In `crates/storyteller-core/src/types/mod.rs`, add after the last existing `pub mod` line:

```rust
pub mod health;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p storyteller-core`
Expected: Clean compilation.

- [ ] **Step 4: Write unit tests**

Add to the bottom of `crates/storyteller-core/src/types/health.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollup_all_healthy() {
        let health = ServerHealth::from_subsystems(vec![
            SubsystemHealth {
                name: "narrator".into(),
                status: HealthStatus::Healthy,
                message: None,
            },
            SubsystemHealth {
                name: "predictor".into(),
                status: HealthStatus::Healthy,
                message: None,
            },
        ]);
        assert_eq!(health.status, HealthStatus::Healthy);
    }

    #[test]
    fn rollup_degraded_wins_over_healthy() {
        let health = ServerHealth::from_subsystems(vec![
            SubsystemHealth {
                name: "narrator".into(),
                status: HealthStatus::Healthy,
                message: None,
            },
            SubsystemHealth {
                name: "predictor".into(),
                status: HealthStatus::Degraded,
                message: Some("ONNX model not loaded".into()),
            },
        ]);
        assert_eq!(health.status, HealthStatus::Degraded);
    }

    #[test]
    fn rollup_unavailable_wins_over_degraded() {
        let health = ServerHealth::from_subsystems(vec![
            SubsystemHealth {
                name: "narrator".into(),
                status: HealthStatus::Unavailable,
                message: Some("Ollama not reachable".into()),
            },
            SubsystemHealth {
                name: "predictor".into(),
                status: HealthStatus::Degraded,
                message: None,
            },
        ]);
        assert_eq!(health.status, HealthStatus::Unavailable);
    }

    #[test]
    fn serde_round_trip() {
        let health = ServerHealth::from_subsystems(vec![SubsystemHealth {
            name: "test".into(),
            status: HealthStatus::Healthy,
            message: None,
        }]);
        let json = serde_json::to_string(&health).unwrap();
        let deserialized: ServerHealth = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.status, HealthStatus::Healthy);
        assert_eq!(deserialized.subsystems.len(), 1);
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p storyteller-core -- health`
Expected: All 4 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/storyteller-core/src/types/health.rs crates/storyteller-core/src/types/mod.rs
git commit -m "feat: add generic health types to storyteller-core"
```

---

### Task 5: Replace CheckLlmStatus with CheckHealth (proto + handler)

Proto changes and Rust handler must be updated together to keep every commit compilable.

**Files:**
- Modify: `proto/storyteller/v1/common.proto` (add `SubsystemHealth`, `HealthResponse`)
- Modify: `proto/storyteller/v1/engine.proto` (replace `CheckLlmStatus`/`LlmStatus`)
- Modify: `crates/storyteller-server/src/grpc/engine_service.rs` (the `check_llm_status` method)

- [ ] **Step 1: Add health messages to common.proto**

Replace `proto/storyteller/v1/common.proto` with:

```protobuf
syntax = "proto3";

package storyteller.v1;

import "google/protobuf/empty.proto";

message SubsystemHealth {
  string name = 1;
  string status = 2;
  optional string message = 3;
}

message HealthResponse {
  string status = 1;
  repeated SubsystemHealth subsystems = 2;
}
```

- [ ] **Step 2: Update engine.proto**

In `proto/storyteller/v1/engine.proto`:

Replace the `CheckLlmStatus` RPC line (line 18) with:
```protobuf
  rpc CheckHealth(google.protobuf.Empty) returns (HealthResponse);
```

`common.proto` is already imported (line 4). Remove the `LlmStatus` message definition (line 124) entirely — it's replaced by `HealthResponse` from common.proto.

- [ ] **Step 3: Update the health RPC handler**

In `crates/storyteller-server/src/grpc/engine_service.rs`, replace the `check_llm_status` method with a `check_health` method that builds `SubsystemHealth` entries from `EngineProviders`:

```rust
async fn check_health(
    &self,
    _request: tonic::Request<()>,
) -> Result<tonic::Response<crate::proto::HealthResponse>, tonic::Status> {
    let providers = &self.providers;

    let mut subsystems = vec![
        crate::proto::SubsystemHealth {
            name: "narrator_llm".to_string(),
            status: "healthy".to_string(),
            message: None,
        },
    ];

    subsystems.push(crate::proto::SubsystemHealth {
        name: "structured_llm".to_string(),
        status: if providers.structured_llm.is_some() {
            "healthy".to_string()
        } else {
            "unavailable".to_string()
        },
        message: if providers.structured_llm.is_none() {
            Some("Structured LLM provider not configured".to_string())
        } else {
            None
        },
    });

    subsystems.push(crate::proto::SubsystemHealth {
        name: "intent_llm".to_string(),
        status: if providers.intent_llm.is_some() {
            "healthy".to_string()
        } else {
            "unavailable".to_string()
        },
        message: if providers.intent_llm.is_none() {
            Some("Intent LLM provider not configured".to_string())
        } else {
            None
        },
    });

    subsystems.push(crate::proto::SubsystemHealth {
        name: "predictor".to_string(),
        status: if providers.predictor_available {
            "healthy".to_string()
        } else {
            "unavailable".to_string()
        },
        message: if !providers.predictor_available {
            Some("Character predictor model not loaded".to_string())
        } else {
            None
        },
    });

    // Rollup: worst status wins
    let has_unavailable = subsystems.iter().any(|s| s.status == "unavailable");
    let overall_status = if has_unavailable {
        "degraded".to_string()  // server is up but missing subsystems
    } else {
        "healthy".to_string()
    };

    Ok(tonic::Response::new(crate::proto::HealthResponse {
        status: overall_status,
        subsystems,
    }))
}
```

Note: narrator_llm is always "healthy" when the server is running because it's required for startup. Optional subsystems report "unavailable" when not configured, which makes the overall status "degraded" rather than "unavailable" (the server itself is functioning).

- [ ] **Step 4: Update any references to the old RPC**

Search for `check_llm_status` or `CheckLlmStatus` or `LlmStatus` in the server crate and update all references. This includes the integration tests if any test the old RPC.

- [ ] **Step 5: Verify compilation and run tests**

Run: `cargo check -p storyteller-server && cargo test -p storyteller-server`
Expected: Clean compilation and all tests pass.

- [ ] **Step 6: Commit**

```bash
git add proto/ crates/storyteller-server/
git commit -m "feat: replace CheckLlmStatus with CheckHealth using generic SubsystemHealth model"
```

---

## Chunk 2: Client Library

### Task 6: Create `storyteller-client` crate scaffold

**Files:**
- Create: `crates/storyteller-client/Cargo.toml`
- Create: `crates/storyteller-client/build.rs`
- Create: `crates/storyteller-client/src/lib.rs`
- Modify: `Cargo.toml` (root, add workspace member)

- [ ] **Step 1: Create crate directory**

```bash
mkdir -p crates/storyteller-client/src
```

- [ ] **Step 2: Create Cargo.toml**

Create `crates/storyteller-client/Cargo.toml`:

```toml
[package]
name = "storyteller-client"
version = "0.1.0"
edition = "2021"

[dependencies]
storyteller-core = { path = "../storyteller-core" }
tonic = { workspace = true }
tonic-prost = { workspace = true }
prost = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }

[build-dependencies]
tonic-build = { workspace = true }
tonic-prost-build = { workspace = true }
```

- [ ] **Step 3: Create build.rs**

Create `crates/storyteller-client/build.rs`:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(
            &[
                "../../proto/storyteller/v1/engine.proto",
                "../../proto/storyteller/v1/composer.proto",
            ],
            &["../../proto"],
        )?;
    Ok(())
}
```

Note: `build_server(false)` — the client crate only needs client stubs.

- [ ] **Step 4: Create initial lib.rs**

Create `crates/storyteller-client/src/lib.rs`:

```rust
pub mod proto {
    tonic::include_proto!("storyteller.v1");
}

mod client;
pub use client::{ClientConfig, ClientError, StorytellerClient};
```

- [ ] **Step 5: Create placeholder client module**

Create `crates/storyteller-client/src/client.rs`:

```rust
use storyteller_core::types::health::ServerHealth;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub endpoint: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:50051".to_string(),
        }
    }
}

impl ClientConfig {
    pub fn from_env() -> Self {
        Self {
            endpoint: std::env::var("STORYTELLER_SERVER_URL")
                .unwrap_or_else(|_| "http://localhost:50051".to_string()),
        }
    }
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Failed to connect to server at {0}")]
    ConnectionFailed(String),

    #[error("RPC error: {0}")]
    RpcError(#[from] tonic::Status),

    #[error("Transport error: {0}")]
    TransportError(#[from] tonic::transport::Error),

    #[error("Subsystem unavailable: {subsystem} - {message}")]
    SubsystemUnavailable {
        subsystem: String,
        message: String,
    },
}

pub struct StorytellerClient {
    engine: crate::proto::storyteller_engine_client::StorytellerEngineClient<tonic::transport::Channel>,
    composer: crate::proto::composer_service_client::ComposerServiceClient<tonic::transport::Channel>,
}

impl StorytellerClient {
    pub async fn connect(config: ClientConfig) -> Result<Self, ClientError> {
        let channel = tonic::transport::Channel::from_shared(config.endpoint.clone())
            .map_err(|e| ClientError::ConnectionFailed(e.to_string()))?
            .connect()
            .await
            .map_err(|_| ClientError::ConnectionFailed(config.endpoint))?;

        Ok(Self {
            engine: crate::proto::storyteller_engine_client::StorytellerEngineClient::new(
                channel.clone(),
            ),
            composer: crate::proto::composer_service_client::ComposerServiceClient::new(channel),
        })
    }
}
```

- [ ] **Step 6: Add to workspace**

In root `Cargo.toml`, add `"crates/storyteller-client"` to the `members` array.

- [ ] **Step 7: Verify compilation**

Run: `cargo check -p storyteller-client`
Expected: Clean compilation.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock crates/storyteller-client/
git commit -m "feat: create storyteller-client crate scaffold with proto compilation"
```

---

### Task 7: Implement client health check

**Files:**
- Modify: `crates/storyteller-client/src/client.rs`

- [ ] **Step 1: Add the check_health method**

Add to the `impl StorytellerClient` block in `crates/storyteller-client/src/client.rs`:

```rust
    /// Two-layer health check.
    /// Layer 1: Can we reach the gRPC server? (ConnectionFailed if not)
    /// Layer 2: What's the server's internal health? (ServerHealth with subsystems)
    pub async fn check_health(&mut self) -> Result<ServerHealth, ClientError> {
        let response = self
            .engine
            .check_health(())
            .await?
            .into_inner();

        let subsystems = response
            .subsystems
            .into_iter()
            .map(|s| storyteller_core::types::health::SubsystemHealth {
                name: s.name,
                status: storyteller_core::types::health::HealthStatus::from_str_lossy(&s.status),
                message: s.message,
            })
            .collect();

        Ok(ServerHealth::from_subsystems(subsystems))
    }
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p storyteller-client`
Expected: Clean compilation.

- [ ] **Step 3: Commit**

```bash
git add crates/storyteller-client/src/client.rs
git commit -m "feat: implement two-layer health check in storyteller-client"
```

---

### Task 8: Implement engine RPCs on client

**Files:**
- Modify: `crates/storyteller-client/src/client.rs`

- [ ] **Step 1: Add engine RPC methods**

Add to the `impl StorytellerClient` block:

```rust
    pub async fn compose_scene(
        &mut self,
        request: crate::proto::ComposeSceneRequest,
    ) -> Result<tonic::Streaming<crate::proto::EngineEvent>, ClientError> {
        let response = self.engine.compose_scene(request).await?;
        Ok(response.into_inner())
    }

    pub async fn submit_input(
        &mut self,
        request: crate::proto::SubmitInputRequest,
    ) -> Result<tonic::Streaming<crate::proto::EngineEvent>, ClientError> {
        let response = self.engine.submit_input(request).await?;
        Ok(response.into_inner())
    }

    pub async fn resume_session(
        &mut self,
        request: crate::proto::ResumeSessionRequest,
    ) -> Result<tonic::Streaming<crate::proto::EngineEvent>, ClientError> {
        let response = self.engine.resume_session(request).await?;
        Ok(response.into_inner())
    }

    pub async fn list_sessions(&mut self) -> Result<crate::proto::SessionList, ClientError> {
        let response = self.engine.list_sessions(()).await?;
        Ok(response.into_inner())
    }

    pub async fn get_scene_state(
        &mut self,
        request: crate::proto::GetSceneStateRequest,
    ) -> Result<crate::proto::SceneState, ClientError> {
        let response = self.engine.get_scene_state(request).await?;
        Ok(response.into_inner())
    }
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p storyteller-client`
Expected: Clean compilation.

- [ ] **Step 3: Commit**

```bash
git add crates/storyteller-client/src/client.rs
git commit -m "feat: implement engine RPCs on storyteller-client"
```

---

### Task 9: Implement composer RPCs on client

**Files:**
- Modify: `crates/storyteller-client/src/client.rs`

- [ ] **Step 1: Add composer RPC methods**

Add to the `impl StorytellerClient` block:

```rust
    pub async fn list_genres(&mut self) -> Result<crate::proto::GenreList, ClientError> {
        let response = self.composer.list_genres(()).await?;
        Ok(response.into_inner())
    }

    pub async fn get_profiles_for_genre(
        &mut self,
        genre_id: &str,
    ) -> Result<crate::proto::ProfileList, ClientError> {
        let response = self
            .composer
            .get_profiles_for_genre(crate::proto::GenreRequest {
                genre_id: genre_id.to_string(),
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn get_archetypes_for_genre(
        &mut self,
        genre_id: &str,
    ) -> Result<crate::proto::ArchetypeList, ClientError> {
        let response = self
            .composer
            .get_archetypes_for_genre(crate::proto::GenreRequest {
                genre_id: genre_id.to_string(),
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn get_dynamics_for_genre(
        &mut self,
        genre_id: &str,
        selected_archetype_ids: Vec<String>,
    ) -> Result<crate::proto::DynamicsList, ClientError> {
        let response = self
            .composer
            .get_dynamics_for_genre(crate::proto::DynamicsRequest {
                genre_id: genre_id.to_string(),
                selected_archetype_ids,
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn get_names_for_genre(
        &mut self,
        genre_id: &str,
    ) -> Result<crate::proto::NameList, ClientError> {
        let response = self
            .composer
            .get_names_for_genre(crate::proto::GenreRequest {
                genre_id: genre_id.to_string(),
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn get_settings_for_genre(
        &mut self,
        genre_id: &str,
    ) -> Result<crate::proto::SettingList, ClientError> {
        let response = self
            .composer
            .get_settings_for_genre(crate::proto::GenreRequest {
                genre_id: genre_id.to_string(),
            })
            .await?;
        Ok(response.into_inner())
    }
```

- [ ] **Step 2: Re-export proto types from lib.rs for consumer convenience**

In `crates/storyteller-client/src/lib.rs`, add re-exports of commonly used proto types:

```rust
pub use proto::{
    engine_event, CastMember, ComposeSceneRequest, DynamicPairing, EngineEvent,
    GetSceneStateRequest, ResumeSessionRequest, SubmitInputRequest,
};
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p storyteller-client`
Expected: Clean compilation.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-client/
git commit -m "feat: implement composer RPCs and re-export common proto types"
```

---

### Task 10: Client unit tests

**Files:**
- Modify: `crates/storyteller-client/src/client.rs`

- [ ] **Step 1: Add unit tests for ClientConfig and ClientError**

Add to the bottom of `crates/storyteller-client/src/client.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_uses_localhost() {
        let config = ClientConfig::default();
        assert_eq!(config.endpoint, "http://localhost:50051");
    }

    #[test]
    fn config_from_env_uses_default_when_unset() {
        // Clear the env var if set
        std::env::remove_var("STORYTELLER_SERVER_URL");
        let config = ClientConfig::from_env();
        assert_eq!(config.endpoint, "http://localhost:50051");
    }

    #[test]
    fn client_error_display() {
        let err = ClientError::ConnectionFailed("http://localhost:50051".to_string());
        assert!(err.to_string().contains("localhost:50051"));

        let err = ClientError::SubsystemUnavailable {
            subsystem: "narrator".to_string(),
            message: "not configured".to_string(),
        };
        assert!(err.to_string().contains("narrator"));
    }

    #[tokio::test]
    async fn connect_fails_with_unreachable_server() {
        let result = StorytellerClient::connect(ClientConfig {
            endpoint: "http://127.0.0.1:1".to_string(), // port 1 should be unreachable
        })
        .await;
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p storyteller-client`
Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/storyteller-client/src/client.rs
git commit -m "test: add unit tests for storyteller-client config and error handling"
```

---

## Chunk 3: Pipeline Port

### Task 11: Wire structured LLM and predictor in server startup

**Files:**
- Modify: `crates/storyteller-server/Cargo.toml` (add `storyteller-ml` dependency)
- Modify: `crates/storyteller-server/src/server.rs` (wire providers)
- Modify: `crates/storyteller-server/src/engine/providers.rs` (update `EngineProviders` if needed)

- [ ] **Step 1: Add storyteller-ml dependency**

In `crates/storyteller-server/Cargo.toml`, add:
```toml
storyteller-ml = { path = "../storyteller-ml" }
```

- [ ] **Step 2: Add model path to ServerConfig**

In `crates/storyteller-server/src/server.rs`, add fields to `ServerConfig`:

```rust
pub model_path: Option<String>,
```

And in `from_env()`:
```rust
model_path: std::env::var("STORYTELLER_MODEL_PATH").ok(),
```

- [ ] **Step 3: Wire structured LLM provider**

In `crates/storyteller-server/src/server.rs` `run_server()`, replace `structured_llm: None` with:

```rust
let structured_llm: Option<Arc<dyn storyteller_core::traits::structured_llm::StructuredLlmProvider>> = {
    let provider = storyteller_engine::inference::structured::OllamaStructuredProvider::new(
        storyteller_core::traits::structured_llm::StructuredLlmConfig {
            base_url: config.ollama_url.clone(),
            model: config.decomposition_model.clone(),
            ..Default::default()
        },
    );
    Some(Arc::new(provider))
};
```

- [ ] **Step 4: Wire character predictor**

In `crates/storyteller-server/src/server.rs` `run_server()`, replace `predictor_available: false` with:

```rust
let predictor = config.model_path.as_ref().and_then(|path| {
    let model_path = std::path::Path::new(path);
    match storyteller_ml::CharacterPredictor::load(model_path) {
        Ok(p) => {
            tracing::info!("Character predictor loaded from {}", path);
            Some(p)
        }
        Err(e) => {
            tracing::warn!("Character predictor not available: {}", e);
            None
        }
    }
});
let predictor_available = predictor.is_some();
```

Update the `EngineProviders` construction to use the new values.

- [ ] **Step 5: Verify compilation**

Run: `cargo check -p storyteller-server`
Expected: Clean compilation. May need to adjust imports.

- [ ] **Step 6: Commit**

```bash
git add crates/storyteller-server/
git commit -m "feat: wire structured LLM and character predictor in server startup"
```

---

### Task 12: Update EngineProviders for full pipeline support

Before porting the pipeline, `EngineProviders` needs to hold the actual predictor and grammar, not just a boolean flag.

**Files:**
- Modify: `crates/storyteller-server/src/engine/providers.rs`
- Modify: `crates/storyteller-server/src/server.rs` (update construction)
- Modify: `crates/storyteller-server/src/grpc/engine_service.rs` (update `check_health` to use new field)

**Reference:** `crates/storyteller-workshop/src-tauri/src/commands.rs:454-508` (workshop provider construction)

- [ ] **Step 1: Update EngineProviders struct**

In `crates/storyteller-server/src/engine/providers.rs`, change:

```rust
pub struct EngineProviders {
    pub narrator_llm: Arc<dyn LlmProvider>,
    pub structured_llm: Option<Arc<dyn StructuredLlmProvider>>,
    pub intent_llm: Option<Arc<dyn LlmProvider>>,
    pub predictor: Option<storyteller_ml::CharacterPredictor>,
    pub grammar: storyteller_engine::inference::frame::PlutchikWestern,
}
```

Replace `predictor_available: bool` with `predictor: Option<CharacterPredictor>`. Add `grammar` field for emotion mapping used by the prediction phase.

- [ ] **Step 2: Update server.rs provider construction**

In `crates/storyteller-server/src/server.rs`, construct `PlutchikWestern::default()` for the grammar and pass the `predictor` (from Task 11) directly instead of `predictor_available`.

- [ ] **Step 3: Update check_health to use new field**

In `check_health`, change `providers.predictor_available` to `providers.predictor.is_some()`.

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p storyteller-server`
Expected: Clean compilation.

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-server/
git commit -m "refactor: store CharacterPredictor and PlutchikWestern in EngineProviders"
```

---

### Task 13: Port turn pipeline into SubmitInput

Ports the pipeline from `crates/storyteller-workshop/src-tauri/src/commands.rs:560-1039` into the server's `SubmitInput` handler.

**Files:**
- Modify: `crates/storyteller-server/src/grpc/engine_service.rs` (lines 264-439)
- Modify: `crates/storyteller-server/Cargo.toml` (may need additional deps)

**Reference:** `crates/storyteller-workshop/src-tauri/src/commands.rs:560-1039`

**Key structural differences from the workshop:**
- Workshop uses `EngineState` struct → server uses `EngineStateManager` with `load()`/`update_runtime_snapshot()`
- Workshop has direct provider references → server has `Arc<EngineProviders>`
- Workshop emits Tauri events → server emits `EngineEvent` via `tokio::sync::mpsc`
- Workshop stores results in local variables → server must persist to `EventWriter`/`TurnWriter`

- [ ] **Step 1: Study the workshop pipeline**

Read `crates/storyteller-workshop/src-tauri/src/commands.rs` lines 560-1039 carefully. Map each phase to its inputs:

| Phase | Workshop function | Key inputs from state |
|-------|------------------|-----------------------|
| Decomposition | `structured_llm.extract(request)` | `structured_llm`, last journal entry + input |
| Prediction | `predict_character_behaviors(...)` | `predictor`, characters, scene, input, grammar, event_features, history |
| Arbitration | `check_action_possibility(...)` | input, CapabilityLexicon |
| Intent synthesis | `synthesize_intents(...)` | `intent_llm`, characters, predictions, journal, input, scene, player_entity_id, intentions |
| Context assembly | `assemble_narrator_context(...)` | scene, characters, journal, resolver_output, input, entity_ids, token_budget |
| Narrator | `NarratorAgent::new(&context, llm).render(...)` | `narrator_llm`, assembled context |

- [ ] **Step 2: Add required dependencies**

Ensure `crates/storyteller-server/Cargo.toml` has all dependencies needed for pipeline functions. The functions live in `storyteller-engine` (inference, systems, agents modules) and `storyteller-ml` (prediction).

- [ ] **Step 3: Port decomposition phase**

Replace the decomposition placeholder (lines ~287-316) with:
1. Load `RuntimeSnapshot` from `EngineStateManager`
2. Format input with narrator context: `format!("[Narrator]\n{}\n\n[Player]\n{}", last_journal_entry, input)`
3. If `providers.structured_llm` is Some, call `structured_llm.extract(request)` and parse via `EventDecomposition::from_json()`
4. Emit `PhaseStarted("decomposition")` then `DecompositionComplete { raw_json }`

- [ ] **Step 4: Port prediction phase**

Replace the prediction placeholder (lines ~318-337) with:
1. If `providers.predictor` is Some, call `predict_character_behaviors()`
2. Build `ResolverOutput` from predictions
3. Emit `PhaseStarted("prediction")` then `PredictionComplete { raw_json }`

- [ ] **Step 5: Port arbitration phase**

Replace the arbitration placeholder (lines ~339-349) with:
1. Call `check_action_possibility(&input, &[], &CapabilityLexicon::new(), None)`
2. Emit `ArbitrationComplete { verdict, details }`

- [ ] **Step 6: Port intent synthesis phase**

Replace the intent synthesis placeholder (lines ~351-360) with:
1. If `providers.intent_llm` is Some, call `synthesize_intents()`
2. Emit `IntentSynthesisComplete { intent_statements }`

- [ ] **Step 7: Port context assembly phase**

Replace the context assembly placeholder (lines ~362-374) with:
1. Call `assemble_narrator_context()` with all accumulated state
2. Inject goals into preamble if available (from `Composition.goals`)
3. Emit `ContextAssembled { preamble_tokens, journal_tokens, retrieved_tokens, total_tokens }`

- [ ] **Step 8: Port narrator phase**

Replace the narrator placeholder (lines ~376-408) with:
1. Create `NarratorAgent::new(&context, Arc::clone(&providers.narrator_llm)).with_temperature(0.8)`
2. Call `narrator.render(&context, &observer).await`
3. Emit `NarratorComplete { prose, generation_ms }`

- [ ] **Step 9: Update state and persist**

After all phases:
1. Update `RuntimeSnapshot` via `EngineStateManager::update_runtime_snapshot()` — increment turn count, append journal entries
2. Persist events to `EventWriter` and turn to `TurnWriter`
3. Emit `TurnComplete { turn, total_ms }`

- [ ] **Step 10: Verify compilation**

Run: `cargo check -p storyteller-server`
Expected: Clean compilation.

- [ ] **Step 11: Commit**

```bash
git add crates/storyteller-server/
git commit -m "feat: port full turn pipeline from workshop into SubmitInput RPC"
```

---

### Task 14: Wire real opening narration in ComposeScene

Note: Task numbering continues from Task 13 above (Tasks 5+6 were merged in Chunk 1).

**Files:**
- Modify: `crates/storyteller-server/src/grpc/engine_service.rs` (ComposeScene handler, around line 211)

- [ ] **Step 1: Study workshop's opening narration**

Read `crates/storyteller-workshop/src-tauri/src/commands.rs` to find how opening narration is generated. Look for calls to `NarratorAgent` or `assemble_narrator_context` in the scene composition flow (likely in a `setup_and_render_opening()` or similar function).

- [ ] **Step 2: Replace placeholder opening narration**

In the `ComposeScene` handler, replace the hardcoded opening narration string with:
1. Build a narrator context from the composed scene (preamble with scene setting, characters, tone)
2. Call `narrator_llm.complete()` or `NarratorAgent::new().render()` to generate the opening
3. Emit the real prose in the `NarratorComplete` event

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p storyteller-server`
Expected: Clean compilation.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-server/src/grpc/engine_service.rs
git commit -m "feat: wire real narrator LLM for opening narration in ComposeScene"
```

---

### Task 15: Pipeline integration tests

**Files:**
- Modify: `crates/storyteller-server/tests/grpc_integration.rs`

- [ ] **Step 1: Add test helper for full server startup**

Extend the existing test helper in `grpc_integration.rs` to start both `ComposerService` and `EngineService` with real providers (or mock providers for unit-level tests).

- [ ] **Step 2: Write ComposeScene integration test**

Add a test that:
1. Starts the server with test fixtures
2. Connects via the generated client stubs
3. Sends a `ComposeSceneRequest` with valid selections
4. Collects the event stream
5. Asserts: receives `PhaseStarted`, `SceneComposed`, `GoalsGenerated`, `NarratorComplete`, `TurnComplete` events in order
6. Asserts: `SceneComposed` contains character names and scene title

- [ ] **Step 3: Write CheckHealth integration test**

Add a test that:
1. Starts the server
2. Calls `CheckHealth`
3. Asserts: response has `narrator_llm` subsystem as "healthy"
4. Asserts: overall status is "healthy" or "degraded" depending on provider availability

- [ ] **Step 4: Run tests**

Run: `cargo test -p storyteller-server`
Expected: All tests pass (integration tests that need Ollama should be behind `test-llm` feature flag).

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-server/tests/
git commit -m "test: add integration tests for CheckHealth and ComposeScene RPCs"
```

---

## Chunk 4: CLI + Composer Cache

### Task 16: Composer cache implementation

**Files:**
- Create: `crates/storyteller-cli/src/composer_cache.rs`
- Modify: `crates/storyteller-cli/Cargo.toml` (add `storyteller-client`, `serde_json`)

- [ ] **Step 1: Add CLI dependencies**

In `crates/storyteller-cli/Cargo.toml`, add:
```toml
storyteller-client = { path = "../storyteller-client" }
storyteller-core = { path = "../storyteller-core" }
serde = { workspace = true }
serde_json = { workspace = true }
```

- [ ] **Step 2: Create cache types and sync logic**

Create `crates/storyteller-cli/src/composer_cache.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use storyteller_client::StorytellerClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub slug: String,
    pub entity_id: String,
    pub display_name: String,
}

pub struct ComposerCache {
    root: PathBuf,
}

impl ComposerCache {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn default_path() -> PathBuf {
        PathBuf::from(".story/composition-cache")
    }

    /// Sync all genres and per-genre descriptor indexes from the server.
    pub async fn sync(&self, client: &mut StorytellerClient) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(&self.root)?;

        // Sync genres
        let genres = client.list_genres().await?;
        let genre_entries: Vec<CacheEntry> = genres
            .genres
            .iter()
            .map(|g| CacheEntry {
                slug: g.id.clone(),
                entity_id: g.entity_id.clone(),
                display_name: g.display_name.clone(),
            })
            .collect();
        self.write_index(&self.root.join("genres.json"), &genre_entries)?;

        // Sync per-genre descriptors
        for genre in &genres.genres {
            let genre_dir = self.root.join(&genre.id);
            std::fs::create_dir_all(&genre_dir)?;

            let genre_id = &genre.entity_id;

            // Archetypes
            let archetypes = client.get_archetypes_for_genre(genre_id).await?;
            let entries: Vec<CacheEntry> = archetypes.archetypes.iter().map(|a| CacheEntry {
                slug: a.id.clone(),
                entity_id: a.entity_id.clone(),
                display_name: a.display_name.clone(),
            }).collect();
            self.write_index(&genre_dir.join("archetypes.json"), &entries)?;

            // Profiles
            let profiles = client.get_profiles_for_genre(genre_id).await?;
            let entries: Vec<CacheEntry> = profiles.profiles.iter().map(|p| CacheEntry {
                slug: p.id.clone(),
                entity_id: p.entity_id.clone(),
                display_name: p.display_name.clone(),
            }).collect();
            self.write_index(&genre_dir.join("profiles.json"), &entries)?;

            // Dynamics
            let dynamics = client.get_dynamics_for_genre(genre_id, vec![]).await?;
            let entries: Vec<CacheEntry> = dynamics.dynamics.iter().map(|d| CacheEntry {
                slug: d.id.clone(),
                entity_id: d.entity_id.clone(),
                display_name: d.display_name.clone(),
            }).collect();
            self.write_index(&genre_dir.join("dynamics.json"), &entries)?;

            // Names
            let names = client.get_names_for_genre(genre_id).await?;
            let entries: Vec<CacheEntry> = names.names.iter().map(|n| CacheEntry {
                slug: n.id.clone(),
                entity_id: n.entity_id.clone(),
                display_name: n.display_name.clone(),
            }).collect();
            self.write_index(&genre_dir.join("names.json"), &entries)?;

            // Settings
            let settings = client.get_settings_for_genre(genre_id).await?;
            let entries: Vec<CacheEntry> = settings.settings.iter().map(|s| CacheEntry {
                slug: s.id.clone(),
                entity_id: s.entity_id.clone(),
                display_name: s.display_name.clone(),
            }).collect();
            self.write_index(&genre_dir.join("settings.json"), &entries)?;
        }

        Ok(())
    }

    /// Resolve a slug to an entity_id from the cached index.
    pub fn resolve_slug(&self, category: &str, genre_slug: Option<&str>, slug: &str) -> Result<String, String> {
        let path = match genre_slug {
            Some(g) => self.root.join(g).join(format!("{category}.json")),
            None => self.root.join(format!("{category}.json")),
        };

        if !path.exists() {
            return Err(format!(
                "Cache not found at {}. Run `storyteller-cli composer sync` first.",
                path.display()
            ));
        }

        let data = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read cache: {e}"))?;
        let entries: Vec<CacheEntry> = serde_json::from_str(&data)
            .map_err(|e| format!("Failed to parse cache: {e}"))?;

        entries
            .iter()
            .find(|e| e.slug == slug)
            .map(|e| e.entity_id.clone())
            .ok_or_else(|| format!(
                "Slug '{slug}' not found in {category} cache. Run `storyteller-cli composer sync` to refresh."
            ))
    }

    /// List entries from a cached index.
    pub fn list(&self, category: &str, genre_slug: Option<&str>) -> Result<Vec<CacheEntry>, String> {
        let path = match genre_slug {
            Some(g) => self.root.join(g).join(format!("{category}.json")),
            None => self.root.join(format!("{category}.json")),
        };

        if !path.exists() {
            return Err(format!(
                "Cache not found at {}. Run `storyteller-cli composer sync` first.",
                path.display()
            ));
        }

        let data = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read cache: {e}"))?;
        serde_json::from_str(&data)
            .map_err(|e| format!("Failed to parse cache: {e}"))
    }

    fn write_index(&self, path: &Path, entries: &[CacheEntry]) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(entries)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}
```

- [ ] **Step 3: Add unit tests for cache**

Add to the bottom of `composer_cache.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn resolve_slug_from_cache() {
        let dir = TempDir::new().unwrap();
        let cache = ComposerCache::new(dir.path().to_path_buf());

        let entries = vec![CacheEntry {
            slug: "dark_fantasy".into(),
            entity_id: "019ce4c3-0001".into(),
            display_name: "Dark Fantasy".into(),
        }];
        cache.write_index(&dir.path().join("genres.json"), &entries).unwrap();

        let id = cache.resolve_slug("genres", None, "dark_fantasy").unwrap();
        assert_eq!(id, "019ce4c3-0001");
    }

    #[test]
    fn resolve_slug_missing_cache_gives_helpful_error() {
        let dir = TempDir::new().unwrap();
        let cache = ComposerCache::new(dir.path().to_path_buf());

        let err = cache.resolve_slug("genres", None, "dark_fantasy").unwrap_err();
        assert!(err.contains("composer sync"));
    }

    #[test]
    fn resolve_slug_not_found_gives_helpful_error() {
        let dir = TempDir::new().unwrap();
        let cache = ComposerCache::new(dir.path().to_path_buf());

        let entries = vec![CacheEntry {
            slug: "low_fantasy".into(),
            entity_id: "019ce4c3-0002".into(),
            display_name: "Low Fantasy".into(),
        }];
        cache.write_index(&dir.path().join("genres.json"), &entries).unwrap();

        let err = cache.resolve_slug("genres", None, "dark_fantasy").unwrap_err();
        assert!(err.contains("not found"));
        assert!(err.contains("composer sync"));
    }

    #[test]
    fn list_entries_from_cache() {
        let dir = TempDir::new().unwrap();
        let cache = ComposerCache::new(dir.path().to_path_buf());

        let entries = vec![
            CacheEntry { slug: "a".into(), entity_id: "1".into(), display_name: "A".into() },
            CacheEntry { slug: "b".into(), entity_id: "2".into(), display_name: "B".into() },
        ];
        cache.write_index(&dir.path().join("genres.json"), &entries).unwrap();

        let listed = cache.list("genres", None).unwrap();
        assert_eq!(listed.len(), 2);
    }
}
```

- [ ] **Step 4: Add tempfile dev-dependency**

In `crates/storyteller-cli/Cargo.toml`:
```toml
[dev-dependencies]
tempfile = { workspace = true }
```

- [ ] **Step 5: Verify tests pass**

Run: `cargo test -p storyteller-cli -- composer_cache`
Expected: All 4 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/storyteller-cli/
git commit -m "feat: implement composer cache with slug resolution for CLI"
```

---

### Task 17: Player simulation module

The CLI must NOT depend on `storyteller-engine` (which pulls in bevy, ort, etc.). Instead, implement a minimal Ollama client directly using `reqwest` — it's a single POST to `/api/chat`.

**Files:**
- Create: `crates/storyteller-cli/src/player_simulation.rs`
- Modify: `crates/storyteller-cli/Cargo.toml` (add `reqwest`)

- [ ] **Step 1: Add reqwest dependency**

In `crates/storyteller-cli/Cargo.toml`:
```toml
reqwest = { workspace = true, features = ["json"] }
```

- [ ] **Step 2: Create the player simulation module**

Create `crates/storyteller-cli/src/player_simulation.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct PlayerSimulation {
    client: reqwest::Client,
    ollama_url: String,
    model: String,
    system_prompt: String,
}

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: u32,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaResponseMessage,
}

#[derive(Deserialize)]
struct OllamaResponseMessage {
    content: String,
}

impl PlayerSimulation {
    pub fn new(ollama_url: &str, model: &str, protagonist_context: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("failed to build HTTP client");

        let system_prompt = format!(
            "You are playing a character in an interactive story. Stay in character and respond \
             naturally to what the narrator describes. Keep responses to 1-3 sentences — you are \
             a player giving input, not writing prose.\n\n\
             Your character:\n{protagonist_context}"
        );

        Self {
            client,
            ollama_url: ollama_url.to_string(),
            model: model.to_string(),
            system_prompt,
        }
    }

    /// Build protagonist context from SceneComposed event data.
    /// Uses composition_json to extract character details since SceneComposed
    /// only has cast_names (strings) at the proto level.
    pub fn build_protagonist_context(
        protagonist_name: &str,
        composition_json: &str,
    ) -> String {
        // Parse composition_json to find protagonist details
        if let Ok(composition) = serde_json::from_str::<serde_json::Value>(composition_json) {
            if let Some(characters) = composition.get("characters").and_then(|c| c.as_array()) {
                for character in characters {
                    let name = character.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    if name == protagonist_name {
                        let performance_notes = character
                            .get("performance_notes")
                            .and_then(|p| p.as_str())
                            .unwrap_or("");
                        let backstory = character
                            .get("backstory")
                            .and_then(|b| b.as_str())
                            .unwrap_or("");
                        return format!(
                            "Name: {name}\n{performance_notes}\n\nBackstory: {backstory}"
                        );
                    }
                }
            }
        }
        format!("Name: {protagonist_name}")
    }

    /// Generate player input given the latest narrator output.
    pub async fn generate_input(
        &self,
        narrator_output: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let request = OllamaChatRequest {
            model: self.model.clone(),
            messages: vec![
                OllamaMessage {
                    role: "system".to_string(),
                    content: self.system_prompt.clone(),
                },
                OllamaMessage {
                    role: "user".to_string(),
                    content: format!(
                        "The narrator says:\n\n{narrator_output}\n\nWhat do you do?"
                    ),
                },
            ],
            stream: false,
            options: OllamaOptions {
                temperature: 0.9,
                num_predict: 200,
            },
        };

        let response = self
            .client
            .post(format!("{}/api/chat", self.ollama_url))
            .json(&request)
            .send()
            .await?
            .json::<OllamaChatResponse>()
            .await?;

        Ok(response.message.content.trim().to_string())
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p storyteller-cli`
Expected: Clean compilation. No dependency on `storyteller-engine`.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-cli/
git commit -m "feat: add player simulation module using direct Ollama HTTP client"
```

---

### Task 18: CLI subcommand structure

**Files:**
- Modify: `crates/storyteller-cli/src/main.rs`
- Create: `crates/storyteller-cli/src/playtest.rs`
- Create: `crates/storyteller-cli/src/compose.rs`

- [ ] **Step 1: Define the full CLI structure**

Replace `crates/storyteller-cli/src/main.rs`:

```rust
use clap::Parser;

mod compose;
mod composer_cache;
mod playtest;
mod player_simulation;

#[derive(Parser)]
#[command(name = "storyteller-cli", about = "Storyteller engine CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Run an automated playtest against the engine server
    Playtest(playtest::PlaytestArgs),

    /// Compose a scene via the engine server
    Compose(compose::ComposeArgs),

    /// Manage the local composer descriptor cache
    #[command(subcommand)]
    Composer(ComposerCommands),
}

#[derive(clap::Subcommand)]
enum ComposerCommands {
    /// Sync descriptor cache from the server
    Sync,

    /// List cached descriptors
    #[command(subcommand)]
    List(ListCommands),
}

#[derive(clap::Subcommand)]
enum ListCommands {
    /// List available genres
    Genres,
    /// List archetypes for a genre
    Archetypes {
        #[arg(long)]
        genre: String,
    },
    /// List profiles for a genre
    Profiles {
        #[arg(long)]
        genre: String,
    },
    /// List dynamics for a genre
    Dynamics {
        #[arg(long)]
        genre: String,
    },
    /// List name pools for a genre
    Names {
        #[arg(long)]
        genre: String,
    },
    /// List settings for a genre
    Settings {
        #[arg(long)]
        genre: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::Playtest(args) => playtest::run(args).await?,
        Commands::Compose(args) => compose::run(args).await?,
        Commands::Composer(cmd) => match cmd {
            ComposerCommands::Sync => {
                let config = storyteller_client::ClientConfig::from_env();
                let mut client = storyteller_client::StorytellerClient::connect(config).await?;
                let cache = composer_cache::ComposerCache::new(
                    composer_cache::ComposerCache::default_path(),
                );
                cache.sync(&mut client).await?;
                println!("Composer cache synced successfully.");
            }
            ComposerCommands::List(list_cmd) => {
                let cache = composer_cache::ComposerCache::new(
                    composer_cache::ComposerCache::default_path(),
                );
                let entries = match list_cmd {
                    ListCommands::Genres => cache.list("genres", None)?,
                    ListCommands::Archetypes { genre } => cache.list("archetypes", Some(&genre))?,
                    ListCommands::Profiles { genre } => cache.list("profiles", Some(&genre))?,
                    ListCommands::Dynamics { genre } => cache.list("dynamics", Some(&genre))?,
                    ListCommands::Names { genre } => cache.list("names", Some(&genre))?,
                    ListCommands::Settings { genre } => cache.list("settings", Some(&genre))?,
                };
                for entry in &entries {
                    println!("{:<30} {:<40} {}", entry.slug, entry.entity_id, entry.display_name);
                }
            }
        },
    }

    Ok(())
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p storyteller-cli`
Expected: Will fail until playtest.rs and compose.rs exist — that's the next steps.

- [ ] **Step 3: Commit (with next steps)**

Hold this commit until playtest.rs and compose.rs are created in the following tasks.

---

### Task 19: Compose subcommand

**Files:**
- Create: `crates/storyteller-cli/src/compose.rs`

- [ ] **Step 1: Implement the compose subcommand**

Create `crates/storyteller-cli/src/compose.rs`:

```rust
use crate::composer_cache::ComposerCache;
use storyteller_client::{StorytellerClient, ClientConfig};

#[derive(clap::Args)]
pub struct ComposeArgs {
    /// Genre slug (from composer cache)
    #[arg(long)]
    genre: String,

    /// Profile slug (from composer cache)
    #[arg(long)]
    profile: String,

    /// Cast selections as "archetype_slug:role" pairs, comma-separated
    #[arg(long, value_delimiter = ',')]
    cast: Vec<String>,

    /// Dynamic pairings as "slug:idx_a,idx_b" entries, semicolon-separated
    #[arg(long, value_delimiter = ';')]
    dynamics: Vec<String>,

    /// Output file path (stdout if not specified)
    #[arg(long, short)]
    output: Option<String>,

    /// Random seed for composition
    #[arg(long)]
    seed: Option<u64>,
}

pub async fn run(args: ComposeArgs) -> Result<(), Box<dyn std::error::Error>> {
    let cache = ComposerCache::new(ComposerCache::default_path());

    // Resolve slugs to entity IDs
    let genre_id = cache.resolve_slug("genres", None, &args.genre)?;
    let profile_id = cache.resolve_slug("profiles", Some(&args.genre), &args.profile)?;

    // Parse cast selections
    let mut cast = Vec::new();
    for entry in &args.cast {
        let parts: Vec<&str> = entry.split(':').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid cast format: '{entry}'. Use 'archetype_slug:role'").into());
        }
        let archetype_id = cache.resolve_slug("archetypes", Some(&args.genre), parts[0])?;
        cast.push(storyteller_client::proto::CastMember {
            archetype_id,
            name: String::new(), // server will assign from name pool
            role: parts[1].to_string(),
        });
    }

    // Parse dynamic pairings
    let mut dynamics = Vec::new();
    for entry in &args.dynamics {
        let parts: Vec<&str> = entry.split(':').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid dynamics format: '{entry}'. Use 'slug:idx_a,idx_b'").into());
        }
        let dynamic_id = cache.resolve_slug("dynamics", Some(&args.genre), parts[0])?;
        let indices: Vec<&str> = parts[1].split(',').collect();
        if indices.len() != 2 {
            return Err(format!("Invalid dynamics indices: '{}'", parts[1]).into());
        }
        dynamics.push(storyteller_client::proto::DynamicPairing {
            dynamic_id,
            cast_index_a: indices[0].parse()?,
            cast_index_b: indices[1].parse()?,
        });
    }

    // Connect and compose
    let config = ClientConfig::from_env();
    let mut client = StorytellerClient::connect(config).await?;

    let request = storyteller_client::proto::ComposeSceneRequest {
        genre_id,
        profile_id,
        cast,
        dynamics,
        seed: args.seed,
        title_override: None,
        setting_override: None,
    };

    let mut stream = client.compose_scene(request).await?;

    // Collect composition data from events.
    // SceneComposed.composition_json contains the full composition (selections + scene +
    // characters + goals) in the same format the server persists to composition.json.
    let mut composition_json_str = None;
    let mut session_id = String::new();
    while let Some(event) = stream.message().await? {
        session_id = event.session_id.clone();
        if let Some(payload) = &event.payload {
            match payload {
                storyteller_client::proto::engine_event::Payload::SceneComposed(scene) => {
                    composition_json_str = Some(scene.composition_json.clone());
                    println!("Scene composed: {}", scene.title);
                }
                storyteller_client::proto::engine_event::Payload::Error(err) => {
                    return Err(format!("Composition failed: {}", err.message).into());
                }
                _ => {}
            }
        }
    }

    let json = composition_json_str.ok_or("No SceneComposed event received")?;
    // Pretty-print if it's valid JSON, otherwise pass through
    let json = serde_json::from_str::<serde_json::Value>(&json)
        .map(|v| serde_json::to_string_pretty(&v).unwrap_or(json.clone()))
        .unwrap_or(json);

    match args.output {
        Some(path) => {
            std::fs::write(&path, &json)?;
            println!("Composition written to {path}");
        }
        None => println!("{json}"),
    }

    Ok(())
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p storyteller-cli`
Expected: May need to adjust proto type field names to match actual generated code.

- [ ] **Step 3: Commit**

Hold until playtest.rs is also created.

---

### Task 20: Playtest subcommand

**Files:**
- Create: `crates/storyteller-cli/src/playtest.rs`

- [ ] **Step 1: Implement the playtest subcommand**

Create `crates/storyteller-cli/src/playtest.rs`:

```rust
use crate::player_simulation::PlayerSimulation;
use storyteller_client::{ClientConfig, StorytellerClient};
use storyteller_client::proto::engine_event;
use std::time::Instant;

#[derive(clap::Args)]
pub struct PlaytestArgs {
    /// Path to composition.json file
    #[arg(long, short)]
    file: String,

    /// Number of turns to play
    #[arg(long, default_value = "5")]
    turns: u32,

    /// Player simulation model name
    #[arg(long, default_value = "qwen2.5:7b-instruct")]
    player_model: String,
}

pub async fn run(args: PlaytestArgs) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();

    // Connect and health check
    let config = ClientConfig::from_env();
    let mut client = StorytellerClient::connect(config).await?;

    let health = client.check_health().await?;
    let narrator_health = health.subsystems.iter().find(|s| s.name == "narrator_llm");
    if let Some(narrator) = narrator_health {
        use storyteller_core::types::health::HealthStatus;
        if narrator.status == HealthStatus::Unavailable {
            return Err("Narrator LLM is unavailable. Cannot run playtest.".into());
        }
    }

    println!("Connected to server. Health: {:?}", health.status);

    // Read composition file
    let composition_data = std::fs::read_to_string(&args.file)?;
    let composition: serde_json::Value = serde_json::from_str(&composition_data)?;

    // Build ComposeSceneRequest from composition file
    // The exact mapping depends on the composition.json schema
    let request = build_compose_request(&composition)?;

    // Compose scene
    println!("Composing scene...");
    let mut stream = client.compose_scene(request).await?;

    let mut session_id = String::new();
    let mut narrator_output = String::new();
    let mut protagonist_name = String::new();
    let mut composition_json = String::new();

    while let Some(event) = stream.message().await? {
        session_id = event.session_id.clone();
        if let Some(payload) = &event.payload {
            match payload {
                engine_event::Payload::SceneComposed(scene) => {
                    println!("Scene: {}", scene.title);
                    composition_json = scene.composition_json.clone();
                    // Protagonist is the first cast member
                    if let Some(name) = scene.cast_names.first() {
                        protagonist_name = name.clone();
                    }
                }
                engine_event::Payload::NarratorComplete(narrator) => {
                    narrator_output = narrator.prose.clone();
                    println!("\n--- Opening ---\n{}\n", narrator.prose);
                }
                engine_event::Payload::Error(err) => {
                    return Err(format!("Composition error: {}", err.message).into());
                }
                _ => {}
            }
        }
    }

    // Set up player simulation
    let ollama_url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());
    let protagonist_context =
        PlayerSimulation::build_protagonist_context(&protagonist_name, &composition_json);
    let player_sim = PlayerSimulation::new(&ollama_url, &args.player_model, &protagonist_context);

    // Turn loop
    for turn in 1..=args.turns {
        println!("--- Turn {turn}/{} ---", args.turns);

        // Generate player input
        let player_input = player_sim.generate_input(&narrator_output).await?;
        println!("[Player]: {player_input}");

        // Submit input
        let submit_request = storyteller_client::proto::SubmitInputRequest {
            session_id: session_id.clone(),
            input: player_input,
        };

        let mut stream = client.submit_input(submit_request).await?;
        while let Some(event) = stream.message().await? {
            if let Some(payload) = &event.payload {
                match payload {
                    engine_event::Payload::NarratorComplete(narrator) => {
                        narrator_output = narrator.prose.clone();
                        println!("[Narrator]: {}\n", narrator.prose);
                    }
                    engine_event::Payload::Error(err) => {
                        eprintln!("Turn error: {}", err.message);
                    }
                    _ => {}
                }
            }
        }
    }

    // Summary
    let elapsed = start.elapsed();
    println!("--- Playtest Complete ---");
    println!("Session:  {session_id}");
    println!("Turns:    {}", args.turns);
    println!("Elapsed:  {:.1}s", elapsed.as_secs_f64());

    Ok(())
}

fn build_compose_request(
    composition: &serde_json::Value,
) -> Result<storyteller_client::proto::ComposeSceneRequest, Box<dyn std::error::Error>> {
    // Parse the composition.json's selections field to build a ComposeSceneRequest.
    // The composition.json is the format written by the server's CompositionWriter:
    // { "selections": { "genre_id": "...", "profile_id": "...", "cast": [...], ... }, "scene": {...}, ... }
    let selections = composition.get("selections")
        .ok_or("composition.json missing 'selections' field")?;

    // Map selections fields to proto request.
    // The selections JSON and proto request have compatible field names.
    let genre_id = selections.get("genre_id")
        .and_then(|v| v.as_str())
        .ok_or("missing genre_id in selections")?
        .to_string();
    let profile_id = selections.get("profile_id")
        .and_then(|v| v.as_str())
        .ok_or("missing profile_id in selections")?
        .to_string();

    // Cast and dynamics need manual mapping from JSON to proto types
    let cast = selections.get("cast")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|c| {
            Some(storyteller_client::proto::CastMember {
                archetype_id: c.get("archetype_id")?.as_str()?.to_string(),
                name: c.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()),
                role: c.get("role")?.as_str()?.to_string(),
            })
        }).collect())
        .unwrap_or_default();

    let dynamics = selections.get("dynamics")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|d| {
            Some(storyteller_client::proto::DynamicPairing {
                dynamic_id: d.get("dynamic_id")?.as_str()?.to_string(),
                cast_index_a: d.get("cast_index_a")?.as_u64()? as u32,
                cast_index_b: d.get("cast_index_b")?.as_u64()? as u32,
            })
        }).collect())
        .unwrap_or_default();

    Ok(storyteller_client::proto::ComposeSceneRequest {
        genre_id,
        profile_id,
        cast,
        dynamics,
        seed: selections.get("seed").and_then(|v| v.as_u64()),
        title_override: selections.get("title_override").and_then(|v| v.as_str()).map(|s| s.to_string()),
        setting_override: selections.get("setting_override").and_then(|v| v.as_str()).map(|s| s.to_string()),
    })
}
```

Note: The `build_compose_request` function will need adjustment based on the actual `composition.json` schema and how `ComposeSceneRequest` proto maps to/from JSON. The proto-generated struct may not directly deserialize from the composition JSON — a manual mapping may be needed.

- [ ] **Step 2: Verify full CLI compilation**

Run: `cargo check -p storyteller-cli`
Expected: Clean compilation.

- [ ] **Step 3: Commit all CLI work**

```bash
git add crates/storyteller-cli/
git commit -m "feat: implement CLI with playtest, compose, and composer cache subcommands"
```

---

### Task 21: End-to-end integration tests

**Files:**
- Create: `crates/storyteller-client/tests/client_integration.rs`

- [ ] **Step 1: Create client integration test**

Create `crates/storyteller-client/tests/client_integration.rs`:

```rust
//! Integration tests requiring a running storyteller-server.
//! Gate behind test-llm feature since they need Ollama.

#[cfg(feature = "test-llm")]
mod integration {
    use storyteller_client::{ClientConfig, StorytellerClient};
    use storyteller_core::types::health::HealthStatus;

    #[tokio::test]
    async fn health_check_reports_subsystems() {
        let config = ClientConfig::from_env();
        let mut client = StorytellerClient::connect(config)
            .await
            .expect("Server should be running for integration tests");

        let health = client.check_health().await.expect("Health check should succeed");

        // Server should report at least narrator_llm
        assert!(!health.subsystems.is_empty());
        let narrator = health.subsystems.iter().find(|s| s.name == "narrator_llm");
        assert!(narrator.is_some(), "Should report narrator_llm subsystem");
    }

    #[tokio::test]
    async fn compose_scene_streams_events() {
        // This test requires valid descriptor data and Ollama running
        let config = ClientConfig::from_env();
        let mut client = StorytellerClient::connect(config).await.unwrap();

        // List genres to get a valid genre_id
        let genres = client.list_genres().await.unwrap();
        assert!(!genres.genres.is_empty(), "Should have at least one genre");

        // Further composition test would need valid selections
        // which depend on the descriptor data available
    }
}
```

- [ ] **Step 2: Add test-llm feature to client crate**

In `crates/storyteller-client/Cargo.toml`:
```toml
[features]
test-llm = []
```

- [ ] **Step 3: Run unit tests (no feature gate)**

Run: `cargo test -p storyteller-client`
Expected: Unit tests pass. Integration tests are skipped (behind feature flag).

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-client/
git commit -m "test: add integration test scaffolding for storyteller-client"
```

---

### Task 22: Final verification and cleanup

- [ ] **Step 1: Full workspace check**

Run: `cargo check --workspace --exclude storyteller-workshop`
Expected: Clean compilation.

- [ ] **Step 2: Full workspace tests**

Run: `cargo test --workspace --exclude storyteller-workshop`
Expected: All tests pass.

- [ ] **Step 3: Clippy**

Run: `cargo clippy --workspace --exclude storyteller-workshop --all-targets --all-features -- -D warnings`
Expected: No warnings.

- [ ] **Step 4: Format check**

Run: `cargo fmt --check`
Expected: No formatting issues.

- [ ] **Step 5: Commit any fixes**

```bash
git add -A
git commit -m "chore: fix clippy warnings and formatting"
```

- [ ] **Step 6: Update .env.example if needed**

Add any new environment variables:
- `STORYTELLER_SERVER_URL` (for client)
- `STORYTELLER_MODEL_PATH` (for predictor)
- `STORYTELLER_PLAYER_MODEL` (for playtest)

- [ ] **Step 7: Final commit**

```bash
git add .env.example
git commit -m "docs: update .env.example with Phase 2 environment variables"
```
