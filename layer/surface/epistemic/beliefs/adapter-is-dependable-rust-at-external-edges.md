---
type: belief
id: adapter-is-dependable-rust-at-external-edges
persona: architect
facets: [architecture, rust, core-principles, gjengset]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-04-03
revised: 2026-04-03
---

# adapter-is-dependable-rust-at-external-edges

The adapter pattern is dependable-rust applied specifically at external system boundaries. The trait is the public interface, the vendor implementation is the private internals. Same structure, different scope.

## Statement

The adapter pattern is dependable-rust applied specifically at external system boundaries. The trait is the public interface, the vendor implementation is the private internals. Same structure, different scope.

## Evidence

- [[session-20260403-070944-045859000]] - Rewriting adapter-pattern.md revealed the pattern is not independent of dependable-rust — it is the same black-box principle (small public interface, private internals) applied at the specific point where Patina touches systems it does not control. (weight: 0.9)

## Supports

- [[gjengset-lens-type-integrity]] — honest signatures and type integrity at the trait seam
- [[boundary-string-internal-enum]] — the adapter boundary is where strings enter and enums take over

## Attacks

## Attacked-By

- "Adapter pattern is independent, not a subset of dependable-rust" — the structural similarity is real but they have different triggers (external boundary vs internal complexity)

## Applied-In

- `src/interface/runtime/claude/mod.rs` + `internal/`: trait is public contract, Claude-specific logic is private — same layout as any dependable-rust module
- `src/embeddings/mod.rs`: `EmbeddingEngine` trait at ONNX boundary, `OnnxEmbedder` implementation behind it
- `src/retrieval/oracle.rs`: explicitly distinguishes strategy (internal) from adapter (external) — proves the boundary matters

## Revision Log

- 2026-04-03: Created — metrics computed by `patina scrape`
