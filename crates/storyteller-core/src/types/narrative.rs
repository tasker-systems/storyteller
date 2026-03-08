//! Narrative graph types — gravitational landscape of the story.
//!
//! See: `docs/technical/narrative-graph-case-study-tfatd.md`
//!
//! Design decision: Stories are not branching trees but gravitational landscapes.
//! Scenes have mass that pulls the narrative; the player navigates attractor basins.
//! Mass formula: authored_base + structural_modifiers + dynamic_adjustment(player_state).

/// Narrative mass — how strongly a scene attracts the story toward it.
///
/// Higher mass means the narrative "wants" to reach this scene.
/// Mass is dynamic: it changes based on player state and prior choices.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NarrativeMass {
    /// Base mass set by the story designer.
    pub authored_base: f32,
    /// Accumulated modifiers from structural position in the graph.
    pub structural_modifier: f32,
    /// Dynamic adjustment based on current player state.
    pub dynamic_adjustment: f32,
}

impl NarrativeMass {
    /// Total effective mass at this moment.
    pub fn effective(&self) -> f32 {
        self.authored_base + self.structural_modifier + self.dynamic_adjustment
    }
}

/// How the player approaches a scene — encodes the journey, not just the destination.
///
/// The same scene reached by different paths is a different experience.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApproachVector {
    /// Emotional state on arrival.
    pub emotional_valence: f32,
    /// Information accumulated before reaching this scene.
    pub information_state: Vec<String>,
    /// Relationships active in the current context.
    pub relational_context: Vec<String>,
    /// Thematic threads the player has been following.
    pub thematic_threads: Vec<String>,
}
