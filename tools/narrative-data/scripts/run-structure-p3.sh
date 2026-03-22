#!/usr/bin/env bash
# Stage 2 Phase 3: Types that reference archetypes (run after P2)
set -euo pipefail

echo "=== Stage 2 Phase 3: Archetype-Dependent Types ==="
narrative-data structure run dynamics --all --clusters
narrative-data structure run goals --all --clusters
narrative-data structure run archetype-dynamics --all --clusters
echo "=== Phase 3 Complete ==="
