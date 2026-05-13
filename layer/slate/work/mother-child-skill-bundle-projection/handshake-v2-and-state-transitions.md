# Handshake v2 and State Transition Draft

Slate: `mother-child-skill-bundle-projection`
Allium anchor: `layer/allium/mother/mother-hitl-skill-lifecycle.allium`

## Handshake v2 goal

When a HITL starts or checks in, Mother can evaluate child skill projections without making the HITL responsible for child installation. The handshake reports deterministic per-tuple lifecycle state and, when policy allows, a safe auto-sync plan.

Tuple key:

```text
(child, hitl, scope)
```

State domain:

```text
absent | installed | stale | conflicted | unsupported | blocked | error
```

## Request schema draft

```jsonc
{
  "version": 2,
  "interface": {
    "name": "pi | claude | opencode | gemini",
    "runtime_version": "optional string",
    "project_root": "absolute path when project-scoped",
    "session_id": "optional string"
  },
  "skills_check": {
    "enabled": true,
    "scope_mode": "project | global | both",
    "children": ["optional child name filter"],
    "auto_sync": {
      "enabled": false,
      "allow_force": false,
      "max_changes": 0
    }
  },
  "runtime_observations": {
    "trusted_folder": "optional bool, e.g. Gemini workspace trust",
    "skills_enabled": "optional bool",
    "external_skills_enabled": "optional bool, e.g. OpenCode",
    "warnings": ["optional HITL-reported anomaly strings"]
  }
}
```

## Response schema draft

```jsonc
{
  "version": 2,
  "interface": {
    "name": "pi | claude | opencode | gemini",
    "project_root": "absolute path when known"
  },
  "capabilities": {
    "project_scope": true,
    "global_scope": true,
    "reload_hint": "none | restart | command",
    "reload_command": "optional string such as /skills reload"
  },
  "tuples": [
    {
      "child": "slate-manager",
      "hitl": "gemini",
      "scope": "project",
      "state": "installed",
      "pin_mode": "latest | pin",
      "pinned_child_hash": null,
      "child_hash": "sha256:...",
      "trusted": true,
      "managed_projection_hash": "sha256:...",
      "source_projection_hash": "sha256:...",
      "manifest_path": ".patina/mother/skills/gemini/project.lock.json",
      "projection_root": ".gemini/skills",
      "effective_roots": [".gemini/skills", ".agents/skills"],
      "conflict_reason": null,
      "last_error": null,
      "warnings": []
    }
  ],
  "plan": {
    "safe_to_apply": false,
    "actions": [
      {
        "kind": "sync | install | uninstall | none",
        "tuple": { "child": "slate-manager", "hitl": "gemini", "scope": "project" },
        "reason": "auto_sync_safe | absent | stale | conflict | unsupported | blocked",
        "writes": ["relative paths"],
        "removes": ["relative paths"],
        "requires_force": false
      }
    ]
  },
  "events": [
    {
      "kind": "tuple_evaluated | sync_planned | install_blocked | projection_changed",
      "tuple": { "child": "slate-manager", "hitl": "gemini", "scope": "project" },
      "message": "human-readable summary"
    }
  ]
}
```

## Evaluation order

Mother should evaluate each tuple in this order so state assignment is deterministic:

1. HITL/scope support from capability matrix.
2. Runtime gating observations, e.g. Gemini folder trust or admin-disabled skills.
3. Child source existence, assignment, hash, and trust.
4. Projection manifest presence.
5. Managed file existence and hash comparison.
6. Unmanaged collision scan.
7. Pin-mode evaluation.
8. Auto-sync planning only after all blocking/conflict checks pass.

## State meaning

| State | Meaning | Normal next command |
|---|---|---|
| `absent` | No Mother manifest/projection exists for the tuple. | `install` |
| `installed` | Manifest exists and managed projection hash matches source projection hash. | no-op/status |
| `stale` | Managed projection exists but source has changed or pinned child hash moved. | `sync` |
| `conflicted` | Desired projection collides with unmanaged or divergent files. | `sync --force` or `install --force` after review |
| `unsupported` | Capability matrix says requested scope/HITL combination cannot be projected. | choose another scope/HITL |
| `blocked` | Projection is structurally possible but currently prohibited by trust, assignment, admin/runtime gating, or untrusted child source. | resolve blocker |
| `error` | Evaluation or IO failed unexpectedly. | inspect `last_error`, retry |

## Command behavior contract

### `status`

- Reads child source metadata, capability matrix, projection manifests, and filesystem hashes.
- Performs no writes.
- Reports tuple state, projection root, manifest path, hashes, caveats, and suggested next action.

### `install`

- Default scope is `project`.
- `--global` selects global/user scope and must fail closed if unsupported.
- Allowed from `absent`; allowed from `conflicted` only as a retry path.
- Without `--force`, unmanaged collisions result in `conflicted` and zero writes.
- With `--force`, collisions may be overwritten only after the plan identifies exact paths.

### `sync`

- Intended for `stale` and `conflicted` recovery.
- Normal sync from `stale` applies when no collisions exist.
- Normal sync from `conflicted` is allowed, but if collisions remain, tuple stays `conflicted`.
- `--force` can overwrite planned collision paths.
- Second successful sync must be idempotent and return `installed`/no-op.

### `uninstall`

- Removes only manifest-tracked Mother-managed artifacts.
- Preserves unmanaged user files even under the same HITL root.
- Returns tuple to `absent` and clears managed hash/conflict/error fields.

## State transition table

| Current | Command / condition | Force? | Next | Notes |
|---|---|---:|---|---|
| any | scope unsupported | n/a | `unsupported` | fail closed, zero writes |
| any | child source untrusted | n/a | `blocked` | zero writes |
| any | project child not assigned | n/a | `blocked` | project scope only |
| `absent` | install, no collision | false | `installed` | write projection + manifest |
| `absent` | install, collision | false | `conflicted` | record collision, zero writes |
| `absent` | install, collision | true | `installed` | write exact forced plan |
| `installed` | source hash differs, latest pin | n/a | `stale` | evaluation transition |
| `installed` | source hash matches | n/a | `installed` | no-op |
| `stale` | sync, no collision | false | `installed` | update projection + manifest |
| `stale` | sync, collision | false | `conflicted` | zero writes |
| `stale` | sync, collision | true | `installed` | forced planned write |
| `conflicted` | install/sync, collision remains | false | `conflicted` | retry allowed, state persists |
| `conflicted` | install/sync with force | true | `installed` | clear conflict/error |
| `blocked` | uninstall tracked projection | n/a | `absent` | open question: admin-blocked removal policy |
| `unsupported` | uninstall tracked projection | n/a | `absent` | useful after matrix changes or HITL removal |
| `error` | uninstall tracked projection | n/a | `absent` | cleanup path |
| `installed`/`stale`/`conflicted` | uninstall | n/a | `absent` | manifest-tracked paths only |

## Open questions carried forward

1. Claude: should status expose the effective winner source per skill name when project and global/enterprise projections overlap?
2. Blocked cleanup: are all `blocked` tuples uninstallable, or should admin-imposed blocks require acknowledgement before local removal?
