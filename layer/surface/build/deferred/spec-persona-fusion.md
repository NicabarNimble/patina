# Spec: Persona Fusion

**Status:** Phase 1 Complete - Observability shipped

**Vision:** PersonaOracle as a first-class oracle in scry, delivering cross-project user knowledge alongside code results.

---

## Current State (What Exists)

### Already Implemented ✅

| Component | Location | Status |
|-----------|----------|--------|
| PersonaOracle | `src/retrieval/oracles/persona.rs` | Working |
| Wired into QueryEngine | `src/retrieval/engine.rs:73` | Working |
| Persona storage | `~/.patina/personas/default/events/` | Working |
| Persona capture | `patina persona note` | Working |
| Persona materialize | `patina persona materialize` | Working |
| Source tracking in fusion | `FusedResult.sources` | Working |
| Contribution details | `FusedResult.contributions["persona"]` | Working |

### The Discovery (What's Missing)

When user ran `patina scry "1password yolo containers" --all-repos`:
1. Cross-repo query executed ✅
2. PersonaOracle queried (if available) ✅
3. **But results were garbage** - lexical matches dominated
4. **No visible persona results** - either not captured or drowned out

---

## Gap Analysis

### Gap 1: Persona Availability

PersonaOracle returns empty if:
- No persona.db exists (`~/.patina/cache/personas/default/persona.db`)
- No notes captured via `patina persona note`
- No `patina persona materialize` run after capturing

**Symptom:** Scry works but persona never contributes.

**Fix:** Better observability - show which oracles contributed in output.

### Gap 2: Source Visibility

Sources are tracked internally but not prominently displayed:

```rust
// fusion.rs - sources ARE tracked
pub struct FusedResult {
    pub sources: Vec<&'static str>,  // ["semantic", "lexical", "persona"]
    pub contributions: HashMap<&'static str, OracleContribution>,
}
```

But MCP/CLI output doesn't show `[PROJECT]` vs `[PERSONA]` tags.

**Fix:** Output formatting should show source provenance.

### Gap 3: Cross-Project Query Quality

The `all_repos` query (engine.rs:207) fuses across repos, but:
- Lexical matches on random tokens dominate
- No query intent detection (code search vs pattern search)
- PersonaOracle correctly included once, not per-repo

**Fix:** This is a retrieval quality issue, not a fusion issue.

---

## Proposed Changes

### Phase 1: Observability (Low Effort)

Make persona contributions visible without changing fusion logic.

**Tasks:**
- [ ] Add `--explain` output showing oracle contributions (exists in observable-scry spec)
- [ ] MCP scry tool: include source in result annotation
- [ ] CLI: show `[persona]` tag when result came from PersonaOracle
- [ ] Add `patina persona status` to check if oracle is available

**Exit Criteria:**
- `patina scry --explain "error handling"` shows persona contribution
- User can diagnose "why didn't persona help?"

### Phase 2: Source Tagging (Medium Effort)

Explicit `[PROJECT]`/`[PERSONA]` tags in output.

**Tasks:**
- [ ] Define source types in FusedResult (Project, Persona, Reference)
- [ ] Tag results based on oracle source + doc_id pattern
- [ ] Update MCP schema with source field
- [ ] Update CLI output formatting

**Output Example:**
```
[PROJECT]   0.92  Use Result<T, AppError> with thiserror
            src/error.rs:15 | semantic #1, lexical #3

[PERSONA]   0.87  Always use Result over panics
            domains: rust, error-handling | captured: 2025-11-20
```

**Exit Criteria:**
- Results visibly tagged by source
- LLM can distinguish project code from user preferences

### Phase 3: Query Intent (Future)

Different retrieval strategies for different query types.

| Query Type | Example | Strategy |
|------------|---------|----------|
| Code location | "where is auth handler" | Lexical + Semantic heavy |
| Pattern search | "error handling patterns" | Persona + Layer docs heavy |
| Orientation | "what's important here" | Structural signals heavy |

**Not in scope for this spec.** Deferred to query-routing work.

---

## Related Specs

- [spec-observable-scry.md](./spec-observable-scry.md) - `--explain` flag work
- [spec-work-deferred.md](./spec-work-deferred.md) - Query-type routing

---

## Acceptance Criteria

1. [x] User can see if PersonaOracle is contributing (`--explain` or similar)
2. [x] Results tagged with source (`[PERSONA]` prefix in CLI and MCP)
3. [x] `patina persona status` shows oracle health
4. [ ] Documentation: how to capture persona for cross-project benefits
