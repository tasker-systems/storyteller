// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Integration tests for `PostgresBedrock` — requires a live database.
//!
//! Run with:
//! ```
//! DATABASE_URL="postgres://storyteller:storyteller@localhost:5435/storyteller_development" \
//!   cargo test -p storyteller-storykeeper --features test-db -- --nocapture
//! ```

#![cfg(feature = "test-db")]

use storyteller_core::errors::StorytellerError;
use storyteller_core::traits::bedrock::BedrockQuery;
use storyteller_storykeeper::PostgresBedrock;

async fn setup() -> PostgresBedrock {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://storyteller:storyteller@localhost:5435/storyteller_development".to_string()
    });
    let pool = sqlx::PgPool::connect(&url)
        .await
        .expect("failed to connect to database");
    PostgresBedrock::new(pool)
}

#[tokio::test]
async fn genre_context_returns_all_types_for_folk_horror() {
    let bedrock = setup().await;
    let ctx = bedrock
        .genre_context("folk-horror")
        .await
        .expect("genre_context should succeed");

    assert_eq!(ctx.genre.slug, "folk-horror");

    assert!(!ctx.archetypes.is_empty(), "archetypes should be non-empty");
    assert!(!ctx.dynamics.is_empty(), "dynamics should be non-empty");
    assert!(!ctx.settings.is_empty(), "settings should be non-empty");
    assert!(!ctx.goals.is_empty(), "goals should be non-empty");
    assert!(!ctx.profiles.is_empty(), "profiles should be non-empty");
    assert!(!ctx.tropes.is_empty(), "tropes should be non-empty");
    assert!(
        !ctx.narrative_shapes.is_empty(),
        "narrative_shapes should be non-empty"
    );

    println!(
        "folk-horror context: archetypes={}, dynamics={}, settings={}, goals={}, profiles={}, tropes={}, narrative_shapes={}, ontological_posture={}, spatial_topology={}, place_entities={}, archetype_dynamics={}",
        ctx.archetypes.len(),
        ctx.dynamics.len(),
        ctx.settings.len(),
        ctx.goals.len(),
        ctx.profiles.len(),
        ctx.tropes.len(),
        ctx.narrative_shapes.len(),
        ctx.ontological_posture.len(),
        ctx.spatial_topology.len(),
        ctx.place_entities.len(),
        ctx.archetype_dynamics.len(),
    );
}

#[tokio::test]
async fn genre_context_not_found_for_unknown_slug() {
    let bedrock = setup().await;
    let result = bedrock.genre_context("nonexistent").await;

    assert!(result.is_err(), "should return an error for unknown genre");
    let err = result.unwrap_err();
    assert!(
        matches!(err, StorytellerError::EntityNotFound(_)),
        "expected EntityNotFound, got: {err:?}"
    );
}

#[tokio::test]
async fn archetypes_by_genre_returns_entities() {
    let bedrock = setup().await;
    let archetypes = bedrock
        .archetypes_by_genre("folk-horror")
        .await
        .expect("archetypes_by_genre should succeed");

    assert!(
        !archetypes.is_empty(),
        "should return at least one archetype"
    );
    for record in &archetypes {
        assert!(
            !record.entity_slug.is_empty(),
            "entity_slug should be set on every record"
        );
    }

    println!("folk-horror archetypes: {}", archetypes.len());
}

#[tokio::test]
async fn archetype_by_slug_returns_known_entity() {
    let bedrock = setup().await;
    let archetypes = bedrock
        .archetypes_by_genre("folk-horror")
        .await
        .expect("archetypes_by_genre should succeed");

    assert!(
        !archetypes.is_empty(),
        "need at least one archetype to test by_slug"
    );
    let slug = archetypes[0].entity_slug.clone();

    let result = bedrock
        .archetype_by_slug("folk-horror", &slug)
        .await
        .expect("archetype_by_slug should not error");

    assert!(
        result.is_some(),
        "archetype_by_slug should return Some for known slug '{slug}'"
    );
    let record = result.unwrap();
    assert_eq!(record.entity_slug, slug);
}

#[tokio::test]
async fn archetype_by_slug_returns_none_for_unknown() {
    let bedrock = setup().await;
    let result = bedrock
        .archetype_by_slug("folk-horror", "nonexistent-slug")
        .await
        .expect("archetype_by_slug should not error for unknown slug");

    assert!(
        result.is_none(),
        "should return None for unknown entity slug"
    );
}

#[tokio::test]
async fn genre_dimensions_returns_profile() {
    let bedrock = setup().await;
    // genre_dimensions returns Ok(Some(...)) when data is loaded, Ok(None) when not yet loaded.
    // The query itself must not error — both outcomes are valid.
    let result = bedrock
        .genre_dimensions("folk-horror")
        .await
        .expect("genre_dimensions should not error");

    match result {
        Some(record) => {
            assert!(
                !record.payload.is_null(),
                "genre dimension payload should be non-null when present"
            );
            println!("folk-horror genre dimensions id: {}", record.id);
        }
        None => {
            // genre_dimensions table not yet loaded — this is acceptable.
            println!("folk-horror genre dimensions: no row present (table not yet populated)");
        }
    }
}

#[tokio::test]
async fn state_variables_returns_canonical_set() {
    let bedrock = setup().await;
    let vars = bedrock
        .state_variables()
        .await
        .expect("state_variables should succeed");

    assert!(
        vars.len() >= 8,
        "expected at least 8 state variables, got {}",
        vars.len()
    );

    for v in &vars {
        println!("  state variable: {} ({})", v.name, v.slug);
    }
}

#[tokio::test]
async fn genres_returns_all_30() {
    let bedrock = setup().await;
    let genres = bedrock.genres().await.expect("genres should succeed");

    assert_eq!(
        genres.len(),
        30,
        "expected exactly 30 genres, got {}",
        genres.len()
    );

    for g in &genres {
        println!("  genre: {} ({})", g.name, g.slug);
    }
}

#[tokio::test]
async fn trope_families_returns_entries() {
    let bedrock = setup().await;
    let families = bedrock
        .trope_families()
        .await
        .expect("trope_families should succeed");

    assert!(
        !families.is_empty(),
        "trope_families should return at least one entry"
    );

    for f in &families {
        println!("  trope family: {} ({})", f.name, f.slug);
    }
}

#[test]
fn object_safety_compiles() {
    fn _assert_object_safe(_: &dyn BedrockQuery) {}
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<Box<dyn BedrockQuery>>();
}
