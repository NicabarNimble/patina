#!/usr/bin/env bash
set -euo pipefail

echo "Checking retired MCP handler surface is not reintroduced..."

if [[ -d "src/mcp/server" ]]; then
    echo "error: src/mcp/server is retired and must not be reintroduced"
    exit 1
fi

if [[ -f "src/mcp/server/scry.rs" || -f "src/mcp/server/assay.rs" ]]; then
    echo "error: legacy MCP handler files (scry.rs/assay.rs) must not exist"
    exit 1
fi

if command -v rg >/dev/null 2>&1; then
    if ! rg -q "MCP server path has been retired" src/main.rs; then
        echo "error: missing retired MCP guard in src/main.rs"
        exit 1
    fi

    if ! rg -q "MCP server path has been retired|start daemon without --mcp" src/commands/mother/mod.rs; then
        echo "error: missing retired MCP guard in src/commands/mother/mod.rs"
        exit 1
    fi
else
    if ! grep -qE "MCP server path has been retired" src/main.rs; then
        echo "error: missing retired MCP guard in src/main.rs"
        exit 1
    fi

    if ! grep -qE "MCP server path has been retired|start daemon without --mcp" src/commands/mother/mod.rs; then
        echo "error: missing retired MCP guard in src/commands/mother/mod.rs"
        exit 1
    fi
fi

echo "ok: retired MCP handler invariants enforced"
