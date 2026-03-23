# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Shared test fixtures for narrative-data tests."""

from pathlib import Path

import pytest


@pytest.fixture
def tmp_output_dir(tmp_path: Path) -> Path:
    """Create a temporary output directory mimicking storyteller-data/narrative-data/."""
    for subdir in ["genres", "spatial", "intersections", "meta/schemas", "meta/runs"]:
        (tmp_path / subdir).mkdir(parents=True)
    return tmp_path


@pytest.fixture
def tmp_descriptor_dir(tmp_path: Path) -> Path:
    """Create a temporary descriptor directory with minimal test data."""
    desc_dir = tmp_path / "descriptors"
    desc_dir.mkdir()
    return desc_dir
