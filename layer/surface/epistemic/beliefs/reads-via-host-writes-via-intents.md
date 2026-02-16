---
type: belief
id: reads-via-host-writes-via-intents
persona: architect
facets: [architecture, plugins, wit]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-14
revised: 2026-02-14
---

# reads-via-host-writes-via-intents

Plugin reads are synchronous host calls (safe, typed, immediate return); plugin writes are returned as intents (host validates, audits, executes) — keeping decision logic in the plugin while destructive execution stays in the host.

## Statement

Plugin reads are synchronous host calls (safe, typed, immediate return); plugin writes are returned as intents (host validates, audits, executes) — keeping decision logic in the plugin while destructive execution stays in the host.

## Evidence

- [[session-20260214-110957]]: [[plugin-extraction-map]] Section 8-9 — toy pattern (mother-child + task worlds) proves intent-based writes work; same pattern recommended for git operations via host/git-read + git-action intents (weight: 0.9)

## Supports

- [[two-layer-capability-grants]] — manifest declares what reads/writes a plugin wants, host decides what to allow; this belief specifies the mechanism (sync calls for reads, intents for writes)
- [[wasi-sandboxed-filesystem]] — plugins can't escape sandbox; host-mediated intents are the safe execution path for writes
- [[separate-worlds-for-isolation]] — read-only worlds (command) import git-read only; action worlds (task) also export git-action intents

## Attacks

<!-- None identified -->

## Attacked-By

- Latency cost: intent-based writes add a round-trip (plugin returns intents → host executes → results not available to plugin). Counter: write operations are terminal (tag, commit) — plugins don't need the result to continue logic.
- Complexity: two patterns (sync reads + deferred writes) vs one (all sync). Counter: the split maps to the natural safety boundary — reads are safe, writes need mediation.

## Applied-In

- `wit/task/task.wit` — `toys()` export returns `list<toy>` intents for host execution (existing proof of intent pattern)
- `wit/mother-child/mother-child.wit` — `tick()` returns `list<toy>` (same pattern in daemon context)
- [[plugin-extraction-map]] Section 9 — recommended `git-actions()` export following `toys()` pattern

## Revision Log

- 2026-02-14: Created — metrics computed by `patina scrape`
