# Slate Manager Skills

Slate manager exposes child-owned skill packages. Mother should eventually broker these packages for active children through a command/help surface such as:

```bash
patina mother skills list
patina mother skills show slate-manager
patina mother skills help slate-manager slate-code
```

Current packages:

- `slate-code/` — create/reuse/promote Slate work before non-trivial Patina code changes.
- `slate-version-control/` — git/archive/version-boundary rules for Slate work.

Project-local `.pi/skills/*` files may mirror or route to these packages until Mother has first-class child skill discovery.
