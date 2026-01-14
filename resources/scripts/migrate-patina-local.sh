#!/bin/bash
# Migrate old .patina/ structure to new .patina/local/ design
# Run this once in projects created before the .patina/local/ change

set -e

echo "Migrating .patina/ to new local/ structure..."

# 1. Update .gitignore
if [ -f .gitignore ]; then
    if grep -q "^\.patina/$" .gitignore; then
        sed -i '' 's|^\.patina/$|.patina/local/|' .gitignore
        echo "✓ Updated .gitignore: .patina/ → .patina/local/"
    elif grep -q "^\.patina/local/$" .gitignore; then
        echo "✓ .gitignore already has .patina/local/"
    else
        echo ".patina/local/" >> .gitignore
        echo "✓ Added .patina/local/ to .gitignore"
    fi
fi

# 2. Move data/ to local/data/
if [ -d .patina/data ]; then
    mkdir -p .patina/local
    mv .patina/data .patina/local/
    echo "✓ Moved .patina/data/ → .patina/local/data/"
fi

# 3. Move backups/ to local/backups/
if [ -d .patina/backups ]; then
    mkdir -p .patina/local
    mv .patina/backups .patina/local/
    echo "✓ Moved .patina/backups/ → .patina/local/backups/"
fi

echo ""
echo "Migration complete. Now run: patina init ."
