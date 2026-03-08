//! Relational web types — asymmetric directed edges between entities.
//!
//! See: `docs/technical/relational-web-tfatd.md`, `docs/foundation/power.md`
//!
//! Design decision: Relationships use 5 substrate dimensions
//! (trust\[3\], affection, debt, history, projection) plus information_state
//! and configuration annotation. Power is emergent — computed from substrate
//! configuration + network topology, never stored.

use super::entity::EntityId;
use super::tensor::AxisValue;

/// A directed edge in the relational web — A's relationship *toward* B.
///
/// Relationships are asymmetric: A→B and B→A are independent edges
/// with different substrate values.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DirectedEdge {
    /// The entity holding this perspective.
    pub source: EntityId,
    /// The entity being related to.
    pub target: EntityId,
    /// The five substrate dimensions.
    pub substrate: RelationalSubstrate,
    /// What the source knows/believes about the target.
    pub information_state: InformationState,
}

/// The five substrate dimensions of a relationship.
///
/// These are not independent — their *configuration* (how they interact)
/// is where relational meaning lives.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RelationalSubstrate {
    /// Reliability trust — do they do what they say?
    pub trust_reliability: AxisValue,
    /// Competence trust — can they do what's needed?
    pub trust_competence: AxisValue,
    /// Benevolence trust — do they want good for me?
    pub trust_benevolence: AxisValue,
    /// Emotional warmth, care, attachment.
    pub affection: AxisValue,
    /// Obligations, favors, what is owed.
    pub debt: AxisValue,
}

/// What one entity knows or believes about another.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InformationState {
    /// Facts the source knows about the target.
    pub known_facts: Vec<String>,
    /// Beliefs that may or may not be true.
    pub beliefs: Vec<String>,
    /// Things the source doesn't know they don't know.
    pub blind_spots: Vec<String>,
}

/// Structural role in the relational network — determines topological power.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TopologicalRole {
    /// Controls access between clusters — power from position.
    Gate,
    /// Connects otherwise disconnected groups.
    Bridge,
    /// Highly connected — many relationships flow through them.
    Hub,
    /// Few connections — limited structural influence.
    Periphery,
}
