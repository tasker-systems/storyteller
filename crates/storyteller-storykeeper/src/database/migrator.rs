//! SQLx migrator for the storyteller database schema.
//!
//! Embeds all migrations at compile time. Use in `#[sqlx::test]` tests:
//!
//! ```ignore
//! #[sqlx::test(migrator = "storyteller_storykeeper::database::migrator::MIGRATOR")]
//! async fn test_something(pool: PgPool) {
//!     // pool has all migrations applied
//! }
//! ```

/// Primary migrator containing all migrations.
///
/// Use this for:
/// - All `#[sqlx::test]` tests that need the storyteller schema
/// - SQLx compile-time query verification
/// - Runtime migration via `MIGRATOR.run(&pool).await`
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");
