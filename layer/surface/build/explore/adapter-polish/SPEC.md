---
type: explore
id: adapter-polish
status: abandoned
created: 2026-01-13
updated: 2026-02-07
related:
- layer/surface/build/feat/skills-focused-adapter/SPEC.md
references:
- adapter-pattern
- dependable-rust
- unix-philosophy
---

# Explore: Adapter Polish

> How should adapter scaffolding work? Minimal injection, not embedded content.

## Core Question

Patina's scaffold should be the smallest possible surface that connects the LLM to patina's capabilities. What's the right pattern?

## Key Ideas

1. **@include pattern** — scaffold is a pointer, not the content. `.claude/CLAUDE.md` is one line pointing to `.patina/context/claude.md`
2. **Central context** — all adapter context lives in `.patina/context/`, not scattered across adapter-specific directories
3. **MCP as intelligence** — the MCP tools ARE the intelligence, context file just tells the LLM they exist
4. **Version tracking** — adapter manifest tracks scaffold version, CLI version, template checksums
5. **Clean refresh** — `adapter refresh` removes obsolete files, preserves user customizations

## Upstream Change Tracking

From ref-repo-health Phase 4: how do we track upstream adapter changes?

We track claude-code issues via forge but don't surface changes relevant to our adapters. Concept: `adapter_observations` table for noting breaking changes, new features, deprecations discovered during sessions.

```bash
patina adapter observe claude "Skills now support frontmatter schema" \
    --source "https://github.com/anthropics/claude-code/issues/17000" \
    --impact "Should migrate /session-* to Skills format"

patina adapter changes claude --since 30d
```

Open: is this a table in patina.db, a layer/ document, or just session notes with tags?

## Open Questions

- How does this interact with the skills system evolution?
- Is `.patina/context/` the right location or should it stay adapter-specific?
- What's the minimum viable scaffold per adapter?
- Should adapter parity (Claude/Gemini/OpenCode) be a goal or should we focus on Claude-first?
- How should upstream adapter changes be tracked and surfaced?

## References

- Claude Code @include: CHANGELOG 0.2.107
- Gemini hierarchical loading: docs/cli/gemini-md.md
- spec/llm-frontends: unified 5-command experience
- Session 20251230-180841: adapter sync concept
