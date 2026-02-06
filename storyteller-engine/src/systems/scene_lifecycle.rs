//! Scene lifecycle management — entry, play, exit.
//!
//! See: `docs/technical/scene-model.md`
//!
//! Scene entry: warm caches, compute psychological frames, instantiate
//! character agents, load rendered space constraints.
//! Scene exit: persist state changes, evaluate departure type, transition.
