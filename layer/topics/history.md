# The Evolution from RCW to Patina: A Development Timeline

## Overview
This document traces the transformation of RCW (Rusty Claude's Workshop) from a Rust project generator into Patina, a development wisdom accumulator that orchestrates context for AI-assisted development.

## Timeline

### Phase 1: Foundation (July 6-8, 2025)
**The Architecture Emerges**
- **July 6**: Session storage refactored to `.claude/context/sessions/`
- **July 8**: Major TUI/Engine separation - main.rs reduced from 378 to 60 lines
- Created modular architecture with clear boundaries between UI and business logic
- Established dual nature: RCW both uses and generates session systems

### Phase 2: Refinement (July 9-10, 2025)
**Testing & Platform Focus**
- Removed Windows support to focus on Unix-like systems
- Implemented comprehensive testing platform with Nextest and Proptest
- Added `--batch` mode for non-interactive CLI testing
- Solved the "testing hell" problem with property-based testing

### Phase 3: MCP Integration (July 11-12, 2025)
**Extended Capabilities**
- MCP server rebuilt and renamed: `system-access` → `dirs`
- Introduced TOML-based configuration for MCP servers
- Session management tools integrated
- Security filters added to MCP directory access

### Phase 4: The Breakthrough (July 14, 2025)
**PROJECT_DESIGN.toml Changes Everything**
- **Morning**: Deep dive revealed RCW stuck in redesign cycles
- **Insight**: PROJECT_DESIGN.toml as single source of truth
- **Shift**: From "project generator" to "context orchestrator"
- Binary renamed: `nica-rcw` → `rcw`
- New commands: `rcw design enhance`, `rcw design regenerate`

**The Real Problem Emerges**
- "I waste so much time building with Claude where I have to reframe and adjust context"
- Solution: Hybrid local/remote context system
- Context as "recipes" (local TOML) + "ingredients" (remote patterns)
- RCW becomes "Rust on Rails" where Claude provides the rails

### Phase 5: Patina is Born (July 15, 2025)
**Morning: The Name and Vision**
- **"Patina"** chosen - wisdom that accumulates over time
- Recognized need for container consistency
- SQL brain concept for intelligent pattern storage
- Git-style command structure adopted

**Key Realizations:**
1. Context builds in layers: Core → Topics → Projects
2. Projects can graduate to become Topics
3. Three paradigms test: abstractions must work in Docker, Dagger, and Nix
4. Two modes: Exploration (Claude Desktop) vs Execution (on rails)

**Afternoon: Implementation**
- Patina MVP created with functional CLI
- Core features implemented:
  - Brain storage system with hierarchical patterns
  - Session management
  - Environment detection
  - Claude adapter pattern
- Git-like commands: `patina init`, `add`, `commit`, `push`

## The Transformation

### What Changed
1. **Purpose**: Project generation → Context orchestration
2. **Philosophy**: Creating new → Accumulating wisdom
3. **Architecture**: Template-based → Brain-based pattern storage
4. **Interface**: Custom commands → Git-like familiarity
5. **Focus**: Rust projects → Any development with Rust tooling

### Core Insights
- "The Rust compiler keeps Claude honest, and Claude makes Rust accessible"
- "One complete tool with focused subcommands" (like git)
- LLMs need rails, and Rust provides the strongest rails
- Context persistence matters more than code generation

### Final Architecture
```
patina/
├── brain/          # Pattern storage (Core/Topics/Projects)
├── sessions/       # Development history
├── adapters/       # LLM integrations
└── commands/       # Git-style CLI
```

## Conclusion
The journey from RCW to Patina represents a fundamental shift in thinking: from a tool that generates projects to one that accumulates and orchestrates development wisdom. Like its namesake, Patina grows more valuable with use, building layers of context and patterns that make each new project more efficient than the last.