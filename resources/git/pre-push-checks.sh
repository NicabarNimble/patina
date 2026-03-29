#!/usr/bin/env bash
# Pre-push checks for Patina.
# Tier 1 (always): structural policy checks, cargo-free.
# Tier 2 (push default): targeted cargo lane.

set -euo pipefail

run_check_script() {
    local step_label="$1"
    local script_path="$2"

    if ! bash "$script_path"; then
        echo ""
        echo "❌ $step_label failed!"
        exit 1
    fi
}

RUN_TARGETED=true
if [[ "${PATINA_PRE_PUSH_RUN_TARGETED:-}" == "0" ]]; then
    RUN_TARGETED=false
fi
if [[ "${1:-}" == "--structural-only" ]]; then
    RUN_TARGETED=false
fi

echo "🔍 Running pre-push checks (Tier 1 structural)..."
echo ""

# Worlds currently shipped by sdk/patina-sdk.
SDK_WORLDS=(knowledge-child pipeline)

# Step 1: WIT consistency — SDK mirror must match canonical wit/
echo "📦 [1/8] Checking WIT consistency..."
wit_ok=true
for world in "${SDK_WORLDS[@]}"; do
    canonical_world="wit/$world/$world.wit"
    sdk_world="sdk/patina-sdk/wit/$world/$world.wit"
    if [[ ! -f "$canonical_world" ]]; then
        echo "   ERROR: missing canonical world: $canonical_world"
        wit_ok=false
        continue
    fi
    if [[ ! -f "$sdk_world" ]]; then
        echo "   ERROR: missing SDK world mirror: $sdk_world"
        wit_ok=false
        continue
    fi
    if ! diff "$canonical_world" "$sdk_world" >/dev/null 2>&1; then
        echo "   ERROR: $sdk_world differs from canonical"
        echo "   Fix: cp $canonical_world $sdk_world"
        wit_ok=false
    fi
done

for sdk_dep in sdk/patina-sdk/wit/*/deps/*.wit; do
    [[ -f "$sdk_dep" ]] || continue
    world=$(basename "$(dirname "$(dirname "$sdk_dep")")")
    dep_name=$(basename "$sdk_dep")
    canonical_dep="wit/$world/deps/$dep_name"
    if [[ ! -f "$canonical_dep" ]]; then
        echo "   ERROR: SDK dep has no canonical source: $sdk_dep"
        echo "   Expected canonical path: $canonical_dep"
        wit_ok=false
        continue
    fi
    if ! diff "$canonical_dep" "$sdk_dep" >/dev/null 2>&1; then
        echo "   ERROR: $sdk_dep differs from canonical"
        echo "   Fix: cp $canonical_dep $sdk_dep"
        wit_ok=false
    fi
done

if [[ "$wit_ok" == false ]]; then
    echo ""
    echo "❌ WIT consistency check failed!"
    exit 1
fi
echo "   ✓ WIT files consistent across SDK and canonical sources"
echo ""

# Step 2: WIT mirror completeness for SDK worlds
echo "📦 [2/8] Checking WIT mirror completeness..."
wit_mirror_ok=true
for world in "${SDK_WORLDS[@]}"; do
    for canonical_dep in "wit/$world"/deps/*.wit; do
        [[ -f "$canonical_dep" ]] || continue
        dep_name=$(basename "$canonical_dep")
        sdk_dep="sdk/patina-sdk/wit/$world/deps/$dep_name"
        if [[ ! -f "$sdk_dep" ]]; then
            echo "   ERROR: missing SDK dep mirror: $sdk_dep"
            echo "   Fix: cp $canonical_dep $sdk_dep"
            wit_mirror_ok=false
        fi
    done
done
if [[ "$wit_mirror_ok" == false ]]; then
    echo ""
    echo "❌ WIT mirror completeness check failed!"
    exit 1
fi
echo "   ✓ SDK mirrors all canonical deps for shipped worlds"
echo ""

echo "📦 [3/8] Checking crate naming policy..."
run_check_script "Crate naming policy check" "resources/scripts/check-crate-names.sh"
echo ""

echo "📦 [4/8] Checking core/protocol dependency direction..."
run_check_script "Core/protocol dependency direction check" "resources/scripts/check-core-protocol-deps.sh"
echo ""

echo "📦 [5/8] Checking single SDK surface..."
run_check_script "Single SDK surface check" "resources/scripts/check-single-sdk-surface.sh"
echo ""

echo "📦 [6/8] Checking runtime boundary drift..."
run_check_script "Runtime boundary drift check" "resources/scripts/check-runtime-boundaries.sh"
echo ""

echo "📦 [7/8] Checking layer output contract..."
run_check_script "Layer output contract check" "resources/scripts/check-layer-output-contract.sh"
echo ""

echo "📦 [8/8] Checking retired MCP surface invariants..."
run_check_script "Retired MCP surface invariant check" "resources/scripts/check-retired-mcp-surface.sh"
echo ""

echo "✅ Tier 1 structural checks passed (cargo-free)."

if [[ "$RUN_TARGETED" == false ]]; then
    echo ""
    echo "ℹ Tier 2 targeted cargo lane not run in this invocation."
    echo "  Use default invocation (or omit --structural-only) to include Tier 2."
    exit 0
fi

echo ""
echo "🚀 Running Tier 2 targeted cargo lane..."
bash resources/git/pre-push-targeted-cargo.sh
