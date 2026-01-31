#!/bin/bash
# A/B Test: Rust patina session vs shell session-*.sh
#
# Runs the full Rust session lifecycle (start → note → update → end),
# picks a recent shell-produced session for comparison, normalizes both,
# and produces a structural diff.
#
# Usage: ./resources/test/ab-session-test.sh [--keep]
#   --keep  Don't clean up the Rust test session from layer/sessions/

set -euo pipefail

KEEP=false
if [[ "${1:-}" == "--keep" ]]; then
    KEEP=true
fi

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}=== A/B Test: Rust vs Shell Session Lifecycle ===${NC}"
echo

# --- PRE-CHECKS ---
if [ -f .patina/local/active-session.md ]; then
    echo -e "${RED}ERROR: Active Rust session exists. End it first: patina session end${NC}"
    exit 1
fi

# Find a recent shell-produced session for comparison (pre-0.9.2 sessions are all shell)
SHELL_SESSION=$(ls layer/sessions/202601*.md 2>/dev/null | sort | tail -1)
if [ -z "$SHELL_SESSION" ]; then
    SHELL_SESSION=$(ls layer/sessions/202*.md 2>/dev/null | sort | tail -1)
fi
echo -e "Shell reference: ${YELLOW}${SHELL_SESSION}${NC}"
echo

# --- RUST LIFECYCLE ---
echo -e "${CYAN}--- Running Rust lifecycle ---${NC}"
echo

echo "1. patina session start"
patina session start "A/B test — Rust vs Shell comparison" 2>&1 | head -20
echo

# Small delay to ensure different timestamps if needed
sleep 1

echo "2. patina session note"
patina session note "Test note for A/B comparison — verifying format parity" 2>&1
echo

sleep 1

echo "3. patina session update"
patina session update 2>&1 | head -30
echo

sleep 1

echo "4. patina session end"
patina session end 2>&1 | head -40
echo

# Find the Rust-produced archived session (most recent)
RUST_SESSION=$(ls layer/sessions/202*.md 2>/dev/null | sort | tail -1)
echo -e "Rust session:  ${YELLOW}${RUST_SESSION}${NC}"
echo

# --- NORMALIZE ---
TMPDIR=$(mktemp -d)
RUST_NORM="$TMPDIR/rust_normalized.md"
SHELL_NORM="$TMPDIR/shell_normalized.md"

# Normalize: replace variable content with placeholders
normalize() {
    sed -E \
        -e "s/[0-9]{8}-[0-9]{6}/YYYYMMDD-HHMMSS/g" \
        -e "s/[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z/YYYY-MM-DDTHH:MM:SSZ/g" \
        -e "s/[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}[+-][0-9]{2}:[0-9]{2}/YYYY-MM-DDTHH:MM:SS+ZZ:ZZ/g" \
        -e "s/[0-9]{10,13}/TIMESTAMP/g" \
        -e "s/[a-f0-9]{40}/FULL_SHA/g" \
        -e "s/[a-f0-9]{7,8}([^a-f0-9])/SHORT_SHA\1/g" \
        -e "s/[0-9]{2}:[0-9]{2}/HH:MM/g" \
        -e "s/  +/ /g" \
        -e "s/A\/B test — Rust vs Shell comparison/TEST_TITLE/g" \
        "$1"
}

normalize "$RUST_SESSION" > "$RUST_NORM"
normalize "$SHELL_SESSION" > "$SHELL_NORM"

# --- STRUCTURAL ANALYSIS ---
echo -e "${CYAN}=== Structural Comparison ===${NC}"
echo

# Extract section headers from both
echo -e "${YELLOW}Rust sections:${NC}"
grep -E "^#{1,3} |^---$|^- " "$RUST_SESSION" | head -30
echo

echo -e "${YELLOW}Shell sections:${NC}"
grep -E "^#{1,3} |^---$|^- " "$SHELL_SESSION" | head -30
echo

# --- DIFF ---
echo -e "${CYAN}=== Normalized Diff ===${NC}"
echo -e "(${RED}--- shell${NC}, ${GREEN}+++ rust${NC})"
echo

# Header format comparison
echo -e "${YELLOW}Header format:${NC}"
head -12 "$RUST_SESSION"
echo -e "${YELLOW}---vs---${NC}"
head -12 "$SHELL_SESSION"
echo

# Full normalized diff
if diff -u "$SHELL_NORM" "$RUST_NORM" > "$TMPDIR/diff.txt" 2>&1; then
    echo -e "${GREEN}No structural differences (after normalization)${NC}"
else
    cat "$TMPDIR/diff.txt"
fi

# --- YAML FRONTMATTER VERIFICATION ---
echo
echo -e "${CYAN}=== YAML Frontmatter Verification ===${NC}"

if head -1 "$RUST_SESSION" | grep -q "^---$"; then
    echo -e "${GREEN}PASS${NC}: Rust session has YAML frontmatter"
    # Verify key fields
    for field in "type:" "id:" "title:" "status:" "llm:" "created:" "git:"; do
        if grep -q "^$field\|^  $field" "$RUST_SESSION"; then
            echo -e "  ${GREEN}OK${NC}: $field found"
        else
            echo -e "  ${RED}MISSING${NC}: $field"
        fi
    done

    # Verify status is archived (session ended)
    if grep -q "status: archived" "$RUST_SESSION"; then
        echo -e "  ${GREEN}OK${NC}: status changed to 'archived'"
    else
        echo -e "  ${RED}FAIL${NC}: status not changed to 'archived'"
    fi
else
    echo -e "${RED}FAIL${NC}: Rust session missing YAML frontmatter"
fi

if head -1 "$SHELL_SESSION" | grep -q "^# Session:"; then
    echo -e "${GREEN}PASS${NC}: Shell session has legacy markdown header"
else
    echo -e "${YELLOW}NOTE${NC}: Shell session format unexpected"
fi

# --- SCRAPER VERIFICATION ---
echo
echo -e "${CYAN}=== Scraper Verification ===${NC}"
echo "Running full session scrape to verify both formats..."
patina scrape sessions --full 2>&1

# Query the Rust session from DB
RUST_ID=$(basename "$RUST_SESSION" .md)
echo
echo -e "Querying scraped data for Rust session: ${YELLOW}${RUST_ID}${NC}"
sqlite3 .patina/local/data/patina.db "SELECT id, title, started_at, branch, classification FROM sessions WHERE id = '${RUST_ID}'" 2>&1 || echo "(query failed)"

# --- INTENTIONAL DIFFERENCES ---
echo
echo -e "${CYAN}=== Intentional Differences ===${NC}"
echo
echo "1. HEADER FORMAT:"
echo "   Shell: # Session: <title> / **Field**: value lines"
echo "   Rust:  --- YAML frontmatter --- (type, id, title, status, llm, created, git)"
echo
echo "2. STATUS TRACKING:"
echo "   Shell: No status field"
echo "   Rust:  status: active → status: archived on end"
echo
echo "3. NUMBER FORMATTING:"
echo -n "   Shell: "
grep "Files Changed:" "$SHELL_SESSION" | head -1 || echo "(not found)"
echo -n "   Rust:  "
grep "Files Changed:" "$RUST_SESSION" | head -1 || echo "(not found)"
echo "   (Shell has leading whitespace from wc -l; Rust has clean numbers)"
echo
echo "4. FILE PATHS:"
echo "   Shell: .claude/context/active-session.md"
echo "   Rust:  .patina/local/active-session.md"
echo
echo "5. START_TIMESTAMP:"
echo "   Shell: \`**Start Timestamp**: <epoch_ms>\` (markdown field)"
echo "   Rust:  \`start_timestamp: <epoch_ms>\` (YAML field)"
echo

# --- CLEANUP ---
if [ "$KEEP" = false ]; then
    echo -e "${YELLOW}Cleaning up test session...${NC}"
    # Remove the test session file and its git tags
    RUST_TAGS=$(git tag | grep "session-${RUST_ID}" 2>/dev/null || true)
    if [ -n "$RUST_TAGS" ]; then
        echo "$RUST_TAGS" | xargs git tag -d 2>/dev/null || true
    fi
    rm -f "$RUST_SESSION"
    echo "  Removed: $RUST_SESSION"
    echo "  Removed tags: $RUST_TAGS"
else
    echo -e "${GREEN}Keeping test session: $RUST_SESSION${NC}"
fi

# Cleanup temp files
rm -rf "$TMPDIR"

echo
echo -e "${CYAN}=== A/B Test Complete ===${NC}"
