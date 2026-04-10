#!/usr/bin/env bash
#
# add-all.sh — Register every canonical BA repo via `patina repo add`.
#
# Reads sdk/ba/repos.toml and iterates each [[repos]] entry, calling
# `patina repo add <url>`. Idempotent: re-running skips already-registered
# repos.
#
# Usage:
#   sdk/ba/scripts/add-all.sh                     # add all repos
#   sdk/ba/scripts/add-all.sh --priority high     # only high-priority
#   sdk/ba/scripts/add-all.sh --priority high,medium
#   sdk/ba/scripts/add-all.sh --no-oxidize        # skip semantic indices (faster)
#   sdk/ba/scripts/add-all.sh --dry-run           # print what would happen
#
# Exit codes:
#   0 — all targeted repos are present (added or already-existed)
#   1 — one or more `patina repo add` calls failed
#   2 — usage error
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPOS_TOML="${SCRIPT_DIR}/../repos.toml"

PRIORITY_FILTER=""
NO_OXIDIZE=""
DRY_RUN=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --priority)
            PRIORITY_FILTER="$2"
            shift 2
            ;;
        --priority=*)
            PRIORITY_FILTER="${1#*=}"
            shift
            ;;
        --no-oxidize)
            NO_OXIDIZE="--no-oxidize"
            shift
            ;;
        --dry-run)
            DRY_RUN=1
            shift
            ;;
        -h|--help)
            sed -n '3,18p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *)
            echo "error: unknown argument '$1'" >&2
            echo "run with --help for usage" >&2
            exit 2
            ;;
    esac
done

if [[ ! -f "$REPOS_TOML" ]]; then
    echo "error: repos.toml not found at $REPOS_TOML" >&2
    exit 2
fi

if ! command -v patina >/dev/null 2>&1; then
    echo "error: 'patina' not in PATH" >&2
    exit 2
fi

# Parse repos.toml. Each [[repos]] block has url= and priority= lines.
# Emit "url|priority" pairs to stdout.
parse_repos() {
    awk '
        BEGIN { url = ""; priority = "" }
        /^\[\[repos\]\]/ {
            if (url != "") print url "|" priority
            url = ""; priority = ""
            next
        }
        /^url = / {
            gsub(/^url = "/, "")
            gsub(/"$/, "")
            url = $0
            next
        }
        /^priority = / {
            gsub(/^priority = "/, "")
            gsub(/"$/, "")
            priority = $0
            next
        }
        END {
            if (url != "") print url "|" priority
        }
    ' "$REPOS_TOML"
}

# Build allowed-priority set from --priority flag (comma-separated).
# Empty filter means all priorities allowed.
priority_allowed() {
    local p="$1"
    if [[ -z "$PRIORITY_FILTER" ]]; then
        return 0
    fi
    local IFS=','
    for allowed in $PRIORITY_FILTER; do
        if [[ "$p" == "$allowed" ]]; then
            return 0
        fi
    done
    return 1
}

# Get list of already-registered repos (org/name format).
already_registered() {
    patina repo list 2>/dev/null \
        | awk 'NR>3 && $1 != "" && $1 !~ /^─/ { print $1 }' \
        || true
}

# Extract org/name from a GitHub URL.
url_to_name() {
    local url="$1"
    # https://github.com/owner/repo[.git] → owner/repo
    url="${url#https://github.com/}"
    url="${url#http://github.com/}"
    url="${url%.git}"
    url="${url%/}"
    echo "$url"
}

# Main loop.
total=0
skipped=0
added=0
failed=0
filtered=0

# Snapshot the registered set once up front.
registered_set=$(already_registered)

while IFS='|' read -r url priority; do
    [[ -z "$url" ]] && continue
    total=$((total + 1))

    if ! priority_allowed "$priority"; then
        filtered=$((filtered + 1))
        continue
    fi

    name=$(url_to_name "$url")

    if echo "$registered_set" | grep -qx "$name"; then
        printf "  skip   %-50s (already registered)\n" "$name"
        skipped=$((skipped + 1))
        continue
    fi

    if [[ $DRY_RUN -eq 1 ]]; then
        printf "  would  %-50s [%s]\n" "$name" "$priority"
        continue
    fi

    printf "  adding %-50s [%s] ... " "$name" "$priority"
    if patina repo add $NO_OXIDIZE "$url" >/dev/null 2>&1; then
        echo "ok"
        added=$((added + 1))
    else
        echo "FAILED"
        failed=$((failed + 1))
    fi
done < <(parse_repos)

echo
echo "Summary:"
echo "  total in repos.toml:    $total"
[[ -n "$PRIORITY_FILTER" ]] && echo "  filtered out:           $filtered"
echo "  already registered:     $skipped"
if [[ $DRY_RUN -eq 1 ]]; then
    echo "  would add:              $((total - filtered - skipped))"
else
    echo "  added this run:         $added"
    echo "  failed:                 $failed"
fi

[[ $failed -gt 0 ]] && exit 1
exit 0
