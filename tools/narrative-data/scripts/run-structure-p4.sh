#!/usr/bin/env bash
# Stage 2 Phase 4: Spatial types
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

echo "=== Stage 2 Phase 4: Spatial Types ==="
run_type "spatial-topology"    spatial-topology    --all --clusters
run_type "place-entities"      place-entities      --all --clusters

if [ "$failures" -gt 0 ]; then
  echo -e "${RED}=== Phase 4 Complete: $failures type(s) had failures ===${NC}"
  exit 1
else
  echo -e "${GREEN}=== Phase 4 Complete: all types succeeded ===${NC}"
fi
