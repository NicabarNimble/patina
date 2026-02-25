---
type: belief
id: local-first-credential-isolation
persona: architect
facets: [secrets, architecture, wasm, local-first]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-23
revised: 2026-02-23
---

# local-first-credential-isolation

Local-first WASM credential isolation (no Docker, no PostgreSQL, machine-bound encryption, MCP-native) is the right architecture for a developer tool — lighter and more composable than server-dependent alternatives like IronClaw.

## Statement

Local-first WASM credential isolation (no Docker, no PostgreSQL, machine-bound encryption, MCP-native) is the right architecture for a developer tool — lighter and more composable than server-dependent alternatives like IronClaw.

## Evidence

- [[session-20260222-200024]]: IronClaw comparison — they require PostgreSQL for secret storage and Docker for full isolation. Patina uses age-encrypted files + SQLite + WASM boundary — zero infrastructure dependencies. (weight: 0.9)
- [[session-20260222-200024]]: Machine-bound encryption (IOPlatformUUID on macOS, /etc/machine-id on Linux) means copied encrypted files are useless on other hardware. IronClaw's PostgreSQL store has no hardware binding — database copy = secret copy. (weight: 0.85)
- [[session-20260222-200024]]: MCP protocol gives native tool discovery for Claude Code, OpenCode, Gemini CLI. IronClaw is coupled to their own agent runtime — no MCP, requires custom integration per LLM. (weight: 0.85)
- [[src/plugin/internal/host_support.rs]]: WASM plugins start in microseconds, use kilobytes of memory. Docker containers take seconds to spin up, megabytes of memory. For a developer tool running on a laptop, weight matters. (weight: 0.8)
- [[src/secrets/encrypted_file.rs]]: ChaCha20-Poly1305 + HKDF key derivation from hardware UUID. Works offline, no server process, no network dependency. (weight: 0.8)

## Supports

- [[wasm-host-boundary-hides-credentials]]: Local-first architecture achieves the same credential isolation as IronClaw's heavier Docker+PostgreSQL stack
- [[defense-in-depth-over-perfect-isolation]]: Each layer (vault encryption, WASM boundary, domain allowlist, leak detection) works independently — no orchestration server required
- [[storage-encryption-vs-runtime-isolation]]: Machine-bound encryption solves at-rest, WASM host boundary solves runtime — both local-first

## Attacks

- IronClaw's Docker layer provides additional process-level isolation beyond WASM. For the LLM threat model (prevent credential exposure, not contain malicious code execution), WASM alone is sufficient. Docker adds defense against plugin sandbox escapes — a real but lower-priority threat.

## Attacked-By

- IronClaw has richer credential features today: usage tracking, leak detection, OAuth flow management, per-secret key derivation. These are features, not architecture — can be added incrementally to the local-first model. Status: acknowledged gap, not a structural weakness.
- PostgreSQL enables team-wide secret sharing natively. Patina's file-based vault requires explicit recipient management (age encryption + recipients.txt). Status: different design choice for different audience (single developer vs team server).

## Applied-In

- `src/secrets/encrypted_file.rs` — Machine-bound encryption, zero infrastructure
- `src/secrets/storage.rs` — Dual-storage orchestrator, works offline
- `src/plugin/internal/host_support.rs` — WASM host boundary with domain allowlisting
- `src/mcp/server.rs` — MCP protocol, LLM-agnostic tool discovery

## Revision Log

- 2026-02-23: Created — metrics computed by `patina scrape`
