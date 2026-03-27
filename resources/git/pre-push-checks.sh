#!/bin/bash
# Pre-push checks for Patina - fast local gate
#
# Runs fmt, clippy, and tests before push (~1-2 min).
# CI handles the full suite (including cargo install --locked).

set -e

echo "🔍 Running pre-push checks..."
echo ""

# Step 1: WIT consistency — SDK mirror must match canonical wit/
echo "📦 [1/14] Checking WIT consistency..."
wit_ok=true
echo "   Checking SDK WIT consistency..."

# Worlds currently shipped by sdk/patina-sdk.
SDK_WORLDS=(knowledge-child pipeline)

for world in "${SDK_WORLDS[@]}"; do
    canonical_world="wit/$world/$world.wit"
    sdk_world="sdk/patina-sdk/wit/$world/$world.wit"
    if [ ! -f "$canonical_world" ]; then
        echo "   ERROR: missing canonical world: $canonical_world"
        wit_ok=false
        continue
    fi
    if [ ! -f "$sdk_world" ]; then
        echo "   ERROR: missing SDK world mirror: $sdk_world"
        wit_ok=false
        continue
    fi
    if ! diff "$canonical_world" "$sdk_world" > /dev/null 2>&1; then
        echo "   ERROR: $sdk_world differs from canonical"
        echo "   Fix: cp $canonical_world $sdk_world"
        wit_ok=false
    fi
done

for sdk_dep in sdk/patina-sdk/wit/*/deps/*.wit; do
    [ -f "$sdk_dep" ] || continue
    world=$(basename "$(dirname "$(dirname "$sdk_dep")")")
    dep_name=$(basename "$sdk_dep")
    canonical_dep="wit/$world/deps/$dep_name"
    if [ ! -f "$canonical_dep" ]; then
        echo "   ERROR: SDK dep has no canonical source: $sdk_dep"
        echo "   Expected canonical path: $canonical_dep"
        wit_ok=false
        continue
    fi
    if ! diff "$canonical_dep" "$sdk_dep" > /dev/null 2>&1; then
        echo "   ERROR: $sdk_dep differs from canonical"
        echo "   Fix: cp $canonical_dep $sdk_dep"
        wit_ok=false
    fi
done

if [ "$wit_ok" = false ]; then
    echo ""
    echo "❌ WIT consistency check failed!"
    exit 1
fi
echo "   ✓ WIT files consistent across SDK and canonical sources"
echo ""

# Step 2: WIT mirror completeness for SDK worlds
echo "📦 [2/14] Checking WIT mirror completeness..."
wit_mirror_ok=true
for world in "${SDK_WORLDS[@]}"; do
    for canonical_dep in "wit/$world"/deps/*.wit; do
        [ -f "$canonical_dep" ] || continue
        dep_name=$(basename "$canonical_dep")
        sdk_dep="sdk/patina-sdk/wit/$world/deps/$dep_name"
        if [ ! -f "$sdk_dep" ]; then
            echo "   ERROR: missing SDK dep mirror: $sdk_dep"
            echo "   Fix: cp $canonical_dep $sdk_dep"
            wit_mirror_ok=false
        fi
    done
done
if [ "$wit_mirror_ok" = false ]; then
    echo ""
    echo "❌ WIT mirror completeness check failed!"
    exit 1
fi
echo "   ✓ SDK mirrors all canonical deps for shipped worlds"
echo ""

# Step 3: Crate naming policy (CI parity)
echo "📦 [3/14] Checking crate naming policy..."
if ! bash resources/scripts/check-crate-names.sh; then
    echo ""
    echo "❌ Crate naming policy check failed!"
    exit 1
fi
echo ""

# Step 4: Core/protocol dependency direction (CI parity)
echo "📦 [4/14] Checking core/protocol dependency direction..."
if ! bash resources/scripts/check-core-protocol-deps.sh; then
    echo ""
    echo "❌ Core/protocol dependency direction check failed!"
    exit 1
fi
echo ""

# Step 5: Single SDK surface (CI parity)
echo "📦 [5/14] Checking single SDK surface..."
if ! bash resources/scripts/check-single-sdk-surface.sh; then
    echo ""
    echo "❌ Single SDK surface check failed!"
    exit 1
fi
echo ""

# Step 6: Runtime boundary drift (CI parity)
echo "📦 [6/14] Checking runtime boundary drift..."
if ! bash resources/scripts/check-runtime-boundaries.sh; then
    echo ""
    echo "❌ Runtime boundary drift check failed!"
    exit 1
fi
echo ""

# Step 7: Layer output contract (CI parity)
echo "📦 [7/14] Checking layer output contract..."
if ! bash resources/scripts/check-layer-output-contract.sh; then
    echo ""
    echo "❌ Layer output contract check failed!"
    exit 1
fi
echo ""

# Step 8: DuckLake wasm parity (CI parity)
echo "📦 [8/14] Checking DuckLake wasm parity..."
if ! bash resources/scripts/check-ducklake-parity.sh; then
    echo ""
    echo "❌ DuckLake wasm parity check failed!"
    exit 1
fi
echo ""

# Step 9: Check formatting (CI uses --check, not --fix)
echo "📦 [9/14] Checking Rust formatting..."
if ! cargo fmt --all -- --check; then
    echo ""
    echo "❌ Formatting check failed!"
    echo "   Run: cargo fmt --all"
    exit 1
fi
echo "   ✓ Formatting OK"
echo ""

# Step 10: Clippy with -D warnings (same as CI)
echo "📦 [10/14] Running clippy (warnings = errors)..."
if ! cargo clippy --workspace -- -D warnings; then
    echo ""
    echo "❌ Clippy failed! Fix warnings above."
    exit 1
fi
echo "   ✓ Clippy OK"
echo ""

# Step 11: Run tests
echo "📦 [11/14] Running tests..."
if ! cargo test --workspace; then
    echo ""
    echo "❌ Tests failed!"
    exit 1
fi
echo "   ✓ Tests OK"
echo ""

# Step 12: Broker integration test — catches cross-crate wire format drift
# Requires: test-child installed at ~/.patina/children/test-child/
echo "📦 [12/14] Broker integration test..."
if cargo metadata --no-deps --format-version=1 2>/dev/null | grep -q '"name":"patina-pipe"' \
   && [ -x "$HOME/.patina/children/test-child/test-child" ] \
   && [ -f ".patina/sources.toml" ]; then
    # Clean slate: remove previous test data so dedup doesn't mask failures
    if [ -f ".patina/local/data/events.db" ]; then
        sqlite3 .patina/local/data/events.db "DELETE FROM eventlog WHERE source_id = 'child:test-child'" 2>/dev/null || true
        sqlite3 .patina/local/data/events.db "DELETE FROM broker_cursors WHERE source_name = 'test'" 2>/dev/null || true
    fi
    # Build test-child from source to match current pipe protocol
    cargo build -p patina-pipe --example test-child --release --quiet 2>/dev/null
    cp target/release/examples/test-child "$HOME/.patina/children/test-child/test-child"
    # Run broker end-to-end (--no-sandbox for CI environments)
    broker_output=$(cargo run --release --quiet -- mother run test --no-sandbox 2>&1 || true)
    if echo "$broker_output" | grep -q "facts written"; then
        echo "   ✓ Broker integration OK (test-child → events.db)"
    elif echo "$broker_output" | grep -q "destination=project, which is retired"; then
        echo "   ⊘ Skipped (test source still uses retired destination=project route)"
    elif echo "$broker_output" | grep -q "connection 'test' is not allowed for api.github.com"; then
        echo "   ⊘ Skipped (test source is not valid for github-only lake route policy)"
    else
        echo "   ❌ Broker integration test failed!"
        echo "   'patina mother run test --no-sandbox' did not produce facts."
        echo "   This usually means the pipe protocol wire format drifted."
        echo ""
        echo "$broker_output"
        exit 1
    fi
else
    echo "   ⊘ Skipped (test-child not installed or no .patina/sources.toml)"
fi
echo ""

# Step 13: MCP thin handler invariants (post mcp-thin-handlers spec)
echo "📦 [13/14] Checking MCP handler invariants..."
SQL_IN_MCP=$(rg -c 'SELECT|FROM .* WHERE|ORDER BY' src/mcp/server/scry.rs src/mcp/server/assay.rs 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
if [ "$SQL_IN_MCP" -gt 0 ]; then
    echo "   ERROR: $SQL_IN_MCP SQL statements found in MCP handlers"
    echo "   MCP handlers should delegate to src/commands/ _json() functions"
    exit 1
fi
MCP_LOC=$(cat src/mcp/server/scry.rs src/mcp/server/assay.rs | wc -l | tr -d ' ')
if [ "$MCP_LOC" -gt 700 ]; then
    echo "   WARNING: MCP scry+assay = $MCP_LOC lines (target: <700)"
    echo "   Note: includes context/mother handlers (~150 LOC) that are not duplication targets"
fi
echo "   ✓ MCP handlers are thin ($MCP_LOC LOC, $SQL_IN_MCP SQL)"
echo ""

# Step 14: Schema consistency — canonical parses, installed matches, manifests agree
echo "📦 [14/14] Checking schema consistency..."
if ! cargo run --release --quiet -- schema check; then
    echo ""
    echo "❌ Schema consistency check failed!"
    exit 1
fi
echo ""

echo "✅ All checks passed! Ready to push."
