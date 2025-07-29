# Context as Wisdom Architecture

## Core Insight
Patina is a wisdom transfer system. Instead of static template files for LLM contexts, we should use Patina's own layer architecture to accumulate and compose contextual knowledge.

## The Vision: Living Context System

### Layer Structure for Contexts
```
layer/
├── core/
│   ├── llm-styles/
│   │   ├── claude.md          # How Claude likes instructions
│   │   ├── gemini.md          # How Gemini responds best
│   │   └── principles.md      # Universal LLM instruction principles
│   └── constraints/
│       ├── simplicity.md      # Universal simplicity principles
│       └── safety.md          # Security constraints
├── topics/
│   ├── languages/
│   │   ├── go/
│   │   │   ├── patterns.md    # Go idioms and patterns
│   │   │   ├── constraints.md # Go-specific constraints
│   │   │   └── llm-pitfalls.md # Common LLM mistakes with Go
│   │   └── rust/
│   │       └── patterns.md
│   └── tools/
│       ├── dagger/
│       │   ├── patterns.md    # Dagger best practices
│       │   ├── caching.md     # Caching strategies
│       │   └── pipelines.md   # Pipeline patterns
│       └── docker/
│           └── patterns.md
└── projects/
    └── patina/
        ├── contexts/          # Composed contexts
        │   ├── claude-dagger.md
        │   └── gemini-docker.md
        └── learned/           # Project-specific learnings
            └── go-mock-hell.md # "Don't let LLMs create mocks"
```

## Evolution Path

### Phase 1: Markdown Composition (Now)
Simple file concatenation based on layer hierarchy:
- Core principles (universal)
- Topic knowledge (domain-specific)
- Project wisdom (learned patterns)

### Phase 2: Structured Composition
Smart merging with metadata:
- Priority levels
- Conditional application
- Conflict resolution

### Phase 3: SQLite Domain Model
```sql
-- Domain-specific knowledge graph
CREATE TABLE wisdom (
    id INTEGER PRIMARY KEY,
    layer TEXT CHECK(layer IN ('core', 'topic', 'project')),
    domain TEXT, -- 'llm-style', 'language', 'tool'
    applies_to TEXT[], -- ['claude', 'go', 'dagger']
    pattern TEXT,
    anti_pattern TEXT,
    confidence REAL, -- How well this has worked
    last_used DATE
);

-- Track what works
CREATE TABLE context_effectiveness (
    context_hash TEXT,
    llm TEXT,
    task_type TEXT,
    success_rate REAL,
    notes TEXT
);
```

## Key Principles

1. **Contexts are accumulated wisdom, not static templates**
2. **The layer system IS the context system**
3. **Every interaction teaches Patina something new**
4. **Wisdom flows upward: projects → topics → core**

## Usage Modes

### Interactive Mode (Current)
- Real-time context composition
- Learning from success/failure
- Immediate layer updates

### Layer-Guided Mode (Future)
```bash
patina compose-context --llm=claude --tool=dagger --lang=go
patina apply-wisdom --scenario=go-pipeline
```

## Implementation Strategy

### Short Term
1. Create markdown-based layer structure
2. Simple concatenation logic
3. Manual wisdom capture

### Medium Term
1. Metadata-driven composition
2. Effectiveness tracking
3. Semi-automated learning

### Long Term
1. Full SQLite knowledge graph
2. ML-based pattern recognition
3. Cross-project wisdom sharing

## Open Questions

1. How do we handle conflicting wisdom from different projects?
2. What's the confidence threshold for promoting project → topic → core?
3. How do we version wisdom as tools and LLMs evolve?
4. Should wisdom decay over time if not reinforced?

## Next Steps

- [ ] Design the initial layer structure for contexts
- [ ] Create prototype composition logic
- [ ] Define wisdom promotion criteria
- [ ] Build feedback loop for context effectiveness