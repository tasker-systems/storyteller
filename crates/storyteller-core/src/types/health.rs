//! Server health types for structured health reporting.
//!
//! Used by the gRPC `CheckHealth` RPC and the client library to represent
//! server and subsystem health in a typed, serializable form.

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
        let status = subsystems.iter().map(|s| &s.status).fold(
            HealthStatus::Healthy,
            |worst, current| match (&worst, current) {
                (HealthStatus::Unavailable, _) | (_, HealthStatus::Unavailable) => {
                    HealthStatus::Unavailable
                }
                (HealthStatus::Degraded, _) | (_, HealthStatus::Degraded) => HealthStatus::Degraded,
                _ => HealthStatus::Healthy,
            },
        );
        Self { status, subsystems }
    }
}

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
