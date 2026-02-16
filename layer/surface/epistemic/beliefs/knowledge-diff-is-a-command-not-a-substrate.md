---
type: belief
id: knowledge-diff-is-a-command-not-a-substrate
persona: architect
facets: [architecture, protocol, diff, beliefs]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-15
revised: 2026-02-15
---

# knowledge-diff-is-a-command-not-a-substrate

Knowledge diff ('what did we learn between v0.20 and v0.23?') is a command built on git2, not a filesystem substrate. Read belief files at two git tags, parse frontmatter, compute delta. The capability is real and missing — the architecture to deliver it is a single Rust command, not a content-addressed object store.

## Statement

Knowledge diff ('what did we learn between v0.20 and v0.23?') is a command built on git2, not a filesystem substrate. Read belief files at two git tags, parse frontmatter, compute delta. The capability is real and missing — the architecture to deliver it is a single Rust command, not a content-addressed object store.

## Evidence

- [[session-20260215-075638]]: Explored knowledge-protocol spec. Every proposed internal service (objects, refs, snapshot, diff) maps to existing git primitives except diff, which needs a parser on top of git2. The parser is ~200 lines of Rust, not a new filesystem layout.

## Supports

- [[git-is-the-knowledge-substrate]] — diff leverages git tags as snapshots, no custom refs needed
- [[unix-philosophy]] — one command, one job: parse beliefs at two git refs, output delta

## Attacks

<!-- none -->

## Attacked-By

<!-- none yet — but if git2 proves too heavy for this, a simpler approach may emerge -->

## Applied-In

- Not yet implemented — candidate for a standalone feat spec (`patina diff`)

## Revision Log

- 2026-02-15: Created — metrics computed by `patina scrape`
