---
id: golden-path
version: 1
created_date: 2025-07-15
confidence: medium
oxidizer: nicabar
tags: []
---

# Golden Path: Claude + Dagger

## Overview
While Patina's architecture supports multiple LLMs and development environments, we're starting with one excellent implementation:

**Claude + Dagger for development → Docker for deployment**

## Why This Combination?

### Claude
- Excellent context handling (200k tokens)
- Strong instruction following
- MCP (Model Context Protocol) support
- Proven track record with development tasks
- Session commands already work well

### Dagger
- "CI/CD as code" aligns with our philosophy
- LLM-native: pipeline as code that Claude can understand
- Superior caching for fast iterations
- Guarantees reproducible builds
- Can build our Docker deployment images

### Docker (Deployment)
- Universal deployment target
- Works on Unraid, cloud, anywhere
- Simple mental model for users
- Industry standard

## Implementation Priority
1. Make Claude + Dagger excellent first
2. Ensure architecture allows alternatives
3. Add other options based on user need

## Command Structure
```bash
# Initial version (implicit Claude + Dagger)
patina init my-app

# Future versions (explicit choice)
patina init my-app --llm claude --dev-env dagger
```

## Architectural Preparation
- Use traits for LLM adapters
- Use traits for environment providers
- Keep core logic independent
- But optimize for Claude + Dagger initially