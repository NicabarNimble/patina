#!/bin/bash
# Create an epistemic belief file with validation
# Usage: create-belief.sh --id ID --statement "..." --persona PERSONA --evidence "..." [--facets "..."] [--entrenchment ...]
#
# Note: No --confidence flag. Confidence is COMPUTED by `patina scrape` from
# real data (citations, evidence links, verified wikilinks). Not fabricated.

set -e

# Default values
BELIEFS_DIR="layer/surface/epistemic/beliefs"
ENTRENCHMENT="medium"
STATUS="active"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --id)
            ID="$2"
            shift 2
            ;;
        --statement)
            STATEMENT="$2"
            shift 2
            ;;
        --persona)
            PERSONA="$2"
            shift 2
            ;;
        --confidence)
            # Accept but ignore --confidence for backwards compatibility
            echo "Note: --confidence is deprecated. Confidence is now computed by 'patina scrape' from real data."
            shift 2
            ;;
        --evidence)
            EVIDENCE="$2"
            shift 2
            ;;
        --facets)
            FACETS="$2"
            shift 2
            ;;
        --entrenchment)
            ENTRENCHMENT="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validation
ERRORS=""

if [ -z "$ID" ]; then
    ERRORS="${ERRORS}Error: --id is required\n"
elif ! [[ "$ID" =~ ^[a-z][a-z0-9-]*$ ]]; then
    ERRORS="${ERRORS}Error: --id must be lowercase letters, numbers, and hyphens (start with letter)\n"
fi

if [ -z "$STATEMENT" ]; then
    ERRORS="${ERRORS}Error: --statement is required\n"
fi

if [ -z "$PERSONA" ]; then
    ERRORS="${ERRORS}Error: --persona is required\n"
fi

if [ -z "$EVIDENCE" ]; then
    ERRORS="${ERRORS}Error: --evidence is required (at least one source)\n"
fi

if [ -n "$ERRORS" ]; then
    echo -e "$ERRORS"
    echo "Usage: create-belief.sh --id ID --statement \"...\" --persona PERSONA --evidence \"...\" [--facets \"...\"]"
    exit 1
fi

# Check if file already exists
OUTPUT_FILE="${BELIEFS_DIR}/${ID}.md"
if [ -f "$OUTPUT_FILE" ]; then
    echo "Error: Belief file already exists: $OUTPUT_FILE"
    echo "To update an existing belief, edit the file directly."
    exit 1
fi

# Ensure directory exists
mkdir -p "$BELIEFS_DIR"

# Format facets as YAML array
if [ -n "$FACETS" ]; then
    FACETS_YAML="[$(echo "$FACETS" | sed 's/,/, /g')]"
else
    FACETS_YAML="[]"
fi

# Get current date
TODAY=$(date +%Y-%m-%d)

# Get active session ID for evidence provenance
SESSION_ID=""
ACTIVE_SESSION=".patina/local/active-session.md"
if [ -f "$ACTIVE_SESSION" ]; then
    SESSION_ID=$(grep "^id:" "$ACTIVE_SESSION" | head -1 | sed 's/id: *//')
fi

# Prepend session provenance to evidence if we have an active session
if [ -n "$SESSION_ID" ]; then
    EVIDENCE="[[session-${SESSION_ID}]]: ${EVIDENCE}"
fi

# Create the belief file — no fabricated confidence scores
# Metrics are computed by `patina scrape` from real data:
#   use: cited_by_beliefs, cited_by_sessions, applied_in
#   truth: evidence_count, evidence_verified, defeated_attacks, external_sources
cat > "$OUTPUT_FILE" << EOF
---
type: belief
id: ${ID}
persona: ${PERSONA}
facets: ${FACETS_YAML}
entrenchment: ${ENTRENCHMENT}
status: ${STATUS}
endorsed: true
extracted: ${TODAY}
revised: ${TODAY}
---

# ${ID}

${STATEMENT}

## Statement

${STATEMENT}

## Evidence

- ${EVIDENCE}

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- ${TODAY}: Created — metrics computed by \`patina scrape\`
EOF

echo "✓ Belief created: $OUTPUT_FILE"
echo ""
echo "Next steps:"
echo "  1. Review and edit the file to add:"
echo "     - Additional evidence links (use [[wikilinks]] for verifiable references)"
echo "     - Support/attack relationships"
echo "     - Applied-in examples"
echo "  2. Run 'patina scrape' to compute use/truth metrics"
echo "  3. Commit: git add $OUTPUT_FILE && git commit -m 'belief: add ${ID}'"
