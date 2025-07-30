---
id: knowledge-hierarchy-with-confidence
version: 1
created_date: 2025-07-29
confidence: medium
oxidizer: nicabar
tags: []
---

# Knowledge Hierarchy with Confidence

## Overview
A tiered knowledge search system where Claude progressively searches from most trusted (Patina layers) to least trusted (web search) sources, with confidence assessment at each level.

## Knowledge Sources

```rust
enum KnowledgeSource {
    Layer(Confidence),      // 90-100% - Proven patterns in layer/
    ProjectDocs(Confidence), // 70-90% - Project specific docs
    WebSearch(Confidence),   // 30-70% - External research
    Experiment(Confidence),  // 0-30% - Need to try it
}
```

## Implementation Flow

### 1. Layer-First Search
```rust
// Claude's internal process:
"Need to implement JWT auth"
→ Search Patina layers:
   - layer/core/auth-principles.md (95% confidence)
   - layer/topics/auth/jwt-pattern.md (90% confidence)
   - layer/projects/patina/auth-decisions.md (85% confidence)
→ "Found established pattern, confidence: 90/100"
```

### 2. Progressive Enhancement
```yaml
# If confidence < 70:
1. Check project docs (README, ARCHITECTURE.md)
2. Search web for recent changes
3. Create spike/POC
4. Update layers with findings
```

### 3. SQLite-Powered Intelligence
```sql
-- Patina's brain.db
CREATE TABLE patterns (
    id INTEGER PRIMARY KEY,
    pattern TEXT,
    confidence INTEGER,
    success_count INTEGER,
    last_used DATE,
    source TEXT -- 'layer', 'session', 'external'
);

-- Claude queries:
SELECT pattern, confidence FROM patterns 
WHERE pattern LIKE '%jwt%' 
ORDER BY confidence DESC, success_count DESC;
```

### 4. Confidence-Based Workflow
When planning implementation:
- **High confidence (>70%)**: Direct implementation using known patterns
- **Medium confidence (30-70%)**: Research phase + documentation before implementation
- **Low confidence (<30%)**: Spike/POC required before real implementation

Example:
```markdown
Human: "Add GraphQL subscriptions"

Claude: *Searches layers*
→ No patterns found
→ Confidence: 20/100

Action plan:
1. Research phase (create spike)
2. Document findings in layer/
3. Implementation with high confidence
```

### 5. Auto-Learning System
```yaml
# After successful implementation:
- Extract patterns from PR
- Add to SQLite with confidence score
- Increment success_count on reuse
- Promote to layer/ when proven (success_count > 3)
```

## Architecture Diagram

```
Claude's Knowledge Stack:
┌─────────────────────┐
│   Web Search (30%)  │ ← Last resort
├─────────────────────┤
│  Project Docs (70%) │ ← Current context  
├─────────────────────┤
│ Patina Layer (90%)  │ ← Proven patterns
├─────────────────────┤
│   SQLite DB (95%)   │ ← Instant lookup
└─────────────────────┘
```

## Benefits

1. **Faster responses** - Check local knowledge first
2. **Higher quality** - Use proven patterns when available
3. **Self-improving** - System gets smarter with use
4. **Transparent confidence** - Know when research is needed
5. **Reduced hallucination** - Explicit "I don't know" triggers research

## Integration with Sessions

Each session can:
1. Query confidence before starting
2. Trigger research phase if needed
3. Update patterns after completion
4. Contribute to collective knowledge

## Future Extensions

- Confidence decay over time (patterns may become outdated)
- Team confidence scores (patterns proven across multiple developers)
- Domain-specific confidence (higher for Rust patterns vs JavaScript)
- Automatic web search for low-confidence areas