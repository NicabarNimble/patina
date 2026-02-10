---
type: belief
id: specs-push-discoveries-outbound
persona: architect
facets: [governance, specs, knowledge-transfer]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-08
revised: 2026-02-08
---

# specs-push-discoveries-outbound

Discoveries made during one spec's execution that affect other specs must be pushed to the destination spec before the originating spec can close.

## Statement

Discoveries made during one spec's execution that affect other specs must be pushed to the destination spec before the originating spec can close.

## Evidence

- [[session-20260208-121343]]: Phase 2 cleanup of [[semantic-structural-split]] discovered ref repo ownership issues. The belief [[mother-owns-ref-repo-indexing]] existed but [[mother-v2]] had no reference to it. Archiving the originating spec would have severed the chain. (weight: 0.95)
- [[session-20260208-121343]]: Discovery lived in session logs (archived with originating spec), belief files (only if searched for), and commit messages (buried in history). None of these paths naturally surface when opening the destination spec. (weight: 0.9)
- [[session-20260208-121343]]: Human vigilance caught the gap ("once we archive, mother-v2 never sees it?"). In an agentic workflow, the AI won't ask that question unless the system requires it. (weight: 0.85)

## Supports

- [[spec-driven-design]]: SPECs are the authority for action. If a discovery doesn't reach the spec that needs it, that spec operates on incomplete information — violating the contract.
- [[dependable-rust]]: Black-box modules need stable interfaces. A spec's "interface" to other specs is its frontmatter links and discovery notes. Missing links are missing API.

## Attacks

- Overhead: requiring outbound discovery checks adds process to every spec closure. Could slow down completion.

## Attacked-By

- "Just search for it later" — beliefs and sessions are searchable, so the knowledge isn't truly lost. Counter: searchability requires knowing what to search for. The destination spec's author may not know a relevant discovery exists.

## Applied-In

- [[mother-v2]] SPEC: added [[mother-owns-ref-repo-indexing]] and [[corpus-composition-over-model]] beliefs, discovery notes in Phase 3 and Phase 6, and cross-link to [[semantic-structural-split]]. (2026-02-08)

## Implementation Direction

This belief should inform three layers:

1. **Process** — add a rule to [[spec-driven-design]]: "before a spec can close, push outbound discoveries to affected specs"
2. **Tooling** — `patina spec discover <target-spec> "note"` to append discovery notes and cross-links with minimal friction
3. **Structural** — a `discoveries` field in spec frontmatter tracking outbound findings; `patina doctor` or `patina spec check` warns about unresolved outbound discoveries at close time

## Revision Log

- 2026-02-08: Created from spec-to-spec knowledge transfer gap discussion
