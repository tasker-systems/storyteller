//! Interactive scene player — DEPRECATED.
//!
//! This binary used to run the hardcoded `the_flute_kept` scene. That scene data
//! is now gated behind `#[cfg(test)]` (available only for test fixtures).
//! Use the workshop scene wizard (`cargo make workshop`) to compose and play scenes.

fn main() {
    eprintln!(
        "The play-scene binary has been deprecated.\n\
         The hardcoded scene (The Flute Kept) is now test-only.\n\
         Use the workshop scene wizard to compose and play scenes."
    );
    std::process::exit(1);
}
