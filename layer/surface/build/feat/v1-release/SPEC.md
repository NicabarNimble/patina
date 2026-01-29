---
type: feat
id: v1-release
status: in_progress
created: 2026-01-27
updated: 2026-01-29
sessions:
  origin: 20260127-085434
  work:
  - 20260129-074742
  - 20260129-093857
related:
- spec/go-public
- spec-epistemic-layer
- spec-mother
milestones:
- version: 0.9.1
  name: Version & spec system alignment
  status: complete
- version: 0.9.2
  name: Session system & adapter parity
  status: in_progress
- version: 0.9.3
  name: Epistemic E4 (belief automation)
  status: pending
- version: 0.9.4
  name: Mother federated query
  status: pending
- version: 0.9.5
  name: Dynamic ONNX loading
  status: pending
- version: 0.9.6
  name: WASM grammars
  status: pending
- version: 0.9.7
  name: GitHub releases + Homebrew
  status: pending
- version: 1.0.0
  name: All pillars complete
  status: pending
current_milestone: 0.9.2
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
0.9.1  ✓ Version system fixed, spec-system aligned
0.9.2  → Session system & adapter parity
0.9.3  - Epistemic E4 (belief extraction automation)
0.9.4  - Mother federated query
0.9.5  - Dynamic ONNX loading
0.9.6  - WASM grammars
0.9.7  - GitHub releases + Homebrew
1.0.0  - All pillars complete
```

Each patch = one meaningful milestone toward a pillar.

**Principle:** All three adapter LLMs must have the same level of excellence.

---

## Immediate Next: 0.9.2 — Session System & Adapter Parity

Two problems, one solution: move session logic from shell scripts into Patina, making sessions adapter-agnostic and event-first.

### Problem 1: Sessions Are Shell Scripts

Current session workflow:
```
/session-start  → .claude/bin/session-start.sh (216 lines of bash)
/session-update → .claude/bin/session-update.sh (136 lines of bash)
/session-end    → .claude/bin/session-end.sh (289 lines of bash)
```

Issues:
- Logic lives in bash (hard to test, hard to extend)
- Each adapter needs its own copy of scripts
- Events created after-the-fact by scraping markdown
- No `patina session` command exists

### Problem 2: Sessions Don't Link to Specs

Sessions and specs are both core to how we work, but they're not connected:
- Sessions have git linkage (tags → commits via timestamp matching)
- Specs have `sessions:` field but it's manually maintained
- Can't easily query "what sessions worked on v1-release?"

### Solution: Event-First Sessions

**Architecture principle:** Events are immutable truth, markdown is derived view.

```
User action (patina session start)
         │
         ▼
┌────────────────────────────────────────┐
│ EVENTLOG (immutable, append-only)      │
│                                        │
│  session.started {                     │
│    id: "20260129-093857",              │
│    title: "Complete v0.9.2",           │
│    spec_id: "v1-release",    ← NEW     │
│    milestone: "0.9.2",       ← NEW     │
│    branch: "patina",                   │
│    start_tag: "session-...-start"      │
│  }                                     │
└────────────────────────────────────────┘
         │
         │ materialize
         ▼
┌────────────────────────────────────────┐
│ layer/sessions/20260129-093857.md      │
│ (human-readable view, regeneratable)   │
└────────────────────────────────────────┘
```

### Deliverables

**1. `patina session` commands (Rust)**

```bash
# Start a session, link to spec
patina session start "complete 0.9.2" --spec v1-release --milestone 0.9.2
  → Creates session.started event
  → Creates git tag
  → Outputs session ID and context

# Update with progress
patina session update
  → Creates session.update event with git metrics
  → Returns structured data for LLM to fill in

# Add observation
patina session note "discovered edge case in parser"
  → Creates session.observation event

# End session
patina session end
  → Creates session.ended event with final metrics
  → Creates git tag
  → Materializes markdown to layer/sessions/
  → Updates spec's sessions.work array (if linked)

# Query sessions
patina session list
patina session show 20260129-093857
patina session show --active
```

**2. Session document format (YAML frontmatter)**

```yaml
---
type: session
id: "20260129-093857"
title: Complete v0.9.2
status: active              # active | archived
llm: claude
created: 2026-01-29T14:38:57Z
git:
  branch: patina
  starting_commit: 81d9e6b1
  start_tag: session-20260129-093857-claude-start
links:
  previous: 20260129-084757
  spec: v1-release          # ← explicit spec linkage
  milestone: 0.9.2
goals:
  - text: Complete v0.9.2
    done: false
---

## Previous Session Context
...

## Activity Log
...
```

**3. Active session location**

Move from `.claude/context/active-session.md` (adapter-specific) to `.patina/active-session.md` (patina-owned).

- Adapters call `patina session` commands
- No adapter-specific session logic
- Shell scripts become thin wrappers or disappear

**4. Spec ↔ Session bidirectional links**

Session events include `spec_id` and `milestone`:
```json
{"event_type": "session.started", "data": {"spec_id": "v1-release", "milestone": "0.9.2"}}
```

On session end, auto-update spec's sessions array:
```yaml
sessions:
  origin: 20260127-085434
  work:
    - 20260129-074742
    - 20260129-093857  ← auto-added
```

**5. Adapter version detection (secondary)**

Keep existing `launch.rs` detection, add `patina adapter status`:
```bash
patina adapter status
# Claude Code: 1.0.17 (installed)
# OpenCode: not installed
# Gemini CLI: 0.5.2 (installed)
```

Remove static `CLAUDE_ADAPTER_VERSION` constants.

### Migration Path

**Phase 1: Add commands (non-breaking)**
- Implement `patina session start/update/note/end`
- Events created in real-time
- Shell scripts still work (parallel paths)

**Phase 2: Update adapters**
- Adapter skills call `patina session` commands
- Shell scripts become one-liners or removed
- Active session moves to `.patina/`

**Phase 3: Remove legacy**
- Delete shell scripts
- Remove `CLAUDE_ADAPTER_VERSION` constants
- Update scrape to handle both old and new session formats

### Exit Criteria

- [ ] `patina session start/update/note/end` commands exist
- [ ] Session events created at action time (not scraped after)
- [ ] `--spec` and `--milestone` flags link sessions to specs
- [ ] Active session lives in `.patina/active-session.md`
- [ ] YAML frontmatter format for session documents
- [ ] Shell scripts deprecated or removed
- [ ] `patina adapter status` shows installed tool versions
- [ ] Spec's `sessions.work` auto-updates on session end

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

## Completed: 0.9.1 — Version & Spec System

**Version system hardening:**
- [x] `patina version show` reflects actual version from Cargo.toml
- [x] Single active milestone: warns if multiple specs have current_milestone
- [x] Coherence check: warns if spec milestone version <= Cargo.toml version
- [x] Silent failures: distinct messages for no DB, query error, no milestones
- [x] Deprecate `version phase` and `version init` commands
- [x] Removed stale `.patina/version.toml`

**Spec system cleanup:**
- [x] Auto-pruning in `scrape layer` and `scrape beliefs`
- [x] YAML parser (serde_yaml) replaces regex for spec updates
- [x] 11 flat-file specs migrated to folder format
- [x] Fixed prune bug (file stem vs frontmatter ID)
- [x] Cleaned stale VERSION_CHANGES in Claude adapter

---

## Historical: 0.9.1 Implementation Details

### 1. Index Staleness: Scrape Prunes Deleted Specs — DONE

**Problem:** When specs are archived/deleted, their entries linger in `patina.db`.

**Solution:** Automatic pruning in `scrape layer` and `scrape beliefs`. After processing files, compares DB entries against files on disk and deletes stale entries. No `--prune` flag needed — follows unix philosophy of doing one job well.

**Changed:**
- `src/commands/scrape/layer/mod.rs` — prunes patterns, pattern_fts, milestones, eventlog
- `src/commands/scrape/beliefs/mod.rs` — prunes beliefs, belief_fts, eventlog

**Verified:** `patina scrape` now reports "Pruned N stale entries" when files are removed.

### 2. YAML Parser for Spec Updates — DONE

**Problem:** `update_spec_milestone()` used regex to modify YAML frontmatter. Fragile if formatting changes.

**Solution:** Replaced regex with proper `serde_yaml` parsing. Added `SpecFrontmatter` struct that models all frontmatter fields with proper type safety.

**Changed:**
- `src/commands/version/internal.rs` — Added `SpecFrontmatter`, `Sessions`, `SpecMilestoneEntry` types
- `parse_spec_file()` and `serialize_spec_file()` helpers for YAML round-trip
- `update_spec_milestone()` now type-safe with validation

**Trade-off accepted:** YAML formatting normalized on write (quotes removed, arrays in block style). Type safety worth the one-time format change.

**Bug fixed during implementation:** Layer/beliefs scrapers used file stems for pruning but DB uses frontmatter IDs. This caused specs with `id` different from filename to be incorrectly pruned. Fixed by tracking frontmatter IDs during parsing.

### 3. Spec-System Folder Migration — DONE

**Problem:** Old flat-file specs (`spec-*.md`) needed migration to folder format.

**Migrated:**
- 2 deleted (archived via git tags): launcher-polish, commit-enrichment
- 3 moved to `reference/`: architectural-alignment, assay, pipeline
- 2 moved to `dust/reviews/` (gitignored): code-audit, review-q4-2025
- 4 migrated to folder format with YAML frontmatter:
  - `feat/epistemic-layer/SPEC.md`
  - `feat/mother/SPEC.md`
  - `feat/ref-repo-semantic/SPEC.md`
  - `refactor/database-identity/SPEC.md`

**Remaining:** 18 specs in `deferred/` - spec-system says "deferred/ can be flat files"

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-01-27 | in_progress | Spec created. Current binary 52MB, 14MB compressed. |
| 2026-01-29 | in_progress | Restructured as three-pillar roadmap. Patch versioning (0.9.x → 1.0.0). |
| 2026-01-29 | in_progress | Version system hardened, YAML parser, spec migration, prune bug fixed. |
| 2026-01-29 | **0.9.1** | Released v0.9.1. Cleaned VERSION_CHANGES, bumped Cargo.toml. |
| 2026-01-29 | in_progress | 0.9.2 scoped: Session system redesign + adapter parity. Event-first architecture. |
