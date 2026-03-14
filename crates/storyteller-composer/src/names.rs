//! Name pool selection with dedup and fallback generation.

use rand::seq::SliceRandom;
use rand::Rng;

/// Select `count` unique names from the pool, shuffled randomly.
///
/// If the pool has fewer than `count` entries, all pool names are used first,
/// then fallback names like "Character N" fill the remainder.
pub fn select_names<R: Rng>(pool: &[String], count: usize, rng: &mut R) -> Vec<String> {
    if count == 0 {
        return Vec::new();
    }

    let mut candidates: Vec<&String> = pool.iter().collect();
    candidates.shuffle(rng);

    let mut names: Vec<String> = candidates.into_iter().take(count).cloned().collect();

    // Generate fallback names if the pool was too small.
    let mut fallback_index = 1;
    while names.len() < count {
        let fallback = format!("Character {fallback_index}");
        if !names.contains(&fallback) {
            names.push(fallback);
        }
        fallback_index += 1;
    }

    names
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn select_names_returns_unique_names() {
        let pool: Vec<String> = vec![
            "Alice".into(),
            "Bob".into(),
            "Carol".into(),
            "Dave".into(),
            "Eve".into(),
        ];
        let mut rng = StdRng::seed_from_u64(42);
        let names = select_names(&pool, 3, &mut rng);

        assert_eq!(names.len(), 3);
        // All unique
        let unique: std::collections::HashSet<&String> = names.iter().collect();
        assert_eq!(unique.len(), 3);
        // All from pool
        for name in &names {
            assert!(pool.contains(name), "name '{name}' should come from pool");
        }
    }

    #[test]
    fn select_names_fallback_when_pool_too_small() {
        let pool: Vec<String> = vec!["Alice".into()];
        let mut rng = StdRng::seed_from_u64(42);
        let names = select_names(&pool, 3, &mut rng);

        assert_eq!(names.len(), 3);
        assert!(names.contains(&"Alice".to_string()));
        assert!(names.contains(&"Character 1".to_string()));
        assert!(names.contains(&"Character 2".to_string()));
    }

    #[test]
    fn select_names_empty_pool_all_fallbacks() {
        let pool: Vec<String> = vec![];
        let mut rng = StdRng::seed_from_u64(42);
        let names = select_names(&pool, 2, &mut rng);

        assert_eq!(names.len(), 2);
        assert_eq!(names[0], "Character 1");
        assert_eq!(names[1], "Character 2");
    }

    #[test]
    fn select_names_deterministic_with_seed() {
        let pool: Vec<String> = vec!["Alice".into(), "Bob".into(), "Carol".into(), "Dave".into()];

        let mut rng1 = StdRng::seed_from_u64(99);
        let names1 = select_names(&pool, 2, &mut rng1);

        let mut rng2 = StdRng::seed_from_u64(99);
        let names2 = select_names(&pool, 2, &mut rng2);

        assert_eq!(names1, names2);
    }

    #[test]
    fn select_names_zero_count_returns_empty() {
        let pool: Vec<String> = vec!["Alice".into()];
        let mut rng = StdRng::seed_from_u64(42);
        let names = select_names(&pool, 0, &mut rng);

        assert!(names.is_empty());
    }
}
