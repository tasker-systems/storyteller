#!/usr/bin/env bash
# Stage 2 Phase 2: Structure independent types (no inter-type dependencies)
set -euo pipefail

echo "=== Stage 2 Phase 2: Independent Types ==="
uv run narrative-data structure run archetypes --all --clusters
uv run narrative-data structure run settings --all --clusters
uv run narrative-data structure run ontological-posture --all --clusters
uv run narrative-data structure run profiles --all --clusters
uv run narrative-data structure run tropes --all
uv run narrative-data structure run narrative-shapes --all
echo "=== Phase 2 Complete ==="
