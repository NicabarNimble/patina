---
type: belief
id: spec-is-a-directory
persona: architect
facets: [specs, architecture, llm-workflow]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-23
revised: 2026-02-23
---

# spec-is-a-directory

A spec is a directory, not a file — SPEC.md is the authority but supporting docs (walkthroughs, design, notes) live alongside it, letting the LLM read the overview first and drill into detail only when needed.

## Statement

A spec is a directory, not a file — SPEC.md is the authority but supporting docs (walkthroughs, design, notes) live alongside it, letting the LLM read the overview first and drill into detail only when needed.

## Evidence

- [[session-20260223-132543]]: walkthroughs.md created alongside SPEC.md for spec-workflow-rigor — spec was 900 lines, adding walkthroughs would have pushed past useful context. Supporting doc solved it. (weight: 0.9)
- [[session-20260223-132543]]: spec-workflow-rigor SPEC.md already references `design.md` for implementation detail — the directory structure is implicit in the spec's own design. (weight: 0.7)

## Supports

- [[plugins-are-three-prong-bundles]] — a plugin is a bundle (CLI + MCP + Skill), a spec is a bundle (SPEC.md + supporting docs). Same principle: the unit of delivery is a directory, not a file.
- [[dependable-rust]] — `mod.rs` is the public interface, `internal.rs` is the detail. SPEC.md is the interface, supporting docs are the detail.

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- `layer/surface/build/feat/spec-workflow-rigor/walkthroughs.md` — first supporting doc created alongside a spec, proving the pattern.

## Revision Log

- 2026-02-23: Created — metrics computed by `patina scrape`
