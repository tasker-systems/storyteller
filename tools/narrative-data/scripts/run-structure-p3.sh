#!/usr/bin/env bash
# Stage 2 Phase 3: Types that reference archetypes (run after P2)
set -uo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

failures=0

run_type() {
  local label="$1"
  shift
  echo "--- $label ---"
  if uv run narrative-data structure run "$@"; then
    echo -e "${GREEN}✓ $label${NC}"
  else
    echo -e "${RED}✗ $label FAILED${NC}" >&2
    ((failures++))
  fi
  echo
}

echo "=== Stage 2 Phase 3: Archetype-Dependent Types ==="
run_type "dynamics"            dynamics            --all --clusters
run_type "goals"               goals               --all --clusters
run_type "archetype-dynamics"  archetype-dynamics   --all --clusters

if [ "$failures" -gt 0 ]; then
  echo -e "${RED}=== Phase 3 Complete: $failures type(s) had failures ===${NC}"
  exit 1
else
  echo -e "${GREEN}=== Phase 3 Complete: all types succeeded ===${NC}"
fi
