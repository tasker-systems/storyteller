#!/bin/bash
# Run genre region elicitation in batches of 5 to avoid saturating the machine.
# Safe to interrupt — already-generated files are preserved via versioning.
# Re-run this script to pick up where you left off.

set -e
cd "$(dirname "$0")"
export STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data

BATCH1="hard-sci-fi,space-opera,cyberpunk,nordic-noir,cozy-mystery"
BATCH2="psychological-thriller,domestic-noir,romantasy,historical-romance,contemporary-romance"
BATCH3="southern-gothic,westerns,swashbuckling-adventure,survival-fiction,horror-comedy"
BATCH4="working-class-realism,pastoral-rural-fiction,classical-tragedy,solarpunk,historical-fiction"
BATCH5="literary-fiction,magical-realism"

for i in 1 2 3 4 5; do
    BATCH_VAR="BATCH${i}"
    REGIONS="${!BATCH_VAR}"
    echo ""
    echo "=========================================="
    echo "  Batch ${i}: ${REGIONS}"
    echo "=========================================="
    echo ""
    uv run narrative-data genre elicit --regions "$REGIONS" --categories region
    echo ""
    echo "  Batch ${i} complete. Pausing 30s to let the machine cool..."
    echo ""
    sleep 30
done

echo ""
echo "All batches complete!"
echo ""
