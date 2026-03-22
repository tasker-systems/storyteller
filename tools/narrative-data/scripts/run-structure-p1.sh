#!/usr/bin/env bash
# Stage 2 Phase 1: Structure genre dimensions (foundation)
# Must complete before P2-P4 — genre dimensions are the base schema
set -euo pipefail

echo "=== Stage 2 Phase 1: Genre Dimensions ==="
narrative-data structure run genre-dimensions --all
echo "=== Phase 1 Complete ==="
