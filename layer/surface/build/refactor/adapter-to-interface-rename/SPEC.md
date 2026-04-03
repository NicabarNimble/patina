---
type: refactor
id: adapter-to-interface-rename
status: complete
created: 2026-04-03
sessions:
  origin: 20260403-070944-045859000
beliefs:
  - "[[vocabulary-drift-compounds]]"
  - "[[adapter-is-dependable-rust-at-external-edges]]"
  - "[[core-principles-contain-blast-radius]]"
related:
  - src/interface/runtime/mod.rs
  - src/interface/runtime/claude/mod.rs
  - src/interface/runtime/gemini/mod.rs
  - src/interface/runtime/opencode/mod.rs
  - src/interface/mod.rs
  - src/interface/internal/bootstrap.rs
  - src/interface/internal/launcher.rs
  - src/interface/internal/bundle.rs
  - src/commands/adapter.rs
  - src/commands/interface/mod.rs
  - src/workspace/internal.rs
  - src/project/internal.rs
  - src/session/internal/artifact.rs
  - src/session/internal/projection.rs
  - src/session/mod.rs
  - src/main.rs
  - layer/core/adapter-pattern.md
exit_criteria:

  - id: air1-structs-renamed
    text: "ClaudeAdapter → ClaudeInterface, GeminiAdapter → GeminiInterface, OpenCodeAdapter → OpenCodeInterface. AdapterConfig → InterfaceConfig, AdaptersConfig → InterfacesConfig, AdapterEntry → InterfaceEntry, AdapterDetection → InterfaceDetection, AdapterManifest → InterfaceManifest. Deprecated re-exports added for ClaudeAdapter/GeminiAdapter/OpenCodeAdapter with #[deprecated(since = \"0.46.0\", note = \"use XxxInterface\")]."
    checked: true

  - id: air2-functions-renamed
    text: "ensure_adapter_bootstrap → ensure_interface_bootstrap, ensure_adapter_projection → ensure_interface_projection, launch_adapter_cli → launch_interface_cli, from_adapter_name → from_interface_name, update_adapter_files → update_interface_files. All call sites updated."
    checked: true

  - id: air3-cli-command-promoted
    text: "`patina interface` becomes the full command surface (list, default, check, add, remove, refresh, doctor). `patina adapter` becomes a deprecated alias that forwards to `patina interface` with a deprecation warning on first use."
    checked: true

  - id: air4-session-field-deprecated
    text: "ArtifactParticipant.adapter field remains as Option<String> for deserialization of old sessions. New sessions no longer emit adapter field. from_adapter_name renamed to from_interface_name."
    checked: true

  - id: air5-config-compat
    text: "TOML config continues to accept [adapter]/[adapters] as aliases for [interface]/[interfaces]. Serde rename/alias attributes preserved. No migration required for existing config files."
    checked: true

  - id: air6-cli-flag-compat
    text: "--adapter remains as alias for --interface global flag. No user-facing breakage."
    checked: true

  - id: air7-docs-updated
    text: "AGENTS.md vocabulary section updated. layer/core/adapter-pattern.md unchanged (it is the design pattern, not the AI interface domain). User-facing help text in CLI updated."
    checked: true

  - id: air8-compile-proof
    text: "cargo check --workspace -q passes. cargo test -q --lib passes. cargo test -q --tests passes."
    checked: true

  - id: air9-no-stale-adapter-refs
    text: "grep -r 'Adapter' src/ mother/ sdk/ returns only: (a) deprecated re-exports, (b) adapter-pattern references in docs/comments about the design pattern, (c) serde alias attributes for backward compat. No live non-deprecated code paths use Adapter as a type name for the AI interface domain."
    checked: true
---

# refactor: Adapter-to-Interface Rename

Rename "adapter" to "interface" everywhere it refers to the AI interface
domain (Claude, Gemini, OpenCode). The term "adapter" is correct for the
design pattern (trait boundaries at external edges) but wrong as a noun for
the AI tools themselves. The codebase already prefers "interface" in
user-facing surfaces (CLI flag, config sections, session fields) but
internally still uses "adapter" in struct names, function names, and the
primary CLI command.

This is the same pattern as child-rename: vocabulary drift where a name
outlived its concept.

## What does NOT change

- `layer/core/adapter-pattern.md` — this is the design pattern, not the domain
- `InterfaceProvider` trait name — already correct
- `InterfaceKind` enum — already correct
- `pub type Adapter = InterfaceKind` alias — deprecated, not removed (backward compat)
- TOML config aliases — preserved for backward compat
- `--adapter` CLI flag alias — preserved for backward compat
- `adapter` field in old session YAML — read but no longer written

## Scope boundary

This spec renames the AI interface domain only. It does not touch:
- Adapter pattern usage in other domains (connect providers, storage backends)
- Any trait definitions (they're already correctly named)
- WIT interfaces (different meaning of "interface" in component model)
