---
type: value
id: dependable-rust
status: active
entrenchment: very-high
facets: [architecture, rust, module-pattern]
references: [unix-philosophy, adapter-pattern]
created: 2026-02-27
distilled_from: layer/core/dependable-rust.md
---
# Dependable Rust

Keep your public interface small and stable. Hide implementation details in private `internal` modules. Create black-box modules that can be completely rewritten internally without breaking callers.

## Test

Before creating a module, can you state what it does in one sentence? If not, split it.

## Consequence

Small interfaces are easy to review and evolve. When internals change, nothing outside the module knows or cares. Leaking implementation details into public API makes every internal change a breaking change.
