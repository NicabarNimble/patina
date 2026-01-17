#!/bin/bash
# Migrate cached ref repos to new .patina/local/ structure
# Run once after upgrading to patina with .patina/local/ paths

set -e

CACHE_DIR="${HOME}/.patina/cache/repos"

if [ ! -d "$CACHE_DIR" ]; then
    echo "No cached repos found at $CACHE_DIR"
    exit 0
fi

echo "Migrating ref repos to .patina/local/ structure..."
echo ""

migrated=0
skipped=0
already_done=0

for repo in "$CACHE_DIR"/*/; do
    repo_name=$(basename "$repo")

    # Check if old path exists
    if [ -d "$repo/.patina/data" ]; then
        # Check if already migrated
        if [ -d "$repo/.patina/local/data" ]; then
            echo "  $repo_name: already has local/data, removing old data/"
            rm -rf "$repo/.patina/data"
            ((already_done++))
        else
            # Migrate
            mkdir -p "$repo/.patina/local"
            mv "$repo/.patina/data" "$repo/.patina/local/"
            echo "  $repo_name: migrated data/ -> local/data/"
            ((migrated++))
        fi
    elif [ -d "$repo/.patina/local/data" ]; then
        echo "  $repo_name: already migrated"
        ((already_done++))
    else
        echo "  $repo_name: no data to migrate"
        ((skipped++))
    fi
done

echo ""
echo "Migration complete:"
echo "  Migrated: $migrated"
echo "  Already done: $already_done"
echo "  Skipped: $skipped"
echo ""
echo "Run 'patina scrape forge --repo <name>' to refresh data."
