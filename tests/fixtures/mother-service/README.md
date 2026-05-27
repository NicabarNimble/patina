# mother-service fixtures

Deterministic fixtures for Mother loader tests that prove backward-safe behavior for handle-based service children.

## Provenance

Preserved from the retired in-repo `belief-verifier` service child.

These fixtures are checked in so Mother loader tests continue to prove backward-safe
handling for legacy handle-based service children without keeping the obsolete source
crate in the workspace.
