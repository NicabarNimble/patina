#!/usr/bin/env bash
set -euo pipefail

SPECS=(
  "mother-lifecycle"
  "mother-child-toy-orchestration"
  "mother-source-graph-routing"
  "mother-secrets-session-coordination"
)

ROOT="layer/allium/mother"

for name in "${SPECS[@]}"; do
  spec="$ROOT/$name.allium"
  allium check "$spec" >/dev/null
  allium analyse "$spec" >/dev/null
  allium plan "$spec" >"$ROOT/$name.plan.json"
  echo "updated plan: $name"
done

echo "all mother allium plans refreshed"
