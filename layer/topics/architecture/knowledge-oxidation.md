---
id: knowledge-oxidation
version: 1
created_date: 2025-07-30
confidence: high
oxidizer: nicabar
tags: [architecture, layer-management, knowledge-evolution]
status: active
supersedes: []
superseded_by: []
---

# Knowledge Oxidation: Managing Layer Decay

## Core Concept
Just as patina forms on metal through oxidation, knowledge in our layer accumulates and oxidizes over time. Some oxidation protects (valuable patterns), while some just obscures (outdated decisions).

## Oxidation States

### Fresh (0-7 days)
- Recent sessions and discoveries
- High relevance to current work
- Not yet validated by practice

### Weathered (7-30 days) 
- Patterns emerging from use
- Some validation through application
- Ready for consolidation

### Oxidized (30-60 days)
- Either proven (→ promote to core)
- Or stale (→ mark for scraping)
- Requires active decision

### Fossilized (60+ days)
- Historical record only
- Archive unless actively referenced
- Candidate for removal

## Scraping Patterns

### 1. Consolidation Scraping
When multiple docs say similar things:
```yaml
consolidate:
  - container-first.md
  - dagger-integration.md  
  - agent-workflows.md
into: container-patterns.md
preserve: [unique-insights, proven-patterns]
```

### 2. Succession Scraping
When patterns evolve:
```yaml
superseded_chain:
  - brain-terminology.md → layer-terminology.md
  - add-command.md → removed (command deprecated)
  - workspace.md → agent.md (renamed)
```

### 3. Extraction Scraping
Mining sessions for patterns:
```yaml
extract_from: sessions/*.md
look_for: [decisions, patterns, lessons]
promote_to: topics/
confidence_threshold: medium
```

## Metadata Schema

```yaml
# Every document needs oxidation metadata
---
id: unique-identifier
version: 1
created_date: 2025-07-30
updated_date: 2025-07-30
confidence: [low, medium, high]
oxidizer: nicabar  # who curated this
status: [draft, active, deprecated, superseded]
supersedes: [older-pattern.md]  # what this replaces
superseded_by: []  # what replaces this
references: [session-123.md]  # source material
tags: [searchable, categories]
token_estimate: 1500  # for context optimization
---
```

## Oxidation Workflow

1. **Capture** - Sessions accumulate raw knowledge
2. **Weather** - Time and use reveal patterns
3. **Scrape** - Remove excess, consolidate similar
4. **Polish** - Refine into clear patterns
5. **Preserve** - Protect proven truths in core

## Implementation Priority

1. Add oxidation metadata to existing docs
2. Build scraping detection (`patina layer detect-decay`)
3. Create consolidation tools (`patina layer consolidate`)
4. Automate extraction (`patina layer extract-patterns`)

## Scraping Commands Vision

```bash
# Detect what needs attention
patina layer detect-decay
patina layer find-overlaps
patina layer show-stale

# Perform scraping operations
patina layer scrape --consolidate "container patterns"
patina layer scrape --extract-from "sessions/*"
patina layer scrape --remove-fossilized

# Track oxidation state
patina layer oxidation-report
patina layer mark-superseded <old> --by <new>
```

## Key Insights from Session

1. **Patina projects are domain-specific** - Each project maintains its own layer
2. **Knowledge oxidation is natural** - Not all accumulated knowledge is valuable
3. **Scraping reveals truth** - Remove oxidation to find core patterns
4. **Metadata enables automation** - Proper tagging allows intelligent processing
5. **Token awareness matters** - For context optimization in AI interactions

## Next Steps

1. Implement metadata schema across existing documents
2. Build `patina layer` command structure for scraping operations
3. Create automated detection of knowledge decay
4. Develop consolidation algorithms for similar patterns
5. Test extraction rules on existing sessions