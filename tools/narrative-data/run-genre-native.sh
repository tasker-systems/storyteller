#!/bin/bash
# Run genre-native elicitation (tropes, narrative-shapes) across all 30 genres.
# Safe to interrupt — already-generated files are skipped on re-run.
#
# Usage:
#   ./run-genre-native.sh tropes              # elicit tropes
#   ./run-genre-native.sh narrative-shapes     # elicit narrative shapes

set -e
cd "$(dirname "$0")"
export STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data

TYPE="${1:?Usage: ./run-genre-native.sh <tropes|narrative-shapes>}"

BATCH1="folk-horror,cosmic-horror,horror-comedy,high-epic-fantasy,dark-fantasy"
BATCH2="cozy-fantasy,fairy-tale-mythic,urban-fantasy,quiet-contemplative-fantasy,hard-sci-fi"
BATCH3="space-opera,cyberpunk,nordic-noir,cozy-mystery,psychological-thriller"
BATCH4="domestic-noir,romantasy,historical-romance,contemporary-romance,southern-gothic"
BATCH5="westerns,swashbuckling-adventure,survival-fiction,working-class-realism,pastoral-rural-fiction"
BATCH6="classical-tragedy,solarpunk,historical-fiction,literary-fiction,magical-realism"

echo ""
echo "============================================"
echo "  Genre-native elicitation: ${TYPE}"
echo "  $(date '+%H:%M:%S')"
echo "============================================"
echo ""

for i in 1 2 3 4 5 6; do
    BATCH_VAR="BATCH${i}"
    GENRES="${!BATCH_VAR}"
    echo ""
    echo "=========================================="
    echo "  Batch ${i}/6: ${GENRES}"
    echo "  $(date '+%H:%M:%S')"
    echo "=========================================="
    echo ""
    uv run narrative-data genre elicit-native --type "$TYPE" --genres "$GENRES"
    echo ""
    echo "  Batch ${i} complete at $(date '+%H:%M:%S'). Pausing 30s..."
    echo ""

    if [ "$i" -lt 6 ]; then
        sleep 30
    fi
done

echo ""
echo "============================================"
echo "  Genre-native elicitation complete: ${TYPE}"
echo "  $(date '+%H:%M:%S')"
echo "============================================"
echo ""
