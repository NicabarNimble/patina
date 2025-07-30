---
id: versioning-strategy
version: 1
created_date: 2025-07-27
confidence: medium
oxidizer: nicabar
tags: []
promoted_from: projects/patina
---

# Git-Based Versioning Strategy for Patina

## Overview
Each tool integration (Claude, Dagger, Docker) is a separate, versioned component that evolves independently.

## Components

1. **patina-core** (`0.1.0`) - The orchestration engine
2. **patina-claude** (`0.3.0`) - Claude AI integration 
3. **patina-dagger** (`0.1.0`) - Dagger CI/CD integration
4. **patina-docker** (`0.1.0`) - Docker integration

Each component:
- Has its own version constant
- Manages its own files
- Can update independently
- Provides active features, not just templates

## Versioning

### Git Tags
- Core: `v0.1.0`
- Tools: `claude-v0.3.0`, `dagger-v0.1.0`, `docker-v0.1.0`

### Version Tracking
`.patina/versions.json` tracks what's installed:
```json
{
  "core": "0.1.0",
  "tools": {
    "claude": {"version": "0.3.0", "active": true},
    "dagger": {"version": "0.1.0", "active": true}
  }
}
```

## Updates

```bash
# Check for updates
patina update --check

# Update specific LLM
patina update --llm claude

# Update specific dev environment
patina update --dev dagger
patina update --dev docker

# Update everything
patina update
```

Tools can evolve independently - Claude can add new session features while Dagger improves caching, without requiring a full Patina rebuild.

## Key Insight

Tools are living integrations that grow smarter over time, not static templates. This architecture sets the foundation for a future plugin ecosystem.