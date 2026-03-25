# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for state variable slug normalization."""

from narrative_data.persistence.sv_normalization import (
    normalize_sv_slug,
    resolve_sv_slug,
)


class TestNormalizeSvSlug:
    def test_already_canonical(self) -> None:
        assert normalize_sv_slug("community-trust") == "community-trust"

    def test_title_case(self) -> None:
        assert normalize_sv_slug("Community Trust") == "community-trust"

    def test_underscores(self) -> None:
        assert normalize_sv_slug("moral_stance") == "moral-stance"

    def test_mixed_case_underscores(self) -> None:
        assert normalize_sv_slug("Moral_Stance") == "moral-stance"

    def test_strips_parentheticals(self) -> None:
        assert normalize_sv_slug("Knowledge (Secrets Known)") == "knowledge"

    def test_strips_whitespace(self) -> None:
        assert normalize_sv_slug("  community-trust  ") == "community-trust"

    def test_collapses_hyphens(self) -> None:
        assert normalize_sv_slug("moral--stance") == "moral-stance"

    def test_strips_trailing_hyphens(self) -> None:
        assert normalize_sv_slug("-community-trust-") == "community-trust"

    def test_empty_string(self) -> None:
        assert normalize_sv_slug("") == ""


class TestResolveSvSlug:
    def _canonical(self) -> set[str]:
        return {
            "community-trust",
            "moral-stance",
            "sanctuary-integrity",
            "knowledge-gap",
            "social-capital",
            "energy",
        }

    def test_exact_match_after_normalization(self) -> None:
        kind, slug = resolve_sv_slug("Community Trust", self._canonical())
        assert kind == "exact"
        assert slug == "community-trust"

    def test_prefix_match(self) -> None:
        kind, slug = resolve_sv_slug("sanctuary", self._canonical())
        assert kind == "prefix"
        assert slug == "sanctuary-integrity"

    def test_unresolved(self) -> None:
        kind, slug = resolve_sv_slug("completely-unknown", self._canonical())
        assert kind == "unresolved"
        assert slug is None

    def test_already_canonical_is_exact(self) -> None:
        kind, slug = resolve_sv_slug("energy", self._canonical())
        assert kind == "exact"
        assert slug == "energy"

    def test_ambiguous_prefix_returns_unresolved(self) -> None:
        # Both "knowledge-gap" and "knowledge-base" would match "knowledge" prefix
        canonical = {"knowledge-gap", "knowledge-base", "energy"}
        kind, slug = resolve_sv_slug("knowledge", canonical)
        assert kind == "unresolved"
        assert slug is None
