# Core Patterns

How we approach problems and structure solutions.

## Working Modes

### On the Program (Development Mode)
- Deep architectural thinking
- Slow, deliberate design decisions
- Test abstractions against multiple paradigms
- Document the "why" not just the "what"

### With the Program (Usage Mode)
- Focus on user's immediate needs
- Apply established patterns consistently
- Generate context from accumulated wisdom
- Respect project-specific conventions

### Through the Program (Execution Mode)
- Stay on established rails
- Don't question fundamentals
- Execute within defined constraints
- Report results clearly

## Problem Solving Patterns

### Three Paradigm Test
When designing abstractions:
1. Test against traditional approach (e.g., Docker)
2. Test against modern approach (e.g., Dagger)
3. Test against alternative approach (e.g., Nix)

If the abstraction handles all three, it's properly designed.

### Context Accumulation
- Start with minimal context
- Learn from each interaction
- Promote project patterns to topics
- Topics become reusable knowledge

### Incremental Understanding
1. Listen to the full problem
2. Ask clarifying questions
3. Propose minimal solution
4. Iterate based on feedback

## Architecture Patterns

### Core + Adapters
```
Core (pure business logic)
  └─ Traits (contracts)
      └─ Adapters (implementations)
```

### Composable Context Hierarchy
```
Core Context (universal)
  └─ Topic Context (domain-specific)
      └─ Project Context (implementation-specific)
```

### Knowledge Evolution
```
Project Pattern → Successful Use → Topic Pattern → Core Pattern
```

## Communication Patterns

### With Humans
- Listen first, propose second
- Explain rationale for decisions
- Provide options when appropriate
- Respect existing preferences

### With AI Systems
- Provide rich context upfront
- Set clear constraints
- Use consistent formatting
- Enable tool discovery