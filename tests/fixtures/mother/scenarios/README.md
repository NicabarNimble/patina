# Mother projection scenarios

Scenario names describe projection state, not child identity.

- `project-empty`: no manifests or HITL skill files.
- `project-installed`: manifest and projected HITL files match source.
- `project-stale`: manifest exists but source/projection hashes should diverge.
- `project-conflicted`: unmanaged HITL files collide with desired projection.
- `global-installed`: global/user scope projection baseline.
- `mixed-all`: multiple tuple states in one sandbox.
