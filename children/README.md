# Children Directory

This directory holds first-party Patina child components.

- Each child is a standalone crate with its own `Cargo.toml` and `child.toml`.
- `child.toml` is the canonical manifest surface (`[child]`, `kind`, and `[needs].toys`).
- Child kinds compose with WIT contracts in `wit/toys/` and `wit/worlds/`.

Current first-party children include runtime services like `ducklake`, `session-writer`, `spec-manager`, and support components like `doctor` and `belief-verifier`.

External child packages maintained outside this monorepo:

- Slate manager
  - local checkout: `/Users/nicabar/Projects/Patina/patina-child-slate`
  - GitHub: <https://github.com/NicabarNimble/patina-child-slate>
- Watcher system bundle
  - local checkout: `/Users/nicabar/Projects/Patina/patina-child-watcher-system`
  - GitHub: <https://github.com/NicabarNimble/patina-child-watcher-system>
  - children: `folder-watch-actor`, `watch-null-sink`

Work on external child behavior in those repos, publish/install release assets through Mother registry or install local builds with `patina child install`.

Use the project template in `sdk/template/` when scaffolding new child crates.
