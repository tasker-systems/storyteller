#!/usr/bin/env bash
# Stage 2 Phase 2: Structure independent types (no inter-type dependencies)
set -euo pipefail

echo "=== Stage 2 Phase 2: Independent Types ==="
narrative-data structure run archetypes --all --clusters
narrative-data structure run settings --all --clusters
narrative-data structure run ontological-posture --all --clusters
narrative-data structure run profiles --all --clusters
narrative-data structure run tropes --all
narrative-data structure run narrative-shapes --all
echo "=== Phase 2 Complete ==="
