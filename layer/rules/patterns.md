# Code Patterns

Development conventions for this project.

## Module Structure (Dependable Rust)

Keep public interface small and stable. Hide implementation in `internal.rs`.

```
module/
├── mod.rs          # External interface: docs + curated exports
└── internal.rs     # Implementation details (or internal/)
```

**Rules:**
- No `pub mod internal` (keeps internal private)
- No `internal::` in public function signatures
- Default to `pub(crate)` in internal.rs

See: `layer/core/dependable-rust.md`

## Error Handling

- Use `anyhow::Result` for CLI commands
- Use `thiserror` for library errors with semantic types
- Add context with `.with_context()` for actionable errors

```rust
fs::read_to_string(&path)
    .with_context(|| format!("Failed to read {}", path.display()))?;
```

## Naming

- `snake_case` for functions and variables
- `PascalCase` for types and traits
- `SCREAMING_CASE` for constants
- Modules named for what they **do** (verbs: `launch`, `scrape`, `oxidize`)

## Testing

- Unit tests colocated in module (`#[cfg(test)] mod tests`)
- Integration tests in `tests/`
- Run before push: `./resources/git/pre-push-checks.sh`

## Git Commits

- One commit = one purpose
- NO "Generated with Claude Code" attribution
- Format: `type(scope): description`
  - `feat`, `fix`, `docs`, `chore`, `refactor`, `test`

## Avoid

- Over-engineering beyond immediate requirements
- Adding features/flags not explicitly requested
- Python subprocess dependencies (pure Rust at runtime)
- Backwards-compatibility hacks for unused code
