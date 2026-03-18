"""Shared helpers for narrative-data commands."""

from datetime import UTC, datetime


def slug_to_name(slug: str) -> str:
    """Convert a slug to a display name: 'folk-horror' → 'Folk Horror'."""
    return slug.replace("-", " ").title()


def now_iso() -> str:
    """Return current UTC time as ISO 8601."""
    return datetime.now(UTC).isoformat()
