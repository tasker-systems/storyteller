//! Narrator context types — the three-tier context assembly system.
//!
//! See: `docs/technical/narrator-architecture.md`
//!
//! The Narrator does not remember. The system remembers for it and provides
//! context on demand. Three tiers of context are assembled for each turn:
//!
//! 1. **Persistent preamble** (~600-800 tokens) — identity, anti-patterns,
//!    setting, cast, and narrative boundaries. Stable across the scene.
//! 2. **Rolling scene journal** (~800-1200 tokens) — progressively compressed
//!    record of what has happened. Recent turns are detailed; older turns
//!    compress to essentials.
//! 3. **Retrieved context** (~400-800 tokens) — on-demand facts, relationships,
//!    and history pulled via graph traversal when referenced entities need
//!    backstory the journal doesn't carry.

use chrono::{DateTime, Utc};

use super::entity::EntityId;
use super::resolver::ResolverOutput;
use super::scene::SceneId;

// ---------------------------------------------------------------------------
// Tier 1: Persistent preamble
// ---------------------------------------------------------------------------

/// Tier 1 — stable context about the Narrator's identity and scene.
///
/// Constructed at scene entry. Updated only when the cast changes
/// (entity promotion/demotion) or scene constraints shift.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersistentPreamble {
    /// The Narrator's voice and personality description.
    pub narrator_identity: String,
    /// Anti-patterns — what the Narrator should never do.
    pub anti_patterns: Vec<String>,
    /// The scene setting description.
    pub setting_description: String,
    /// Cast list with roles and brief descriptions.
    pub cast_descriptions: Vec<CastDescription>,
    /// Hard narrative boundaries that cannot be crossed.
    pub boundaries: Vec<String>,
}

/// A cast member's description within the preamble.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CastDescription {
    /// Entity identity.
    pub entity_id: EntityId,
    /// Display name.
    pub name: String,
    /// Role in this scene.
    pub role: String,
    /// Brief voice/manner description for the Narrator.
    pub voice_note: String,
}

// ---------------------------------------------------------------------------
// Tier 2: Scene journal with progressive compression
// ---------------------------------------------------------------------------

/// How compressed a journal entry is — recent entries are detailed,
/// older entries are progressively compressed.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum CompressionLevel {
    /// Full detail — recent turns. All actions, speech, subtext.
    Full,
    /// Key actions and speech preserved, subtext summarized.
    Summary,
    /// One-sentence essence of what happened.
    Skeleton,
}

/// A single entry in the scene journal — one turn's record.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JournalEntry {
    /// Which turn this records.
    pub turn_number: u32,
    /// When this turn occurred.
    pub timestamp: DateTime<Utc>,
    /// Current compression level.
    pub compression: CompressionLevel,
    /// The content at current compression level.
    pub content: String,
    /// Entity IDs mentioned in this entry (for retrieval indexing).
    pub referenced_entities: Vec<EntityId>,
    /// Emotional dynamics noted in this turn (for compression priority).
    pub emotional_markers: Vec<String>,
}

/// The rolling scene journal — Tier 2 of the Narrator's context.
///
/// Maintains a progressively compressed record of the scene so far.
/// Recent turns are detailed; older turns compress as the journal
/// approaches its token budget.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SceneJournal {
    /// Scene this journal belongs to.
    pub scene_id: SceneId,
    /// Journal entries in chronological order.
    pub entries: Vec<JournalEntry>,
    /// Target token budget for the journal. Compression triggers when exceeded.
    pub token_budget: u32,
}

impl SceneJournal {
    /// Create a new empty journal for a scene.
    pub fn new(scene_id: SceneId, token_budget: u32) -> Self {
        Self {
            scene_id,
            entries: Vec::new(),
            token_budget,
        }
    }

    /// Number of turns recorded.
    pub fn turn_count(&self) -> usize {
        self.entries.len()
    }
}

// ---------------------------------------------------------------------------
// Tier 3: Retrieved context
// ---------------------------------------------------------------------------

/// A piece of retrieved context — a fact, relationship, or history item
/// pulled on demand when the current turn references entities that need
/// backstory the journal doesn't carry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RetrievedContext {
    /// What this context is about.
    pub subject: String,
    /// The context content itself.
    pub content: String,
    /// Whether this information has been revealed to the player.
    pub revealed: bool,
    /// Emotional context — how this information is emotionally charged.
    pub emotional_context: Option<String>,
    /// Which events or entities sourced this context.
    pub source_entities: Vec<EntityId>,
}

// ---------------------------------------------------------------------------
// Assembled narrator context — the complete input for one turn
// ---------------------------------------------------------------------------

/// The complete assembled context for the Narrator for a single turn.
///
/// Built by the Storykeeper (context assembly system) from the three tiers
/// plus the Resolver's output for this turn.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NarratorContextInput {
    /// Tier 1: Persistent preamble — who the Narrator is and what the scene is.
    pub preamble: PersistentPreamble,
    /// Tier 2: Scene journal — what has happened so far.
    pub journal: SceneJournal,
    /// Tier 3: Retrieved context — on-demand backstory and facts.
    pub retrieved: Vec<RetrievedContext>,
    /// The Resolver's output for this turn — what the characters did.
    pub resolver_output: ResolverOutput,
    /// The classified player input that triggered this turn.
    pub player_input_summary: String,
    /// Total estimated token count across all tiers.
    pub estimated_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preamble_is_constructible() {
        let preamble = PersistentPreamble {
            narrator_identity: "Literary fiction, present tense, close third person".to_string(),
            anti_patterns: vec![
                "Exclamation marks".to_string(),
                "Fantasy exposition".to_string(),
            ],
            setting_description: "A smallholding outside Svyoritch".to_string(),
            cast_descriptions: vec![CastDescription {
                entity_id: EntityId::new(),
                name: "Bramblehoof".to_string(),
                role: "Visitor, catalyst".to_string(),
                voice_note: "Warm, reaches for metaphor".to_string(),
            }],
            boundaries: vec!["Pyotir cannot leave".to_string()],
        };
        assert_eq!(preamble.cast_descriptions.len(), 1);
        assert_eq!(preamble.anti_patterns.len(), 2);
    }

    #[test]
    fn journal_tracks_turns() {
        let mut journal = SceneJournal::new(SceneId::new(), 1200);
        assert_eq!(journal.turn_count(), 0);

        journal.entries.push(JournalEntry {
            turn_number: 1,
            timestamp: Utc::now(),
            compression: CompressionLevel::Full,
            content: "Bramblehoof approaches the fence.".to_string(),
            referenced_entities: vec![EntityId::new()],
            emotional_markers: vec!["anticipation".to_string()],
        });
        assert_eq!(journal.turn_count(), 1);
    }

    #[test]
    fn compression_levels_are_ordered() {
        assert!(CompressionLevel::Full < CompressionLevel::Summary);
        assert!(CompressionLevel::Summary < CompressionLevel::Skeleton);
    }

    #[test]
    fn retrieved_context_is_constructible() {
        let ctx = RetrievedContext {
            subject: "Bramblehoof's previous visit to Svyoritch".to_string(),
            content: "Years ago, gave a flute to a boy with musical talent".to_string(),
            revealed: false,
            emotional_context: Some("Hope mixed with guilt about leaving".to_string()),
            source_entities: vec![EntityId::new()],
        };
        assert!(!ctx.revealed);
        assert!(ctx.emotional_context.is_some());
    }
}
