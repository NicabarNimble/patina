#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo not found" >&2
    exit 1
fi

echo "Running DuckLake parity assertions..."

# Collapsed from 5 separate invocations into 1.
# Each invocation re-linked the ~700-test binary (~2 min each).
# Single invocation: link once, run 4 tests.
cargo test -q -p patina-ai --lib -- \
    migration_copies_legacy_cursor_into_per_type_lake_cursors \
    migration_is_idempotent_and_does_not_overwrite_existing_cursor \
    lake_route_fails_closed_without_oauth_credential \
    lake_route_fails_closed_without_github_domain_scope

echo "ok: DuckLake parity assertions passed"
