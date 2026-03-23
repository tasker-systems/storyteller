# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for database connection management."""

import pytest

from narrative_data.persistence.connection import get_connection_string


class TestConnection:
    def test_reads_database_url_from_env(self, monkeypatch):
        monkeypatch.setenv("DATABASE_URL", "postgres://test:test@localhost:5435/test_db")
        assert get_connection_string() == "postgres://test:test@localhost:5435/test_db"

    def test_raises_without_database_url(self, monkeypatch):
        monkeypatch.delenv("DATABASE_URL", raising=False)
        with pytest.raises(ValueError, match="DATABASE_URL"):
            get_connection_string()
