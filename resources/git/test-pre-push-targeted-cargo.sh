#!/usr/bin/env bash
# Deterministic behavior locks for Tier 2 impact classification.

set -euo pipefail

SCRIPT="resources/git/pre-push-targeted-cargo.sh"

assert_contains() {
    local haystack="$1"
    local needle="$2"
    local name="$3"
    if grep -Fq "$needle" <<<"$haystack"; then
        echo "  pass: $name"
    else
        echo "  FAIL: $name"
        echo "    missing: $needle"
        exit 1
    fi
}

echo "Case 1: docs-only changes skip cargo lane"
out=$(PATINA_PRE_PUSH_TEST_MODE=1 PATINA_PRE_PUSH_CHANGED_FILES=$'README.md\nlayer/core/values/unix-philosophy.md' bash "$SCRIPT")
assert_contains "$out" "No cargo-impacting files detected. Skipping cargo clippy/test lane." "docs-only skip message"
assert_contains "$out" "✅ Tier 2 targeted checks passed (cargo lane skipped; no cargo-impacting paths)." "docs-only success message"

echo ""
echo "Case 2: unresolved code path escalates fail-closed"
out=$(PATINA_PRE_PUSH_TEST_MODE=1 PATINA_PRE_PUSH_CHANGED_FILES=$'tools/custom.rs' bash "$SCRIPT")
assert_contains "$out" "Could not resolve impacted package set. Escalating to full workspace checks." "fail-closed escalation message"
assert_contains "$out" "[test-mode] cargo clippy --workspace -- -D warnings" "fail-closed runs clippy"
assert_contains "$out" "[test-mode] cargo test --workspace --lib" "fail-closed runs workspace tests"

echo ""
echo "Case 3: trigger-only schema path still runs guarded check"
out=$(PATINA_PRE_PUSH_TEST_MODE=1 PATINA_PRE_PUSH_CHANGED_FILES=$'resources/schemas/demo.json' bash "$SCRIPT")
assert_contains "$out" "No cargo-impacting files detected. Skipping cargo clippy/test lane." "trigger-only skips cargo lane"
assert_contains "$out" "📦 Path-triggered check: schema consistency" "trigger-only schema gate"
assert_contains "$out" "[test-mode] cargo run --release --quiet -- schema check" "trigger-only schema command"

echo ""
echo "All Tier 2 targeted cargo behavior locks passed."
