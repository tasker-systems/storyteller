#!/usr/bin/env bash
# Stage 2 Phase 4: Spatial types
set -euo pipefail

echo "=== Stage 2 Phase 4: Spatial Types ==="
narrative-data structure run spatial-topology --all --clusters
narrative-data structure run place-entities --all --clusters
echo "=== Phase 4 Complete ==="
