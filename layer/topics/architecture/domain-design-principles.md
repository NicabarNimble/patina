---
id: domain-design-principles
version: 1
created_date: 2025-07-29
confidence: emerging
oxidizer: nicabar
tags: [architecture, domains, knowledge-organization]
---

# Domain Design Principles

## Single Domain Focus

When building a domain, focus on that domain exclusively. The patina-domain contains everything needed to build and understand Patina - nothing more, nothing less.

### Domain Structure
```
patina-domain/
└── layer/
    ├── core/        # Proven truths for this domain
    ├── topics/      # Learning areas within this domain
    └── sessions/    # Daily discoveries about this domain
```

## Topics as Learning Spaces

Topics are where we experiment with organization patterns:
- `topics/dagger/` - How Patina uses Dagger (not everything about Dagger)
- `topics/go/` - What Patina needs from Go (not a Go tutorial)
- `topics/claude/` - How Patina works with Claude (not Claude's life story)

## Truth Hierarchy

Knowledge flows upward based on confidence:
1. **Sessions**: Raw discoveries, explorations, "what happened today"
2. **Topics**: Emerging patterns, "we're seeing this work repeatedly"
3. **Core**: Proven truths, "this is how we do things"

## The Oxidizer's Role

The user (oxidizer) is the curator who:
- Decides when patterns are ready for promotion
- Maintains the quality of each truth level
- Keeps the domain focused on its purpose

## Metadata Strategy

All knowledge items should include frontmatter for future database migration:
```yaml
---
id: unique-identifier
version: 1
promoted_from: sessions/source-session
promoted_date: 2025-07-29
confidence: low|medium|high|proven
oxidizer: username
tags: [relevant, tags]
---
```

## Design Principles

1. **Domain-centric**: Everything relates to the domain's purpose
2. **Evolution-ready**: Markdown now, database later
3. **Tool-agnostic**: Works in Obsidian, VS Code, or plain text
4. **Human-curated**: LLMs suggest, oxidizers decide
5. **Local-first**: Everything needed is in the domain

## Future Vision

These markdown-based domains will evolve into distributed databases (rqlite), but the organizational principles remain:
- Domains are self-contained knowledge islands
- References create connections (but we don't chase them yet)
- The structure we build now becomes the schema later