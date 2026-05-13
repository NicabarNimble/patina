# HITL Skill Capability Matrix

Slate: `mother-child-skill-bundle-projection`

Purpose: lock the per-HITL facts Mother needs before implementing `patina mother skills {status,install,sync,uninstall}`. Mother installs children in Mother scope; these rows describe only skill projection behavior for a `(child, hitl, scope)` tuple.

## Matrix

| HITL | Project scope | Global scope | Projection roots | Precedence / conflicts | Reload / refresh | Gating / caveats |
|---|---|---|---|---|---|---|
| PI | Supported | Supported | Project: `.pi/skills`, `.agents/skills`; Global: `~/.pi/agent/skills`, `~/.agents/skills` | Upstream precedence: project settings > project auto > user settings > user auto > package. Treat PI as its own HITL even if internals share OpenCode concepts. | PI-managed project files use marker-based updates for markdown/toml and direct executable writes in Patina interface bootstrap. | Current local environment has interface-template drift (`~/.patina/interfaces/pi` absent); status should report environment anomalies separately from tuple projection state. |
| Claude | Supported | Supported | Project: `.claude/skills`; Global/personal: `~/.claude/skills`; Enterprise also exists but is not Mother-managed. | Documented precedence: enterprise > personal/global > project. Skills can beat slash-command names. | Live reload supported for skill edits; creating a new top-level skills directory may require restart. | Closed-source runtime; carry user-reported visibility anomalies for sub-agent/project-vs-global behavior as status warnings, not as baseline model changes. |
| OpenCode | Supported | Supported | Native config roots: global `~/.config/opencode`, home `~/.opencode`, project/ancestor `.opencode`; skill dirs under `{skill,skills}/**/SKILL.md`; external `.agents/skills` plus optional `.claude/skills`; opt-in `skills.paths[]` and `skills.urls[]`. | Duplicate skill names warn and the last discovered entry wins. Later scan sources override earlier sources. | Loaded through instance discovery; safe operational assumption is restart/reload instance after Mother projection changes unless OpenCode exposes a first-class skill reload path. | Project config can be disabled by flags; external skill discovery can be disabled by flags; remote URL skills are pulled/cached and should not be Mother’s default projection mechanism. |
| Gemini CLI | Supported as `workspace` scope | Supported as `user` scope | Project/workspace: `.gemini/skills`, `.agents/skills`; Global/user: `~/.gemini/skills`, `~/.agents/skills`; extension-bundled skills under extension `skills/`. | Precedence: built-in < extension < user < workspace. Within user or workspace tier, `.agents/skills` overrides `.gemini/skills`. Same-name conflicts warn/override by higher precedence. | `/skills reload` and `/skills refresh` re-discover skills. Terminal commands `gemini skills install/link/uninstall` default to user scope and accept `--scope workspace`. | Workspace skills load only when the folder is trusted. Skills can be disabled by user settings and admin settings. Install/link overwrites destination directories after consent; Mother should use its own lock/manifest rather than relying on Gemini’s unmanaged overwrite semantics. |

## Mother projection policy derived from the matrix

1. **Project is default**: absence of `--global` means project/workspace projection.
2. **Global is explicit**: `--global` maps to HITL user/global scope and must be capability-gated.
3. **Mother-managed layout should avoid native same-name conflicts**: default projection path should be namespaced by child, e.g. `<hitl-root>/skills/<child>/<skill>/SKILL.md` where the HITL supports nested skill dirs.
4. **Native overwrite behavior is not Mother policy**: even when a HITL command overwrites existing skills, Mother lifecycle commands must fail closed on unmanaged collisions unless `--force` is explicit.
5. **Effective visibility is a status concern**: status should report both tuple state and notable runtime caveats such as Claude precedence anomalies, Gemini folder trust, or disabled OpenCode external/project discovery.
6. **Supply-chain trust remains child-level**: matrix rows govern projection drift; child hash/trust governs whether projection may occur.

## Proposed capability record shape

```rust
struct HitlSkillCapability {
    hitl: HitlKind,
    project: ScopeCapability,
    global: Option<ScopeCapability>,
    precedence: PrecedenceModel,
    reload: ReloadModel,
    conflict_policy: NativeConflictModel,
    caveats: &'static [&'static str],
}

struct ScopeCapability {
    scope: ProjectionScope,
    primary_root: PathTemplate,
    alias_roots: &'static [PathTemplate],
    supports_nested_skill_dirs: bool,
    requires_trust: bool,
    requires_runtime_enabled: &'static [&'static str],
}
```

## Implementation-facing notes

- The runtime matrix should be data, not scattered `match` logic.
- `status` should evaluate every selected `(child, hitl, scope)` tuple against this matrix and the projection manifest.
- `install` and `sync` should plan all writes first, detect collisions before writing, and apply atomically where practical.
- `uninstall` should remove only manifest-tracked Mother-managed files.
