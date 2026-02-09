//! JSONL export for event classification training data.
//!
//! Same pattern as `matrix::export` — one JSON object per line,
//! plus a manifest file with generation statistics.

use std::io::Write;
use std::path::Path;

use super::{AnnotatedExample, GenerationManifest};
use storyteller_core::errors::StorytellerError;

/// Write annotated examples as JSONL (one JSON object per line).
pub fn write_jsonl(
    examples: &[AnnotatedExample],
    writer: &mut dyn Write,
) -> Result<(), StorytellerError> {
    for example in examples {
        let line = serde_json::to_string(example).map_err(|e| {
            StorytellerError::Inference(format!("failed to serialize example: {e}"))
        })?;
        writeln!(writer, "{line}")
            .map_err(|e| StorytellerError::Inference(format!("failed to write JSONL line: {e}")))?;
    }
    Ok(())
}

/// Write the generation manifest as pretty-printed JSON.
pub fn write_manifest(manifest: &GenerationManifest, path: &Path) -> Result<(), StorytellerError> {
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| StorytellerError::Inference(format!("failed to serialize manifest: {e}")))?;
    std::fs::write(path, json).map_err(|e| {
        StorytellerError::Inference(format!(
            "failed to write manifest to {}: {e}",
            path.display()
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_templates::{EntityAnnotation, NerCategory, TextRegister};

    #[test]
    fn write_jsonl_produces_valid_output() {
        let examples = vec![AnnotatedExample {
            id: "test-1".to_string(),
            text: "I pick up the stone".to_string(),
            register: TextRegister::Player,
            event_kinds: vec!["ActionOccurrence".to_string()],
            action_type: Some("Perform".to_string()),
            entities: vec![EntityAnnotation {
                start: 0,
                end: 1,
                text: "I".to_string(),
                category: NerCategory::Character,
                role: "Actor".to_string(),
            }],
        }];

        let mut buf = Vec::new();
        write_jsonl(&examples, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();

        // Should be valid JSON
        let parsed: AnnotatedExample = serde_json::from_str(output.trim()).unwrap();
        assert_eq!(parsed.id, "test-1");
        assert_eq!(parsed.text, "I pick up the stone");
    }

    #[test]
    fn write_jsonl_one_line_per_example() {
        let examples = vec![
            AnnotatedExample {
                id: "a".to_string(),
                text: "first".to_string(),
                register: TextRegister::Player,
                event_kinds: vec!["ActionOccurrence".to_string()],
                action_type: None,
                entities: vec![],
            },
            AnnotatedExample {
                id: "b".to_string(),
                text: "second".to_string(),
                register: TextRegister::Narrator,
                event_kinds: vec!["SpeechAct".to_string()],
                action_type: None,
                entities: vec![],
            },
        ];

        let mut buf = Vec::new();
        write_jsonl(&examples, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = output.trim().lines().collect();
        assert_eq!(lines.len(), 2);
    }
}
