// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Session persistence — composition, events, turn index, and directives.
//!
//! ## Four-file model
//!
//! Each session is a directory under `.story/sessions/{uuidv7}/` containing:
//!
//! - `composition.json` — write-once scene composition and setup data
//! - `events.jsonl` — append-only event stream (one [`PersistedEvent`] per line)
//! - `turns.jsonl` — append-only turn index referencing event UUIDs
//! - `directives.jsonl` — append-only async agent directives (dramaturge, world agent)
//!
//! ## Usage
//!
//! ```rust,ignore
//! let store = SessionStore::new(Path::new(".story/sessions"))?;
//! let session_id = store.create_session()?;
//!
//! // Write once
//! store.composition.write(&session_id, &composition_json)?;
//!
//! // Append events
//! let event_id = store.events.append(&session_id, "PlayerInput", Some(1), &payload)?;
//!
//! // Record a turn
//! store.turns.append(&session_id, &TurnEntry { turn: 1, event_ids: vec![event_id], .. })?;
//! ```

pub mod composition;
pub mod directives;
pub mod events;
pub mod session_store;
pub mod turns;

pub use composition::CompositionWriter;
pub use directives::{DirectiveEntry, DirectiveStore};
pub use events::{EventWriter, PersistedEvent};
pub use session_store::SessionStore;
pub use turns::{TurnEntry, TurnWriter};
