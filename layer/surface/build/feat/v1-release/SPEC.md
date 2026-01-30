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

Move session mechanics from ~640 lines of bash into Patina Rust commands, making sessions adapter-agnostic while preserving the division of labor that makes them work: **Patina handles mechanics (git tags, scaffolding, metrics, classification, archival), LLMs handle meaning (narrative, context bridging, belief capture, goal tracking).**

### What We Have (and Why It's Clever)

**858 git tags. 538 session files. Every single development session tracked.**

The current system has three layers:

1. **Shell scripts** (`.claude/bin/session-*.sh`, ~640 lines) — mechanical work: git tags, markdown scaffolding, metrics computation, work classification, archival, prompt extraction
2. **Skill definitions** (`resources/claude/session-*.md`) — teach the LLM what to do *around* the script: read previous session, write real summary, fill in goals, suggest beliefs, bridge context
3. **Rust scraper** (`src/commands/scrape/sessions/mod.rs`) — extracts structured events from finished markdown after the fact

The clever design: **scripts produce structure, the LLM fills in meaning.** Bash creates the scaffold and captures metrics; the LLM writes the narrative. Neither is redundant.

### What's Wrong

- **640 lines of bash doing things Patina does natively** — YAML parsing, git operations, SQLite writes all have Rust equivalents
- **Three adapter copies** — Claude, Gemini, OpenCode each deploy identical scripts. Bug fix = change in 3+ places plus `session_scripts.rs` embeds
- **Events reconstructed, not recorded** — scraper parses markdown *after* session ends. If LLM writes in unexpected format, scraper misses it
- **Active session is adapter-specific** — `.claude/context/active-session.md` means switching adapters mid-session loses tracking
- **`navigation.db` is vestigial** — scripts write to `state_transitions` table, nothing reads it. Eventlog already captures the same data
- **`history.jsonl` path is hardcoded to Claude** — all three adapter scripts (including Gemini and OpenCode) reference `$HOME/.claude/history.jsonl` for prompt extraction. Gemini/OpenCode never capture user prompts because they look in the wrong place. Rust implementation must detect the active adapter or check all known paths.
- **No spec linkage** — sessions and specs are both central but not connected

### Solution: Dual-Write Sessions

**Architecture principle:** Markdown is the collaboration artifact (LLM actively edits it). Events are the structured query layer (written at action time). Both are primary — neither is derived from the other.

```
patina session start "complete 0.9.2"
         │
         ├──→ .patina/local/active-session.md  (collaboration artifact)
         │    - YAML frontmatter + markdown scaffold
         │    - LLM reads/writes this throughout session
         │    - Archived to layer/sessions/ on end
         │
         ├──→ EVENTLOG (append-only)        (structured query layer)
         │    session.started { id, title, branch, tag }
         │    - Written at action time, not scraped after
         │    - Enables "what sessions worked on X?" queries
         │
         └──→ GIT TAG                       (durable boundary marker)
              session-{ID}-{frontend}-start
              - Survives rebasing, branch deletion, cloning
              - `git log tag-start..tag-end` always works
```

### Deliverables

**1. `patina session` commands (Rust)**

Replace shell script mechanics with native Patina commands. Same outputs, same git tag conventions, same markdown format — but testable, single-source, and event-aware.

```bash
patina session start "complete 0.9.2"
  → Creates git tag (session-{ID}-{frontend}-start)
  → Writes active session markdown to .patina/local/active-session.md
  → Writes session.started event to eventlog
  → Handles incomplete previous session (cleanup/archive)
  → Shows beliefs context, previous session pointer
  → Outputs session ID, branch, tag for LLM

patina session update
  → Computes git metrics (commits, files changed, last commit time)
  → Appends timestamped update section to active session markdown
  → Writes session.update event to eventlog
  → Shows commit coaching (stale work warnings, large change alerts)

patina session note "discovered edge case in parser"
  → Appends timestamped note with [branch@sha] to active session
  → Writes session.observation event to eventlog
  → Detects importance keywords, suggests checkpoint commit

patina session end
  → Creates git tag (session-{ID}-{frontend}-end)
  → Computes final metrics and work classification
  → Counts beliefs captured during session
  → Extracts user prompts (from history.jsonl if available)
  → Appends classification + beliefs sections to markdown
  → Archives to layer/sessions/{ID}.md
  → Updates .patina/local/last-session.md pointer
  → Writes session.ended event to eventlog
  → Cleans up active session file
```

**What the LLM still does** (via skill definitions, unchanged role):
- Reads previous session file and writes substantive summary
- Fills in activity log with narrative, decisions, patterns
- Checks for beliefs to capture during updates
- Runs final update before ending session

**2. Active session in `.patina/local/` (adapter-agnostic, gitignored)**

Move from `.claude/context/active-session.md` to `.patina/local/active-session.md`.

- All adapters read/write the same file
- `patina session` commands own the file lifecycle
- Adapter skill definitions reference the new path
- `.patina/local/last-session.md` replaces `.claude/context/last-session.md`
- Transient files stay in `.patina/local/` (gitignored), not `.patina/` (committed)

**3. YAML frontmatter for session documents**

New sessions get machine-parseable frontmatter (consistent with spec format):

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
---

## Previous Session Context
<!-- AI: Summarize the last session from last-session.md -->

## Goals
- [ ] Complete v0.9.2

## Activity Log
### 09:38 - Session Start
...
```

Scraper updated to read YAML frontmatter when present, fall back to markdown header parsing for 538 existing sessions.

**4. Skill definitions updated**

Each adapter's skill `.md` files change one line — from calling a shell script to calling `patina session`:

```diff
- `.claude/bin/session-start.sh $ARGUMENTS`
+ `patina session start "$ARGUMENTS"`
```

The rest of the skill definition (read previous session, fill in summary, suggest beliefs, etc.) stays identical. This is where adapter parity actually lives: all adapters call the same `patina session` commands, with only skill definition format differing (markdown for Claude/OpenCode, TOML for Gemini).

**5. Drop `navigation.db` session tracking**

Shell scripts currently write `SessionStart`/`SessionEnd` transitions to `.patina/navigation.db`. Nothing reads this data — the eventlog captures the same information. New Rust commands write to eventlog only. Clean cut.

### Implementation Decisions

**Transient file location:** `.patina/local/active-session.md` and `.patina/local/last-session.md`. Transient files (deleted on session end) stay in `.patina/local/` which is gitignored. `.patina/` root is for committed config.

**Adapter detection:** Resolution chain — explicit `--adapter` flag wins, otherwise read `config.adapters.default` from `.patina/config.toml`. No env vars, no magic. Function signature is honest about what it needs: `resolve_adapter(explicit: Option<&str>, project_root: &Path) -> Result<String>`.

**Eventlog as shared infrastructure:** Extract eventlog (schema, `insert_event()`, event types) from `src/commands/scrape/database.rs` into `src/eventlog.rs`. The eventlog is a pipe between commands — scrape writes events, session writes events, scry reads events. No single command owns it.

**A/B testing:** Rust commands write to `.patina/local/active-session.md`. Shell scripts keep writing to `.claude/context/active-session.md`. Both run in parallel during the same session. Diff the outputs. If they match, update skill definitions to point to Rust commands. If they don't, fix the Rust commands until they do.

### Stretch Goals (not required for 0.9.2)

- `--spec` and `--milestone` flags for explicit spec linkage
- Auto-update spec's `sessions.work` array on session end
- `patina session list` / `patina session show` query commands
- `patina adapter status` (unrelated to sessions — separate task)

### Migration Path

**Phase 1: Build Rust commands (non-breaking)**
- Implement `patina session start/update/note/end`
- Dual-write: markdown + eventlog events at action time
- Scraper handles both YAML frontmatter and legacy markdown headers
- Shell scripts still exist and work (parallel paths for testing)
- Active session written to `.patina/local/active-session.md`

**Phase 2: Cut over adapters**
- Skill definitions updated to call `patina session` commands
- Active session path changed in all adapter skill definitions
- Adapter `session_scripts.rs` deploys `patina session` wrappers instead of full bash scripts
- Test: run full session lifecycle with each adapter

**Phase 3: Remove legacy (separate commit, after validation)**
- Delete shell scripts from `resources/{claude,gemini,opencode}/`
- Remove embedded script constants from `session_scripts.rs`
- Remove `navigation.db` writes from any remaining code
- Keep shell scripts accessible via git history

### Exit Criteria

- [ ] `patina session start/update/note/end` commands exist in Rust
- [ ] Commands produce identical markdown output to current shell scripts
- [ ] Session events written to eventlog at action time
- [ ] Active session lives in `.patina/local/active-session.md`
- [ ] YAML frontmatter on new session documents
- [ ] Scraper handles both YAML frontmatter and legacy markdown headers
- [ ] Skill definitions call `patina session` instead of shell scripts
- [ ] All three adapters (Claude, Gemini, OpenCode) use same commands
- [ ] `navigation.db` session writes removed
- [ ] Full session lifecycle tested (start → update → note → end → archive)

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
| 2026-01-29 | in_progress | 0.9.2 revised: Dual-write sessions, bash→Rust, adapter-agnostic active session. |
