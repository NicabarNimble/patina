# Design: interface-setup-generalization

## Approach

- Keep one typed setup seam for the currently supported native code
  interfaces (`opencode`, `gemini`).
- Add `patina interface setup <name>` as the general project-local
  interface projection command.
- Preserve `patina ai setup <name>` as a compatibility alias by routing
  it through the same setup function.
- Move canonical root-interface text out of Rust string builders and
  into `resources/interfaces/code/`.
- Load those resource files from disk at runtime when available, with
  `include_str!` embedded fallback so installed binaries still work.
- Continue rendering capability truth in Rust so MCP/native fallback
  guidance stays typed and truthful.

## Commits
1. `refactor(interface): generalize interface setup`
   - add `patina interface setup`
   - keep `patina ai setup` as compatibility alias
   - load root code-interface assets from `resources/interfaces/code/`
2. `spec(interface): complete interface setup generalization`
   - mark the spec complete after verification

## Key Files
- `src/commands/interface/mod.rs`
  - public CLI surface for the generalized interface setup command
- `src/commands/interface/internal.rs`
  - typed setup implementation and tests for the new command path
- `src/commands/ai/mod.rs`
  - compatibility alias to the generalized setup path
- `src/adapters/launch.rs`
  - renders canonical `AGENTS.md` and vendor shims from file-backed
    interface assets
- `src/interface/internal/assets.rs`
  - runtime resource loading plus embedded fallback for interface assets
- `resources/interfaces/code/AGENTS.md`
  - canonical root code-interface template
- `resources/interfaces/code/CLAUDE.md`
  - Claude compatibility shim template
- `resources/interfaces/code/GEMINI.md`
  - Gemini compatibility shim template

## Open Questions

- Whether `patina interface` should grow beyond code/AI interfaces in the
  next slice, or whether a family subcommand such as
  `patina interface setup code <name>` is worth introducing later.
