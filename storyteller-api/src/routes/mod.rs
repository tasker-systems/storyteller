//! Route definitions for the player-facing API.
//!
//! Each submodule defines a group of related routes. The top-level
//! [`crate::router()`] merges them all.

pub mod health;
pub mod player;
pub mod session;
