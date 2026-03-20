#!/bin/bash
# Run Phase 2 cluster synthesis for a primitive type.
# Synthesizes per-genre extractions into deduplicated cluster-level archetype lists.
# Safe to interrupt — already-generated files are preserved via manifest staleness.
# Re-run this script to pick up where you left off (completed clusters are skipped).
#
# Usage:
#   ./run-discovery-synthesize.sh                  # archetypes (default)
#   ./run-discovery-synthesize.sh dynamics          # specific type

set -e
cd "$(dirname "$0")"
export STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data

TYPE="${1:-archetypes}"

CLUSTERS="horror fantasy sci-fi mystery-thriller romance realism-gothic-other"

echo ""
echo "============================================"
echo "  Phase 2: Synthesize ${TYPE} across clusters"
echo "  $(date '+%H:%M:%S')"
echo "============================================"
echo ""

for cluster in $CLUSTERS; do
    echo ""
    echo "=========================================="
    echo "  Cluster: ${cluster}"
    echo "  $(date '+%H:%M:%S')"
    echo "=========================================="
    echo ""
    uv run narrative-data discover synthesize --type "$TYPE" --cluster "$cluster"
    echo ""
    echo "  ${cluster} complete at $(date '+%H:%M:%S'). Pausing 30s..."
    echo ""
    sleep 30
done

echo ""
echo "============================================"
echo "  Phase 2 synthesis complete for ${TYPE}!"
echo "  $(date '+%H:%M:%S')"
echo "============================================"
echo ""

# Show pipeline status
uv run narrative-data pipeline status --type "$TYPE"
