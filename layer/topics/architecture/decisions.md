---
id: decisions
version: 1
created_date: 2025-07-15
confidence: medium
oxidizer: nicabar
tags: []
promoted_from: projects/patina
---

# Patina Architecture Decisions

## 2025-07-15: Initial Architecture

### Core Architecture
1. **Brain as document store initially**
   - Start with markdown files
   - Move to SQLite later
   - Allows immediate progress

2. **Three-dimensional context model**
   - LLM (how we communicate)
   - Knowledge (what we know)
   - Environment (where we execute)

3. **Deployment by project type**
   - Apps → Docker
   - Tools → Native binaries
   - Libraries → crates.io

4. **Golden path: Claude + Dagger**
   - Start with what works
   - Architecture allows alternatives
   - Focus on excellence over options

### Key Principles Established
- Unix philosophy: one tool, one job
- LLM-agnostic brain storage
- Environment-agnostic knowledge
- Composable context hierarchy
- Projects can become topics