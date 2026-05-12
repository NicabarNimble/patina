# Children Directory

This directory holds first-party Patina child components.

- Each child is a standalone crate with its own `Cargo.toml` and `child.toml`.
- `child.toml` is the canonical manifest surface (`[child]`, `kind`, and `[needs].toys`).
- Child kinds compose with WIT contracts in `wit/toys/` and `wit/worlds/`.

Current first-party children include runtime services like `ducklake`, `session-writer`, `spec-manager`, and support components like `doctor` and `belief-verifier`.

Slate is being extracted as an app-like public child package at:

- local checkout: `/Users/nicabar/Projects/Patina/patina-child-slate`
- GitHub: <https://github.com/NicabarNimble/patina-child-slate>

Keep the in-tree `children/slate-manager` copy only as a compatibility/development mirror until Patina's workspace and tests consume installed or registry-provided children cleanly.

Use the project template in `sdk/template/` when scaffolding new child crates.
