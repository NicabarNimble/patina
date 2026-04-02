#!/usr/bin/env bash
set -euo pipefail

echo "Running broker integration invariants..."

# Collapsed from 3 separate invocations into 1.
# Deterministic broker invariants from src/mother/broker/mod.rs tests.
cargo test -q -p patina-ai --lib -- \
    run_source_rejects_project_destination_path \
    lake_route_fails_closed_without_oauth_credential \
    lake_route_fails_closed_without_github_domain_scope

echo "ok: broker integration invariants passed"
