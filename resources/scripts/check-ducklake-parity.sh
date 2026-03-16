#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo not found" >&2
    exit 1
fi

echo "Running DuckLake parity assertions..."

cargo test -q -p patina-ai migration_copies_legacy_cursor_into_per_type_lake_cursors
cargo test -q -p patina-ai migration_is_idempotent_and_does_not_overwrite_existing_cursor
cargo test -q -p patina-ai task_dedupe_and_leasing_work
cargo test -q -p patina-ai lake_route_fails_closed_without_oauth_credential
cargo test -q -p patina-ai lake_route_fails_closed_without_github_domain_scope

echo "ok: DuckLake parity assertions passed"
