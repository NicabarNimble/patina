---
type: belief
id: git-tags-must-be-real-or-not-claimed
persona: architect
facets: [sessions, git, integrity]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-20
revised: 2026-03-20
---

# git-tags-must-be-real-or-not-claimed

Every git tag name written to session frontmatter must correspond to an actual git tag — writing a tag name without creating the tag is a lie that breaks tooling and auditability

## Statement

Every git tag name written to session frontmatter must correspond to an actual git tag — writing a tag name without creating the tag is a lie that breaks tooling and auditability

## Evidence

- [[session-20260320-075256-088035000]] - audit found 11 sessions with end_tag fields like -tmux-lost and -superseded that have no corresponding git tags; only clean /session-end creates real end tags (weight: 1.0)

## Supports

- [[durability-lives-outside-interface-process]] — the WASM child owning end tag creation ensures tags are real even on crash

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Performance: creating git tags on crash paths adds I/O when the system may be in a degraded state. Mitigated by the child being a separate process from the dying interface.

## Applied-In

- [[spec-interface-session-model]] — Thread 5: Git tag integrity
- Bug identified: `src/interface/internal/checkin.rs` tmux reconciliation writes `end_tag` to frontmatter without `git tag` call for `-tmux-lost` and `-superseded` end states

## Revision Log

- 2026-03-20: Created — metrics computed by `patina scrape`
