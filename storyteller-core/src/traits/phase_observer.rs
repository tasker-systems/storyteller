//! Pipeline phase observability — Layer 2 (session debug) event emission.
//!
//! See: `docs/technical/infrastructure-architecture.md` § Layer 2: Session Observability
//!
//! Each pipeline phase emits structured domain-language events through a
//! `PhaseObserver`. These are not infrastructure metrics — they carry
//! narrative-system semantics: "journal compressed turn 3 from Full to
//! Summary", "retrieved backstory for Bramblehoof (2 items, ~180 tokens)".
//!
//! The observer pattern decouples emission from consumption. Pipeline code
//! calls `observer.emit(event)` without knowing whether the consumer is a
//! debug WebSocket, an event ledger writer, or `/dev/null`.
//!
//! A `NoopObserver` is provided for tests and contexts where observability
//! is not needed.

use std::fmt;

use chrono::{DateTime, Utc};

use crate::types::entity::EntityId;
use crate::types::narrator_context::CompressionLevel;
use crate::types::turn_cycle::TurnCycleStage;

/// A structured event emitted by a pipeline stage.
///
/// These events form Layer 2 (session observability) — domain-language
/// records of what the system did and why, not just that it did something.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhaseEvent {
    /// When this event occurred.
    pub timestamp: DateTime<Utc>,
    /// Which turn this belongs to (0 for scene-entry events).
    pub turn_number: u32,
    /// Which pipeline stage emitted this event.
    pub stage: TurnCycleStage,
    /// The specific event detail.
    pub detail: PhaseEventDetail,
}

/// The specific detail of a pipeline phase event.
///
/// Variants correspond to observable moments in the pipeline — each
/// carries enough context to reconstruct what happened and why.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PhaseEventDetail {
    // -- Context assembly events --
    /// Preamble constructed from scene data.
    PreambleBuilt {
        /// Number of cast members included.
        cast_count: usize,
        /// Number of boundaries included.
        boundary_count: usize,
        /// Estimated token count for the preamble.
        estimated_tokens: u32,
    },

    /// A journal entry was added for the current turn.
    JournalEntryAdded {
        /// Turn number of the new entry.
        entry_turn: u32,
        /// How many entities were referenced.
        referenced_entity_count: usize,
        /// How many emotional markers were recorded.
        emotional_marker_count: usize,
    },

    /// Journal compression was triggered.
    JournalCompressed {
        /// How many entries were compressed.
        entries_compressed: usize,
        /// How many entries resisted compression (emotional significance).
        entries_resisted: usize,
        /// Token count before compression.
        tokens_before: u32,
        /// Token count after compression.
        tokens_after: u32,
    },

    /// An individual journal entry's compression level changed.
    JournalEntryCompressed {
        /// Which turn's entry was compressed.
        turn_number: u32,
        /// Previous compression level.
        from: CompressionLevel,
        /// New compression level.
        to: CompressionLevel,
    },

    /// Context was retrieved for referenced entities.
    ContextRetrieved {
        /// Entities for which context was retrieved.
        entity_ids: Vec<EntityId>,
        /// Number of context items returned.
        item_count: usize,
        /// Estimated tokens for all retrieved context.
        estimated_tokens: u32,
    },

    /// An entity's context was filtered by information boundaries.
    InformationBoundaryApplied {
        /// Which entity's information was filtered.
        entity_id: EntityId,
        /// How many items were available before filtering.
        available: usize,
        /// How many items passed the boundary filter.
        permitted: usize,
    },

    /// Full narrator context was assembled from all three tiers.
    ContextAssembled {
        /// Tier 1 (preamble) estimated tokens.
        preamble_tokens: u32,
        /// Tier 2 (journal) estimated tokens.
        journal_tokens: u32,
        /// Tier 3 (retrieved) estimated tokens.
        retrieved_tokens: u32,
        /// Total estimated tokens.
        total_tokens: u32,
        /// Whether any tier was trimmed to fit the budget.
        trimmed: bool,
    },

    /// Predictions enriched from raw ML output for Narrator consumption.
    PredictionsEnriched {
        /// Number of characters with predictions.
        character_count: usize,
        /// Total number of predicted actions across all characters.
        total_actions: usize,
        /// Estimated tokens for the rendered predictions block.
        estimated_tokens: u32,
    },

    // -- Narrator rendering events --
    /// Narrator prompt was constructed from assembled context.
    NarratorPromptBuilt {
        /// System prompt length in characters.
        system_prompt_chars: usize,
        /// User message length in characters.
        user_message_chars: usize,
    },

    /// Narrator rendering completed.
    NarratorRenderingComplete {
        /// LLM tokens used (if reported by the provider).
        tokens_used: Option<u32>,
        /// Wall-clock milliseconds for the LLM call.
        elapsed_ms: u64,
    },

    // -- Generic phase events --
    /// A phase started processing.
    PhaseStarted {
        /// Optional description of what the phase is about to do.
        description: Option<String>,
    },

    /// A phase completed processing.
    PhaseCompleted {
        /// Wall-clock milliseconds for this phase.
        elapsed_ms: u64,
        /// Optional summary of what the phase accomplished.
        summary: Option<String>,
    },
}

impl fmt::Display for PhaseEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[turn {}] {:?}: {}",
            self.turn_number, self.stage, self.detail
        )
    }
}

impl fmt::Display for PhaseEventDetail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PreambleBuilt {
                cast_count,
                boundary_count,
                estimated_tokens,
            } => write!(
                f,
                "Preamble built ({cast_count} cast, {boundary_count} boundaries, ~{estimated_tokens}t)"
            ),
            Self::JournalEntryAdded {
                entry_turn,
                referenced_entity_count,
                emotional_marker_count,
            } => write!(
                f,
                "Journal entry added for turn {entry_turn} ({referenced_entity_count} entities, {emotional_marker_count} markers)"
            ),
            Self::JournalCompressed {
                entries_compressed,
                entries_resisted,
                tokens_before,
                tokens_after,
            } => write!(
                f,
                "Journal compressed: {entries_compressed} entries ({entries_resisted} resisted), {tokens_before}t → {tokens_after}t"
            ),
            Self::JournalEntryCompressed {
                turn_number,
                from,
                to,
            } => write!(
                f,
                "Turn {turn_number} compressed: {from:?} → {to:?}"
            ),
            Self::ContextRetrieved {
                entity_ids,
                item_count,
                estimated_tokens,
            } => write!(
                f,
                "Retrieved context for {} entities: {item_count} items, ~{estimated_tokens}t",
                entity_ids.len()
            ),
            Self::InformationBoundaryApplied {
                entity_id: _,
                available,
                permitted,
            } => write!(
                f,
                "Information boundary: {permitted}/{available} items permitted"
            ),
            Self::ContextAssembled {
                preamble_tokens,
                journal_tokens,
                retrieved_tokens,
                total_tokens,
                trimmed,
            } => {
                write!(
                    f,
                    "Context assembled: preamble ~{preamble_tokens}t + journal ~{journal_tokens}t + retrieved ~{retrieved_tokens}t = ~{total_tokens}t"
                )?;
                if *trimmed {
                    write!(f, " (trimmed)")?;
                }
                Ok(())
            }
            Self::PredictionsEnriched {
                character_count,
                total_actions,
                estimated_tokens,
            } => write!(
                f,
                "Predictions enriched: {character_count} characters, {total_actions} actions, ~{estimated_tokens}t"
            ),
            Self::NarratorPromptBuilt {
                system_prompt_chars,
                user_message_chars,
            } => write!(
                f,
                "Narrator prompt built: system {system_prompt_chars} chars, user {user_message_chars} chars"
            ),
            Self::NarratorRenderingComplete {
                tokens_used,
                elapsed_ms,
            } => {
                write!(f, "Narrator rendering complete: {elapsed_ms}ms")?;
                if let Some(tokens) = tokens_used {
                    write!(f, ", {tokens} tokens")?;
                }
                Ok(())
            }
            Self::PhaseStarted { description } => match description {
                Some(desc) => write!(f, "Phase started: {desc}"),
                None => write!(f, "Phase started"),
            },
            Self::PhaseCompleted {
                elapsed_ms,
                summary,
            } => match summary {
                Some(s) => write!(f, "Phase completed in {elapsed_ms}ms: {s}"),
                None => write!(f, "Phase completed in {elapsed_ms}ms"),
            },
        }
    }
}

/// Trait for receiving pipeline phase events (Layer 2 observability).
///
/// Implementations decide where events go — debug channel, event ledger,
/// tracing bridge, or nowhere (`NoopObserver`).
///
/// Inspired by tasker-core's handler/actor pattern: emission is decoupled
/// from consumption. In production, an implementation backed by a bounded
/// MPSC channel sends events to a dedicated observer system. For tests
/// and simple contexts, `NoopObserver` or `CollectingObserver` are provided.
pub trait PhaseObserver: fmt::Debug + Send + Sync {
    /// Emit a phase event. Implementations should not block.
    fn emit(&self, event: PhaseEvent);
}

/// Observer that discards all events — for tests and contexts where
/// observability is not needed.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopObserver;

impl PhaseObserver for NoopObserver {
    fn emit(&self, _event: PhaseEvent) {}
}

/// Observer that collects events in a `Vec` — for tests that want to
/// assert on emitted events.
#[derive(Debug, Default)]
pub struct CollectingObserver {
    events: std::sync::Mutex<Vec<PhaseEvent>>,
}

impl CollectingObserver {
    /// Create a new empty collecting observer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Take all collected events, leaving the collection empty.
    pub fn take_events(&self) -> Vec<PhaseEvent> {
        self.events
            .lock()
            .expect("CollectingObserver mutex poisoned")
            .drain(..)
            .collect()
    }

    /// Number of events collected so far.
    pub fn event_count(&self) -> usize {
        self.events
            .lock()
            .expect("CollectingObserver mutex poisoned")
            .len()
    }
}

impl PhaseObserver for CollectingObserver {
    fn emit(&self, event: PhaseEvent) {
        self.events
            .lock()
            .expect("CollectingObserver mutex poisoned")
            .push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_observer_discards_events() {
        let observer = NoopObserver;
        observer.emit(PhaseEvent {
            timestamp: Utc::now(),
            turn_number: 1,
            stage: TurnCycleStage::AssemblingContext,
            detail: PhaseEventDetail::PreambleBuilt {
                cast_count: 2,
                boundary_count: 3,
                estimated_tokens: 600,
            },
        });
        // No panic, no effect — that's the point
    }

    #[test]
    fn collecting_observer_captures_events() {
        let observer = CollectingObserver::new();
        assert_eq!(observer.event_count(), 0);

        observer.emit(PhaseEvent {
            timestamp: Utc::now(),
            turn_number: 1,
            stage: TurnCycleStage::AssemblingContext,
            detail: PhaseEventDetail::PreambleBuilt {
                cast_count: 2,
                boundary_count: 3,
                estimated_tokens: 600,
            },
        });
        observer.emit(PhaseEvent {
            timestamp: Utc::now(),
            turn_number: 1,
            stage: TurnCycleStage::AssemblingContext,
            detail: PhaseEventDetail::JournalEntryAdded {
                entry_turn: 1,
                referenced_entity_count: 2,
                emotional_marker_count: 1,
            },
        });

        assert_eq!(observer.event_count(), 2);
        let events = observer.take_events();
        assert_eq!(events.len(), 2);
        assert_eq!(observer.event_count(), 0);
    }

    #[test]
    fn phase_event_display_is_readable() {
        let event = PhaseEvent {
            timestamp: Utc::now(),
            turn_number: 3,
            stage: TurnCycleStage::AssemblingContext,
            detail: PhaseEventDetail::JournalCompressed {
                entries_compressed: 2,
                entries_resisted: 1,
                tokens_before: 1400,
                tokens_after: 900,
            },
        };
        let display = format!("{event}");
        assert!(display.contains("turn 3"));
        assert!(display.contains("AssemblingContext"));
        assert!(display.contains("2 entries"));
        assert!(display.contains("1 resisted"));
        assert!(display.contains("1400t"));
        assert!(display.contains("900t"));
    }

    #[test]
    fn context_assembled_display_shows_trimmed() {
        let detail = PhaseEventDetail::ContextAssembled {
            preamble_tokens: 600,
            journal_tokens: 1000,
            retrieved_tokens: 400,
            total_tokens: 2000,
            trimmed: true,
        };
        let display = format!("{detail}");
        assert!(display.contains("trimmed"));
        assert!(display.contains("2000t"));
    }
}
