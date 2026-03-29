#!/usr/bin/env bash
set -euo pipefail

echo "Running broker integration invariants..."

# Deterministic broker invariants from src/mother/broker/mod.rs tests.
# These replace the retired local test-child wire-format gate.
cargo test -q -p patina-ai run_source_rejects_project_destination_path
cargo test -q -p patina-ai lake_route_fails_closed_without_oauth_credential
cargo test -q -p patina-ai lake_route_fails_closed_without_github_domain_scope

echo "ok: broker integration invariants passed"
