#!/usr/bin/env bash
# Stage 2 Phase 2: Structure independent types (no inter-type dependencies)
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

echo "=== Stage 2 Phase 2: Independent Types ==="
run_type "archetypes"          archetypes          --all --clusters
run_type "settings"            settings            --all --clusters
run_type "ontological-posture" ontological-posture  --all --clusters
run_type "profiles"            profiles             --all --clusters
run_type "tropes"              tropes               --all
run_type "narrative-shapes"    narrative-shapes      --all

if [ "$failures" -gt 0 ]; then
  echo -e "${RED}=== Phase 2 Complete: $failures type(s) had failures ===${NC}"
  exit 1
else
  echo -e "${GREEN}=== Phase 2 Complete: all types succeeded ===${NC}"
fi
