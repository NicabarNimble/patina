---
type: feat
id: v1-release
status: in_progress
created: 2026-01-27
updated: 2026-01-29
sessions:
  origin: 20260127-085434
  work: [20260129-074742]
related:
  - spec/go-public
  - spec-epistemic-layer
  - spec-mother
milestones:
  - version: "0.9.1"
    name: Version & spec system alignment
    status: in_progress
  - version: "0.9.2"
    name: Epistemic E4 (belief automation)
    status: pending
  - version: "0.9.3"
    name: Mother federated query
    status: pending
  - version: "0.9.4"
    name: Dynamic ONNX loading
    status: pending
  - version: "0.9.5"
    name: WASM grammars
    status: pending
  - version: "0.9.6"
    name: GitHub releases + Homebrew
    status: pending
  - version: "1.0.0"
    name: All pillars complete
    status: pending
current_milestone: "0.9.1"
---

# feat: v1.0 Release

> Finalize Patina's core architecture: epistemic beliefs, federated mother, and modular distribution.

**Goal:** A stable foundation that enables proper iteration. v1.0 means the three pillars are complete and the system can evolve without architectural rewrites.

---

## Three Pillars to v1.0

| Pillar | Current State | Finalized Means |
|--------|---------------|-----------------|
| **Epistemic Layer** | E0-E3 done, 35 beliefs indexed | E4 automation, validation stable, beliefs queryable |
| **Mother** | Registry works, `serve` daemon exists | Federated query across repos, persona fusion |
| **Distribution** | 52MB fat binary, source-only | Slim binary, `patina setup`, Homebrew tap |

All three must be complete for v1.0.

---

## Versioning Strategy

**Model:** Semver patches from 0.9.0 → 1.0.0

```
0.9.0  - Current (public release, fat binary)
0.9.1  - Version system fixed, spec-system aligned
0.9.2  - Epistemic E4 (belief extraction automation)
0.9.3  - Mother federated query
0.9.4  - Dynamic ONNX loading
0.9.5  - WASM grammars
0.9.6  - GitHub releases + Homebrew
1.0.0  - All pillars complete
```

Each patch = one meaningful milestone toward a pillar.

---

## Pillar 1: Epistemic Layer

**Spec:** [[spec-epistemic-layer.md]]

**Current:** E0-E3 complete. 35 beliefs captured and indexed in scry. Queryable via `patina scry "what do we believe about X"`.

**Remaining:**
- E4: Belief extraction automation (suggest beliefs from session patterns)
- Validation stability (confidence signals, revision workflow)

**Exit criteria:**
- [ ] `patina` suggests beliefs from session content
- [ ] Belief confidence updates based on evidence accumulation
- [ ] Belief query integrated into MCP tools

---

## Pillar 2: Mother (Federated Query)

**Spec:** [[spec-mother.md]]

**Current:** Registry works, `patina serve` daemon exists, ref repos indexed.

**Remaining:**
- Federated query across multiple repos
- Persona fusion (cross-project learning)
- Vocabulary bridging between repos

**Exit criteria:**
- [ ] `patina scry` queries mother registry (not just local project)
- [ ] Results ranked by relevance across repos
- [ ] Persona preferences influence retrieval

---

## Pillar 3: Distribution

**Current:** 52MB binary with everything baked in. Install requires building from source.

**Target:** Slim binary (~5-10MB), heavy assets download on demand.

### Packaging Architecture

| Asset | Current | Target |
|-------|---------|--------|
| Tree-sitter grammars | Compiled C (~10-15MB) | WASM, downloaded |
| ONNX Runtime | Static link (~15-20MB) | Dynamic `.dylib`, downloaded |
| Embedding models | Downloaded (existing) | Same |

### Runtime Asset Management

```
patina (slim binary)
├── patina doctor        → checks what's installed, what's missing
├── patina setup         → downloads all runtime assets
│   ├── grammars/*.wasm     (~10MB, tree-sitter WASM)
│   ├── libonnxruntime.dylib (~15MB)
│   └── models/*.onnx       (~30-90MB, existing flow)
└── ~/.patina/lib/       → runtime assets directory
```

### WASM Grammars

Replace compiled-in C grammars with tree-sitter WASM modules loaded at runtime.

**Why WASM:**
- Portable across architectures (same .wasm on arm64 and x86_64)
- Sandboxed execution
- Tree-sitter has native WASM support

**Trade-off:** Slower parsing than native C. Acceptable for scraping (not real-time editing).

### ONNX Runtime

Currently statically linked via `ort` crate's `download-binaries` feature.

**Target:** Download `libonnxruntime.dylib` on demand, load via `ORT_DYLIB_PATH`.

### Distribution Channels

**GitHub Releases (primary):**
- Release workflow on version tags (`v*`)
- macOS arm64 binary
- Stripped with release profile optimizations

**Homebrew Tap:**
- Separate repo: `NicabarNimble/homebrew-tap`
- Install: `brew install NicabarNimble/tap/patina`

**Exit criteria:**
- [ ] WASM grammar loading replaces compiled-in C grammars
- [ ] ONNX Runtime loaded dynamically
- [ ] `patina setup` downloads all runtime assets
- [ ] `patina doctor` reports asset status
- [ ] GitHub release workflow produces macOS arm64 binary
- [ ] Homebrew tap formula works
- [ ] Binary under 15MB (stripped, before compression)

---

## Immediate Next: 0.9.1

Fix the foundation before building on it.

**Done:**
- [x] `patina version show` reflects actual version (0.9.0)
- [x] Version tracking reads from Cargo.toml (sole source of truth)
- [x] build.md updated with v1.0 pillar roadmap
- [x] Removed stale `.patina/version.toml`

**Version system hardening (done):**
- [x] Single active milestone: warns if multiple specs have current_milestone
- [x] Coherence check: warns if spec milestone version <= Cargo.toml version
- [x] Silent failures: distinct messages for no DB, query error, no milestones
- [x] Deprecate `version phase` and `version init` commands (warn + doc)
- [x] Remove dead code: `get_spec_milestones()` function deleted

---

## Remaining Work (Next Session)

### 1. Index Staleness: Scrape Prunes Deleted Specs

**Problem:** When specs are archived/deleted, their entries linger in `patina.db`. We had to manually `DELETE FROM patterns WHERE id = 'go-public'` to clean up.

**Where to fix:** `src/indexer/` — the scrape layer command should:
- Track which files were processed
- Delete DB entries for files that no longer exist
- Or: add `--prune` flag to explicitly clean stale entries

**Context:** See session 20260129-074742 where go-public was manually cleaned from index.

### 2. YAML Parser for Spec Updates (Low Priority)

**Problem:** `update_spec_milestone()` in `src/commands/version/internal.rs:362` uses regex to modify YAML frontmatter. Fragile if formatting changes.

**Where to fix:** Replace regex with proper YAML parsing (e.g., `serde_yaml` or `yaml-rust`).

**Why low priority:** Works today, only breaks if spec format changes significantly.

### 3. Spec-System Folder Migration

**Problem:** Old flat-file specs (`spec-*.md`) need migration to folder format (`feat/name/SPEC.md`).

**Where:** `layer/surface/build/` — see `refactor/spec-system/SPEC.md` for the format.

**Scope:** ~10-15 specs to migrate, mostly in `deferred/`.

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-01-27 | in_progress | Spec created. Current binary 52MB, 14MB compressed. |
| 2026-01-29 | in_progress | Restructured as three-pillar roadmap. Patch versioning (0.9.x → 1.0.0). |
| 2026-01-29 | in_progress | Version system hardened: multi-milestone warning, coherence check, deprecation warnings, dead code removed. |
