#!/bin/bash
# Run Phase 1 primitive discovery extraction in batches.
# Safe to interrupt — already-generated files are preserved via manifest staleness.
# Re-run this script to pick up where you left off (completed genres are skipped).
#
# Usage:
#   ./run-discovery-extract.sh                    # archetypes (default)
#   ./run-discovery-extract.sh dynamics            # specific type
#   ./run-discovery-extract.sh archetypes 2        # start from batch 2

set -e
cd "$(dirname "$0")"
export STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data

TYPE="${1:-archetypes}"
START_BATCH="${2:-1}"

# Batches of 5-6 genres, same grouping as region elicitation
BATCH1="folk-horror,cosmic-horror,horror-comedy,high-epic-fantasy,dark-fantasy"
BATCH2="cozy-fantasy,fairy-tale-mythic,urban-fantasy,quiet-contemplative-fantasy,hard-sci-fi"
BATCH3="space-opera,cyberpunk,nordic-noir,cozy-mystery,psychological-thriller"
BATCH4="domestic-noir,romantasy,historical-romance,contemporary-romance,southern-gothic"
BATCH5="westerns,swashbuckling-adventure,survival-fiction,working-class-realism,pastoral-rural-fiction"
BATCH6="classical-tragedy,solarpunk,historical-fiction,literary-fiction,magical-realism"

echo ""
echo "============================================"
echo "  Phase 1: Extract ${TYPE} from genre corpus"
echo "  Starting from batch ${START_BATCH}"
echo "============================================"
echo ""

for i in 1 2 3 4 5 6; do
    if [ "$i" -lt "$START_BATCH" ]; then
        continue
    fi

    BATCH_VAR="BATCH${i}"
    REGIONS="${!BATCH_VAR}"
    echo ""
    echo "=========================================="
    echo "  Batch ${i}/6: ${REGIONS}"
    echo "  $(date '+%H:%M:%S')"
    echo "=========================================="
    echo ""
    uv run narrative-data discover extract --type "$TYPE" --genres "$REGIONS"
    echo ""
    echo "  Batch ${i} complete at $(date '+%H:%M:%S'). Pausing 30s..."
    echo ""

    # Don't sleep after the last batch
    if [ "$i" -lt 6 ]; then
        sleep 30
    fi
done

echo ""
echo "============================================"
echo "  Phase 1 extraction complete for ${TYPE}!"
echo "  $(date '+%H:%M:%S')"
echo "============================================"
echo ""

# Show pipeline status
uv run narrative-data pipeline status --type "$TYPE"
