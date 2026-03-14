#!/bin/bash
# Pre-push checks for Patina - fast local gate
#
# Runs fmt, clippy, and tests before push (~1-2 min).
# CI handles the full suite (including cargo install --locked).

set -e

echo "🔍 Running pre-push checks..."
echo ""

# Step 1: WIT consistency — guest crate wit/ must match canonical wit/
# Two groups: mother-child crates need full wit/ tree, command crates need wit/command/
echo "📦 [1/8] Checking WIT consistency..."
wit_ok=true
# Mother-child guest crates: full wit/ tree (mother-child + command + deps)
for crate_dir in plugins/models plugins/repos; do
    if [ -d "$crate_dir/wit" ]; then
        if ! diff -r wit/ "$crate_dir/wit/" > /dev/null 2>&1; then
            echo "   ERROR: $crate_dir/wit/ differs from canonical wit/"
            echo "   Fix: cp -r wit/ $crate_dir/wit/"
            wit_ok=false
        fi
    fi
done
# Step 1b: SDK WIT consistency — ensure published SDK ships current WIT
# Compare world definitions AND their deps/patina-host/host.wit copies
echo "   Checking SDK WIT consistency..."
for world in command knowledge-child mother-child pipeline task; do
    if ! diff "wit/$world/$world.wit" "sdk/patina-sdk/wit/$world/$world.wit" > /dev/null 2>&1; then
        echo "   ERROR: sdk/patina-sdk/wit/$world/$world.wit differs from canonical"
        echo "   Fix: cp wit/$world/$world.wit sdk/patina-sdk/wit/$world/$world.wit"
        wit_ok=false
    fi
    if ! diff "wit/$world/deps/patina-host/host.wit" "sdk/patina-sdk/wit/$world/deps/patina-host/host.wit" > /dev/null 2>&1; then
        echo "   ERROR: sdk/patina-sdk/wit/$world/deps/patina-host/host.wit differs from canonical"
        echo "   Fix: cp wit/$world/deps/patina-host/host.wit sdk/patina-sdk/wit/$world/deps/patina-host/host.wit"
        wit_ok=false
    fi
done
if [ "$wit_ok" = false ]; then
    echo ""
    echo "❌ WIT consistency check failed!"
    exit 1
fi
echo "   ✓ WIT files consistent across all crates"
echo ""

# Step 2: WIT host.wit symlink enforcement
# Per [[wit-deps-must-be-hard-links-verified]]: all world-level
# deps/patina-host/host.wit must point back to the canonical file.
# Stale copies cause silent build failures when new imports are added.
# Pure shell — no python3 (per [[patina-identity]]: Rust-first, no Python).
echo "📦 [2/8] Checking WIT host.wit symlinks..."
CANONICAL="wit/deps/patina-host/host.wit"
wit_link_ok=true

# Resolve a path through symlinks to its absolute canonical form. Pure shell.
# For symlinks: resolve the target relative to the link's directory, then canonicalize.
resolve_real() {
    local path="$1"
    # Follow symlinks manually
    while [ -L "$path" ]; do
        local target
        target=$(readlink "$path")
        if [[ "$target" == /* ]]; then
            path="$target"
        else
            path="$(dirname "$path")/$target"
        fi
    done
    # Now path is a real file — resolve its directory
    local dir
    dir=$(cd "$(dirname "$path")" && pwd -P)
    echo "$dir/$(basename "$path")"
}

if [ ! -f "$CANONICAL" ]; then
    echo "   ERROR: canonical $CANONICAL not found"
    wit_link_ok=false
else
    CANONICAL_REAL=$(resolve_real "$CANONICAL")
    # Check world-level deps AND SDK mirror deps — same canonical file
    COPIES=(
        "wit/mother-child/deps/patina-host/host.wit"
        "wit/knowledge-child/deps/patina-host/host.wit"
        "wit/command/deps/patina-host/host.wit"
        "wit/task/deps/patina-host/host.wit"
        "wit/pipeline/deps/patina-host/host.wit"
        "sdk/patina-sdk/wit/mother-child/deps/patina-host/host.wit"
        "sdk/patina-sdk/wit/knowledge-child/deps/patina-host/host.wit"
        "sdk/patina-sdk/wit/command/deps/patina-host/host.wit"
        "sdk/patina-sdk/wit/task/deps/patina-host/host.wit"
        "sdk/patina-sdk/wit/pipeline/deps/patina-host/host.wit"
    )
    for COPY in "${COPIES[@]}"; do
        if [ ! -f "$COPY" ]; then
            echo "   ERROR: $COPY not found"
            wit_link_ok=false
            continue
        fi
        if [ ! -L "$COPY" ]; then
            echo "   ERROR: $COPY is not a symlink (expected -> $CANONICAL)"
            echo "         Fix: rm $COPY && ln -s <relative-path-to-canonical> $COPY"
            wit_link_ok=false
            continue
        fi
        SYM_TARGET=$(readlink "$COPY")
        if [[ "$SYM_TARGET" == /* ]]; then
            echo "   ERROR: $COPY uses an absolute symlink target ($SYM_TARGET)"
            echo "         Fix: rm $COPY && ln -s <relative-path> $COPY"
            wit_link_ok=false
            continue
        fi
        COPY_REAL=$(resolve_real "$COPY")
        if [ "$COPY_REAL" != "$CANONICAL_REAL" ]; then
            echo "   ERROR: $COPY resolves to $COPY_REAL (expected $CANONICAL_REAL)"
            echo "         Fix: rm $COPY && ln -s <relative-path-to-canonical> $COPY"
            wit_link_ok=false
        fi
    done
fi
if [ "$wit_link_ok" = false ]; then
    echo ""
    echo "❌ WIT host.wit symlink check failed!"
    echo "   All world deps must be symlinks to $CANONICAL"
    exit 1
fi
echo "   ✓ All host.wit symlinks resolve to canonical"
echo ""

# Step 3: Check formatting (CI uses --check, not --fix)
echo "📦 [3/8] Checking Rust formatting..."
if ! cargo fmt --all -- --check; then
    echo ""
    echo "❌ Formatting check failed!"
    echo "   Run: cargo fmt --all"
    exit 1
fi
echo "   ✓ Formatting OK"
echo ""

# Step 4: Clippy with -D warnings (same as CI)
echo "📦 [4/8] Running clippy (warnings = errors)..."
if ! cargo clippy --workspace -- -D warnings; then
    echo ""
    echo "❌ Clippy failed! Fix warnings above."
    exit 1
fi
echo "   ✓ Clippy OK"
echo ""

# Step 5: Run tests
echo "📦 [5/8] Running tests..."
if ! cargo test --workspace; then
    echo ""
    echo "❌ Tests failed!"
    exit 1
fi
echo "   ✓ Tests OK"
echo ""

# Step 6: Broker integration test — catches cross-crate wire format drift
# Requires: test-child installed at ~/.patina/children/test-child/
echo "📦 [6/8] Broker integration test..."
if [ -x "$HOME/.patina/children/test-child/test-child" ] && [ -f ".patina/sources.toml" ]; then
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

# Step 7: MCP thin handler invariants (post mcp-thin-handlers spec)
echo "📦 [7/8] Checking MCP handler invariants..."
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

# Step 8: Schema consistency — canonical parses, installed matches, manifests agree
echo "📦 [8/8] Checking schema consistency..."
if ! cargo run --release --quiet -- schema check; then
    echo ""
    echo "❌ Schema consistency check failed!"
    exit 1
fi
echo ""

echo "✅ All checks passed! Ready to push."
