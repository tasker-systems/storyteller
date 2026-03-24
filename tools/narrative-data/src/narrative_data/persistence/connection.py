# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Database connection management."""

import os


def get_connection_string() -> str:
    """Read DATABASE_URL from environment. Raises ValueError if not set."""
    url = os.environ.get("DATABASE_URL")
    if not url:
        raise ValueError(
            "DATABASE_URL environment variable is not set. "
            "Set it to the storyteller database connection string, e.g. "
            "postgres://storyteller:storyteller@localhost:5435/storyteller_development"
        )
    return url
