---
type: feat
id: repo-reliability
status: complete
created: 2026-01-25
updated: 2026-01-25
sessions:
  origin: 20260125-141105
related:
  - layer/core/unix-philosophy.md
  - layer/core/dependable-rust.md
---

# feat: Repo Reliability

> Make `patina repo add` work reliably for external codebases with full semantic search.

**Goal:** A user can add any external repo and immediately query it with semantic search. No manual steps. No panics. One command, complete result.

---

## The Problem

Adding clawdbot as a reference repo exposed multiple failures:

| Step | Expected | Actual |
|------|----------|--------|
| `repo add <url>` | Full semantic search | Only lexical FTS5 fallback |
| `scry --repo` | Semantic results | "semantic index not found" warning |
| `oxidize --repo` | Build embeddings | Flag doesn't exist |
| `oxidize` from repo dir | Build embeddings | "Missing oxidize.yaml" error |
| After manual recipe copy | Embeddings built | **Panic** on Unicode boundary |

**Root cause:** The repo pipeline is incomplete. `repo add` runs `scrape` but not `oxidize`. The pieces exist but aren't wired together.

---

## Anchoring in Core Values

### Unix Philosophy

> "One tool, one job, done well."

The current state violates this:
- `repo add` does half the job (scrape but not oxidize)
- User must manually run `oxidize`
- But `oxidize` has no `--repo` flag
- So user must `cd` to repo directory
- Then copy `oxidize.yaml` manually
- **This is not one tool doing one job.**

**Corrected model:**
```
repo add    → Clone + scaffold + scrape + oxidize (complete result)
repo update → Pull + rescrape + oxidize (optional via --oxidize)
oxidize     → Build embeddings (works on any repo context)
```

### Dependable Rust

> "Keep your public interface small and stable. Hide implementation details."

The Unicode panic in `dependency.rs:125` is a leaked implementation detail:
```rust
// Panics on multi-byte UTF-8 characters
let truncated = &name[..MAX_FUNCTION_NAME_LEN];  // 💥
```

**Corrected model:**
- Internal string handling must be UTF-8 safe
- Errors should be handled, not panicked
- The public interface (`oxidize`) should never crash

---

## Reality Check

### EXISTS (can use today)

| Component | Location | Status |
|-----------|----------|--------|
| `repo add` command | `src/commands/repo/` | ✓ Clones, scaffolds, scrapes |
| `repo update` command | `src/commands/repo/` | ✓ Has `--oxidize` flag |
| `oxidize_repo()` function | `src/commands/repo/internal.rs:600` | ✓ Creates recipe, runs oxidize |
| `scrape_repo()` function | `src/commands/repo/internal.rs:577` | ✓ Runs code + git scrape |
| Default recipe template | `src/commands/repo/internal.rs:630` | ✓ Embedded in code |
| Temporal projection | `src/commands/oxidize/temporal.rs` | ✓ Works |
| Dependency projection | `src/commands/oxidize/dependency.rs` | ⚠️ Unicode panic |
| Semantic projection | `src/commands/oxidize/semantic.rs` | ? Untested |

### NEEDS TO BE FIXED (bugs)

| Bug | Location | Severity | Fix |
|-----|----------|----------|-----|
| Unicode panic in truncation | `dependency.rs:125` | **Critical** | Use `char_indices()` for safe boundary |
| Same pattern may exist | `temporal.rs`, `semantic.rs` | Medium | Audit all string truncation |

### NEEDS TO BE WIRED (features)

| Feature | Current State | Required Change |
|---------|---------------|-----------------|
| `repo add --oxidize` | Flag doesn't exist | Add flag, default to true |
| `repo add` calls oxidize | Doesn't | Wire `oxidize_repo()` call |
| `oxidize --repo <name>` | Flag doesn't exist | Add flag to oxidize command |

### NEEDS DECISION

| Question | Options | Recommendation |
|----------|---------|----------------|
| Should `repo add` oxidize by default? | Yes / No / `--no-oxidize` to skip | **Yes** - complete result is the default |
| Should oxidize failures fail `repo add`? | Yes (fail fast) / No (warn and continue) | **No** - scrape value exists without embeddings |

---

## Implementation Plan

### Phase 1: Fix the Panic (Critical)

**Task 1.1:** Fix Unicode boundary in `dependency.rs:125`

```rust
// Before (panics on multi-byte chars)
let truncated = if name.len() > MAX_FUNCTION_NAME_LEN {
    &name[..MAX_FUNCTION_NAME_LEN]
} else {
    name
};

// After (safe truncation)
let truncated = if name.len() > MAX_FUNCTION_NAME_LEN {
    // Find last char boundary at or before limit
    let end = name
        .char_indices()
        .take_while(|(i, _)| *i < MAX_FUNCTION_NAME_LEN)
        .last()
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    &name[..end]
} else {
    name
};
```

**Task 1.2:** Audit for same pattern in:
- `temporal.rs`
- `semantic.rs`
- Any other string truncation in oxidize

**Task 1.3:** Add test case with multi-byte UTF-8 characters

### Phase 2: Wire Oxidize into Repo Add

**Task 2.1:** Add `--oxidize` / `--no-oxidize` flags to `repo add`

```rust
// src/commands/repo/mod.rs
/// Add an external repository
Add {
    url: String,

    /// Skip building embeddings (faster, lexical search only)
    #[arg(long)]
    no_oxidize: bool,

    // ... existing flags
}
```

**Task 2.2:** Call `oxidize_repo()` at end of `add_repo()`

```rust
// src/commands/repo/internal.rs, in add_repo()

// Run scrape (existing)
let event_count = scrape_repo(&repo_path)?;

// NEW: Run oxidize unless skipped
if !no_oxidize {
    println!("\n🧪 Building semantic indices...");
    if let Err(e) = oxidize_repo(&repo_path) {
        // Warn but don't fail - scrape value still exists
        eprintln!("  ⚠️  Oxidize failed: {}. Semantic search unavailable.", e);
        eprintln!("      Run 'patina repo update {} --oxidize' to retry.", name);
    }
}
```

### Phase 3: Add `--repo` to Oxidize Command (Optional)

**Task 3.1:** Add `--repo` flag to `patina oxidize`

This enables:
```bash
patina oxidize --repo clawdbot/clawdbot
```

Instead of requiring:
```bash
cd ~/.patina/cache/repos/clawdbot/clawdbot && patina oxidize
```

**Implementation:** Look up repo path from registry, temporarily change working directory, run oxidize, restore.

---

## Verification

After implementation, this flow should work:

```bash
# Add repo with full semantic search (default)
$ patina repo add https://github.com/clawdbot/clawdbot.git
📥 Cloning...
🔍 Scraping codebase...
🧪 Building semantic indices...
✅ Repository added with semantic search enabled!

# Query immediately works
$ patina scry "skills plugin architecture" --repo clawdbot/clawdbot
# Returns semantic results, not FTS5 fallback

# Skip oxidize if desired (faster)
$ patina repo add https://github.com/other/repo.git --no-oxidize
# Lexical search only, warns user
```

---

## Open Questions

1. **Oxidize timeout:** Large repos may take minutes. Add progress indicator? Timeout?
2. **Incremental oxidize:** After `repo update`, should we rebuild all embeddings or just changed files?
3. **Recipe customization:** Should repos be able to have custom `oxidize.yaml` committed?

---

## References

- [Unix Philosophy](../../../core/unix-philosophy.md) - One tool, one job
- [Dependable Rust](../../../core/dependable-rust.md) - Stable interfaces, hidden implementation
- Session: 20260125-141105 (this session - discovered issues via clawdbot repo)
