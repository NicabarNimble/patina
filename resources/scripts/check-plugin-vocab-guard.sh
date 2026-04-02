#!/usr/bin/env bash
set -euo pipefail

echo "Checking plugin-vocabulary guardrails..."

if rg -n "resources/templates/plugin" src/child/scaffold.rs >/dev/null 2>&1; then
    echo "error: scaffold still references resources/templates/plugin"
    exit 1
fi

if rg -n "register_mother_child!|kind = \"mother-child\"|features = \[\"mother-child\"\]" resources/templates/child >/dev/null 2>&1; then
    echo "error: retired mother-child template vocabulary found under resources/templates/child"
    exit 1
fi

if rg -n "enum PluginCommands|Some\(Commands::Plugin|Manage WASM plugins" src/main.rs >/dev/null 2>&1; then
    echo "error: legacy plugin command surface reintroduced in src/main.rs"
    exit 1
fi

if ! rg -n "name = \"child\", visible_alias = \"plugin\"" src/main.rs >/dev/null 2>&1; then
    echo "error: canonical child command alias contract missing in src/main.rs"
    exit 1
fi

echo "ok: plugin-vocabulary guardrails passed"
