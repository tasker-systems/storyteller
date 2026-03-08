//! Scene journal management — Tier 2 of the Narrator's context.
//!
//! See: `docs/technical/narrator-architecture.md` § Rolling Scene Journal
//!
//! The scene journal is a progressively compressed record of what has
//! happened in the scene. Recent turns are detailed; older turns compress
//! to essentials. Emotionally significant entries resist compression.
//!
//! Compression strategy (initial):
//! - Turns 0 to N-3: Skeleton
//! - Turns N-2 to N-1: Summary
//! - Turn N (current): Full
//!
//! Entries with emotional markers matching scene stakes resist one
//! compression level (e.g., stay at Summary when they would become Skeleton).

use chrono::Utc;

use storyteller_core::traits::phase_observer::{PhaseEvent, PhaseEventDetail, PhaseObserver};
use storyteller_core::types::entity::EntityId;
use storyteller_core::types::narrator_context::{CompressionLevel, JournalEntry, SceneJournal};
use storyteller_core::types::turn_cycle::TurnCycleStage;

use super::tokens::estimate_tokens;

/// Add a new turn entry to the journal and compress if needed.
///
/// The new entry is added at `CompressionLevel::Full`. After adding,
/// the journal is compressed to respect the token budget.
pub fn add_turn(
    journal: &mut SceneJournal,
    turn_number: u32,
    content: &str,
    referenced_entities: Vec<EntityId>,
    emotional_markers: Vec<String>,
    observer: &dyn PhaseObserver,
) {
    let entry = JournalEntry {
        turn_number,
        timestamp: Utc::now(),
        compression: CompressionLevel::Full,
        content: content.to_string(),
        referenced_entities,
        emotional_markers: emotional_markers.clone(),
    };

    observer.emit(PhaseEvent {
        timestamp: Utc::now(),
        turn_number,
        stage: TurnCycleStage::AssemblingContext,
        detail: PhaseEventDetail::JournalEntryAdded {
            entry_turn: turn_number,
            referenced_entity_count: entry.referenced_entities.len(),
            emotional_marker_count: entry.emotional_markers.len(),
        },
    });

    journal.entries.push(entry);
    compress_if_needed(journal, observer);
}

/// Compress journal entries based on recency, respecting emotional resistance.
///
/// Compression levels by position (from end):
/// - Current turn (last entry): Full
/// - Previous 1-2 turns: Summary
/// - Older turns: Skeleton
///
/// Entries with emotional markers resist one compression level.
pub fn compress_if_needed(journal: &mut SceneJournal, observer: &dyn PhaseObserver) {
    let entry_count = journal.entries.len();
    if entry_count <= 1 {
        return;
    }

    let tokens_before = estimate_journal_tokens(journal);
    let mut entries_compressed: usize = 0;
    let mut entries_resisted: usize = 0;

    for i in 0..entry_count {
        let distance_from_end = entry_count - 1 - i;
        let has_emotional_markers = !journal.entries[i].emotional_markers.is_empty();

        let target_level = match distance_from_end {
            0 => CompressionLevel::Full,        // current turn
            1..=2 => CompressionLevel::Summary, // recent turns
            _ => CompressionLevel::Skeleton,    // older turns
        };

        // Emotional resistance: entries with markers resist one level
        let actual_target = if has_emotional_markers && target_level > CompressionLevel::Full {
            resist_one_level(target_level)
        } else {
            target_level
        };

        let current = journal.entries[i].compression;
        if actual_target > current {
            let from = current;

            // Apply compression by transforming the content
            journal.entries[i].content =
                compress_content(&journal.entries[i].content, actual_target);
            journal.entries[i].compression = actual_target;
            entries_compressed += 1;

            observer.emit(PhaseEvent {
                timestamp: Utc::now(),
                turn_number: journal.entries[i].turn_number,
                stage: TurnCycleStage::AssemblingContext,
                detail: PhaseEventDetail::JournalEntryCompressed {
                    turn_number: journal.entries[i].turn_number,
                    from,
                    to: actual_target,
                },
            });
        } else if has_emotional_markers && target_level > current {
            // Entry resisted compression
            entries_resisted += 1;
        }
    }

    if entries_compressed > 0 || entries_resisted > 0 {
        let tokens_after = estimate_journal_tokens(journal);
        observer.emit(PhaseEvent {
            timestamp: Utc::now(),
            turn_number: journal.entries.last().map_or(0, |e| e.turn_number),
            stage: TurnCycleStage::AssemblingContext,
            detail: PhaseEventDetail::JournalCompressed {
                entries_compressed,
                entries_resisted,
                tokens_before,
                tokens_after,
            },
        });
    }
}

/// Compress content to the target level.
///
/// - Full: unchanged
/// - Summary: first sentence + emotional marker summary
/// - Skeleton: very brief essence
fn compress_content(content: &str, target: CompressionLevel) -> String {
    match target {
        CompressionLevel::Full => content.to_string(),
        CompressionLevel::Summary => {
            // Take the first sentence (or first 100 chars) as summary
            let first_sentence = content
                .find(". ")
                .map(|i| &content[..=i])
                .unwrap_or_else(|| {
                    if content.len() <= 100 {
                        content
                    } else {
                        &content[..100]
                    }
                });
            first_sentence.to_string()
        }
        CompressionLevel::Skeleton => {
            // Very brief — first clause or first 50 chars
            let skeleton = content
                .find(", ")
                .or_else(|| content.find(". "))
                .map(|i| &content[..i])
                .unwrap_or_else(|| {
                    if content.len() <= 50 {
                        content
                    } else {
                        &content[..50]
                    }
                });
            skeleton.to_string()
        }
    }
}

/// Resist one compression level (e.g., Skeleton → Summary).
fn resist_one_level(level: CompressionLevel) -> CompressionLevel {
    match level {
        CompressionLevel::Full => CompressionLevel::Full,
        CompressionLevel::Summary => CompressionLevel::Full,
        CompressionLevel::Skeleton => CompressionLevel::Summary,
    }
}

/// Estimate the total token count for the journal.
pub fn estimate_journal_tokens(journal: &SceneJournal) -> u32 {
    journal
        .entries
        .iter()
        .map(|e| estimate_tokens(&e.content))
        .sum()
}

/// Render the journal to a string for the Narrator's context.
pub fn render_journal(journal: &SceneJournal) -> String {
    if journal.entries.is_empty() {
        return String::from("[Scene just began — no prior turns.]");
    }

    let mut output = String::new();
    for entry in &journal.entries {
        let level_tag = match entry.compression {
            CompressionLevel::Full => "",
            CompressionLevel::Summary => " [summary]",
            CompressionLevel::Skeleton => " [skeleton]",
        };
        output.push_str(&format!(
            "Turn {}{}: {}\n",
            entry.turn_number, level_tag, entry.content
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use storyteller_core::traits::phase_observer::CollectingObserver;
    use storyteller_core::types::scene::SceneId;

    fn make_journal() -> SceneJournal {
        SceneJournal::new(SceneId::new(), 1200)
    }

    #[test]
    fn add_turn_creates_full_entry() {
        let mut journal = make_journal();
        let observer = CollectingObserver::new();

        add_turn(
            &mut journal,
            1,
            "Bramblehoof approaches the fence. The light catches his horns.",
            vec![EntityId::new()],
            vec!["anticipation".to_string()],
            &observer,
        );

        assert_eq!(journal.turn_count(), 1);
        assert_eq!(journal.entries[0].compression, CompressionLevel::Full);
        assert_eq!(journal.entries[0].turn_number, 1);

        // Observer got at least the JournalEntryAdded event
        let events = observer.take_events();
        assert!(events
            .iter()
            .any(|e| matches!(e.detail, PhaseEventDetail::JournalEntryAdded { .. })));
    }

    #[test]
    fn compression_applies_by_recency() {
        let mut journal = make_journal();
        let observer = CollectingObserver::new();

        // Add 5 turns
        for i in 1..=5 {
            add_turn(
                &mut journal,
                i,
                &format!("Turn {i} content. Something happened here that matters to the story."),
                vec![],
                vec![], // no emotional markers
                &observer,
            );
        }

        assert_eq!(journal.turn_count(), 5);

        // Turn 5 (current): Full
        assert_eq!(journal.entries[4].compression, CompressionLevel::Full);
        // Turn 4 (recent): Summary
        assert_eq!(journal.entries[3].compression, CompressionLevel::Summary);
        // Turn 3 (recent): Summary
        assert_eq!(journal.entries[2].compression, CompressionLevel::Summary);
        // Turn 1-2 (older): Skeleton
        assert_eq!(journal.entries[0].compression, CompressionLevel::Skeleton);
        assert_eq!(journal.entries[1].compression, CompressionLevel::Skeleton);
    }

    #[test]
    fn emotional_markers_resist_compression() {
        let mut journal = make_journal();
        let observer = CollectingObserver::new();

        // Turn 1: emotional marker
        add_turn(
            &mut journal,
            1,
            "Pyotir glances at the flute. A pause. Something shifts in his breathing.",
            vec![],
            vec!["grief".to_string(), "longing".to_string()],
            &observer,
        );

        // Turn 2: no markers
        add_turn(
            &mut journal,
            2,
            "Surface conversation about the crops. Measured answers.",
            vec![],
            vec![],
            &observer,
        );

        // Turn 3-5: push turn 1 into skeleton range
        for i in 3..=5 {
            add_turn(
                &mut journal,
                i,
                &format!("Turn {i} content with more discussion."),
                vec![],
                vec![],
                &observer,
            );
        }

        // Turn 1 has emotional markers — should resist to Summary, not Skeleton
        assert_eq!(
            journal.entries[0].compression,
            CompressionLevel::Summary,
            "Emotionally marked entry should resist to Summary, not Skeleton"
        );

        // Turn 2 (no markers, in skeleton range) should be Skeleton
        assert_eq!(journal.entries[1].compression, CompressionLevel::Skeleton);
    }

    #[test]
    fn compression_reduces_content_length() {
        let long_content = "Bramblehoof walks slowly toward the fence, \
            his hooves leaving shallow prints in the turned earth. \
            The evening light catches the grain of the wood where \
            Pyotir has mended it most recently.";

        let summary = compress_content(long_content, CompressionLevel::Summary);
        let skeleton = compress_content(long_content, CompressionLevel::Skeleton);

        assert!(summary.len() < long_content.len());
        assert!(skeleton.len() < summary.len());
    }

    #[test]
    fn journal_token_estimation() {
        let mut journal = make_journal();
        let observer = storyteller_core::traits::NoopObserver;

        add_turn(
            &mut journal,
            1,
            "Bramblehoof approaches the fence slowly.",
            vec![],
            vec![],
            &observer,
        );
        add_turn(
            &mut journal,
            2,
            "Pyotir keeps working, doesn't look up immediately.",
            vec![],
            vec![],
            &observer,
        );

        let tokens = estimate_journal_tokens(&journal);
        assert!(tokens > 0);
        // Two short sentences → roughly 15-25 tokens
        assert!(tokens < 100);
    }

    #[test]
    fn render_journal_empty() {
        let journal = make_journal();
        let rendered = render_journal(&journal);
        assert!(rendered.contains("no prior turns"));
    }

    #[test]
    fn render_journal_with_entries() {
        let mut journal = make_journal();
        let observer = storyteller_core::traits::NoopObserver;

        add_turn(
            &mut journal,
            1,
            "Bramblehoof approaches the fence.",
            vec![],
            vec![],
            &observer,
        );
        add_turn(
            &mut journal,
            2,
            "Pyotir nods in recognition.",
            vec![],
            vec![],
            &observer,
        );

        let rendered = render_journal(&journal);
        assert!(rendered.contains("Turn 1"));
        assert!(rendered.contains("Turn 2"));
    }

    #[test]
    fn observer_receives_compression_events() {
        let mut journal = make_journal();
        let observer = CollectingObserver::new();

        // Add enough turns to trigger compression
        for i in 1..=4 {
            add_turn(
                &mut journal,
                i,
                &format!("Turn {i}: something happens in the scene."),
                vec![],
                vec![],
                &observer,
            );
        }

        let events = observer.take_events();

        // Should have JournalEntryAdded for each turn
        let added_count = events
            .iter()
            .filter(|e| matches!(e.detail, PhaseEventDetail::JournalEntryAdded { .. }))
            .count();
        assert_eq!(added_count, 4);

        // Should have at least one JournalCompressed event
        let compressed = events
            .iter()
            .any(|e| matches!(e.detail, PhaseEventDetail::JournalCompressed { .. }));
        assert!(compressed, "Expected at least one JournalCompressed event");
    }
}
