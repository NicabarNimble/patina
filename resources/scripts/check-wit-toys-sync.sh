#!/usr/bin/env bash
set -euo pipefail

TOYS_DIR="wit/toys"
WORLDS_DIR="wit/worlds"

if [[ ! -d "$TOYS_DIR" ]]; then
    echo "error: missing $TOYS_DIR" >&2
    exit 1
fi

if [[ ! -d "$WORLDS_DIR" ]]; then
    echo "error: missing $WORLDS_DIR" >&2
    exit 1
fi

ok=true

for toy in "$TOYS_DIR"/*.wit; do
    base=$(basename "$toy")
    world_copy="$WORLDS_DIR/$base"

    if [[ ! -f "$world_copy" ]]; then
        echo "error: missing mirrored WIT file $world_copy"
        ok=false
        continue
    fi

    if ! diff "$toy" "$world_copy" >/dev/null 2>&1; then
        echo "error: WIT drift detected between $toy and $world_copy"
        ok=false
    fi
done

for world_file in "$WORLDS_DIR"/*.wit; do
    base=$(basename "$world_file")
    if [[ "$base" == "ducklake.wit" || "$base" == "belief-verifier.wit" ]]; then
        continue
    fi

    if [[ ! -f "$TOYS_DIR/$base" ]]; then
        echo "error: unexpected non-world WIT file in $WORLDS_DIR: $base"
        ok=false
    fi
done

if [[ "$ok" == false ]]; then
    echo "fix: cp wit/toys/*.wit wit/worlds/"
    exit 1
fi

echo "ok: wit/toys and wit/worlds interface files are synchronized"
