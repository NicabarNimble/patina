---
type: value
id: adapter-pattern
status: active
entrenchment: very-high
facets: [architecture, patterns, traits]
references: [dependable-rust, unix-philosophy]
created: 2026-02-27
distilled_from: layer/core/adapter-pattern.md
---
# Adapter Pattern

Use trait-based adapters to integrate with external systems without coupling core logic to any specific implementation. Commands use trait objects, never concrete adapter types.

## Test

Can you swap this external system for a mock in tests without changing the calling code? If not, you've coupled to the implementation.

## Consequence

New adapters are added without changing core. Each adapter optimizes for its system. Testing is clean with mocks. Swap Claude for Gemini without touching protocol code.
