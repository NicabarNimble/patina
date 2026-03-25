# Plugins Directory (Transitional)

This directory is a transitional compatibility surface during the plugin-to-child migration.

- Runtime vocabulary is child-first (`child.toml`, child kinds, toy grants).
- Some workspace members still live under `plugins/` for continuity while structure is converged.
- New doctrine/runtime work should prefer `children/` and child terminology.

If you are adding a new runtime component, place it under `children/` unless there is a documented compatibility reason to remain here.

Legacy note:

- `plugins/models` and `plugins/repos` are retired from active workspace membership as part of SDK `mother-child` retirement.
- Keep these directories as historical reference only unless a new spec reactivates them.
