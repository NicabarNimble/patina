---
type: explore
id: llm-adapter-refactor
status: design
created: 2026-02-05
sessions:
  origin: 20260205-102402
related:
  - layer/surface/build/feat/skills-focused-adapter/SPEC.md
  - layer/surface/build/feat/patina-platform/SPEC.md
  - layer/core/dependable-rust.md
---

# explore: LLMAdapter Refactor

> Declarative manifest + generic operations = plugin-ready.

## Problem

`LLMAdapter` trait has 12 methods mixing concerns:

```rust
pub trait LLMAdapter {
    fn name(&self) -> &'static str;              // identity
    fn init_project(&self, ...) -> Result<()>;   // scaffolding
    fn post_init(&self, ...) -> Result<()>;      // scaffolding
    fn get_custom_commands(&self) -> Vec<...>;   // metadata
    fn get_context_file_path(&self, ...) -> PathBuf; // metadata
    fn check_for_updates(&self, ...) -> ...;     // versioning
    fn update_adapter_files(&self, ...) -> ...;  // versioning
    fn get_version_changes(&self, ...) -> ...;   // versioning
    fn get_changelog_since(&self, ...) -> ...;   // versioning
    fn get_sessions_path(&self, ...) -> ...;     // metadata
    fn version(&self) -> &'static str;           // identity
}
```

**Mixed concerns:**
- Identity/metadata (5 methods)
- Scaffolding (2 methods)
- Versioning/updates (4 methods)

**Result:** Each adapter reimplements similar logic. Not plugin-ready.

---

## Current Adapters

| Adapter | Context File | Special Logic |
|---------|--------------|---------------|
| Claude | `CLAUDE.md` | MCP config in `.claude/` |
| Gemini | `.gemini/config.yaml` | Different structure |
| OpenCode | `AGENTS.md` | Similar to Claude |

All do roughly the same thing: create config files from templates.

---

## Proposed Design

### Declarative Manifest

```rust
pub struct AdapterManifest {
    /// Adapter identity
    pub name: &'static str,
    pub version: &'static str,

    /// File structure
    pub context_file: &'static str,      // "CLAUDE.md"
    pub config_dir: Option<&'static str>, // ".claude/"
    pub sessions_dir: Option<&'static str>,

    /// Templates to create on init
    pub templates: &'static [Template],

    /// Custom commands this adapter provides
    pub commands: &'static [AdapterCommand],
}

pub struct Template {
    pub path: &'static str,      // ".claude/settings.json"
    pub content: &'static str,   // Template content or resource key
    pub overwrite: bool,         // Replace if exists?
}

pub struct AdapterCommand {
    pub name: &'static str,      // "session-start"
    pub description: &'static str,
}
```

### Minimal Trait

```rust
pub trait LLMAdapter: Send + Sync {
    /// Static manifest describing the adapter
    fn manifest(&self) -> &'static AdapterManifest;

    /// Launch the adapter (start Claude Code, Gemini CLI, etc.)
    fn launch(&self, project_path: &Path, args: &[String]) -> Result<Child>;
}
```

### Generic Operations

```rust
// Core provides these, not each adapter
impl dyn LLMAdapter {
    fn init(&self, project_path: &Path) -> Result<()> {
        let m = self.manifest();
        for template in m.templates {
            render_template(project_path, template)?;
        }
        Ok(())
    }

    fn check_updates(&self, project_path: &Path) -> Result<Option<UpdateInfo>> {
        let m = self.manifest();
        let installed = read_version_file(project_path, m.name)?;
        Ok(compare_versions(&installed, m.version))
    }

    fn update(&self, project_path: &Path) -> Result<()> {
        // Re-render templates that have changed
    }

    fn context_path(&self, project_path: &Path) -> PathBuf {
        project_path.join(self.manifest().context_file)
    }
}
```

---

## Migration

### Phase 1: Add Manifest

Add `AdapterManifest` alongside existing trait. Adapters implement both.

```rust
impl LLMAdapter for ClaudeAdapter {
    fn manifest(&self) -> &'static AdapterManifest {
        &CLAUDE_MANIFEST
    }

    // Keep old methods during transition
    fn name(&self) -> &'static str {
        self.manifest().name
    }
}
```

### Phase 2: Migrate Callers

Update callers to use manifest instead of individual methods.

### Phase 3: Remove Old Methods

Delete the 12 methods, keep only `manifest()` + `launch()`.

---

## Plugin Implications

With this design, a plugin adapter is trivial:

```rust
// Plugin just provides static data + launch logic
static MY_ADAPTER: AdapterManifest = AdapterManifest {
    name: "my-llm",
    version: "0.1.0",
    context_file: "MY_LLM.md",
    templates: &[
        Template { path: "MY_LLM.md", content: CONTEXT_TEMPLATE, overwrite: false },
    ],
    commands: &[],
    ..
};

fn launch(project_path: &Path, args: &[String]) -> Result<Child> {
    Command::new("my-llm-cli")
        .current_dir(project_path)
        .args(args)
        .spawn()
}
```

Core handles init, update, version checking generically.

---

## Questions

1. **Template storage:** Embed in binary or load from files?
   - Current: Embedded as `include_str!`
   - Plugin: Would need to be in the .wasm or fetched

2. **Launch mechanism:** Should `launch()` return a `Child` or abstract it?
   - Some adapters might be in-process (future local LLMs?)

3. **MCP config:** Claude needs MCP server config. Part of manifest or separate?

---

## Comparison to Other Traits

| Trait | Methods | Clean? | Notes |
|-------|---------|--------|-------|
| Oracle | 3 | ✅ | Already plugin-ready |
| EmbeddingEngine | 6 | ✅ | Already plugin-ready |
| ForgeReader | 7 | ✅ | Already plugin-ready |
| LLMAdapter (current) | 12 | ❌ | Mixed concerns |
| LLMAdapter (proposed) | 2 | ✅ | Manifest + launch |

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-05 | active | Created from trait audit during patina-platform design. Key insight: most adapter methods are reimplementing generic operations that should use declarative manifest. |
