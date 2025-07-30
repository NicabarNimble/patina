---
id: no-liquid-for-core
version: 1
created_date: 2025-07-27
confidence: medium
oxidizer: nicabar
tags: []
promoted_from: projects/patina
---

# No Liquid Templating for Patina Core

## Decision
Patina will NOT use liquid templating (or any complex template engine) for its core functionality. Simple string replacement is sufficient for context generation.

## Context
During the "Dagger setup and liquid integration" session (2025-07-27), we analyzed why liquid was originally considered and why it doesn't fit Patina's architecture.

### Why Liquid Was Considered
1. **cargo-generate inspiration** - Their `.liquid` convention for mixing static/dynamic files
2. **Digital garden vision** - Living patterns that evolve with usage
3. **Pattern templates** - Dynamic code generation based on user choices

### Why It Doesn't Fit
1. **Wrong layer** - Liquid was for generating Rust code, but that's the LLM's job
2. **Complexity creep** - Patina becomes a code generator instead of context orchestrator
3. **Duplicate work** - LLMs already excel at conditional code generation
4. **Focus drift** - Moves away from core job: orchestrating User + LLM + Brain

## The Real Need: Context Templates

What actually needs templating is context generation for different LLMs:
- `CLAUDE.md` - Detailed, with session workflow
- `GEMINI.md` - Concise, different structure
- `OPENAI.md` - Their preferred format

Simple string replacement is sufficient:
```rust
content.replace("{{PROJECT_NAME}}", project_name)
       .replace("{{PATTERNS}}", pattern_list)
       .replace("{{ENVIRONMENT}}", env_info)
```

## Implementation
1. Keep Dagger/Docker files completely static
2. Use simple string replacement for context generation
3. Each adapter owns its context template
4. No external templating dependencies

## Future Consideration
The "living patterns" vision with liquid templates could exist in a separate layer - perhaps a future `patina-patterns` crate for managing reusable code patterns. The core Patina should stay focused on context orchestration.

## Rationale
- Maintains focus on Patina's core job
- Avoids dependency bloat
- Keeps architecture simple
- Lets LLMs do what they do best (write code)
- Templates only where truly needed (context generation)