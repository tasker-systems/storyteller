//! Bevy plugin registration for the storyteller engine.
//!
//! The `StorytellerEnginePlugin` registers all systems, components, events,
//! and resources needed to run the storytelling engine within a Bevy App.

use bevy_app::{App, Plugin};

/// Main plugin for the storyteller engine.
///
/// Registers turn cycle systems, scene lifecycle, agent systems,
/// event pipeline, and observability infrastructure.
#[derive(Debug)]
pub struct StorytellerEnginePlugin;

impl Plugin for StorytellerEnginePlugin {
    fn build(&self, _app: &mut App) {
        // Systems, events, and resources will be registered here
        // as they are implemented. For now, this is a valid empty plugin.
    }
}
