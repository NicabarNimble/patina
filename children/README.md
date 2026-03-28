# Children Directory

This directory holds first-party Patina child components.

- Each child is a standalone crate with its own `Cargo.toml` and `child.toml`.
- `child.toml` is the canonical manifest surface (`[child]`, `kind`, and `[needs].toys`).
- Child kinds compose with WIT contracts in `wit/toys/` and `wit/worlds/`.

Current first-party children include runtime services like `ducklake`, `session-writer`, `spec-manager`, and support components like `doctor` and `belief-verifier`.

Use the project template in `children/template/` when scaffolding new child crates.
