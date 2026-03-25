#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# Run LLM patch fills for spatial-topology type across all 30 genres.
# Batches of 5-6 genres with 30s cooling between batches.
set -euo pipefail

cd "$(dirname "$0")/.."

genre_flags() {
    IFS=',' read -ra GENRES <<< "$1"
    for g in "${GENRES[@]}"; do echo -n "--genre $g "; done
}

BATCH1="folk-horror,cosmic-horror,horror-comedy,high-epic-fantasy,dark-fantasy"
BATCH2="cozy-fantasy,fairy-tale-mythic,urban-fantasy,quiet-contemplative-fantasy,hard-sci-fi"
BATCH3="space-opera,cyberpunk,solarpunk,dystopian,post-apocalyptic"
BATCH4="cozy-mystery,noir,psychological-thriller,spy-thriller,detective-procedural"
BATCH5="contemporary-romance,historical-romance,paranormal-romance,romantasy,literary-fiction"
BATCH6="historical-fiction,southern-gothic,magical-realism,afrofuturism,classical-tragedy,pastoral"

echo "Starting spatial-topology LLM patch fill — $(date)"

for i in 1 2 3 4 5 6; do
    BATCH_VAR="BATCH$i"
    GENRES="${!BATCH_VAR}"
    echo ""
    echo "=== Batch $i/6: $GENRES ==="
    eval uv run narrative-data fill --tier llm-patch --type spatial-topology $(genre_flags "$GENRES")
    if [ "$i" -lt 6 ]; then
        echo "Cooling for 30s..."
        sleep 30
    fi
done

echo ""
echo "Spatial-topology patch fill complete — $(date)"
